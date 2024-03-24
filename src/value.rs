use std::{cell::RefCell, fmt::Display, rc::Rc};

use crate::object::Object;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Bool(bool),
    Nil,
    Number(f64),
    String(String),
    Object(Rc<RefCell<Object>>),
}

impl Value {
    pub fn truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Nil => false,
            Value::Number(n) => *n != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::Object(_) => true,
        }
    }

    pub fn type_name(&self) -> &str {
        match self {
            Value::Bool(_) => "bool",
            Value::Nil => "nil",
            Value::Number(_) => "number",
            Value::String(_) => "string",
            Value::Object(_) => "object",
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Bool(b) => write!(f, "{}", b),
            Value::Nil => write!(f, "nil"),
            Value::Number(n) => write!(f, "{}", n),
            Value::String(s) => write!(f, "{}", s),
            Value::Object(o) => write!(f, "{}", o.borrow()),
        }
    }
}