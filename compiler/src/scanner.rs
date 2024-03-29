use std::{fmt::Display, ops::Range};

use peg::{Parse, ParseElem};
use shared::error::{ErrorItem, InterpretError, InterpretResult, Label};

#[rustfmt::skip]
pub enum Token {
    // Single character tokens.
    LeftParenthesis, RightParenthesis, LeftBrace, RightBrace,
    Comma, Dot, Minus, Plus, Semicolon, Slash, Star,

    // One or two character tokens.
    Bang, BangEqual, Equal, EqualEqual,
    Greater, GreaterEqual, Less, LessEqual,

    // Literals.
    Identifier(String), String(String), Number(f64),

    // Keywords.
    And, Class, Else, False, For, Fun, If, Nil,
    Or, Print, Return, Super, This, True, Var, While,

    // Special
    Error,
}

impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::LeftParenthesis => write!(f, "("),
            Token::RightParenthesis => write!(f, ")"),
            Token::LeftBrace => write!(f, "{{"),
            Token::RightBrace => write!(f, "}}"),
            Token::Comma => write!(f, ","),
            Token::Dot => write!(f, "."),
            Token::Minus => write!(f, "-"),
            Token::Plus => write!(f, "+"),
            Token::Semicolon => write!(f, ";"),
            Token::Slash => write!(f, "/"),
            Token::Star => write!(f, "*"),
            Token::Bang => write!(f, "!"),
            Token::BangEqual => write!(f, "!="),
            Token::Equal => write!(f, "="),
            Token::EqualEqual => write!(f, "=="),
            Token::Greater => write!(f, ">"),
            Token::GreaterEqual => write!(f, ">="),
            Token::Less => write!(f, "<"),
            Token::LessEqual => write!(f, "<="),
            Token::Identifier(_) => write!(f, "identifier"),
            Token::String(_) => write!(f, "string literal"),
            Token::Number(_) => write!(f, "number literal"),
            Token::And => write!(f, "and"),
            Token::Class => write!(f, "class"),
            Token::Else => write!(f, "else"),
            Token::False => write!(f, "false"),
            Token::For => write!(f, "for"),
            Token::Fun => write!(f, "fun"),
            Token::If => write!(f, "if"),
            Token::Nil => write!(f, "nil"),
            Token::Or => write!(f, "or"),
            Token::Print => write!(f, "print"),
            Token::Return => write!(f, "return"),
            Token::Super => write!(f, "super"),
            Token::This => write!(f, "this"),
            Token::True => write!(f, "true"),
            Token::Var => write!(f, "var"),
            Token::While => write!(f, "while"),
            Token::Error => write!(f, "<internal error>"),
        }
    }
}

pub struct ScannedContext {
    pub tokens: Vec<Token>,
    pub positions: Vec<Range<usize>>,
    pub errors: Vec<ErrorItem>,
}

impl ScannedContext {
    fn new() -> Self {
        Self {
            tokens: Vec::new(),
            positions: Vec::new(),
            errors: Vec::new(),
        }
    }

    fn record(&mut self, token: Token, position: Range<usize>) {
        self.tokens.push(token);
        self.positions.push(position);
    }

    fn report(&mut self, error: ErrorItem) {
        self.errors.push(error);
    }
}

impl Parse for ScannedContext {
    type PositionRepr = usize;

    fn start<'input>(&'input self) -> usize {
        0
    }

    fn is_eof<'input>(&'input self, p: usize) -> bool {
        p >= self.tokens.len()
    }

    fn position_repr<'input>(&'input self, p: usize) -> Self::PositionRepr {
        p
    }
}

impl<'a> ParseElem<'a> for ScannedContext {
    type Element = &'a Token;

    fn parse_elem(&'a self, pos: usize) -> peg::RuleResult<Self::Element> {
        if pos < self.tokens.len() {
            peg::RuleResult::Matched(pos + 1, &self.tokens[pos])
        } else {
            peg::RuleResult::Failed
        }
    }
}

