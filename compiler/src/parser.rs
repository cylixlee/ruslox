use std::cell::RefCell;

use shared::error::{InterpretError, InterpretResult};

#[derive(Debug)]
pub(crate) enum Expression {
    Number(f64),
    Negation(Box<Expression>),
    Add(Box<Expression>, Box<Expression>),
    Subtract(Box<Expression>, Box<Expression>),
    Multiply(Box<Expression>, Box<Expression>),
    Divide(Box<Expression>, Box<Expression>),

    // Special expression for error recovery.
    Error,
}

struct ParseContext<'input> {
    source: &'input str,
    errors: Option<Vec<InterpretError>>,
    linesizes: Option<Vec<usize>>,
}

impl<'input> ParseContext<'input> {
    fn new(source: &'input str) -> Self {
        Self {
            source,
            errors: None,
            linesizes: None,
        }
    }

    fn report(&mut self, message: impl Into<String>, offset: usize) {
        if self.linesizes.is_none() {
            self.calculate_linesizes();
        }
        let mut remaining = offset;
        let mut line = 1;
        let mut column = 1;
        for linesize in self.linesizes.as_ref().unwrap() {
            if remaining <= *linesize {
                column = remaining;
                break;
            }
            remaining -= *linesize;
            line += 1;
            column = 1;
        }

        let error = InterpretError::CompileError(message.into(), Some((line, column)));
        match &mut self.errors {
            Some(errors) => errors.push(error),
            None => self.errors = Some(vec![error]),
        }
    }

    fn calculate_linesizes(&mut self) {
        let mut linesizes = Vec::new();
        let mut size = 0;
        for character in self.source.chars() {
            size += 1;
            if character == '\n' {
                linesizes.push(size);
                size = 0;
            }
        }
        self.linesizes = Some(linesizes);
    }
}

peg::parser!(grammar pegparser(context: &RefCell<ParseContext>) for str {
    use std::str::FromStr;
    use Expression::*;

    pub rule expression() -> Expression
        = _ e:expression_precedence('\n') _ { e }

    rule expression_precedence(boundary: char) -> Expression = precedence! {
        // Term Binary Expressions
        x:(@) _ "+" _ y:@ { Add(Box::new(x), Box::new(y)) }
        x:(@) _ "-" _ y:@ { Subtract(Box::new(x), Box::new(y)) }

        -- // Factor Binary Expressions
        x:(@) _ "*" _ y:@ { Multiply(Box::new(x), Box::new(y)) }
        x:(@) _ "/" _ y:@ { Divide(Box::new(x), Box::new(y)) }

        -- // Unary Expressions
        "-" _ e:(@) { Negation(Box::new(e)) }

        -- // Primary expressions.
        n:number()                    { n } // Number
        "(" _ e:expression_precedence(')') _ ")" { e } // Grouping
        // Error token until the expression boundary.
        pos:position!() s:$([c if c != boundary]+) {
            context.borrow_mut().report(format!("unrecognized token {}", s), pos);
            Error
        }
    }

    rule number() -> Expression
        = pos:position!() s:$(numeric()+ ("." numeric()+)?) {
            match f64::from_str(s) {
                Ok(n)  => Number(n),
                Err(_) => {
                    context.borrow_mut().report(format!("invalid number {}", s), pos);
                    Error
                }
            }
        }

    rule _ = [' ' | '\t' | '\r' | '\n']*

    // Helper rules
    rule alpha()        = ['a'..='z' | 'A'..='Z' | '_']
    rule numeric()      = ['0'..='9']
    rule alphanumeric() = ['a'..='z' | 'A'..='Z' | '_' | '0'..='9']
});

pub(crate) fn parse<'input>(source: &'input str) -> InterpretResult<Expression> {
    let parse_context = RefCell::new(ParseContext::new(source));
    let expression = pegparser::expression(source, &parse_context).unwrap();
    let mut parse_context = parse_context.borrow_mut();
    match parse_context.errors.is_some() {
        true => Err(InterpretError::CompoundError(
            parse_context.errors.take().unwrap(),
        )),
        false => Ok(expression),
    }
}
