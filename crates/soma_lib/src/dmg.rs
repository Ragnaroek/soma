use crate::ROM;
use crate::io::IO;
use crate::memory::MemoryController;
use crate::sm83::{Debugger, SM83};

pub struct DMG<'a, T> {
    time: Time<T>,
    sm83: SM83,
    mc: MemoryController<'a>,
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

impl<'a, T> DMG<'a, T> {
    /// Initialise a original gameboy system (DMG)
    pub fn init(time: Time<T>) -> DMG<'a, T> {
        let mut sm83 = SM83::init();
        sm83.set_pc(0x100);
        let mc = MemoryController {
            io: IO::init(),
            rom: None,
        };
        DMG { time, sm83, mc }
    }

    /// Run the ROM. This function does not terminate until
    /// the run is cancelled or execution ends with a HALT or
    /// execution error.
    pub fn run(&mut self, rom: ROM<'a>) -> Result<(), &'static str> {
        self.mc.rom = Some(rom);
        while !self.sm83.halted() {
            self.sm83.execute(&mut self.mc)?;

            // update IO according to time progress
            let now = (self.time.now)(&self.time.ref_time);
            let at_scanline = (now % VBLANK_SCANLINE_MILLIS) as u8;
            self.mc.write(0xFF44, at_scanline);
        }
        Ok(())
    }

    pub fn attach_debugger(&mut self, debugger: Debugger) {
        self.sm83.attach_debugger(debugger);
    }
}
