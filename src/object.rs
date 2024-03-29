use std::{borrow::BorrowMut, cell::RefCell, fmt::{Debug, Display}, ops::Deref, rc::Rc};

use crate::{chunk::Chunk, compiler, value::Value, vm::VM};

pub type NativeFn = fn(&mut [Value]) -> Value;

#[derive(Debug, PartialEq)]
pub enum Object {
    Closure(Closure),
    NativeFunction(NativeFn),
}

impl Display for Object {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Object::Closure(func) => write!(f, "{}", func),
            Object::NativeFunction(_) => write!(f, "<native fn>"),
        }
    }
}

impl Object {
    pub fn as_function(&self) -> &Function {
        match self {
            Object::Closure(closure) => closure.function.as_ref(),
            _ => panic!("Expected function object"),
        }
    }

    pub fn as_closure(&self) -> &Closure {
        match self {
            Object::Closure(closure) => closure,
            _ => panic!("Expected closure object"),
        }
    }

    pub fn as_closure_mut(&mut self) -> &mut Closure {
        match self {
            Object::Closure(closure) => closure,
            _ => panic!("Expected closure object"),
        }
    }

    pub fn is_function(&self) -> bool {
        matches!(self, Object::Closure(_)) || matches!(self, Object::NativeFunction(_))
    }
}

impl Into<Value> for Object {
    fn into(self) -> Value {
        Value::Object(Rc::new(RefCell::new(self)))
    }
}

#[derive(Debug, PartialEq)]
pub enum Upvalue {
    LocalOpen(usize),
    ForeignOpen(usize),
    Closed(Value),
}

impl Upvalue {
    pub fn get_value(&self, vm: &VM) -> Value {
        match self {
            Upvalue::LocalOpen(index) | Upvalue::ForeignOpen(index) => vm.get_value(*index),
            Upvalue::Closed(value) => value.clone(),
        }
    }

    pub fn set_value(&mut self, vm: &mut VM, value: Value) {
        match self {
            Upvalue::LocalOpen(index) | Upvalue::ForeignOpen(index) => vm.set_value(*index, value),
            Upvalue::Closed(closed) => *closed = value,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Closure {
    pub function: Rc<Function>,
    pub upvalues: Vec<Rc<RefCell<Upvalue>>>,
}

impl Closure {
    pub fn new(function: Rc<Function>) -> Self {
        Self {
            function,
            upvalues: Vec::new(),
        }
    }
}

impl Display for Closure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.function)
    }
}

pub struct Function {
    pub arity: usize,
    pub chunk: Chunk,
    pub name: String,
    pub upvalues: Vec<compiler::Upvalue>,
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
            upvalues: Vec::new(),
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