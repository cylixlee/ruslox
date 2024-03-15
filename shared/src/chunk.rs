use std::{fmt::Display, ops::Range};

use crate::constant::Constant;

#[rustfmt::skip]
pub enum Instruction {
    // Instructions with operand.
    Constant(u8), DefineGlobal(u8), GetGlobal(u8), SetGlobal(u8),
    GetLocal(u8), SetLocal(u8), JumpFalse(u16), Jump(u16), Loop(u16),

    // Literal instructions.
    Nil, True, False,

    // Arithmetic instructions.
    Add, Subtract, Multiply, Divide, Negate,

    // Logic instructions.
    Not, Equal, Greater, Less,

    // Miscellaneous.
    Return, Print, Pop,
}

pub struct Chunk {
    pub file_id: usize,
    pub code: Vec<Instruction>,
    pub positions: Vec<Range<usize>>,
    pub constants: Vec<Constant>,
}

macro_rules! register_backpatch {
    ($($instruction:ident), *) => {
        paste::paste! {
            $(
                pub fn [<spare_ $instruction:lower>](&mut self, position: &Range<usize>) -> usize {
                    let len = self.code.len();
                    self.write(Instruction::$instruction(len as u16), position);
                    len
                }
            )*

            pub fn patch(&mut self, offset: usize) {
                let len = self.code.len() as u16;
                match &mut self.code[offset] {
                    $(
                        Instruction::$instruction(offset) => *offset = len - *offset,
                    )*
                    _ => unreachable!("internal error when backpatch"),
                }
            }
        }
    };
}

impl Chunk {
    register_backpatch!(JumpFalse, Jump, Loop);

    pub fn new(file_id: usize) -> Self {
        Self {
            file_id,
            code: Vec::new(),
            positions: Vec::new(),
            constants: Vec::with_capacity(u8::MAX as usize + 1),
        }
    }

    pub fn write(&mut self, instruction: Instruction, position: &Range<usize>) {
        self.code.push(instruction);
        self.positions.push(position.clone());
    }

    pub fn add_constant(&mut self, value: Constant) -> Option<u8> {
        if self.constants.len() >= u8::MAX as usize + 1 {
            return None;
        }
        self.constants.push(value);
        Some((self.constants.len() - 1) as u8)
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
            // Instructions with operand.
            Instruction::Constant(constant_index) => {
                constant_instruction("CONST", constant_index, self)
            }
            Instruction::DefineGlobal(index) => constant_instruction("DEFINEGLOBAL", index, self),
            Instruction::GetGlobal(index) => constant_instruction("GETGLOBAL", index, self),
            Instruction::SetGlobal(index) => constant_instruction("SETGLOBAL", index, self),
            Instruction::GetLocal(index) => offset_instruction("GETLOCAL", index),
            Instruction::SetLocal(index) => offset_instruction("SETLOCAL", index),
            Instruction::JumpFalse(offset) => offset_instruction("JMPFALSE", offset),
            Instruction::Jump(offset) => offset_instruction("JUMP", offset),
            Instruction::Loop(offset) => offset_instruction("LOOP", offset),

            // Literal instructions.
            Instruction::Nil => simple_instruction("NIL"),
            Instruction::True => simple_instruction("TRUE"),
            Instruction::False => simple_instruction("FALSE"),

            // Arithmetic instructions.
            Instruction::Add => simple_instruction("ADD"),
            Instruction::Subtract => simple_instruction("SUB"),
            Instruction::Multiply => simple_instruction("MUL"),
            Instruction::Divide => simple_instruction("DIV"),
            Instruction::Negate => simple_instruction("NEG"),

            // Logic instructions.
            Instruction::Not => simple_instruction("NOT"),
            Instruction::Equal => simple_instruction("EQUAL"),
            Instruction::Greater => simple_instruction("GREATER"),
            Instruction::Less => simple_instruction("LESS"),

            // Miscellaneous.
            Instruction::Return => simple_instruction("RET"),
            Instruction::Print => simple_instruction("PRINT"),
            Instruction::Pop => simple_instruction("POP"),
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

fn offset_instruction<N: Display + Copy>(name: impl AsRef<str>, offset: &N) {
    println!("{:<16} {:4}", name.as_ref(), offset);
}
