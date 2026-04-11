#![no_std]

pub mod dmg;
pub mod sm83;

pub struct ROM<'a> {
    data: &'a [u8],
}

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
