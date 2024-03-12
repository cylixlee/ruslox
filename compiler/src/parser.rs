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

enum Expression<'a> {
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

enum Statement<'a> {
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

pub fn parse(file_id: usize, scanned: &ScannedContext) -> Result<Chunk, Vec<Diagnostic<usize>>> {
    let parser = RefCell::new(Parser::new());
    let declarations = pegparser::declarations(scanned, file_id, &scanned.positions, &parser)
        .expect("internal parse error");
    let parser = parser.into_inner();
    match !parser.diagnostics.is_empty() {
        true => Err(parser.diagnostics),
        false => {
            let mut chunk = Chunk::new();
            match emit(&mut chunk, &declarations) {
                Ok(_) => {
                    chunk.write(Instruction::Return);
                    Ok(chunk)
                }
                Err(diagnostic) => Err(vec![diagnostic]),
            }
        }
    }
}

fn emit(chunk: &mut Chunk, declarations: &Vec<Statement>) -> Result<(), Diagnostic<usize>> {
    for declaration in declarations {
        emit_statement(chunk, declaration)?;
    }
    Ok(())
}

fn emit_statement(chunk: &mut Chunk, statement: &Statement) -> Result<(), Diagnostic<usize>> {
    match statement {
        Statement::VarDeclaration(name, initializer) => {
            let index = chunk.add_constant(Constant::String((*name).clone()))?;
            match initializer {
                Some(expression) => emit_expression(chunk, expression)?,
                None => chunk.write(Instruction::Nil),
            };
            chunk.write(Instruction::DefineGlobal(index));
        }
        Statement::Print(expression) => {
            emit_expression(chunk, expression)?;
            chunk.write(Instruction::Print);
        }
        Statement::Expressional(expression) => {
            emit_expression(chunk, expression)?;
            chunk.write(Instruction::Pop);
        }
        Statement::Error => unreachable!("still trying to emit after reporting diagnostics"),
    }
    Ok(())
}

fn emit_expression(chunk: &mut Chunk, expression: &Expression) -> Result<(), Diagnostic<usize>> {
    match expression {
        Expression::String(string) => emit_constant(chunk, Constant::String((*string).clone()))?,
        Expression::Number(number) => emit_constant(chunk, Constant::Number(*number))?,
        Expression::Identifier(identifier) => {
            let index = chunk.add_constant(Constant::String((*identifier).clone()))?;
            chunk.write(Instruction::GetGlobal(index));
        }
        Expression::True => chunk.write(Instruction::True),
        Expression::False => chunk.write(Instruction::False),
        Expression::Nil => chunk.write(Instruction::Nil),
        Expression::Unary(operator, expression) => {
            emit_expression(chunk, expression)?;
            match operator {
                Token::Minus => chunk.write(Instruction::Negate),
                Token::Bang => chunk.write(Instruction::Not),
                _ => unreachable!("emit failure due to parse error at unary expressions."),
            }
        }
        Expression::Binary(left, operator, right) => {
            if let Token::Equal = operator {
                match &**left {
                    Expression::Identifier(identifier) => {
                        let index = chunk.add_constant(Constant::String((*identifier).clone()))?;
                        emit_expression(chunk, &right)?;
                        chunk.write(Instruction::SetGlobal(index));
                    }
                    _ => {
                        return Err(Diagnostic::error()
                            .with_code("E0008")
                            .with_message("invalid assignment target"))
                    }
                }
            } else {
                emit_expression(chunk, &left)?;
                emit_expression(chunk, &right)?;
            }
        }
    }
    Ok(())
}

fn emit_constant(chunk: &mut Chunk, constant: Constant) -> Result<(), Diagnostic<usize>> {
    let index = chunk.add_constant(constant)?;
    chunk.write(Instruction::Constant(index));
    Ok(())
}
