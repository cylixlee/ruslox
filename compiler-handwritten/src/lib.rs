use codespan_reporting::diagnostic::Diagnostic;
use scanner::Scanner;

mod parser;
mod scanner;

pub fn compile(source: &Vec<char>, file_id: usize) -> Result<(), Vec<Diagnostic<usize>>> {
    let mut scanner = Scanner::new(source, file_id);
    let mut errors = Vec::new();

    while !scanner.is_eof() {
        match scanner.scan() {
            Ok(token) => println!("{:?}", token),
            Err(e) => errors.push(e),
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }
    Ok(())
}
