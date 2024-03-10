use std::fmt::Display;

use shared::constant::Constant;

pub(crate) enum Value {
    Nil,
    Number(f64),
    Boolean(bool),
}

impl From<Constant> for Value {
    fn from(value: Constant) -> Self {
        match value {
            Constant::Number(number) => Self::Number(number),
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Nil => write!(f, "nil"),
            Value::Number(number) => write!(f, "{}", number),
            Value::Boolean(boolean) => write!(f, "{}", boolean),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Nil, Self::Nil) => true,
            (Self::Number(left), Self::Number(right)) => (left - right).abs() < f64::EPSILON,
            (Self::Boolean(left), Self::Boolean(right)) => left == right,

            _ => false,
        }
    }
}

impl Eq for Value {}
