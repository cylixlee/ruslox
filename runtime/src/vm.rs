use codespan_reporting::diagnostic::Diagnostic;
use shared::{
    chunk::{Chunk, Instruction},
    constant::Constant,
};

use crate::{
    object::{Downcast, FromUnmanaged, ManagedReference, ObjectType, StringObject},
    value::Value,
};

use self::{heap::Heap, stack::Stack};

mod heap;
mod stack;

const STACK_CAPACITY: usize = u8::MAX as usize + 1;

pub struct VirtualMachine {
    chunk: Option<Chunk>,
    offset: usize,
    stack: Stack<Value, STACK_CAPACITY>,
    pub heap: Heap,
}

impl VirtualMachine {
    pub fn new() -> Self {
        Self {
            chunk: None,
            offset: 0,
            stack: Stack::new(),
            heap: Heap::new(),
        }
    }

    pub fn interpret(&mut self, chunk: Chunk) -> Result<(), Diagnostic<usize>> {
        self.chunk = Some(chunk);
        self.offset = 0;
        self.run()
    }

    pub fn clear_stack(&mut self) {
        self.stack.clear();
    }

    fn run(&mut self) -> Result<(), Diagnostic<usize>> {
        macro_rules! arithmetic {
            ($operator:tt, $typ:ident) => {{
                let right = self.stack.pop()?;
                let left = self.stack.pop()?;

                match (left, right) {
                    (Value::Number(left), Value::Number(right)) => {
                        self.stack.push(Value::$typ(left $operator right))?;
                    }

                    _ => {
                        return Err(Diagnostic::error()
                            .with_code("E1003")
                            .with_message("operands must be numbers"));
                    }
                }
            }};
        }

        #[rustfmt::skip] macro_rules! arithmetic_calc {
            ($operator:tt) => { arithmetic!($operator, Number) };
        }

        #[rustfmt::skip] macro_rules! arithmetic_cmp {
            ($operator:tt) => { arithmetic!($operator, Boolean) };
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
                // Instructions with operand.
                Instruction::Constant(constant_index) => {
                    let constant = chunk.constants[*constant_index as usize].clone();
                    match constant {
                        Constant::Number(number) => self.stack.push(Value::Number(number))?,
                        Constant::String(string) => {
                            self.stack
                                .push(Value::Object(ManagedReference::from_unmanaged(
                                    string,
                                    &mut self.heap,
                                )))?
                        }
                    }
                }

                // Literal instructions.
                Instruction::Nil => self.stack.push(Value::Nil)?,
                Instruction::True => self.stack.push(Value::Boolean(true))?,
                Instruction::False => self.stack.push(Value::Boolean(false))?,

                // Arithmetic instructions.
                Instruction::Add => {
                    let right = self.stack.pop()?;
                    let left = self.stack.pop()?;
                    match (left, right) {
                        (Value::Number(left), Value::Number(right)) => {
                            self.stack.push(Value::Number(left + right))?
                        }
                        (Value::Object(left), Value::Object(right)) => {
                            match (left.typ, right.typ) {
                                (ObjectType::String, ObjectType::String) => {
                                    let left: &StringObject = left.downcast().unwrap();
                                    let right: &StringObject = right.downcast().unwrap();
                                    let concat = format!("{}{}", left, right);
                                    self.stack.push(Value::Object(
                                        ManagedReference::from_unmanaged(concat, &mut self.heap),
                                    ))?;
                                }
                            }
                        }
                        _ => {
                            return Err(Diagnostic::error().with_code("E1005").with_message(
                                "concatenation operands must be both numbers or both strings.",
                            ));
                        }
                    }
                }
                Instruction::Subtract => arithmetic_calc!(-),
                Instruction::Multiply => arithmetic_calc!(*),
                Instruction::Divide => arithmetic_calc!(/),
                Instruction::Negate => match self.stack.pop()? {
                    Value::Number(number) => self.stack.push(Value::Number(-number))?,
                    _ => {
                        return Err(Diagnostic::error()
                            .with_code("E1004")
                            .with_message("operand must be number"));
                    }
                },

                // Logic instructions.
                Instruction::Not => match self.stack.pop()? {
                    // Regular unary-not operation on booleans.
                    Value::Boolean(boolean) => self.stack.push(Value::Boolean(!boolean))?,
                    // Nil is falsy.
                    Value::Nil => self.stack.push(Value::Boolean(true))?,
                    // Other value is implicit converted to true.
                    _ => self.stack.push(Value::Boolean(false))?,
                },
                Instruction::Equal => {
                    let right = self.stack.pop()?;
                    let left = self.stack.pop()?;
                    self.stack.push(Value::Boolean(left == right))?;
                }
                Instruction::Greater => arithmetic_cmp!(>),
                Instruction::Less => arithmetic_cmp!(<),

                // Miscellaneous.
                Instruction::Return => {
                    println!("{}", self.stack.pop()?);
                    return Ok(());
                }
            }
            self.offset += 1;
        }
    }
}
