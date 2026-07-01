// Define a type alias for the selected size
#[cfg(feature = "max_rom_32kb")]
const MAX_ROM_SIZE: usize = 32 * 1024;

#[cfg(feature = "max_rom_512kb")]
const MAX_ROM_SIZE: usize = 512 * 1024;

pub struct ROM {
    pub data: [u8; MAX_ROM_SIZE],
}

/// A ROM can contain more than u16::MAX data (the maximum address space
/// of the SM83). This is why the indexes on read are usize on the ROM.
impl ROM {
    pub fn new_copy_from_slice(data_in: &[u8]) -> ROM {
        if data_in.len() > MAX_ROM_SIZE {
            panic!("ERR: Cannot create ROM, MAX_ROM_SIZE: {}", MAX_ROM_SIZE);
        }
        let mut data = [0; MAX_ROM_SIZE];
        data[..data_in.len()].copy_from_slice(data_in);
        ROM { data }
    }

    pub fn read_u8(&self, ix: usize) -> u8 {
        self.data[ix]
    }

    pub fn read_u16(&self, ix: usize) -> u16 {
        u16::from_le_bytes([self.data[ix], self.data[ix + 1]])
    }
}
