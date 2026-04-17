use crate::ROM;
use crate::sm83::{Debugger, SM83};

pub struct DMG<T> {
    time: Time<T>,
    sm83: SM83,
}

const CPU_FREQ: f64 = 4194304.0; // Hz
const VBLANK_FREQ: f64 = CPU_FREQ / 70224.0; // ~59.7 Hz
const VBLANK_SCANLINE_FREQ: f64 = VBLANK_FREQ / 154.0;
const VBLANK_SCANLINE_MILLIS: f64 = 1000.0 / VBLANK_SCANLINE_FREQ;

/// should return milliseconds elapsed since a reference time.
/// requirement is just monotonic increasing time, not absolute
/// time.
type RelativeTime<T> = fn(&T) -> f64;

pub struct Time<T> {
    pub ref_time: T,
    pub now: RelativeTime<T>,
}

impl<T> DMG<T> {
    /// Initialise a original gameboy system (DMG)
    pub fn init(time: Time<T>) -> DMG<T> {
        let mut sm83 = SM83::init();
        sm83.set_pc(0x100);
        DMG { time, sm83 }
    }

    /// Run the ROM. This function does not terminate until
    /// the run is cancelled or execution ends with a HALT or
    /// execution error.
    pub fn run(&mut self, rom: ROM) -> Result<(), &'static str> {
        while !self.sm83.halted() {
            self.sm83.execute(&rom)?;

            // update IO according to time progress
            let now = (self.time.now)(&self.time.ref_time);
            let at_scanline = (now % VBLANK_SCANLINE_MILLIS) as u8;
            self.sm83.mem_write(0xFF44, at_scanline);
        }
        Ok(())
    }

    pub fn attach_debugger(&mut self, debugger: Debugger) {
        self.sm83.attach_debugger(debugger);
    }
}
