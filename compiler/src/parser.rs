use std::{cell::RefCell, mem, ops::Range};

use codespan_reporting::diagnostic::{Diagnostic, Label};
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

peg::parser!(grammar pegparser(
    file_id: usize,
    ranges: &Vec<Range<usize>>,
    chunk: &RefCell<Chunk>,
    parser: &RefCell<Parser>
) for ScannedContext {

    pub rule declarations()
        = declaration()*

    rule declaration()
        = recognized_declaration()
        / recognized_statement()
        / pos:position!() ![Token::Semicolon] [_]+ [Token::Semicolon]? {
            report(parser, Diagnostic::error()
                .with_code("E0005")
                .with_message("unrecognized statement")
                .with_labels(vec![
                    Label::primary(file_id, ranges[pos].clone())
                        .with_message("statement starting from here is unrecognizable")
                ])
            );
            parser.borrow_mut().panic_mode = false;
        }

    rule recognized_declaration()
        = [Token::Var] index:must_variable_name() init:([Token::Equal] expression())? must_consume(Token::Semicolon) {
            if let Some(index) = index {
                if init.is_none() {
                    emit(chunk, Instruction::Nil);
                }
                define_variable(chunk, index);
            }
        }

    rule must_variable_name() -> Option<u8>
        = [Token::Identifier(identifier)] {
            match identifier_constant(chunk, identifier) {
                Ok(index) => Some(index),
                Err(diagnostic) => {
                    report(parser, diagnostic);
                    None
                }
            }
        }
        / pos:position!() {
            report(parser, Diagnostic::error()
                .with_code("E0007")
                .with_message("missing variable name")
                .with_labels(vec![
                    Label::primary(file_id, ranges[pos - 1].clone())
                        .with_message("expected variable name after this")
                ])
            );
            None
        }

    rule recognized_statement()
        = [Token::Print] expression() must_consume(Token::Semicolon) {
            // Print Statement
            emit(chunk, Instruction::Print);
        }
        / expression() must_consume(Token::Semicolon) {
            // Expression Statement
            emit(chunk, Instruction::Pop);
        }

    rule must_consume(token: Token)
        = [t if mem::discriminant(t) == mem::discriminant(&token)]
        / pos:position!() {
            report(parser, Diagnostic::error()
                .with_code("E0006")
                .with_message("missing specific token")
                .with_labels(vec![
                    Label::primary(file_id, ranges[pos - 1].clone())
                        .with_message(format!("expected {} after this", token))
                ])
                .with_notes(vec![format!("try adding {} or re-checking the code logic here", token)])
            );
        }

    rule expression() = precedence! {
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
        [Token::Identifier(identifier)] assign:([Token::Equal] expression())? {
            if let Err(diagnostic) = named_variable(chunk, identifier, assign) {
                report(parser, diagnostic);
            }
        }
        [Token::True]  { emit(chunk, Instruction::True) }
        [Token::False] { emit(chunk, Instruction::False) }
        [Token::Nil]   { emit(chunk, Instruction::Nil) }
        [Token::LeftParenthesis] expression() [Token::RightParenthesis] {}
    }
});

pub fn parse(file_id: usize, scanned: &ScannedContext) -> Result<Chunk, Vec<Diagnostic<usize>>> {
    let chunk = RefCell::new(Chunk::new());
    let parser = RefCell::new(Parser::new());
    pegparser::declarations(scanned, file_id, &scanned.positions, &chunk, &parser)
        .expect("internal parse error");
    let (mut chunk, parser) = (chunk.into_inner(), parser.into_inner());
    if !parser.diagnostics.is_empty() {
        Err(parser.diagnostics)
    } else {
        chunk.write(Instruction::Return);
        Ok(chunk)
    }
}

// ============ Helper functions to reduce RefCell::borrow_mut calls. ============
fn report(parser: &RefCell<Parser>, diagnostic: Diagnostic<usize>) {
    if !parser.borrow().panic_mode {
        parser.borrow_mut().report(diagnostic);
        parser.borrow_mut().panic_mode = true;
    }
}

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

fn identifier_constant(
    chunk: &RefCell<Chunk>,
    identifier: &String,
) -> Result<u8, Diagnostic<usize>> {
    chunk
        .borrow_mut()
        .add_constant(Constant::String(identifier.clone()))
}

fn define_variable(chunk: &RefCell<Chunk>, index: u8) {
    emit(chunk, Instruction::DefineGlobal(index));
}

fn named_variable(
    chunk: &RefCell<Chunk>,
    name: &String,
    assign: Option<()>,
) -> Result<(), Diagnostic<usize>> {
    let index = identifier_constant(chunk, name)?;
    match assign {
        Some(_) => todo!(),
        None => emit(chunk, Instruction::GetGlobal(index)),
    }
    Ok(())
}
