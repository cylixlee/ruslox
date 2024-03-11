use std::ops::Range;

use codespan_reporting::diagnostic::{Diagnostic, Label};
use phf::phf_map;

#[rustfmt::skip]
#[derive(Debug, Clone)]
pub(crate) enum Token {
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
    EndOfFile,
}

static KEYWORDS: phf::Map<&'static str, Token> = phf_map! {
    "and"    => Token::And,
    "class"  => Token::Class,
    "else"   => Token::Else,
    "false"  => Token::False,
    "for"    => Token::For,
    "fun"    => Token::Fun,
    "if"     => Token::If,
    "nil"    => Token::Nil,
    "or"     => Token::Or,
    "print"  => Token::Print,
    "return" => Token::Return,
    "super"  => Token::Super,
    "this"   => Token::This,
    "true"   => Token::True,
    "var"    => Token::Var,
    "while"  => Token::While,
};

pub(crate) struct Scanner<'a> {
    source: &'a Vec<char>,
    file_id: usize,
    start: usize,
    current: usize,
}

impl<'a> Scanner<'a> {
    pub fn new(source: &'a Vec<char>, file_id: usize) -> Self {
        Self {
            source,
            file_id,
            start: 0,
            current: 0,
        }
    }

    pub fn is_eof(&self) -> bool {
        self.current >= self.source.len()
    }

    pub fn scan(&mut self) -> Result<(Token, Range<usize>), Diagnostic<usize>> {
        macro_rules! positioned {
            ($e:expr) => {
                Ok(($e, self.start..self.current))
            };
        }

        self.skip_whitespace();
        self.start = self.current;

        let prefix = self.advance();
        if prefix.is_none() {
            return positioned!(Token::EndOfFile);
        }

        let prefix = prefix.unwrap();
        match prefix {
            // Single character tokens.
            '(' => positioned!(Token::LeftParenthesis),
            ')' => positioned!(Token::RightParenthesis),
            '{' => positioned!(Token::LeftBrace),
            '}' => positioned!(Token::RightBrace),
            ';' => positioned!(Token::Semicolon),
            ',' => positioned!(Token::Comma),
            '.' => positioned!(Token::Dot),
            '-' => positioned!(Token::Minus),
            '+' => positioned!(Token::Plus),
            '/' => positioned!(Token::Slash),
            '*' => positioned!(Token::Star),

            // One or two character tokens.
            '!' if self.try_consume('=') => positioned!(Token::BangEqual),
            '=' if self.try_consume('=') => positioned!(Token::EqualEqual),
            '<' if self.try_consume('=') => positioned!(Token::LessEqual),
            '>' if self.try_consume('=') => positioned!(Token::GreaterEqual),
            '!' => positioned!(Token::Bang),
            '=' => positioned!(Token::Equal),
            '<' => positioned!(Token::Less),
            '>' => positioned!(Token::Greater),

            // Literals.
            '"' => self.scan_string(),
            c if c.is_ascii_digit() => self.scan_number(),
            c if c.is_ascii_alphabetic() || c == '_' => Ok(self.scan_identifier()),

            // Error recovery.
            _ => Err(Diagnostic::error()
                .with_code("E0002")
                .with_message("unexpected character")
                .with_labels(vec![Label::primary(self.file_id, self.start..self.current)
                    .with_message("this character is beyond Lox's syntax rule.")])),
        }
    }

    // This function will never fail. It's an identifier with at least 1 valid character.
    fn scan_identifier(&mut self) -> (Token, Range<usize>) {
        while let Some(peek) = self.peek() {
            if !peek.is_ascii_alphanumeric() && peek != '_' {
                break;
            }
            self.advance();
        }
        let lexeme: String = (&self.source[self.start..self.current]).iter().collect();
        if let Some(keyword) = KEYWORDS.get(&lexeme) {
            return (keyword.clone(), self.start..self.current);
        }
        (Token::Identifier(lexeme), self.start..self.current)
    }

    fn scan_number(&mut self) -> Result<(Token, Range<usize>), Diagnostic<usize>> {
        while let Some(peek) = self.peek() {
            if !peek.is_ascii_digit() {
                break;
            }
            self.advance();
        }

        if let (Some('.'), Some(next)) = (self.peek(), self.peek_next()) {
            if next.is_ascii_digit() {
                self.advance(); // Consumes the dot.
                while let Some(peek) = self.peek() {
                    if !peek.is_ascii_digit() {
                        break;
                    }
                    self.advance();
                }
            }
        }

        let lexeme: String = (&self.source[self.start..self.current]).iter().collect();
        match lexeme.parse::<f64>() {
            Ok(number) => Ok((Token::Number(number), self.start..self.current)),
            Err(_) => Err(Diagnostic::error()
                .with_code("E0003")
                .with_message("uninterpretable number literal")
                .with_labels(vec![Label::primary(self.file_id, self.start..self.current)
                    .with_message(
                        "this number is valid in syntax but cannot be converted or stored as f64.",
                    )])),
        }
    }

    fn scan_string(&mut self) -> Result<(Token, Range<usize>), Diagnostic<usize>> {
        let mut terminated = false;
        let mut lexeme = String::new();
        while let Some(character) = self.advance() {
            if character == '\"' {
                terminated = true;
                break;
            }
            lexeme.push(character);
        }

        if !terminated {
            return Err(Diagnostic::error()
                .with_code("E0004")
                .with_message("unterminated string")
                .with_labels(vec![Label::primary(
                    self.file_id,
                    self.start..self.start + 1,
                )
                .with_message("the string literal started here does not end")])
                .with_notes(vec!["did you forget the ending double-quote?".into()]));
        }
        Ok((Token::String(lexeme), self.start..self.current))
    }

    fn skip_whitespace(&mut self) {
        while let Some(peek) = self.peek() {
            if !peek.is_whitespace() {
                if let ('/', Some('/')) = (peek, self.peek_next()) {
                    while let Some(character) = self.advance() {
                        if character == '\n' {
                            break;
                        }
                    }
                } else {
                    break;
                }
            }
            self.advance();
        }
    }

    fn peek(&self) -> Option<char> {
        if self.current < self.source.len() {
            Some(self.source[self.current])
        } else {
            None
        }
    }

    fn peek_next(&self) -> Option<char> {
        if self.current + 1 < self.source.len() {
            Some(self.source[self.current + 1])
        } else {
            None
        }
    }

    fn advance(&mut self) -> Option<char> {
        if self.is_eof() {
            return None;
        }
        self.current += 1;
        Some(self.source[self.current - 1])
    }

    fn try_consume(&mut self, expect: char) -> bool {
        if let Some(peek) = self.peek() {
            if peek == expect {
                self.advance();
                return true;
            }
        }
        false
    }
}
