use psy::arch::sm83::{Sm83Instr, decode};

use crate::{ROM, sm83::SM83};

pub struct DMG<'a> {
    rom: ROM<'a>,
    sm83: SM83,

    debugger: Option<Debugger>,
}

pub struct Debugger {
    debug: fn(&Sm83Instr, &DMG),
}

impl Debugger {
    pub fn new(debug: fn(&Sm83Instr, &DMG)) -> Debugger {
        Debugger { debug }
    }
}

impl<'a> DMG<'a> {
    /// Initialise a original gameboy system (DMG)
    pub fn init(rom: ROM) -> DMG {
        let mut sm83 = SM83::init();
        sm83.set_pc(0x100);
        DMG {
            rom,
            sm83,
            debugger: None,
        }
    }

    /// Run the ROM. This function does not terminate until
    /// the run is cancelled or execution ends with a HALT or
    /// execution error.
    pub fn run(&mut self) {
        let instr = decode(self.rom.value_at(self.sm83.pc() as usize));
        // TODO execute instruction

        if let Some(debugger) = &self.debugger {
            (debugger.debug)(instr, self)
        }
    }

    pub fn attach_debugger(&mut self, debugger: Debugger) {
        self.debugger = Some(debugger)
    }
}
