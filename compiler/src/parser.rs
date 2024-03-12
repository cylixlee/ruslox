use std::{cell::RefCell, mem, ops::Range};

use codespan_reporting::diagnostic::{Diagnostic, Label};

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

pub enum Expression<'a> {
    // Literal expressions. Since we've known their types at parsing time, we don't have
    // to store Token and match its type later.
    String(&'a String),
    Number(f64),
    Identifier(&'a String),
    True,
    False,
    Nil,

    Unary(&'a Token, Box<Expression<'a>>),
    Binary(Box<Expression<'a>>, &'a Token, Box<Expression<'a>>),
}

pub enum Statement<'a> {
    VarDeclaration(&'a String, Option<Box<Expression<'a>>>),
    Print(Box<Expression<'a>>),
    Expressional(Box<Expression<'a>>),

    // Special variant for error recovery.
    Error,
}

peg::parser!(grammar pegparser(file_id: usize, ranges: &Vec<Range<usize>>, parser: &RefCell<Parser>) for ScannedContext {

    pub rule declarations() -> Vec<Statement<'input>>
        = declaration()*

    rule declaration() -> Statement<'input>
        = recognized_declaration()
        / recognized_statement()
        / pos:position!() ![Token::Semicolon] [_]+ [Token::Semicolon]? {
            parser.borrow_mut().report(Diagnostic::error()
                .with_code("E0005")
                .with_message("unrecognized statement")
                .with_labels(vec![
                    Label::primary(file_id, ranges[pos].clone())
                        .with_message("statement starting from here is unrecognizable")
                ])
            );
            parser.borrow_mut().panic_mode = false;
            Statement::Error
        }

    rule recognized_declaration() -> Statement<'input>
        = var_declaration()

    rule var_declaration() -> Statement<'input>
        = [Token::Var] name:variable_name() [Token::Equal] init:expression() must_consume(Token::Semicolon) {
            match name {
                Some(name) => Statement::VarDeclaration(name, Some(Box::new(init))),
                None => Statement::Error,
            }
        }
        / [Token::Var] name:variable_name() must_consume(Token::Semicolon) {
            match name {
                Some(name) => Statement::VarDeclaration(name, None),
                None => Statement::Error,
            }
        }

    rule variable_name() -> Option<&'input String>
        = [Token::Identifier(identifier)] { Some(identifier) }
        / pos:position!() {
            parser.borrow_mut().report(Diagnostic::error()
                .with_code("E0007")
                .with_message("missing variable name")
                .with_labels(vec![
                    Label::primary(file_id, ranges[pos - 1].clone())
                        .with_message("expected variable name after this")
                ])
            );
            None
        }

    rule recognized_statement() -> Statement<'input>
        = [Token::Print] e:expression() must_consume(Token::Semicolon) {
            Statement::Print(Box::new(e))
        }
        / e:expression() must_consume(Token::Semicolon) { Statement::Expressional(Box::new(e)) }

    rule must_consume(token: Token)
        = [t if mem::discriminant(t) == mem::discriminant(&token)]
        / pos:position!() {
            parser.borrow_mut().report(Diagnostic::error()
                .with_code("E0006")
                .with_message("missing specific token")
                .with_labels(vec![
                    Label::primary(file_id, ranges[pos - 1].clone())
                        .with_message(format!("expected {} after this", token))
                ])
                .with_notes(vec![format!("try adding {} or re-checking the code logic here", token)])
            );
        }

    rule expression() -> Expression<'input> = precedence! {
        // Assignment
        x:@ op:[Token::Equal] y:(@) { Expression::Binary(Box::new(x), op, Box::new(y)) }
        -- // Equality
        x:(@) op:[Token::EqualEqual | Token::BangEqual] y:@ { Expression::Binary(Box::new(x), op, Box::new(y)) }
        -- // Comparison
        x:(@) op:[Token::Greater | Token::Less | Token::GreaterEqual | Token::LessEqual] y:@ {
            Expression::Binary(Box::new(x), op, Box::new(y))
        }
        -- // Term
        x:(@) op:[Token::Plus| Token::Minus] y:@ { Expression::Binary(Box::new(x), op, Box::new(y)) }
        -- // Factor
        x:(@) op:[Token::Star | Token::Slash] y:@ { Expression::Binary(Box::new(x), op, Box::new(y)) }
        -- // Unary
        op:[Token::Minus | Token::Bang] e:(@) { Expression::Unary(op, Box::new(e)) }
        -- // Primary
        [Token::Number(n)] { Expression::Number(*n) }
        [Token::String(s)] { Expression::String(s) }
        [Token::Identifier(identifier)] { Expression::Identifier(identifier) }
        [Token::True]  { Expression::True }
        [Token::False] { Expression::False }
        [Token::Nil]   { Expression::Nil }
        [Token::LeftParenthesis] e:expression() [Token::RightParenthesis] { e }
    }
});

pub fn parse(
    file_id: usize,
    scanned: &ScannedContext,
) -> Result<Vec<Statement>, Vec<Diagnostic<usize>>> {
    let parser = RefCell::new(Parser::new());
    let declarations = pegparser::declarations(scanned, file_id, &scanned.positions, &parser)
        .expect("internal parse error");
    let parser = parser.into_inner();
    if !parser.diagnostics.is_empty() {
        Err(parser.diagnostics)
    } else {
        Ok(declarations)
    }
}
