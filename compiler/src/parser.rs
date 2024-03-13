use std::{cell::RefCell, mem, ops::Range};

use shared::error::{ErrorItem, InterpretError, InterpretResult, Label};

use crate::scanner::{ScannedContext, Token};

pub struct ParsedContext<'a> {
    pub statements: Vec<Statement<'a>>,
    pub positions: Vec<Range<usize>>,
    pub errors: Vec<ErrorItem>,
    panic_mode: bool,
}

impl<'a> ParsedContext<'a> {
    fn new() -> Self {
        Self {
            statements: Vec::new(),
            positions: Vec::new(),
            errors: Vec::new(),
            panic_mode: false,
        }
    }

    fn record(&mut self, statement: Statement<'a>, position: Range<usize>) {
        self.statements.push(statement);
        self.positions.push(position);
    }

    fn report(&mut self, error: ErrorItem) {
        if !self.panic_mode {
            self.errors.push(error);
            self.panic_mode = true;
        }
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

peg::parser!(grammar pegparser(file_id: usize, ranges: &Vec<Range<usize>>, context: &RefCell<ParsedContext<'input>>) for ScannedContext {

    pub rule declarations()
        = declaration()*

    rule declaration()
        = start:position!() s:recognized_declaration() {
            context.borrow_mut().record(s, ranges[start].clone());
        }
        / start:position!() s:recognized_statement() {
            context.borrow_mut().record(s, ranges[start].clone());
        }
        / pos:position!() ![Token::Semicolon] [_]+ [Token::Semicolon]? {
            context.borrow_mut().report(ErrorItem::error()
                .with_code("E0005")
                .with_message("unrecognized statement")
                .with_labels(vec![
                    Label::primary(file_id, ranges[pos].clone())
                        .with_message("statement starting from here is unrecognizable")
                ])
            );
            context.borrow_mut().panic_mode = false;
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
            context.borrow_mut().report(ErrorItem::error()
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
            context.borrow_mut().report(ErrorItem::error()
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

pub fn parse(file_id: usize, scanned: &ScannedContext) -> InterpretResult<ParsedContext> {
    let context = RefCell::new(ParsedContext::new());
    pegparser::declarations(scanned, file_id, &scanned.positions, &context)
        .expect("internal parse error");
    let context = RefCell::into_inner(context);
    match context.errors.is_empty() {
        true => Ok(context),
        false => Err(InterpretError::Compound(context.errors)),
    }
}
