use parser::{Expression, ParsedContext, Statement};
use scanner::Token;
use shared::{
    chunk::{Chunk, Instruction},
    constant::Constant,
    error::{ErrorItem, InterpretError, InterpretResult, Label},
    stack::Stack,
};

mod parser;
mod scanner;

struct Local {
    depth: usize,
    name: String,
}

struct Compiler<'a> {
    file_id: usize,
    parsed_context: &'a ParsedContext<'a>,
    chunk: &'a mut Chunk,
    current_statement: usize,
    locals: Stack<Local>,
    local_depth: usize,
}

impl<'a> Compiler<'a> {
    fn new(file_id: usize, parsed_context: &'a ParsedContext, chunk: &'a mut Chunk) -> Self {
        Self {
            file_id,
            parsed_context,
            chunk,
            current_statement: 0,
            locals: Stack::new(),
            local_depth: 0,
        }
    }

    fn compile(&mut self) -> InterpretResult {
        for statement in &self.parsed_context.statements {
            self.emit_statement(statement)?;
            self.current_statement += 1;
        }
        Ok(())
    }

    fn emit_statement(&mut self, statement: &Statement) -> InterpretResult {
        match statement {
            Statement::VarDeclaration(name, initializer) => {
                match initializer {
                    Some(expression) => self.emit_expression(expression)?,
                    None => self.emit(Instruction::Nil),
                };
                let index = self.emit_constant(Constant::String((*name).clone()))?;
                match self.local_depth {
                    0 => self.emit(Instruction::DefineGlobal(index)),
                    _ => {
                        self.locals.push(Local {
                            depth: self.local_depth,
                            name: (*name).clone(),
                        })?;
                    }
                }
            }
            Statement::Print(expression) => {
                self.emit_expression(expression)?;
                self.emit(Instruction::Print);
            }
            Statement::Expressional(expression) => {
                self.emit_expression(expression)?;
                self.emit(Instruction::Pop);
            }
            Statement::Error => unreachable!("still trying to emit after reporting diagnostics"),
            Statement::Block(statements) => {
                self.local_depth += 1;
                for statement in statements {
                    self.emit_statement(statement)?;
                }
                while let Some(local) = self.locals.peek() {
                    if local.depth == self.local_depth {
                        self.emit(Instruction::Pop);
                        self.locals.pop()?;
                    }
                }
                self.local_depth -= 1;
            }
        }
        Ok(())
    }

    fn emit_expression(&mut self, expression: &Expression) -> InterpretResult {
        match expression {
            Expression::String(string) => {
                let index = self.emit_constant(Constant::String((*string).clone()))?;
                self.emit(Instruction::Constant(index));
            }
            Expression::Number(number) => {
                let index = self.emit_constant(Constant::Number(*number))?;
                self.emit(Instruction::Constant(index));
            }
            Expression::Identifier(identifier) => {
                let index = self.emit_constant(Constant::String((*identifier).clone()))?;
                let mut is_local = false;
                for slot in (0..self.locals.len()).rev() {
                    let local = &self.locals[slot];
                    if local.name == **identifier {
                        self.emit(Instruction::GetLocal(slot as u8));
                        is_local = true;
                        break;
                    }
                }
                if !is_local {
                    self.emit(Instruction::GetGlobal(index));
                }
            }
            Expression::True => self.emit(Instruction::True),
            Expression::False => self.emit(Instruction::False),
            Expression::Nil => self.emit(Instruction::Nil),
            Expression::Unary(operator, expression) => {
                self.emit_expression(expression)?;
                match operator {
                    Token::Minus => self.emit(Instruction::Negate),
                    Token::Bang => self.emit(Instruction::Not),
                    _ => unreachable!("emit failure due to parse error at unary expressions."),
                }
            }
            Expression::Binary(left, operator, right) => {
                if let Token::Equal = operator {
                    match &**left {
                        Expression::Identifier(identifier) => {
                            let index =
                                self.emit_constant(Constant::String((*identifier).clone()))?;
                            self.emit_expression(&right)?;
                            let mut is_local = false;
                            for slot in (0..self.locals.len()).rev() {
                                let local = &self.locals[slot];
                                if local.name == **identifier {
                                    self.emit(Instruction::SetLocal(slot as u8));
                                    is_local = true;
                                    break;
                                }
                            }
                            if !is_local {
                                self.emit(Instruction::SetGlobal(index));
                            }
                        }
                        _ => {
                            return self.report(
                                "E0008",
                                "invalid assignment target",
                                "assignment within this statement",
                            )
                        }
                    }
                } else {
                    self.emit_expression(&left)?;
                    self.emit_expression(&right)?;
                    match operator {
                        Token::Plus => self.emit(Instruction::Add),
                        Token::Minus => self.emit(Instruction::Subtract),
                        Token::Star => self.emit(Instruction::Multiply),
                        Token::Slash => self.emit(Instruction::Multiply),
                        Token::Greater => self.emit(Instruction::Greater),
                        Token::Less => self.emit(Instruction::Less),
                        Token::EqualEqual => self.emit(Instruction::Equal),
                        Token::GreaterEqual => {
                            self.emit(Instruction::Less);
                            self.emit(Instruction::Not);
                        }
                        Token::LessEqual => {
                            self.emit(Instruction::Greater);
                            self.emit(Instruction::Not);
                        }
                        Token::BangEqual => {
                            self.emit(Instruction::Equal);
                            self.emit(Instruction::Not);
                        }
                        _ => unreachable!("emit failure due to parse error at binary expressions."),
                    }
                }
            }
        }
        Ok(())
    }

    fn emit_constant(&mut self, constant: Constant) -> InterpretResult<u8> {
        let index = match self.chunk.add_constant(constant) {
            Some(index) => index,
            None => {
                return self
                    .report(
                        "E0001",
                        "too many constants in one chunk",
                        "error originated within this statement",
                    )
                    .map(|_| Default::default());
            }
        };
        Ok(index)
    }

    fn emit(&mut self, instruction: Instruction) {
        self.chunk.write(
            instruction,
            self.parsed_context.positions[self.current_statement].clone(),
        );
    }

    #[inline(always)]
    fn report(
        &self,
        code: impl Into<String>,
        message: impl Into<String>,
        label: impl Into<String>,
    ) -> InterpretResult {
        Err(InterpretError::Simple(
            ErrorItem::error()
                .with_code(code)
                .with_message(message)
                .with_labels(vec![Label::secondary(
                    self.file_id,
                    self.parsed_context.positions[self.current_statement].clone(),
                )
                .with_message(label)]),
        ))
    }
}

pub fn compile(file_id: usize, source: impl AsRef<str>) -> InterpretResult<Chunk> {
    let scanned = scanner::scan(file_id, source.as_ref())?;
    let parsed = parser::parse(file_id, &scanned)?;
    let mut chunk = Chunk::new(file_id);
    Compiler::new(file_id, &parsed, &mut chunk).compile()?;
    chunk.write(Instruction::Return, chunk.positions.last().unwrap().clone());
    Ok(chunk)
}
