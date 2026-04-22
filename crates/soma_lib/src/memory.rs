use crate::{ROM, io::IO};

pub struct MemoryController<'a> {
    pub rom: Option<ROM<'a>>,
    pub io: IO,
}

// mem space definition (inclusive intervals)
const ROM_0_END: u16 = 0x3FFF;
const IO_START: u16 = 0xFF00;
const IO_END: u16 = 0xFFFF;

impl<'a> MemoryController<'a> {
    pub fn read(&self, addr: u16) -> u8 {
        if addr <= ROM_0_END {
            if let Some(rom) = &self.rom {
                rom.read_u8(addr as usize)
            } else {
                panic!("no ROM attached")
            }
        } else if addr >= IO_START && addr <= IO_END {
            self.io.read(addr)
        } else {
            panic!("mem read error");
        }
    }

    /// Only possible from the ROM address space.
    pub fn read_u16(&self, addr: u16) -> u16 {
        if addr < ROM_0_END {
            if let Some(rom) = &self.rom {
                return rom.read_u16(addr as usize);
            } else {
                panic!("no ROM attached")
            }
        }
        panic!("mem read double outside ROM space")
    }

    pub fn write(&mut self, addr: u16, v: u8) {
        if addr >= IO_START && addr <= IO_END {
            self.io.write(addr, v);
        } else {
            panic!("mem write error");
        }
    }
}
