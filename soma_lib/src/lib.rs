#![no_std]

pub mod dmg;

pub struct ROM<'a> {
    data: &'a [u8],
}

impl<'a> ROM<'a> {
    pub fn new(data: &'a [u8]) -> ROM<'a> {
        ROM { data }
    }
}
