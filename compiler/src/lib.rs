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
                    None => self.chunk.write(Instruction::Nil, position),
                };
                let index = self.emit_constant(Constant::String((*name).clone()), position)?;
                match self.local_depth {
                    0 => self.chunk.write(Instruction::DefineGlobal(index), position),
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
                self.chunk.write(Instruction::Print, position);
            }
            Statement::If(condition, then, otherwise) => {
                self.emit_expression(&condition, position)?;
                // spare the patch.
                let then_patch = self.chunk.spare_jumpfalse(position);
                self.chunk.write(Instruction::Pop, position);
                self.emit_statement(&then, position)?;
                let otherwise_patch = self.chunk.spare_jump(position);
                // backpatch BEFORE the destination
                self.chunk.patch(then_patch);
                self.chunk.write(Instruction::Pop, position);
                if let Some(otherwise) = otherwise {
                    self.emit_statement(&otherwise, position)?;
                }
                self.chunk.patch(otherwise_patch);
            }
            Statement::While(condition, body) => {
                let loop_patch = self.chunk.code.len();
                self.emit_expression(condition, position)?;
                let condition_patch = self.chunk.spare_jumpfalse(position);
                self.chunk.write(Instruction::Pop, position);
                self.emit_statement(body, position)?;
                self.chunk.write(
                    Instruction::Loop((self.chunk.code.len() - loop_patch) as u16),
                    position,
                );
                self.chunk.patch(condition_patch);
                self.chunk.write(Instruction::Pop, position);
            }
            Statement::For(init, condition, inc, body) => {
                self.local_depth += 1;
                if let Some(init) = init {
                    self.emit_expression(init, position)?;
                }
                let condition_forwardpatch = self.chunk.code.len();
                if let Some(condition) = condition {
                    self.emit_expression(condition, position)?;
                }
                let break_backpatch = self.chunk.spare_jumpfalse(position);
                self.chunk.write(Instruction::Pop, position);
                let body_backpatch = self.chunk.spare_jump(position);
                let inc_forwardpatch = self.chunk.code.len();
                if let Some(inc) = inc {
                    self.emit_expression(inc, position)?;
                }
                self.chunk.write(Instruction::Pop, position);
                self.chunk.write(
                    Instruction::Loop((self.chunk.code.len() - condition_forwardpatch) as u16),
                    position,
                );
                self.chunk.patch(body_backpatch);
                self.emit_statement(body, position)?;
                self.chunk.write(
                    Instruction::Loop((self.chunk.code.len() - inc_forwardpatch) as u16),
                    position,
                );
                self.chunk.patch(break_backpatch);
                self.chunk.write(Instruction::Pop, position);
                self.local_depth -= 1;
            }
            Statement::ForWithInit(init, condition, inc, body) => {
                self.local_depth += 1;
                if let Some(init) = init {
                    self.emit_statement(init, position)?;
                }
                let condition_forwardpatch = self.chunk.code.len();
                if let Some(condition) = condition {
                    self.emit_expression(condition, position)?;
                }
                let break_backpatch = self.chunk.spare_jumpfalse(position);
                self.chunk.write(Instruction::Pop, position);
                let body_backpatch = self.chunk.spare_jump(position);
                let inc_forwardpatch = self.chunk.code.len();
                if let Some(inc) = inc {
                    self.emit_expression(inc, position)?;
                }
                self.chunk.write(Instruction::Pop, position);
                self.chunk.write(
                    Instruction::Loop((self.chunk.code.len() - condition_forwardpatch) as u16),
                    position,
                );
                self.chunk.patch(body_backpatch);
                self.emit_statement(body, position)?;
                self.chunk.write(
                    Instruction::Loop((self.chunk.code.len() - inc_forwardpatch) as u16),
                    position,
                );
                self.chunk.patch(break_backpatch);
                self.chunk.write(Instruction::Pop, position);
                self.local_depth -= 1;
            }
            Statement::Block(statements, positions) => {
                self.local_depth += 1;
                for (statement, position) in statements.iter().zip(positions) {
                    self.emit_statement(statement, position)?;
                }
                while let Some(local) = self.locals.peek() {
                    if local.depth == self.local_depth {
                        self.chunk.write(Instruction::Pop, position);
                        self.locals.pop()?;
                    } else {
                        break;
                    }
                }
                self.local_depth -= 1;
            }
            Statement::Expressional(expression) => {
                self.emit_expression(expression, position)?;
                self.chunk.write(Instruction::Pop, position);
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
                self.chunk.write(Instruction::Constant(index), position);
            }
            Expression::Number(number) => {
                let index = self.emit_constant(Constant::Number(*number), position)?;
                self.chunk.write(Instruction::Constant(index), position);
            }
            Expression::Identifier(identifier) => {
                let index =
                    self.emit_constant(Constant::String((*identifier).clone()), position)?;
                let mut is_local = false;
                for slot in (0..self.locals.len()).rev() {
                    let local = &self.locals[slot];
                    if local.name == **identifier {
                        self.chunk
                            .write(Instruction::GetLocal(slot as u8), position);
                        is_local = true;
                        break;
                    }
                }
                if !is_local {
                    self.chunk.write(Instruction::GetGlobal(index), position);
                }
            }
            Expression::True => self.chunk.write(Instruction::True, position),
            Expression::False => self.chunk.write(Instruction::False, position),
            Expression::Nil => self.chunk.write(Instruction::Nil, position),
            Expression::Unary(operator, expression) => {
                self.emit_expression(expression, position)?;
                match operator {
                    Token::Minus => self.chunk.write(Instruction::Negate, position),
                    Token::Bang => self.chunk.write(Instruction::Not, position),
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
                                .write(Instruction::SetLocal(slot as u8), position);
                            is_local = true;
                            break;
                        }
                    }
                    if !is_local {
                        self.chunk.write(Instruction::SetGlobal(index), position);
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
                    Token::Plus => self.chunk.write(Instruction::Add, position),
                    Token::Minus => self.chunk.write(Instruction::Subtract, position),
                    Token::Star => self.chunk.write(Instruction::Multiply, position),
                    Token::Slash => self.chunk.write(Instruction::Multiply, position),
                    Token::Greater => self.chunk.write(Instruction::Greater, position),
                    Token::Less => self.chunk.write(Instruction::Less, position),
                    Token::EqualEqual => self.chunk.write(Instruction::Equal, position),
                    Token::GreaterEqual => {
                        self.chunk.write(Instruction::Less, position);
                        self.chunk.write(Instruction::Not, position);
                    }
                    Token::LessEqual => {
                        self.chunk.write(Instruction::Greater, position);
                        self.chunk.write(Instruction::Not, position);
                    }
                    Token::BangEqual => {
                        self.chunk.write(Instruction::Equal, position);
                        self.chunk.write(Instruction::Not, position);
                    }
                    _ => unreachable!("emit failure due to parse error at binary expressions."),
                }
            }
            Expression::Logic(left, operator, right) => match operator {
                Token::And => {
                    self.emit_expression(left, position)?;
                    let patch = self.chunk.spare_jumpfalse(position);
                    self.chunk.write(Instruction::Pop, position);
                    self.emit_expression(right, position)?;
                    self.chunk.patch(patch);
                }
                Token::Or => {
                    self.emit_expression(left, position)?;
                    let false_patch = self.chunk.spare_jumpfalse(position);
                    let patch = self.chunk.spare_jump(position);
                    self.chunk.patch(false_patch);
                    self.chunk.write(Instruction::Pop, position);
                    self.emit_expression(right, position)?;
                    self.chunk.patch(patch);
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
    chunk.write(Instruction::Return, &(0..0));
    Ok(chunk)
}
