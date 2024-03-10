use std::cell::RefCell;

use shared::error::{InterpretError, InterpretResult};

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

#[rustfmt::skip]
pub(crate) enum Token {
    // Literal tokens.
    Number(f64), True, False, Nil,

    // Operator tokens.
    Plus, Minus, Star, Slash, Bang,
    Greater, GreaterEqual, Less, LessEqual, EqualEqual, BangEqual,
    LeftParenthesis, RightParenthesis,

    // Placeholder token for error recovery.
    ErrorToken,
}

pub(crate) enum Expression {
    Literal(Token),
    Unary(Token, Box<Expression>),
    Binary(Box<Expression>, Token, Box<Expression>),

    // Placeholder token for error recovery.
    ErrorExpr,
}

peg::parser!(grammar pegparser(context: &RefCell<ParseContext>) for str {
    use std::str::FromStr;
    use Token::*;
    use Expression::*;

    // The alternative branch is actually uncecessary when we introduce statements.
    // Expressions have no clear boundary with the only exception GroupingExpression.
    pub rule expression() -> Expression
        = valid_expression()
        / pos:position!() (!valid_expression() token())+ {
            context.borrow_mut().report("expected expression", pos);
            ErrorExpr
        }

    rule valid_expression() -> Expression = precedence! {
        // Equality expressions
        x:(@) op:equality_op() y:@ { Binary(Box::new(x), op, Box::new(y)) }
        -- // Comparison expressions
        x:(@) op:comparison_op() y:@ { Binary(Box::new(x), op, Box::new(y)) }
        -- // Term expressions
        x:(@) op:term_op() y:@ { Binary(Box::new(x), op, Box::new(y)) }
        -- // Factor expressions
        x:(@) op:factor_op() y:@ { Binary(Box::new(x), op, Box::new(y)) }
        -- // Unary expressions
        op:unary_op() e:(@) { Unary(op, Box::new(e)) }

        -- // Primary expressions.
        l:literal() { Literal(l) }                                        // Literal
        left_parenthesis() e:valid_expression() right_parenthesis() { e } // Grouping

        // Error recovery within GroupingExpression.
        left_parenthesis() pos:position!() (!right_parenthesis() token())+ right_parenthesis() {
            context.borrow_mut().report("expected expression", pos);
            ErrorExpr
        }
    }

    // ========================== Scanner part ==========================
    // ------------ the token rule mainly for error recovery ------------
    // Token is self-delimited. That allows a possible error recovery.
    rule token() -> Token
        = valid_token()
        / _ pos:position!() s:$((!valid_token() [_])+) _ {
            context.borrow_mut().report(format!("invalid token {}", s.trim()), pos);
            ErrorToken
        }

    rule valid_token() -> Token
        = literal()
        / binary_op()
        / unary_op()
        / left_parenthesis()
        / right_parenthesis()

    // --------------------- parser-preferred rules ---------------------
    rule literal() -> Token
        = _ pos:position!() s:$(numeric()+ ("." numeric()+)?) _ {
            match f64::from_str(s) {
                Ok(n)  => Number(n),
                Err(_) => {
                    context.borrow_mut().report(format!("invalid number {}", s), pos);
                    ErrorToken
                }
            }
        }
        / true_()
        / false_()
        / nil()
    rule binary_op() -> Token
        = equality_op()
        / comparison_op()
        / term_op()
        / factor_op()
    rule unary_op() -> Token = minus() / bang()

    // BinaryOp helper rules.
    rule equality_op() -> Token
        = equal_equal()
        / bang_equal()
    rule comparison_op() -> Token
        = greater_equal()
        / less_equal()
        / greater()
        / less()
    rule term_op()   -> Token = plus() / minus()
    rule factor_op() -> Token = star() / slash()


    // ------------------- literal-string rules -------------------
    rule equal_equal()   -> Token = _ "==" _ { EqualEqual }
    rule bang_equal()    -> Token = _ "!=" _ { BangEqual }
    rule greater_equal() -> Token = _ ">=" _ { GreaterEqual }
    rule less_equal()    -> Token = _ "<=" _ { LessEqual }
    rule greater() -> Token = _ ">" _ { Greater }
    rule less()    -> Token = _ "<" _ { Less }
    rule plus()    -> Token = _ "+" _ { Plus }
    rule minus()   -> Token = _ "-" _ { Minus }
    rule star()    -> Token = _ "*" _ { Star }
    rule slash()   -> Token = _ "/" _ { Slash }
    rule bang()    -> Token = _ "!" _ { Bang }
    rule left_parenthesis()  -> Token = _ "(" _ { LeftParenthesis }
    rule right_parenthesis() -> Token = _ ")" _ { RightParenthesis }

    rule true_()  -> Token = _ "true"  _ { True }
    rule false_() -> Token = _ "false" _ { False }
    rule nil()    -> Token = _ "nil"   _ { Nil }

    rule _ = blank()* comment()? blank()*
    rule blank() = [' ' | '\t' | '\r' | '\n']
    rule comment() = "//" [^'\n']*

    // Helper range rules
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
