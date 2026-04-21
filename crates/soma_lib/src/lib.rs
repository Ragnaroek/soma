#![no_std]

pub mod dmg;
pub mod io;
pub mod sm83;

pub struct ROM<'a> {
    data: &'a [u8],
}

/// A ROM can contain more than u16::MAX data (the maximum address space
/// of the SM83). This is why the indexes on read are usize on the ROM.
impl<'a> ROM<'a> {
    pub fn new(data: &'a [u8]) -> ROM<'a> {
        ROM { data }
    }

    pub fn read_u8(&self, ix: usize) -> u8 {
        self.data[ix]
    }

    pub fn read_u16(&self, ix: usize) -> u16 {
        u16::from_le_bytes([self.data[ix], self.data[ix + 1]])
    }
}
