use std::ops::Range;

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
    locals: Stack<Local>,
    local_depth: usize,
}

impl<'a> Compiler<'a> {
    fn new(file_id: usize, parsed_context: &'a ParsedContext, chunk: &'a mut Chunk) -> Self {
        Self {
            file_id,
            parsed_context,
            chunk,
            locals: Stack::new(),
            local_depth: 0,
        }
    }

    fn compile(&mut self) -> InterpretResult {
        for (statement, position) in self
            .parsed_context
            .statements
            .iter()
            .zip(self.parsed_context.positions.iter())
        {
            self.emit_statement(statement, position)?;
        }
        Ok(())
    }

    fn emit_statement(
        &mut self,
        statement: &Statement,
        position: &Range<usize>,
    ) -> InterpretResult {
        match statement {
            Statement::VarDeclaration(name, initializer) => {
                match initializer {
                    Some(expression) => self.emit_expression(expression, position)?,
                    None => self.chunk.write(Instruction::Nil, position.clone()),
                };
                let index = self.emit_constant(Constant::String((*name).clone()), position)?;
                match self.local_depth {
                    0 => self
                        .chunk
                        .write(Instruction::DefineGlobal(index), position.clone()),
                    _ => {
                        self.locals.push(Local {
                            depth: self.local_depth,
                            name: (*name).clone(),
                        })?;
                    }
                }
            }
            Statement::Print(expression) => {
                self.emit_expression(expression, position)?;
                self.chunk.write(Instruction::Print, position.clone());
            }
            Statement::If(condition, then, otherwise) => {
                self.emit_expression(&condition, position)?;
                // mark patch BEFORE the jump instruction.
                let then_patch = self.chunk.code.len();
                self.chunk
                    .write(Instruction::JumpFalse(u16::MAX), position.clone());
                // if we record patch here, we'll let patch=len - 1. this subtraction is unnecessary.
                self.chunk.write(Instruction::Pop, position.clone());
                self.emit_statement(&then, position)?;
                let otherwise_patch = self.chunk.code.len();
                self.chunk
                    .write(Instruction::Jump(u16::MAX), position.clone());
                // backpatch BEFORE the destination
                self.chunk.backpatch(
                    then_patch,
                    Instruction::JumpFalse((self.chunk.code.len() - then_patch) as u16),
                );
                self.chunk.write(Instruction::Pop, position.clone());
                // jump actually does vm.offset += offset. if we backpatch here, well use offset - 1 to skip the backpatched jump.
                // this subtraction is unnecessary.
                if let Some(otherwise) = otherwise {
                    self.emit_statement(&otherwise, position)?;
                }
                self.chunk.backpatch(
                    otherwise_patch,
                    Instruction::Jump((self.chunk.code.len() - otherwise_patch) as u16),
                );
            }
            Statement::While(condition, body) => {
                let loop_patch = self.chunk.code.len();
                self.emit_expression(condition, position)?;
                let condition_patch = self.chunk.code.len();
                self.chunk
                    .write(Instruction::JumpFalse(u16::MAX), position.clone());
                self.chunk.write(Instruction::Pop, position.clone());
                self.emit_statement(body, position)?;
                self.chunk.write(
                    Instruction::Loop((self.chunk.code.len() - loop_patch) as u16),
                    position.clone(),
                );
                self.chunk.backpatch(
                    condition_patch,
                    Instruction::JumpFalse((self.chunk.code.len() - condition_patch) as u16),
                );
                self.chunk.write(Instruction::Pop, position.clone());
            }
            Statement::For(init, condition, inc, body) => {
                self.local_depth += 1;
                if let Some(init) = init {
                    self.emit_expression(init, position)?;
                }
                let condition_patch = self.chunk.code.len();
                if let Some(condition) = condition {
                    self.emit_expression(condition, position)?;
                }
                let break_patch = self.chunk.code.len();
                self.chunk
                    .write(Instruction::JumpFalse(u16::MAX), position.clone());
                self.chunk.write(Instruction::Pop, position.clone());
                let body_patch = self.chunk.code.len();
                self.chunk
                    .write(Instruction::Jump(u16::MAX), position.clone());
                let inc_patch = self.chunk.code.len();
                if let Some(inc) = inc {
                    self.emit_expression(inc, position)?;
                }
                self.chunk.write(Instruction::Pop, position.clone());
                self.chunk.write(
                    Instruction::Loop((self.chunk.code.len() - condition_patch) as u16),
                    position.clone(),
                );
                self.chunk.backpatch(
                    body_patch,
                    Instruction::Jump((self.chunk.code.len() - body_patch) as u16),
                );
                self.emit_statement(body, position)?;
                self.chunk.write(
                    Instruction::Loop((self.chunk.code.len() - inc_patch) as u16),
                    position.clone(),
                );
                self.chunk.backpatch(
                    break_patch,
                    Instruction::JumpFalse((self.chunk.code.len() - break_patch) as u16),
                );
                self.chunk.write(Instruction::Pop, position.clone());
                self.local_depth -= 1;
            }
            Statement::ForWithInit(init, condition, inc, body) => {
                self.local_depth += 1;
                if let Some(init) = init {
                    self.emit_statement(init, position)?;
                }
                let condition_patch = self.chunk.code.len();
                if let Some(condition) = condition {
                    self.emit_expression(condition, position)?;
                }
                let break_patch = self.chunk.code.len();
                self.chunk
                    .write(Instruction::JumpFalse(u16::MAX), position.clone());
                self.chunk.write(Instruction::Pop, position.clone());
                let body_patch = self.chunk.code.len();
                self.chunk
                    .write(Instruction::Jump(u16::MAX), position.clone());
                let inc_patch = self.chunk.code.len();
                if let Some(inc) = inc {
                    self.emit_expression(inc, position)?;
                }
                self.chunk.write(Instruction::Pop, position.clone());
                self.chunk.write(
                    Instruction::Loop((self.chunk.code.len() - condition_patch) as u16),
                    position.clone(),
                );
                self.chunk.backpatch(
                    body_patch,
                    Instruction::Jump((self.chunk.code.len() - body_patch) as u16),
                );
                self.emit_statement(body, position)?;
                self.chunk.write(
                    Instruction::Loop((self.chunk.code.len() - inc_patch) as u16),
                    position.clone(),
                );
                self.chunk.backpatch(
                    break_patch,
                    Instruction::JumpFalse((self.chunk.code.len() - break_patch) as u16),
                );
                self.chunk.write(Instruction::Pop, position.clone());
                self.local_depth -= 1;
            }
            Statement::Block(statements, positions) => {
                self.local_depth += 1;
                for (statement, position) in statements.iter().zip(positions) {
                    self.emit_statement(statement, position)?;
                }
                while let Some(local) = self.locals.peek() {
                    if local.depth == self.local_depth {
                        self.chunk.write(Instruction::Pop, position.clone());
                        self.locals.pop()?;
                    } else {
                        break;
                    }
                }
                self.local_depth -= 1;
            }
            Statement::Expressional(expression) => {
                self.emit_expression(expression, position)?;
                self.chunk.write(Instruction::Pop, position.clone());
            }
            // Unreachable
            Statement::Error => unreachable!("still trying to emit after reporting diagnostics"),
        }
        Ok(())
    }

