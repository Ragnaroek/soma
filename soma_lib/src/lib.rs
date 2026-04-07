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

    pub fn value_at(&self, ix: usize) -> u8 {
        self.data[ix]
    }
}
