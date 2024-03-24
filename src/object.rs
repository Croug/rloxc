use std::{cell::RefCell, fmt::{Debug, Display}, rc::Rc};

use crate::{chunk::Chunk, value::Value};

pub type NativeFn = fn(&mut [Value]) -> Value;

#[derive(Debug, PartialEq)]
pub enum Object {
    Function(Function),
    NativeFunction(NativeFn),
}

impl Display for Object {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Object::Function(func) => write!(f, "{}", func),
            Object::NativeFunction(_) => write!(f, "<native fn>"),
        }
    }
}

impl Object {
    pub fn as_function(&self) -> &Function {
        match self {
            Object::Function(func) => func,
            _ => panic!("Expected function object"),
        }
    }
    pub fn as_function_mut(&mut self) -> &mut Function {
        match self {
            Object::Function(func) => func,
            _ => panic!("Expected function object"),
        }
    }
    pub fn is_function(&self) -> bool {
        matches!(self, Object::Function(_)) || matches!(self, Object::NativeFunction(_))
    }
}

impl Into<Value> for Object {
    fn into(self) -> Value {
        Value::Object(Rc::new(RefCell::new(self)))
    }
}

pub struct Function {
    pub arity: usize,
    pub chunk: Chunk,
    pub name: String,
}

impl PartialEq for Function {
    fn eq(&self, other: &Self) -> bool {
        self.arity == other.arity && self.name == other.name
    }
}

impl Function {
    pub fn new() -> Self {
        Self {
            arity: 0,
            chunk: Chunk::new(),
            name: String::new(),
        }
    }

    pub fn get_chunk(&self) -> &Chunk {
        &self.chunk
    }

    pub fn get_chunk_mut(&mut self) -> &mut Chunk {
        &mut self.chunk
    }
}

impl Debug for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<fn {}>\n{:?}", if self.name.is_empty() { "<script>" } else { &self.name }, self.chunk)
    }
}

impl Display for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<fn {}>", if self.name.is_empty() { "<script>" } else { &self.name })
    }
}

impl Into<Chunk> for Function {
    fn into(self) -> Chunk {
        self.chunk
    }
}