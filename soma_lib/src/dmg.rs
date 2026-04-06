use crate::ROM;

pub struct DMG<'a> {
    rom: ROM<'a>,
}

impl<'a> DMG<'a> {
    /// Initialise a original gameboy system (DMG)
    pub fn init(rom: ROM) -> DMG {
        DMG { rom }
    }

    /// Run the ROM. This function does not terminate until
    /// the run is cancelled or execution ends with a HALT or
    /// execution error.
    pub fn run(&self) {
        // TODO
    }
}
