use core::ops::{Index, Range};

#[cfg(feature = "max_rom_512b")]
const MAX_ROM_SIZE: usize = 512;

#[cfg(feature = "max_rom_16kb")]
const MAX_ROM_SIZE: usize = 16 * 1024;

#[cfg(feature = "max_rom_32kb")]
const MAX_ROM_SIZE: usize = 32 * 1024;

#[cfg(feature = "max_rom_512kb")]
const MAX_ROM_SIZE: usize = 512 * 1024;

#[cfg(not(any(
    feature = "max_rom_512b",
    feature = "max_rom_16kb",
    feature = "max_rom_32kb",
    feature = "max_rom_512kb"
)))]
const MAX_ROM_SIZE: usize = 16 * 1024;

pub struct ROM {
    data_buffer: [u8; MAX_ROM_SIZE],
    size: usize,
}

/// A ROM can contain more than u16::MAX data (the maximum address space
/// of the SM83). This is why the indexes on read are usize on the ROM.
impl ROM {
    pub fn new_copy_from_slice(data_in: &[u8]) -> ROM {
        if data_in.len() > MAX_ROM_SIZE {
            panic!(
                "ERR: Cannot create ROM, MAX_ROM_SIZE: {}, requested: {}",
                MAX_ROM_SIZE,
                data_in.len()
            );
        }
        let mut data_buffer = [0; MAX_ROM_SIZE];
        data_buffer[..data_in.len()].copy_from_slice(data_in);
        ROM {
            data_buffer,
            size: data_in.len(),
        }
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn read_u8(&self, ix: usize) -> Result<u8, &'static str> {
        Ok(self.data_buffer[ix])
    }

    pub fn read_u16(&self, ix: usize) -> Result<u16, &'static str> {
        Ok(u16::from_le_bytes([
            self.data_buffer[ix],
            self.data_buffer[ix + 1],
        ]))
    }
}

impl Index<usize> for ROM {
    type Output = u8;

    fn index(&self, index: usize) -> &u8 {
        &self.data_buffer[index]
    }
}

impl Index<Range<usize>> for ROM {
    type Output = [u8];

    fn index(&self, index: core::ops::Range<usize>) -> &[u8] {
        &self.data_buffer[index]
    }
}
