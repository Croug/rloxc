use chunk::Chunk;

use crate::chunk::OpCode;

mod chunk;

fn main() {
    let mut chunk = Chunk::new();
    let constant = chunk.set_constant(1.2);
    chunk.write_chunk(OpCode::Constant(constant), 0);
    chunk.write_chunk(OpCode::Return, 0);
    println!("{:?}", chunk);
}
