use std::result;

use crate::{chunk::{Chunk, OpCode}, compiler, value::Value};

#[derive(Debug)]
pub enum InterpretError {
    CompileError,
    RuntimeError,
}

pub(crate) type Result<T> = std::result::Result<T, InterpretError>;

macro_rules! runtime_error {
    ($vm:ident, $( $arg:tt )*) => {
        {
            eprintln!($( $arg )*);
            $vm._error();
        }
    };
}

macro_rules! binary_op {
    ($vm:ident, $discriminant:ident, $op:tt) => {
        {
            let b = $vm.pop().unwrap();
            let a = $vm.pop().unwrap();
            if let (Value::Number(a), Value::Number(b)) = (a, b) {
                $vm.push(Value::$discriminant(a $op b));
            } else {
                runtime_error!($vm, "Operands must be numbers.");
                return Err(InterpretError::RuntimeError);
            }
        }
    };
}
pub struct VM {
    chunk: Option<Chunk>,
    ip: usize,
    stack: Vec<Value>,
}

impl VM {
    pub fn new() -> Self {
        Self {
            chunk: None,
            ip: 0,
            stack: Vec::new(),
        }
    }

    fn read_instruction(&mut self) -> OpCode {
        self.ip += 1;
        self.chunk.as_ref().unwrap().code[self.ip - 1]
    }

    pub fn interpret_source(&mut self, source: &str) -> Result<()> {
        self.interpret_chunk(compiler::compile(source)?)
    }

    pub fn interpret_chunk(&mut self, chunk: Chunk) -> Result<()> {
        self.chunk = Some(chunk);
        self.ip = 0;

        self.run()
    }
    
    fn get_chunk(&mut self) -> result::Result<&mut Chunk, InterpretError> {
        match self.chunk.as_mut() {
            Some(chunk) => Ok(chunk),
            None => Err(InterpretError::RuntimeError),
        }
    }
    
    fn push(&mut self, value: Value) {
        self.stack.push(value);
    }

    fn pop(&mut self) -> Option<Value> {
        self.stack.pop()
    }
    
    fn peek(&mut self, distance: usize) -> &Value {
        &self.stack[self.stack.len() - 1 - distance]
    }
    
    fn _error(&mut self) {
        let line = self.chunk.as_ref().unwrap().line(self.ip - 1);
        eprintln!("[line {}] in script", line);
    }
    
    fn run(&mut self) -> Result<()> {
        loop {
            let instruction = self.read_instruction();

            #[cfg(debug_trace_execution)] {
                print!("\t");
                for value in &self.stack {
                    print!("[ {} ]", value);
                }
                println!();
                println!("{:?}", instruction);
            }

            match instruction {
                OpCode::Return => {
                    println!("{:?}", self.pop());
                    return Ok(());
                }
                OpCode::Add => binary_op!(self, Number, +),
                OpCode::Subtract => binary_op!(self, Number, -),
                OpCode::Multiply => binary_op!(self, Number, *),
                OpCode::Divide => binary_op!(self, Number, /),
                OpCode::Not => {
                    let value = self.pop().unwrap();
                    self.push(Value::Bool(!value.truthy()));
                }
                OpCode::Negate => {
                    let value = self.pop().unwrap();
                    if let Value::Number(n) = value {
                        self.push(Value::Number(-n));
                    } else {
                        runtime_error!(self, "Operand must be a number.");
                        return Err(InterpretError::RuntimeError);
                    }
                }
                OpCode::Constant(index) => {
                    let chunk = self.get_chunk()?;
                    let value = chunk.get_constant(index);
                    self.push(value);
                }
                OpCode::Nil => self.push(Value::Nil),
                OpCode::True => self.push(Value::Bool(true)),
                OpCode::False => self.push(Value::Bool(false)),
                OpCode::Equal => {
                    let b = self.pop().unwrap();
                    let a = self.pop().unwrap();
                    self.push(Value::Bool(a == b));
                }
                OpCode::Greater => binary_op!(self, Bool, >),
                OpCode::Less => binary_op!(self, Bool, <),
            }
        }
    }
}