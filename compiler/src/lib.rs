use parser::Expression;
use shared::{
    chunk::{Chunk, Instruction},
    constant::Constant,
    error::InterpretResult,
};

use crate::parser::Token;

mod parser;

pub fn compile(source: impl AsRef<str>) -> InterpretResult<Chunk> {
    let expression = parser::parse(source.as_ref())?;
    let mut chunk = Chunk::new();
    emit(&mut chunk, &expression)?;
    chunk.write(Instruction::Return);
    Ok(chunk)
}

fn emit(chunk: &mut Chunk, expression: &Expression) -> InterpretResult {
    match expression {
        Expression::Literal(literal) => match literal {
            Token::Number(n) => {
                let constant_index = chunk.add_constant(Constant::Number(*n))?;
                chunk.write(Instruction::Constant(constant_index));
            }
            Token::True => chunk.write(Instruction::True),
            Token::False => chunk.write(Instruction::False),
            Token::Nil => chunk.write(Instruction::Nil),
            _ => unreachable!("internal error when parsing literals."),
        },
        Expression::Unary(operator, expr) => {
            emit(chunk, &expr)?;
            match operator {
                Token::Minus => chunk.write(Instruction::Negate),
                Token::Bang => chunk.write(Instruction::Not),
                _ => unreachable!("internal error when parsing unary expressions."),
            }
        }
        Expression::Binary(left, operator, right) => {
            emit(chunk, &left)?;
            emit(chunk, &right)?;
            match operator {
                Token::Plus => chunk.write(Instruction::Add),
                Token::Minus => chunk.write(Instruction::Subtract),
                Token::Star => chunk.write(Instruction::Multiply),
                Token::Slash => chunk.write(Instruction::Divide),
                Token::Greater => chunk.write(Instruction::Greater),
                Token::Less => chunk.write(Instruction::Less),
                Token::EqualEqual => chunk.write(Instruction::Equal),
                Token::GreaterEqual => {
                    chunk.write(Instruction::Less);
                    chunk.write(Instruction::Not);
                }
                Token::LessEqual => {
                    chunk.write(Instruction::Greater);
                    chunk.write(Instruction::Not);
                }
                Token::NotEqual => {
                    chunk.write(Instruction::Equal);
                    chunk.write(Instruction::Not);
                }
                _ => unreachable!("internal error when parsing binary expressions."),
            }
        }
        Expression::Error => {
            unreachable!("error expressions should be reported rather than emitted.")
        }
    }
    Ok(())
}
