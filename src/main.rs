use chunk::Chunk;

use crate::chunk::OpCode;

mod chunk;
mod vm;

fn main() {
    let mut chunk = Chunk::new();
    let constant = chunk.set_constant(1.2);
    chunk.write_chunk(OpCode::Constant(constant), 0);
    chunk.write_chunk(OpCode::Return, 0);
    _ = vm::VM::new().interpret(chunk);
}
