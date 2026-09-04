use psy::arch::sm83::Sm83Instr;

use crate::io::IO;
use crate::memory::MemoryController;
use crate::rom::ROM;
use crate::sm83::{ExecErr, SM83};

pub const RESOLUTION_X: usize = 160;
pub const RESOLUTION_Y: usize = 144;

const NUM_TILES_WIDTH: usize = 20;
const NUM_TILES_HEIGHT: usize = 18;
const TILE_DIM: usize = 8;
const TILE_WIDTH_BYTES: usize = TILE_DIM * 3;
const TILE_HEIGHT_BYTES: usize = TILE_DIM * 3;

pub struct DMG<T> {
    time: Time<T>,
    pub last_refresh_at: f64,

    pub sm83: SM83,
    pub mc: MemoryController,
    debug: fn(&str, u16),
}

const CPU_FREQ: f64 = 4194304.0; // Hz
/// One cyle in the CPU takes this amount of ms
const CPU_CYCLE_MILLIS: f64 = 1000.0 / CPU_FREQ; // ~0.0002384185791015625 ms
/// including the VBLANK period lines 144 to 153s
const LC_H_LINES: f64 = 154.0;
/// Amount of cycle one line render takes in the LC. In reference
/// to the CPU frequency and cycles
const LC_H_LINE_NUM_CYCLE: f64 = 456.0;
/// Milliseconds it takes to render one line to the LC.
const LC_H_LINE_MILLIS: f64 = LC_H_LINE_NUM_CYCLE * CPU_CYCLE_MILLIS;

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
    pub wait_time_millis: f64,
    /// The instruction that was executed in the step
    pub pc: u16,
    pub instr: &'static Sm83Instr,
    pub fb_refresh: bool,
}

impl<T> DMG<T> {
    /// Initialise a original gameboy system (DMG)
    pub fn init(rom: ROM, time: Time<T>, debug: fn(&str, u16)) -> DMG<T> {
        let mut sm83 = SM83::init();
        sm83.set_pc(0x100);
        sm83.reg.sp = 0xFFFE;

        // allocate the DMG memory
        let mc = MemoryController::new(IO::init(), rom);

        DMG {
            last_refresh_at: 0.0,
            time,
            sm83,
            mc,
            debug,
        }
    }

    /// Run one step in the emulation. The returned value is the expected
    /// wait time for the next step call that must be awaited by the caller.
    pub fn step(&mut self) -> Result<StepResult, ExecErr> {
        if self.sm83.halted() {
            return Err(ExecErr::GeneralError("Halted"));
        }

        let pc_before = self.sm83.pc();
        let instr = self.sm83.execute(&mut self.mc)?;

        // update IO according to time progress
        let now = (self.time.now)(&self.time.ref_time);

        let h_line = ((now / LC_H_LINE_MILLIS) % LC_H_LINES) as u8;
        self.mc.write(0xFF44, h_line)?;

        (self.debug)("scanline: ", h_line as u16);

        if h_line == 144 {
            // VLANK start
            panic!("vblank start");
        }

        let fb_refresh = if (now - self.last_refresh_at) > 14.0 {
            self.last_refresh_at = now;
            true
        } else {
            false
        };

        Ok(StepResult {
            wait_time_millis: 0.0, //14, // TODO compute wait time here for next step
            pc: pc_before,
            instr,
            fb_refresh,
        })
    }

    /// Write the current content of the video-ram to the suppplied
    /// framebuffer in RGB format.
    /// The size needs to be at least 20 x 18 x 64 x 3 bytes.
    /// 1.474.560 pixel
    pub fn fb_rgb(&self, fb: &mut [u8]) -> Result<(), ExecErr> {
        // 20x18 only visible, only render that and figure out
        // how the window is slided

        for y in 0..18 {
            //32usize {
            for x in 0..20 {
                //32usize {
                // upper left corner of tile
                // x = 12, y = 17
                let mut dst =
                    (y * NUM_TILES_WIDTH * TILE_HEIGHT_BYTES * TILE_DIM) + (x * TILE_WIDTH_BYTES);

                //                if x == 19 && y == 0 {
                //                    panic!("first dest = {}", dst);
                //                }

                let tile_map_addr = 0x9800 + y as u16 * NUM_TILES_WIDTH as u16 + x as u16;
                let tile_map_ix = self.mc.read(tile_map_addr)? as u16;
                let tile_start = self.tile_start_offset()? + tile_map_ix * 16;

                let mut tile_i = tile_start;
                for tile_row in 0..8 {
                    let col_0 = self.mc.read(tile_i)?;
                    tile_i += 1;
                    let col_1 = self.mc.read(tile_i)?;
                    tile_i += 1;

                    for tile_col in (0..8).rev() {
                        /*if dst >= fb.len() - 3 {
                            panic!(
                                "dst={} x={},y={},tile_row={},tile_col={}",
                                dst, x, y, tile_row, tile_col
                            );
                            //    return Ok(());
                        }*/
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

                    dst += (NUM_TILES_WIDTH - 1) * TILE_WIDTH_BYTES;
                }
            }
        }
        Ok(())
    }

    fn tile_start_offset(&self) -> Result<u16, ExecErr> {
        let lcdc = self.mc.read(0xFF40)?;
        if (lcdc & (1 << 4)) != 0 {
            Ok(0x8000)
        } else {
            Ok(0x8800)
        }
    }
}
