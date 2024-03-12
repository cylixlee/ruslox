use std::cell::RefCell;

use codespan_reporting::diagnostic::Diagnostic;
use shared::{
    chunk::{Chunk, Instruction},
    constant::Constant,
};

use crate::scanner::{ScannedContext, Token};

struct Parser {
    diagnostics: Vec<Diagnostic<usize>>,
    panic_mode: bool,
}

impl Parser {
    fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
            panic_mode: false,
        }
    }

    fn report(&mut self, diagnostic: Diagnostic<usize>) {
        self.diagnostics.push(diagnostic);
        self.panic_mode = true;
    }
}

peg::parser!(grammar pegparser(chunk: &RefCell<Chunk>, parser: &RefCell<Parser>) for ScannedContext {
    pub rule expression() = precedence! {
        // Equality
        (@) [Token::EqualEqual] @ { emit(chunk, Instruction::Equal) }
        (@) [Token::BangEqual] @ {
            emit(chunk, Instruction::Equal);
            emit(chunk, Instruction::Not);
        }
        -- // Comparison
        (@) [Token::Greater] @ { emit(chunk, Instruction::Greater) }
        (@) [Token::Less]    @ { emit(chunk, Instruction::Less) }
        (@) [Token::GreaterEqual] @ {
            emit(chunk, Instruction::Less);
            emit(chunk, Instruction::Not);
        }
        (@) [Token::LessEqual] @ {
            emit(chunk, Instruction::Greater);
            emit(chunk, Instruction::Not);
        }
        -- // Term
        (@) [Token::Plus]  @ { emit(chunk, Instruction::Add) }
        (@) [Token::Minus] @ { emit(chunk, Instruction::Subtract) }
        -- // Factor
        (@) [Token::Star]  @ { emit(chunk, Instruction::Multiply) }
        (@) [Token::Slash] @ { emit(chunk, Instruction::Divide) }
        -- // Unary
        [Token::Minus] (@) { emit(chunk, Instruction::Negate) }
        [Token::Bang]  (@) { emit(chunk, Instruction::Not) }
        -- // Primary
        [Token::Number(n)] {
            if let Err(diagnostic) = emit_constant(chunk, Constant::Number(*n)) {
                report(parser, diagnostic);
            }
        }
        [Token::String(s)] {
            if let Err(diagnostic) = emit_constant(chunk, Constant::String(s.clone())) {
                report(parser, diagnostic);
            }
        }
        [Token::True]  { emit(chunk, Instruction::True) }
        [Token::False] { emit(chunk, Instruction::False) }
        [Token::Nil]   { emit(chunk, Instruction::Nil) }
        [Token::LeftParenthesis] expression() [Token::RightParenthesis] {}
    }
});

pub fn parse(scanned: &ScannedContext) -> Result<Chunk, Vec<Diagnostic<usize>>> {
    let chunk = RefCell::new(Chunk::new());
    let parser = RefCell::new(Parser::new());
    pegparser::expression(scanned, &chunk, &parser).expect("internal parse error");
    let (mut chunk, parser) = (chunk.into_inner(), parser.into_inner());
    if !parser.diagnostics.is_empty() {
        Err(parser.diagnostics)
    } else {
        chunk.write(Instruction::Return);
        Ok(chunk)
    }
}

// ============ Helper functions to reduce RefCell::borrow_mut calls. ============

fn emit(chunk: &RefCell<Chunk>, instruction: Instruction) {
    chunk.borrow_mut().write(instruction);
}

fn emit_constant(chunk: &RefCell<Chunk>, constant: Constant) -> Result<(), Diagnostic<usize>> {
    let constant_index = chunk.borrow_mut().add_constant(constant)?;
    chunk
        .borrow_mut()
        .write(Instruction::Constant(constant_index));
    Ok(())
}

fn report(parser: &RefCell<Parser>, diagnostic: Diagnostic<usize>) {
    parser.borrow_mut().report(diagnostic);
}
