use std::fmt::Display;

#[derive(Clone)]
pub enum Value {
    Boolean(bool),
    Nil,
    Number(f64),
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Number(number) => write!(f, "{}", number),
            Value::Boolean(boolean) => write!(f, "{}", boolean),
            Value::Nil => write!(f, "nil"),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Boolean(left), Self::Boolean(right)) => left == right,
            (Self::Number(left), Self::Number(right)) => (left - right).abs() < f64::EPSILON,
            (Self::Nil, Self::Nil) => true,

            _ => false,
        }
    }
}

impl Eq for Value {}
