use std::fmt::Display;

use codespan_reporting::term::{
    self,
    termcolor::{ColorChoice, StandardStream},
    Config,
};

pub type SourceFileManager<N, S> = codespan_reporting::files::SimpleFiles<N, S>;
pub type ErrorItem = codespan_reporting::diagnostic::Diagnostic<usize>;
pub type Label = codespan_reporting::diagnostic::Label<usize>;

#[derive(Debug)]
pub enum InterpretError {
    Simple(ErrorItem),
    Compound(Vec<ErrorItem>),
}

impl InterpretError {
    pub fn emit<N, S>(self, files: &SourceFileManager<N, S>)
    where
        N: Display + Clone,
        S: AsRef<str>,
    {
        let stream = StandardStream::stderr(ColorChoice::Always);
        let stream = &mut stream.lock();
        let config = Config::default();

        match self {
            InterpretError::Simple(diagnostic) => term::emit(stream, &config, files, &diagnostic)
                .expect("internal diagnostic emission error"),
            InterpretError::Compound(diagnostics) => {
                for diagnostic in diagnostics {
                    term::emit(stream, &config, files, &diagnostic)
                        .expect("internal diagnostic emission error");
                }
            }
        }
    }
}

pub type InterpretResult<T = ()> = Result<T, InterpretError>;
