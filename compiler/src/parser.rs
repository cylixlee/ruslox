use std::{cell::RefCell, mem, ops::Range};

use shared::error::{ErrorItem, InterpretError, InterpretResult, Label};

use crate::scanner::{ScannedContext, Token};

pub struct ParsedContext<'a> {
    file_id: usize,
    pub statements: Vec<Statement<'a>>,
    pub positions: Vec<Range<usize>>,
    pub errors: Vec<ErrorItem>,
    panic_mode: bool,
}

impl<'a> ParsedContext<'a> {
    fn new(file_id: usize) -> Self {
        Self {
            file_id,
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

    fn report(
        &mut self,
        token_position: Range<usize>,
        code: impl Into<String>,
        message: impl Into<String>,
        label: impl Into<String>,
    ) {
        self.report_error(token_position, code, message, label, None);
    }

    fn report_noted(
        &mut self,
        token_position: Range<usize>,
        code: impl Into<String>,
        message: impl Into<String>,
        label: impl Into<String>,
        note: impl Into<String>,
    ) {
        self.report_error(token_position, code, message, label, Some(note.into()));
    }

    fn report_error(
        &mut self,
        token_position: Range<usize>,
        code: impl Into<String>,
        message: impl Into<String>,
        label: impl Into<String>,
        note: Option<String>,
    ) {
        if !self.panic_mode {
            let mut error_item = ErrorItem::error()
                .with_code(code)
                .with_message(message)
                .with_labels(vec![
                    Label::secondary(self.file_id, token_position).with_message(label)
                ]);
            if let Some(note) = note {
                error_item = error_item.with_notes(vec![note]);
            }
            self.errors.push(error_item);
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

peg::parser!(grammar pegparser(
    file_id: usize,
    token_positions: &Vec<Range<usize>>,
    context: &RefCell<ParsedContext<'input>>
) for ScannedContext {

    pub rule declarations()
        = declaration()*

    rule declaration()
        = start:position!() s:recognized_declaration() {
            context.borrow_mut().record(s, token_positions[start].clone());
        }
        / start:position!() s:recognized_statement() {
            context.borrow_mut().record(s, token_positions[start].clone());
        }
        / pos:position!() ![Token::Semicolon] [_]+ [Token::Semicolon]? {
            context.borrow_mut().report(
                token_positions[pos].clone(),
                "E0005",
                "unrecognized statement",
                "statement starting from here is unrecognizable",
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
            context.borrow_mut().report(
                token_positions[pos - 1].clone(),
                "E0007",
                "missing variable name",
                "expected variable name after this",
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
            context.borrow_mut().report_noted(
                token_positions[pos - 1].clone(),
                "E0006",
                "missing specific token",
                format!("expected {} after this", token),
                format!("try adding {} or re-checking the code logic here", token)
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
    let context = RefCell::new(ParsedContext::new(file_id));
    pegparser::declarations(scanned, file_id, &scanned.positions, &context)
        .expect("internal parse error");
    let context = RefCell::into_inner(context);
    match context.errors.is_empty() {
        true => Ok(context),
        false => Err(InterpretError::Compound(context.errors)),
    }
}
