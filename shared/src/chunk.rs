use codespan_reporting::diagnostic::Diagnostic;

use crate::constant::Constant;

#[rustfmt::skip]
pub enum Instruction {
    // Instructions with operand.
    Constant(u8), DefineGlobal(u8), GetGlobal(u8), SetGlobal(u8),

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
    pub code: Vec<Instruction>,
    pub constants: Vec<Constant>,
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

    pub fn add_constant(&mut self, value: Constant) -> Result<u8, Diagnostic<usize>> {
        if self.constants.len() >= u8::MAX as usize + 1 {
            return Err(Diagnostic::error()
                .with_code("E0001")
                .with_message("too many constants in one chunk"));
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
            // Instructions with operand.
            Instruction::Constant(constant_index) => {
                constant_instruction("CONST", constant_index, self)
            }
            Instruction::DefineGlobal(index) => constant_instruction("DEFINEGLOBAL", index, self),
            Instruction::GetGlobal(index) => constant_instruction("GETGLOBAL", index, self),
            Instruction::SetGlobal(index) => constant_instruction("SETGLOBAL", index, self),

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
