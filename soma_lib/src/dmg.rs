use psy::arch::sm83::decode;

use crate::{ROM, sm83::SM83};

pub struct DMG<'a> {
    rom: ROM<'a>,
    sm83: SM83,
}

impl<'a> DMG<'a> {
    /// Initialise a original gameboy system (DMG)
    pub fn init(rom: ROM) -> DMG {
        let mut sm83 = SM83::init();
        sm83.set_pc(0x100);
        DMG { rom, sm83 }
    }

    /// Run the ROM. This function does not terminate until
    /// the run is cancelled or execution ends with a HALT or
    /// execution error.
    pub fn run(&mut self) {
        let instr = decode(self.rom.value_at(self.sm83.pc() as usize));
        panic!("first instruction = {}", instr.mnemonic);
        // TODO
    }
}
