#![feature(duration_millis_float)]
#![feature(str_as_str)]

mod app;
mod gdb;

use clap::Parser;

use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;
use std::{fs, time::Instant};

use libsoma::dmg::{DMG, Time};
use libsoma::rom::ROM;

use crate::app::{Debugger, Emulation, FrameBuffer, SomaApp, StepControl};
use crate::gdb::gdb_serve;

#[derive(Parser)]
#[command(version = "0.0.1", about = "gameboy emulator")]
struct Cli {
    /// ROM file input
    #[arg(value_name = "ROM_FILE")]
    rom: String,

    /// Enable debug view mode
    #[arg(long)]
    debugger: bool,

    /// Enable gdb support
    #[arg(long)]
    gdb: bool,
}

fn main() -> eframe::Result {
    let args = Cli::parse();

    let rom_data = fs::read(args.rom).unwrap();
    let rom = ROM::new_copy_from_slice(&rom_data);

    let frame_buffer = Arc::new(RwLock::new(FrameBuffer {
        buffer: vec![0u8; 32 * 32 * 64 * 3], //vec![0u8; dmg::RESOLUTION_X * dmg::RESOLUTION_Y * 3],
        needs_update: true,
    }));
    let frame_buffer_emu = frame_buffer.clone();

    let step_control_init = if args.debugger {
        StepControl::Halt
    } else {
        StepControl::Run
    };

    let timer = Time {
        ref_time: Instant::now(),
        now: std_now,
    };
    let emulation = Emulation::new(DMG::init(rom, timer, debug_print), step_control_init);

    let shared_emulation = Arc::new(emulation);
    let shared_emulation_gdb = shared_emulation.clone();
    let shared_emulation_emu = shared_emulation.clone();

    if args.gdb {
        std::thread::spawn(|| {
            gdb_serve(shared_emulation_gdb).expect("gdb serve");
        });
    }

    let (debugger, dim) = if args.debugger {
        (
            Some(Debugger::new(shared_emulation.clone())),
            [1000.0, 850.0],
        )
    } else {
        (None, [256.0 + 20.0, 256.0 + 20.0])
    };

    std::thread::spawn(|| {
        emulation_loop(shared_emulation_emu, frame_buffer_emu);
    });

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(dim)
            .with_min_inner_size(dim),
        ..Default::default()
    };

    eframe::run_native(
        "Soma",
        native_options,
        Box::new(|cc| Ok(Box::new(SomaApp::new(cc, frame_buffer, debugger)))),
    )
}

fn emulation_loop(emulation: Arc<Emulation>, frame_buffer_lock: Arc<RwLock<FrameBuffer>>) {
    loop {
        {
            let step_control = emulation.step_control();
            if let StepControl::Halt = step_control {
                thread::sleep(Duration::from_millis(30));
                continue;
            }
            let pc = { emulation.dmg_read_lock().sm83.pc() };
            if let StepControl::Resume = step_control {
                emulation.set_step_control(StepControl::Run);
            } else if emulation.has_breakpoint_at(pc) && step_control != StepControl::NextStep {
                emulation.set_step_control(StepControl::Halt);
                thread::sleep(Duration::from_millis(30));
                continue;
            }
        }
        let r = {
            let mut dmg = emulation.dmg_write_lock();
            let r = dmg.step();
            if emulation.step_control() == StepControl::NextStep {
                emulation.set_step_control(StepControl::Halt);
            }
            r
        };
        if let Ok(step_result) = r {
            if step_result.wait_time_millis != 0.0 {
                thread::sleep(Duration::from_micros(
                    (step_result.wait_time_millis * 1000.0) as u64,
                ));
            }

            if step_result.fb_refresh {
                // update framebuffer
                let mut fb = frame_buffer_lock.write().unwrap();
                let dmg = emulation.dmg_read_lock();
                dmg.fb_rgb(&mut fb.buffer);
                fb.needs_update = true;
            }
        } else {
            println!("ERR: {:?}", r.err().unwrap());
        }
    }
}

fn std_now(ref_time: &Instant) -> f64 {
    ref_time.elapsed().as_millis_f64()
}

fn debug_print(txt: &str, v: u16) {
    println!("{} = 0x{:x}", txt, v)
}
