use crate::io::IO;
use crate::rom::ROM;
use crate::sm83::ExecErr;

pub struct MemoryController {
    bank_selected: u16,

    /// 0x0000 to 0x3FFF ROM Bank 00
    /// 0x4000 to 0x7FFF ROM Bank NN (switchable banks)
    pub rom: Option<ROM>,

    /// 0x8000 to 0x8FFF Tiles/Sprites
    /// 0x9000 to 0x97FF Tiles At
    /// 0x9800 to 0x9BFF Tilemap 1
    /// 0x9C00 to 0x9FFF Tilemap 2
    pub vram: [u8; 8192],

    // 0xD000 to 0xDFFF 8KB Work RAM
    pub ram: [u8; 8192],

    // 0xFE00 to 0xFEFF 256 bytes Object Attribute Memory.
    // Including non usable memory from 0xFEA0 to 0xFEFF
    pub oam: [u8; 256],

    /// 0xFF00 to 0xFF7F I/O Ports
    pub io: IO,
}

// bank selection related things
const BANK_SIZE: u16 = 0x3FFF;
const ROM_BANK_CONTROL_START: u16 = 0x2000;
const ROM_BANK_CONTORL_END: u16 = 0x3FFF;

// mem space definition (inclusive intervals)
const ROM_0_END: u16 = 0x3FFF;
const ROM_N_START: u16 = 0x4000;
const ROM_N_END: u16 = 0x7FFF;
const VRAM_START: u16 = 0x8000;
const VRAM_END: u16 = 0x9FFF;
const RAM_START: u16 = 0xC000;
const RAM_END: u16 = 0xDFFF;
const OAM_START: u16 = 0xFE00;
const OAM_END: u16 = 0xFEFF;
const IO_START: u16 = 0xFF00;
const IO_END: u16 = 0xFFFF;

impl MemoryController {
    pub fn new(io: IO, rom: ROM) -> MemoryController {
        MemoryController {
            bank_selected: 0x01,
            rom: Some(rom),
            vram: [0; 8192],
            ram: [0; 8192],
            oam: [0; 256],
            io: io,
        }
    }

    pub fn read(&self, addr: u16) -> Result<u8, ExecErr> {
        if addr <= ROM_0_END {
            if let Some(rom) = &self.rom {
                rom.read_u8(addr as usize)
                    .map_err(|e| ExecErr::GeneralError(e))
            } else {
                return Err(ExecErr::GeneralError("no ROM attached"));
            }
        } else if addr >= ROM_N_START && addr <= ROM_N_END {
            let banked_rom_addr = addr + ((self.bank_selected - 1) * BANK_SIZE);
            if let Some(rom) = &self.rom {
                rom.read_u8(banked_rom_addr as usize)
                    .map_err(|e| ExecErr::GeneralError(e))
            } else {
                return Err(ExecErr::GeneralError("no ROM attached"));
            }
        } else if addr >= IO_START && addr <= IO_END {
            self.io.read(addr)
        } else if addr >= VRAM_START && addr <= VRAM_END {
            Ok(self.vram[(addr - VRAM_START) as usize])
        } else if addr >= RAM_START && addr <= RAM_END {
            Ok(self.ram[(addr - RAM_START) as usize])
        } else if addr >= OAM_START && addr <= OAM_END {
            Ok(self.oam[(addr - OAM_START) as usize])
        } else {
            return Err(ExecErr::GeneralError("mem read error"));
        }
    }

    /// Only possible from the ROM address space.
    pub fn read_u16(&self, addr: u16) -> Result<u16, ExecErr> {
        if addr < ROM_0_END {
            if let Some(rom) = &self.rom {
                return rom
                    .read_u16(addr as usize)
                    .map_err(|e| ExecErr::GeneralError(e));
            } else {
                return Err(ExecErr::GeneralError("no ROM attached"));
            }
        }
        return Err(ExecErr::GeneralError("mem read addr outside ROM space"));
    }

    pub fn write(&mut self, addr: u16, v: u8) -> Result<(), ExecErr> {
        if addr >= ROM_BANK_CONTROL_START && addr <= ROM_BANK_CONTORL_END {
            self.bank_selected = (v & 0x03) as u16;
        } else if addr >= IO_START && addr <= IO_END {
            self.io.write(addr, v)?;
        } else if addr >= VRAM_START && addr <= VRAM_END {
            self.vram[(addr - VRAM_START) as usize] = v;
        } else if addr >= RAM_START && addr <= RAM_END {
            self.ram[(addr - RAM_START) as usize] = v;
        } else if addr >= OAM_START && addr <= OAM_END {
            self.oam[(addr - OAM_START) as usize] = v;
        } else {
            return Err(ExecErr::GeneralError("memory location not writable"));
        }
        Ok(())
    }
}
