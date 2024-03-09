use std::fmt::Display;

#[derive(Clone)]
pub enum Constant {
    Number(f64),
}

impl Display for Constant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Constant::Number(number) => write!(f, "{}", number),
        }
    }
}
