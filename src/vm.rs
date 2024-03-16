use std::result;

use crate::chunk::{Chunk, OpCode, Value};

pub enum InterpretError {
    CompileError,
    RuntimeError,
}

type Result = std::result::Result<(), InterpretError>;

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

    pub fn interpret(&mut self, chunk: Chunk) -> Result {
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
    
    fn run(&mut self) -> Result {
        loop {
            let instruction = self.read_instruction();

            #[cfg(debug)] {
                print!("\t");
                for value in &self.stack {
                    print!("[ {} ]", value);
                }
                println!();
                println!("{:?}", instruction);
            }

            match instruction {
                OpCode::Return => {
                    println!("{}", self.pop().unwrap());
                    return Ok(());
                }
                OpCode::Constant(index) => {
                    let chunk = self.get_chunk()?;
                    let value = chunk.get_constant(index);
                    self.push(value);
                }
            }
        }
    }
}