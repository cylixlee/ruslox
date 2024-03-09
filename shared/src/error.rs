use std::{
    error::Error,
    fmt::{Debug, Display},
    io,
};

pub enum InterpretError {
    IOError(io::Error),
    CompileError(String, Option<(usize, usize)>),
    RuntimeError(String),
    CompoundError(Vec<InterpretError>),
}

impl From<io::Error> for InterpretError {
    fn from(value: io::Error) -> Self {
        Self::IOError(value)
    }
}

impl Display for InterpretError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InterpretError::IOError(err) => write!(f, "{}", err),
            InterpretError::CompileError(message, position) => match position {
                Some((line, column)) => write!(
                    f,
                    "Compile Error at line {}, column {}: {}",
                    *line, *column, message
                ),
                None => write!(f, "Compile Error: {}", message),
            },
            InterpretError::RuntimeError(message) => {
                write!(f, "Runtime Error: {}", message)
            }
            InterpretError::CompoundError(compound) => {
                for error in compound {
                    writeln!(f, "{}", error)?;
                }
                Ok(())
            }
        }
    }
}

impl Debug for InterpretError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <Self as Display>::fmt(&self, f)
    }
}

impl Error for InterpretError {}

pub type InterpretResult<T = ()> = Result<T, InterpretError>;
