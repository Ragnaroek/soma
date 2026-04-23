use crate::{ROM, io::IO};

pub struct MemoryController<'a> {
    /// 0x0000 to 0x3FFF ROM Bank 00
    /// 0x4000 to 0x7FFF ROM Bank NN (switchable banks)
    pub rom: Option<ROM<'a>>,

    /// 0x8000 to 0x8FFF Tiles/Sprites
    /// 0x9000 to 0x97FF Tiles At
    /// 0x9800 to 0x9BFF Tilemap 1
    /// 0x9C00 to 0x9FFF Tilemap 2
    pub vram: [u8; 8192],

    /// 0xFF00 to 0xFF7F I/O Ports
    pub io: IO,
}

// mem space definition (inclusive intervals)
const ROM_0_END: u16 = 0x3FFF;
const VRAM_START: u16 = 0x8000;
const VRAM_END: u16 = 0x9FFF;
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
        } else if addr >= VRAM_START && addr <= VRAM_END {
            self.vram[(addr - VRAM_START) as usize] = v;
        } else {
            panic!("mem write error");
        }
    }
}
