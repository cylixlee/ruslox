use parser::Expression;
use shared::{
    chunk::{Chunk, Instruction},
    error::InterpretResult,
    value::Value,
};

mod parser;

pub fn compile(source: impl AsRef<str>) -> InterpretResult<Chunk> {
    let expression = parser::parse(source.as_ref())?;
    let mut chunk = Chunk::new();
    emit(&mut chunk, &expression)?;
    chunk.write(Instruction::Return);
    Ok(chunk)
}

fn emit(chunk: &mut Chunk, expression: &Expression) -> InterpretResult {
    macro_rules! emit_binary {
        ($left:expr, $right:expr, $inst:ident) => {{
            emit(chunk, &$left)?;
            emit(chunk, &$right)?;
            chunk.write(Instruction::$inst);
        }};
    }

    match expression {
        Expression::Number(number) => {
            let index = chunk.add_constant(Value::Number(*number))?;
            chunk.write(Instruction::Constant(index));
        }
        Expression::Negation(expression) => {
            emit(chunk, expression)?;
            chunk.write(Instruction::Negate);
        }
        Expression::Add(left, right) => emit_binary!(left, right, Add),
        Expression::Subtract(left, right) => emit_binary!(left, right, Subtract),
        Expression::Multiply(left, right) => emit_binary!(left, right, Multiply),
        Expression::Divide(left, right) => emit_binary!(left, right, Divide),

        // Unreachable.
        Expression::Error => unreachable!("error should be reported rather than emitted."),
    }
    Ok(())
}