peg::parser!(grammar pegscanner(file_id: usize, context: &mut ScannedContext) for str {
    use Token::*;

    pub rule scan() = _ token()**_ _

    rule token()
        = start:position!() t:recognized_token() end:position!() {
            context.record(t, start..end);
        }
        / start:position!() [_] {
            context.record(Error, start..start + 1);
            context.report(ErrorItem::error()
                .with_code("E0002")
                .with_message("unexpected character")
                .with_labels(vec![
                    Label::primary(file_id, start..start + 1)
                        .with_message("this character is beyond Lox's syntax rule.")
                ])
            );
        }

    rule recognized_token() -> Token
        = single()
        / one_or_two()
        / keywords()
        / literals()

    rule single() -> Token
        = "(" { LeftParenthesis }
        / ")" { RightParenthesis }
        / "{" { LeftBrace }
        / "}" { RightBrace }
        / "," { Comma }
        / "." { Dot }
        / "-" { Minus }
        / "+" { Plus }
        / ";" { Semicolon }
        / "/" { Slash }
        / "*" { Star }
    rule one_or_two() -> Token
        = "!=" { BangEqual }
        / "==" { EqualEqual }
        / ">=" { GreaterEqual }
        / "<=" { LessEqual }
        / "!" { Bang }
        / "=" { Equal }
        / ">" { Greater }
        / "<" { Less }
    rule keywords() -> Token
        = "and"    { And }
        / "class"  { Class }
        / "else"   { Else }
        / "false"  { False }
        / "for"    { For }
        / "fun"    { Fun }
        / "if"     { If }
        / "nil"    { Nil }
        / "or"     { Or }
        / "print"  { Print }
        / "return" { Return }
        / "super"  { Super }
        / "this"   { This }
        / "true"   { True }
        / "var"    { Var }
        / "while"  { While }
    rule literals() -> Token
        = identifier()
        / number()
        / string()

    rule identifier() -> Token
        = s:$(alpha() alphanumeric()*) { Identifier(s.into()) }
    rule number() -> Token
        = start:position!() s:$(numeric()+ ("." numeric()+)?) end:position!() {
            match s.parse::<f64>() {
                Ok(n) => Number(n),
                Err(_) => {
                    context.report(ErrorItem::error()
                        .with_code("E0003")
                        .with_message("uninterpretable number literal")
                        .with_labels(vec![
                            Label::primary(file_id, start..end)
                                .with_message("this number is valid in syntax but cannot be converted or stored as f64.")
                        ])
                    );
                    Error
                }
            }
        }
    rule string() -> Token
        = "\"" s:$([^'"']*) "\"" { String(s.into()) }
        / start:position!() "\"" [_]* {
            context.report(ErrorItem::error()
                .with_code("E0004")
                .with_message("unterminated string")
                .with_labels(vec![
                    Label::primary(file_id, start..start + 1)
                        .with_message("the string literal started here does not end")
                ])
                .with_notes(vec!["did you forget the ending double-quote?".into()])
            );
            Error
        }

    // Helper rules.
    rule alpha() = ['a'..='z' | 'A'..='Z' | '_']
    rule numeric() = ['0'..='9']
    rule alphanumeric() = ['a'..='z' | 'A'..='Z' | '_' | '0'..='9']

    rule _ = blank()* comment()**"\n" blank()*
    rule blank() = [' ' | '\t' | '\r' | '\n']
    rule comment() = "//" [^'\n']*
});

pub fn scan(file_id: usize, input: &str) -> InterpretResult<ScannedContext> {
    let mut context = ScannedContext::new();
    pegscanner::scan(input, file_id, &mut context).expect("internal scan error.");
    match context.errors.is_empty() {
        true => Ok(context),
        false => Err(InterpretError::Compound(context.errors)),
    }
}
