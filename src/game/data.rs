use std::io::Write;

use cookie_factory::SerializeFn;

#[derive(Debug)]
pub struct BlockPosition {
    x: i32,
    z: i16,
    y: i32,
}

impl BlockPosition {
    pub(crate) fn generate<W: Write>(&self) -> impl SerializeFn<W> {
        |w| {
            cookie_factory::bytes::be_u64(
                (self.x as u64) << 38
                    | ((self.z as u64) & 0x3FFFFFF) << 12
                    | (self.y as u64) & 0xFFF,
            )(w)
        }
    }
}