    fn emit_expression(
        &mut self,
        expression: &Expression,
        position: &Range<usize>,
    ) -> InterpretResult {
        match expression {
            Expression::String(string) => {
                let index = self.emit_constant(Constant::String((*string).clone()), position)?;
                self.chunk
                    .write(Instruction::Constant(index), position.clone());
            }
            Expression::Number(number) => {
                let index = self.emit_constant(Constant::Number(*number), position)?;
                self.chunk
                    .write(Instruction::Constant(index), position.clone());
            }
            Expression::Identifier(identifier) => {
                let index =
                    self.emit_constant(Constant::String((*identifier).clone()), position)?;
                let mut is_local = false;
                for slot in (0..self.locals.len()).rev() {
                    let local = &self.locals[slot];
                    if local.name == **identifier {
                        self.chunk
                            .write(Instruction::GetLocal(slot as u8), position.clone());
                        is_local = true;
                        break;
                    }
                }
                if !is_local {
                    self.chunk
                        .write(Instruction::GetGlobal(index), position.clone());
                }
            }
            Expression::True => self.chunk.write(Instruction::True, position.clone()),
            Expression::False => self.chunk.write(Instruction::False, position.clone()),
            Expression::Nil => self.chunk.write(Instruction::Nil, position.clone()),
            Expression::Unary(operator, expression) => {
                self.emit_expression(expression, position)?;
                match operator {
                    Token::Minus => self.chunk.write(Instruction::Negate, position.clone()),
                    Token::Bang => self.chunk.write(Instruction::Not, position.clone()),
                    _ => unreachable!("emit failure due to parse error at unary expressions."),
                }
            }
            Expression::Assign(target, source) => match &**target {
                Expression::Identifier(identifier) => {
                    let index =
                        self.emit_constant(Constant::String((*identifier).clone()), position)?;
                    self.emit_expression(&source, position)?;
                    let mut is_local = false;
                    for slot in (0..self.locals.len()).rev() {
                        let local = &self.locals[slot];
                        if local.name == **identifier {
                            self.chunk
                                .write(Instruction::SetLocal(slot as u8), position.clone());
                            is_local = true;
                            break;
                        }
                    }
                    if !is_local {
                        self.chunk
                            .write(Instruction::SetGlobal(index), position.clone());
                    }
                }
                _ => {
                    return self.report(
                        position,
                        "E0008",
                        "invalid assignment target",
                        "assignment within this statement",
                    )
                }
            },
            Expression::Arithmetic(left, operator, right) => {
                self.emit_expression(&left, position)?;
                self.emit_expression(&right, position)?;
                match operator {
                    Token::Plus => self.chunk.write(Instruction::Add, position.clone()),
                    Token::Minus => self.chunk.write(Instruction::Subtract, position.clone()),
                    Token::Star => self.chunk.write(Instruction::Multiply, position.clone()),
                    Token::Slash => self.chunk.write(Instruction::Multiply, position.clone()),
                    Token::Greater => self.chunk.write(Instruction::Greater, position.clone()),
                    Token::Less => self.chunk.write(Instruction::Less, position.clone()),
                    Token::EqualEqual => self.chunk.write(Instruction::Equal, position.clone()),
                    Token::GreaterEqual => {
                        self.chunk.write(Instruction::Less, position.clone());
                        self.chunk.write(Instruction::Not, position.clone());
                    }
                    Token::LessEqual => {
                        self.chunk.write(Instruction::Greater, position.clone());
                        self.chunk.write(Instruction::Not, position.clone());
                    }
                    Token::BangEqual => {
                        self.chunk.write(Instruction::Equal, position.clone());
                        self.chunk.write(Instruction::Not, position.clone());
                    }
                    _ => unreachable!("emit failure due to parse error at binary expressions."),
                }
            }
            Expression::Logic(left, operator, right) => match operator {
                Token::And => {
                    self.emit_expression(left, position)?;
                    let patch = self.chunk.code.len();
                    self.chunk
                        .write(Instruction::JumpFalse(u16::MAX), position.clone());
                    self.chunk.write(Instruction::Pop, position.clone());
                    self.emit_expression(right, position)?;
                    self.chunk.backpatch(
                        patch,
                        Instruction::JumpFalse((self.chunk.code.len() - patch) as u16),
                    );
                }
                Token::Or => {
                    self.emit_expression(left, position)?;
                    let false_patch = self.chunk.code.len();
                    self.chunk
                        .write(Instruction::JumpFalse(u16::MAX), position.clone());
                    let patch = self.chunk.code.len();
                    self.chunk
                        .write(Instruction::Jump(u16::MAX), position.clone());
                    self.chunk.backpatch(
                        false_patch,
                        Instruction::JumpFalse((self.chunk.code.len() - false_patch) as u16),
                    );
                    self.chunk.write(Instruction::Pop, position.clone());
                    self.emit_expression(right, position)?;
                    self.chunk.backpatch(
                        patch,
                        Instruction::Jump((self.chunk.code.len() - patch) as u16),
                    );
                }
                _ => unreachable!("emit failure due to parse error at logic expressions."),
            },
        }
        Ok(())
    }

