use std::fmt::{self, Debug, Formatter};



#[repr(u8)]
#[derive(Debug, Copy, Clone)]
pub enum OpCode {
    Constant(usize),
    Return,
}

pub type Value = f64;

pub struct Chunk {
    pub code: Vec<OpCode>,
    lines: Vec<usize>,
    constants: Vec<Value>,
}

impl OpCode {
    pub fn to_string_resolved(&self, chunk: &Chunk) -> String {
        match self {
            OpCode::Constant(index) => {
                let value = chunk.constants[*index];
                format!("OP_CONSTANT({}:{})", index, value)
            }
            OpCode::Return => "OP_RETURN".to_string(),
        }
    }
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