use crate::ROM;
use crate::sm83::{Debugger, SM83};

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
        while !self.sm83.halted() {
            self.sm83.execute(&self.rom);
        }
    }

    pub fn attach_debugger(&mut self, debugger: Debugger) {
        self.sm83.attach_debugger(debugger);
    }
}
