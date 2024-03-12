use std::fmt::Display;

use crate::object::{Downcast, ManagedReference, ObjectType, StringObject};

#[derive(Clone)]
pub enum Value {
    Nil,
    Number(f64),
    Boolean(bool),
    Object(ManagedReference),
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Nil => write!(f, "nil"),
            Value::Number(number) => write!(f, "{}", number),
            Value::Boolean(boolean) => write!(f, "{}", boolean),
            Value::Object(reference) => match reference.typ {
                ObjectType::String => {
                    let string_object: &StringObject = reference.downcast().unwrap();
                    write!(f, "\"{}\"", string_object)
                }
                #[allow(unreachable_patterns)]
                _ => write!(f, "<object at {:#x}>", reference.ptr()),
            },
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Nil, Self::Nil) => true,
            (Self::Number(left), Self::Number(right)) => (left - right).abs() < f64::EPSILON,
            (Self::Boolean(left), Self::Boolean(right)) => left == right,
            (Self::Object(left), Self::Object(right)) => {
                if left == right {
                    return true;
                }

                match (left.typ, right.typ) {
                    (ObjectType::String, ObjectType::String) => {
                        let left: &StringObject = left.downcast().unwrap();
                        let right: &StringObject = right.downcast().unwrap();
                        left == right
                    }
                }
            }

            _ => false,
        }
    }
}

impl Eq for Value {}
