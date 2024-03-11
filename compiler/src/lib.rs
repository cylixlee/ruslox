use codespan_reporting::diagnostic::Diagnostic;
use shared::chunk::Chunk;

mod parser;
mod scanner;

pub fn compile(file_id: usize, source: impl AsRef<str>) -> Result<Chunk, Vec<Diagnostic<usize>>> {
    let scanned = scanner::scan(file_id, source.as_ref());
    if !scanned.diagnostics.is_empty() {
        return Err(scanned.diagnostics);
    }
    parser::parse(&scanned)
}
