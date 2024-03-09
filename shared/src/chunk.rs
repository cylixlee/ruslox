use crate::{
    error::{InterpretError, InterpretResult},
    value::Value,
};

pub enum Instruction {
    Constant(u8),
    Nil,
    True,
    False,
    Add,
    Subtract,
    Multiply,
    Divide,
    Negate,
    Return,
}

pub struct Chunk {
    pub code: Vec<Instruction>,
    pub constants: Vec<Value>,
}

impl Chunk {
    pub fn new() -> Self {
        Self {
            code: Vec::new(),
            constants: Vec::with_capacity(u8::MAX as usize + 1),
        }
    }

    pub fn write(&mut self, instruction: Instruction) {
        self.code.push(instruction);
    }

    pub fn add_constant(&mut self, value: Value) -> InterpretResult<u8> {
        if self.constants.len() >= u8::MAX as usize + 1 {
            return Err(InterpretError::CompileError(
                "Too many constants in one chunk.".into(),
                None,
            ));
        }
        self.constants.push(value);
        Ok((self.constants.len() - 1) as u8)
    }

    pub fn disassemble(&self, title: impl AsRef<str>) {
        println!("== {} ==", title.as_ref());
        for offset in 0..self.code.len() {
            self.disassemble_instruction(offset);
        }
    }

    pub fn disassemble_instruction(&self, offset: usize) {
        print!("{:04} ", offset);

        match &self.code[offset] {
            Instruction::Constant(constant_index) => {
                constant_instruction("CONST", constant_index, self)
            }
            Instruction::Nil => simple_instruction("NIL"),
            Instruction::True => simple_instruction("TRUE"),
            Instruction::False => simple_instruction("FALSE"),
            Instruction::Add => simple_instruction("ADD"),
            Instruction::Subtract => simple_instruction("SUB"),
            Instruction::Multiply => simple_instruction("MUL"),
            Instruction::Divide => simple_instruction("DIV"),
            Instruction::Negate => simple_instruction("NEG"),
            Instruction::Return => simple_instruction("RET"),
        }
    }
}

fn simple_instruction(name: impl AsRef<str>) {
    println!("{}", name.as_ref());
}

fn constant_instruction(name: impl AsRef<str>, constant_index: &u8, chunk: &Chunk) {
    println!(
        "{:<16} {:4} '{}'",
        name.as_ref(),
        constant_index,
        chunk.constants[*constant_index as usize]
    );
}
