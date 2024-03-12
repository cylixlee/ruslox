use codespan_reporting::diagnostic::Diagnostic;
use parser::{Expression, Statement};
use scanner::Token;
use shared::{
    chunk::{Chunk, Instruction},
    constant::Constant,
};

mod parser;
mod scanner;

pub fn compile(file_id: usize, source: impl AsRef<str>) -> Result<Chunk, Vec<Diagnostic<usize>>> {
    let scanned = scanner::scan(file_id, source.as_ref());
    if !scanned.diagnostics.is_empty() {
        return Err(scanned.diagnostics);
    }
    let declarations = parser::parse(file_id, &scanned)?;
    let mut chunk = Chunk::new();
    for declaration in &declarations {
        if let Err(diagnostic) = emit_statement(&mut chunk, declaration) {
            return Err(vec![diagnostic]);
        }
    }
    Ok(chunk)
}

fn emit_statement(chunk: &mut Chunk, statement: &Statement) -> Result<(), Diagnostic<usize>> {
    match statement {
        Statement::VarDeclaration(name, initializer) => {
            let index = chunk.add_constant(Constant::String((*name).clone()))?;
            match initializer {
                Some(expression) => emit_expression(chunk, expression)?,
                None => chunk.write(Instruction::Nil),
            };
            chunk.write(Instruction::DefineGlobal(index));
        }
        Statement::Print(expression) => {
            emit_expression(chunk, expression)?;
            chunk.write(Instruction::Print);
        }
        Statement::Expressional(expression) => {
            emit_expression(chunk, expression)?;
            chunk.write(Instruction::Pop);
        }
        Statement::Error => unreachable!("still trying to emit after reporting diagnostics"),
    }
    Ok(())
}

fn emit_expression(chunk: &mut Chunk, expression: &Expression) -> Result<(), Diagnostic<usize>> {
    match expression {
        Expression::String(string) => emit_constant(chunk, Constant::String((*string).clone()))?,
        Expression::Number(number) => emit_constant(chunk, Constant::Number(*number))?,
        Expression::Identifier(identifier) => {
            let index = chunk.add_constant(Constant::String((*identifier).clone()))?;
            chunk.write(Instruction::GetGlobal(index));
        }
        Expression::True => chunk.write(Instruction::True),
        Expression::False => chunk.write(Instruction::False),
        Expression::Nil => chunk.write(Instruction::Nil),
        Expression::Unary(operator, expression) => {
            emit_expression(chunk, expression)?;
            match operator {
                Token::Minus => chunk.write(Instruction::Negate),
                Token::Bang => chunk.write(Instruction::Not),
                _ => unreachable!("emit failure due to parse error at unary expressions."),
            }
        }
        Expression::Binary(left, operator, right) => {
            if let Token::Equal = operator {
                match &**left {
                    Expression::Identifier(identifier) => {
                        let index = chunk.add_constant(Constant::String((*identifier).clone()))?;
                        emit_expression(chunk, &right)?;
                        chunk.write(Instruction::SetGlobal(index));
                    }
                    _ => {
                        return Err(Diagnostic::error()
                            .with_code("E0008")
                            .with_message("invalid assignment target"))
                    }
                }
            } else {
                emit_expression(chunk, &left)?;
                emit_expression(chunk, &right)?;
            }
        }
    }
    Ok(())
}

fn emit_constant(chunk: &mut Chunk, constant: Constant) -> Result<(), Diagnostic<usize>> {
    let index = chunk.add_constant(constant)?;
    chunk.write(Instruction::Constant(index));
    Ok(())
}
