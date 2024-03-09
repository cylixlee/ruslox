use shared::{
    chunk::{Chunk, Instruction},
    error::{InterpretError, InterpretResult},
    value::Value,
};

use self::stack::Stack;

mod stack;

const STACK_CAPACITY: usize = u8::MAX as usize + 1;

pub struct VirtualMachine {
    chunk: Option<Chunk>,
    offset: usize,
    stack: Stack<Value, STACK_CAPACITY>,
}

impl VirtualMachine {
    pub fn new() -> Self {
        Self {
            chunk: None,
            offset: 0,
            stack: Stack::new(),
        }
    }

    pub fn interpret(&mut self, chunk: Chunk) -> InterpretResult {
        self.chunk = Some(chunk);
        self.offset = 0;
        self.run()
    }

    pub fn clear_stack(&mut self) {
        self.stack.clear();
    }

    fn run(&mut self) -> InterpretResult {
        macro_rules! arithmetic {
            ($operator: tt) => {{
                let right = self.stack.pop()?;
                let left = self.stack.pop()?;

                match (left, right) {
                    (Value::Number(left), Value::Number(right)) => {
                        self.stack.push(Value::Number(left $operator right))?;
                    }

                    _ => {
                        return Err(InterpretError::RuntimeError(
                            "operand must be numbers.".into()
                        ));
                    }
                }
            }};
        }

        let chunk = self.chunk.as_ref().unwrap();

        loop {
            #[cfg(debug_assertions)]
            {
                print!("          ");
                for i in 0..self.stack.len() {
                    print!("[ {} ]", self.stack[i]);
                }
                println!();
                chunk.disassemble_instruction(self.offset);
            }

            match &chunk.code[self.offset] {
                Instruction::Constant(constant_index) => {
                    let value = chunk.constants[*constant_index as usize].clone();
                    self.stack.push(value)?;
                }
                Instruction::Nil => self.stack.push(Value::Nil)?,
                Instruction::True => self.stack.push(Value::Boolean(true))?,
                Instruction::False => self.stack.push(Value::Boolean(false))?,
                Instruction::Add => arithmetic!(+),
                Instruction::Subtract => arithmetic!(-),
                Instruction::Multiply => arithmetic!(*),
                Instruction::Divide => arithmetic!(/),
                Instruction::Negate => match self.stack.pop()? {
                    Value::Number(number) => self.stack.push(Value::Number(-number))?,
                    _ => {
                        return Err(InterpretError::RuntimeError(
                            "operand must be a number.".into(),
                        ))
                    }
                },
                Instruction::Return => {
                    println!("{}", self.stack.pop()?);
                    return Ok(());
                }
            }
            self.offset += 1;
        }
    }
}
