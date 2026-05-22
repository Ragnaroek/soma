use psy::arch::sm83::Sm83Instr;

use crate::ROM;
use crate::io::IO;
use crate::memory::MemoryController;
use crate::sm83::SM83;

pub const RESOLUTION_X: usize = 166;
pub const RESOLUTION_Y: usize = 144;

pub struct DMG<'a, T> {
    time: Time<T>,
    pub sm83: SM83,
    pub mc: MemoryController<'a>,
    debug: fn(&str, u16),
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

pub struct StepResult {
    /// The time the caller should wait for the next step to simulate
    /// the instruction timing.
    pub wait_time_millis: u32,
    /// The instruction that was executed in the step
    pub instr: &'static Sm83Instr,
}

impl<'a, T> DMG<'a, T> {
    /// Initialise a original gameboy system (DMG)
    pub fn init(rom: ROM<'a>, time: Time<T>, debug: fn(&str, u16)) -> DMG<'a, T> {
        let mut sm83 = SM83::init();
        sm83.set_pc(0x100);

        // allocate the DMG memory
        let mc = MemoryController {
            io: IO::init(),
            vram: [0; 8192],
            rom: Some(rom),
        };
        DMG {
            time,
            sm83,
            mc,
            debug,
        }
    }

    /// Run one step in the emulation. The returned value is the expected
    /// wait time for the next step call that must be awaited by the caller.
    pub fn step(&mut self) -> Result<StepResult, &'static str> {
        if self.sm83.halted() {
            return Err("Halted");
        }

        let instr = self.sm83.execute(&mut self.mc)?;

        // update IO according to time progress
        let now = (self.time.now)(&self.time.ref_time);
        let at_scanline = (now % VBLANK_SCANLINE_MILLIS) as u8;
        self.mc.write(0xFF44, at_scanline);
        Ok(StepResult {
            wait_time_millis: 1, //14, // TODO compute wait time here for next step
            instr,
        })
    }

    /// Write the current content of the video-ram to the suppplied
    /// framebuffer in RGB format.
    /// The size needs to be at least 20 x 18 x 64 x 3 bytes.
    /// 1.474.560 pixel
    pub fn fb_rgb(&self, fb: &mut [u8]) {
        // 20x18 only visible, only render that and figure out
        // how the window is slided
        for y in 0..32usize {
            for x in 0..32usize {
                if x == 0 {
                    (self.debug)("tile_start", 0);
                }
                // upper left corner of tile
                let mut dst = (y * 8 * 3 * 32 * 8) + (x * 8 * 3);

                let tile_map_addr = 0x9800 + x as u16 * 32 + y as u16;
                let tile_map_ix = self.mc.read(tile_map_addr) as u16;
                if x == 0 {
                    (self.debug)("map_addr", tile_map_addr);
                    (self.debug)("tile_ix", tile_map_ix);
                }
                let tile_start = 0x9000 + tile_map_ix * 16;

                let mut tile_i = tile_start;
                for _tile_row in 0..8 {
                    let col_0 = self.mc.read(tile_i);
                    tile_i += 1;
                    let col_1 = self.mc.read(tile_i);
                    tile_i += 1;

                    for tile_col in (0..8).rev() {
                        let bit_mask = 1 << tile_col;

                        let mut p = 0b00;
                        if col_0 & bit_mask != 0 {
                            p = 0b10;
                        }
                        if col_1 & bit_mask != 0 {
                            p |= 0b01;
                        }

                        match p {
                            // TODO this can be done with a 4x3 static array with the colour data!
                            0 => {
                                fb[dst + 0] = 0xEA;
                                fb[dst + 1] = 0xEA;
                                fb[dst + 2] = 0xEA;
                            }
                            1 => {
                                fb[dst + 0] = 0x91;
                                fb[dst + 1] = 0xCC;
                                fb[dst + 2] = 0x78;
                            }
                            2 => {
                                fb[dst + 0] = 0x51;
                                fb[dst + 1] = 0x8C;
                                fb[dst + 2] = 0x52;
                            }
                            3 => {
                                fb[dst + 0] = 0x1F;
                                fb[dst + 1] = 0x60;
                                fb[dst + 2] = 0x18;
                            }
                            _ => {
                                unreachable!("2 bits per pixel guaranteed through the shift")
                            }
                        }
                        dst += 3;
                    }

                    dst += 31 * 8 * 3;
                }
                if x == 0 {
                    (self.debug)("tile_end", 0);
                }
            }
        }
    }
}