    fn emit_constant(
        &mut self,
        constant: Constant,
        position: &Range<usize>,
    ) -> InterpretResult<u8> {
        let index = match self.chunk.add_constant(constant) {
            Some(index) => index,
            None => {
                return Err(InterpretError::Simple(
                    ErrorItem::error()
                        .with_code("E0001")
                        .with_message("too many constants in one chunk")
                        .with_labels(vec![Label::secondary(self.file_id, position.clone())
                            .with_message("error originated within this statement")]),
                ))
            }
        };
        Ok(index)
    }

    #[inline(always)]
    fn report(
        &self,
        position: &Range<usize>,
        code: impl Into<String>,
        message: impl Into<String>,
        label: impl Into<String>,
    ) -> InterpretResult {
        Err(InterpretError::Simple(
            ErrorItem::error()
                .with_code(code)
                .with_message(message)
                .with_labels(vec![
                    Label::secondary(self.file_id, position.clone()).with_message(label)
                ]),
        ))
    }
}

pub fn compile(file_id: usize, source: impl AsRef<str>) -> InterpretResult<Chunk> {
    let scanned = scanner::scan(file_id, source.as_ref())?;
    let parsed = parser::parse(file_id, &scanned)?;
    let mut chunk = Chunk::new(file_id);
    Compiler::new(file_id, &parsed, &mut chunk).compile()?;
    chunk.write(Instruction::Return, 0..0);
    Ok(chunk)
}
