use crate::ROM;
use crate::io::IO;
use crate::memory::MemoryController;
use crate::sm83::{Debugger, SM83};

pub const RESOLUTION_X: usize = 166;
pub const RESOLUTION_Y: usize = 144;

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
    pub fn init(rom: ROM<'a>, time: Time<T>) -> DMG<'a, T> {
        let mut sm83 = SM83::init();
        sm83.set_pc(0x100);

        // allocate the DMG memory
        let mc = MemoryController {
            io: IO::init(),
            vram: [0; 8192],
            rom: Some(rom),
        };
        DMG { time, sm83, mc }
    }

    /// Run one step in the emulation. The returned value is the expected
    /// wait time for the next step call that must be awaited by the caller.
    pub fn step(&mut self) -> Result<u32, &'static str> {
        if self.sm83.halted() {
            return Err("Halted");
        }

        self.sm83.execute(&mut self.mc)?;

        // update IO according to time progress
        let now = (self.time.now)(&self.time.ref_time);
        let at_scanline = (now % VBLANK_SCANLINE_MILLIS) as u8;
        self.mc.write(0xFF44, at_scanline);
        Ok(14) // TODO compute wait time here for next step
    }

    pub fn attach_debugger(&mut self, debugger: Debugger) {
        self.sm83.attach_debugger(debugger);
    }
}
