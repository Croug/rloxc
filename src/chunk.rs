use std::fmt::{self, Debug, Formatter};

use crate::value::Value;

#[repr(u8)]
#[derive(Debug, Copy, Clone)]
pub enum OpCode {
    Constant(usize),
    Nil,
    True,
    False,
    Equal,
    Greater,
    Less,
    Add,
    Subtract,
    Multiply,
    Divide,
    Not,
    Negate,
    Return,
}

impl OpCode {
    pub fn to_string_resolved(&self, chunk: &Chunk) -> String {
        match self {
            OpCode::Constant(index) => {
                let value = &chunk.constants[*index];
                format!("OP_CONSTANT({}:{})", index, value)
            }
            OpCode::Nil => "OP_NIL".to_string(),
            OpCode::True => "OP_TRUE".to_string(),
            OpCode::False => "OP_FALSE".to_string(),
            OpCode::Equal => "OP_EQUAL".to_string(),
            OpCode::Greater => "OP_GREATER".to_string(),
            OpCode::Less => "OP_LESS".to_string(),
            OpCode::Add => "OP_ADD".to_string(),
            OpCode::Subtract => "OP_SUBTRACT".to_string(),
            OpCode::Multiply => "OP_MULTIPLY".to_string(),
            OpCode::Divide => "OP_DIVIDE".to_string(),
            OpCode::Not => "OP_NOT".to_string(),
            OpCode::Negate => "OP_NEGATE".to_string(),
            OpCode::Return => "OP_RETURN".to_string(),
        }
    }
}

pub struct Chunk {
    pub code: Vec<OpCode>,
    lines: Vec<usize>,
    constants: Vec<Value>,
}

impl Debug for Chunk {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "{txt:=^30}", txt = " BEGIN CHUNK ")?;
        writeln!(f, "LINE SRCLN OP_CODE")?;

        let mut last_line = usize::MAX;
        for (i, op) in self.code.iter().enumerate() {
            let line = self.lines[i];
            let line_text = if line == last_line {
                "   ||".to_string()
            } else {
                format!("{line:0>5}")
            };
            last_line = line;
            writeln!(f, "{i:0>4} {line} {op}", i = i, op = op.to_string_resolved(&self), line = line_text)?;
        }

        writeln!(f, "{txt:=^30}", txt = " END CHUNK ")
    }
}

impl Chunk {
    pub fn new() -> Self {
        Self {
            code: Vec::new(),
            lines: Vec::new(),
            constants: Vec::new(),
        }
    }

    pub fn line(&self, offset: usize) -> usize {
        self.lines[offset]
    }

    pub fn write_chunk(&mut self, byte: OpCode, line: usize) {
        self.code.push(byte);
        self.lines.push(line);
    }

    pub fn set_constant(&mut self, value: Value) -> usize {
        self.constants.push(value);
        self.constants.len() - 1
    }

    pub fn get_constant(&mut self, index: usize) -> Value {
        self.constants[index]
    }
}