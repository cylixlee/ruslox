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

    use Expression::*;
    match expression {
        Number(number) => {
            let index = chunk.add_constant(Value::Number(*number))?;
            chunk.write(Instruction::Constant(index));
        }
        Boolean(boolean) => match boolean {
            true => chunk.write(Instruction::True),
            false => chunk.write(Instruction::False),
        },
        Nil => chunk.write(Instruction::Nil),
        Negation(expression) => {
            emit(chunk, expression)?;
            chunk.write(Instruction::Negate);
        }
        Not(expression) => {
            emit(chunk, expression)?;
            chunk.write(Instruction::Not);
        }
        Add(left, right) => emit_binary!(left, right, Add),
        Subtract(left, right) => emit_binary!(left, right, Subtract),
        Multiply(left, right) => emit_binary!(left, right, Multiply),
        Divide(left, right) => emit_binary!(left, right, Divide),

        Greater(left, right) => emit_binary!(left, right, Greater),
        Less(left, right) => emit_binary!(left, right, Less),
        GreaterEqual(left, right) => {
            emit_binary!(left, right, Less);
            chunk.write(Instruction::Not);
        }
        LessEqual(left, right) => {
            emit_binary!(left, right, Greater);
            chunk.write(Instruction::Not);
        }
        Equal(left, right) => emit_binary!(left, right, Equal),
        NotEqual(left, right) => {
            emit_binary!(left, right, Equal);
            chunk.write(Instruction::Not);
        }

        // Unreachable.
        Error => unreachable!("error should be reported rather than emitted."),
    }
    Ok(())
}
