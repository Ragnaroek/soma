#![feature(duration_millis_float)]

mod app;
mod util;

use clap::Parser;
use std::sync::{Arc, RwLock};
use std::{fs, time::Instant};

use libsoma::{
    ROM,
    dmg::{DMG, Time},
};

use crate::app::{Debug, DebuggerSharedState, FrameBuffer, SomaApp, StepControl};
use crate::util::{sleep, spawn_async};

#[derive(Parser)]
#[command(version = "0.0.1", about = "gameboy emulator")]
struct Cli {
    /// ROM file input
    #[arg(value_name = "ROM_FILE")]
    rom: String,

    /// Enable debug view mode
    #[arg(long)]
    debugger: bool,
}

fn main() -> eframe::Result {
    let args = Cli::parse();

    let rom_data = fs::read(args.rom).unwrap();
    let debug_rom_data = if args.debugger {
        rom_data.clone()
    } else {
        Vec::with_capacity(0)
    };

    let frame_buffer = Arc::new(RwLock::new(FrameBuffer {
        buffer: vec![0u8; 32 * 32 * 64 * 3], //vec![0u8; dmg::RESOLUTION_X * dmg::RESOLUTION_Y * 3],
        needs_update: true,
    }));
    let frame_buffer_emu = frame_buffer.clone();

    let step_control_init = if args.debugger {
        StepControl::Break
    } else {
        StepControl::Run
    };
    let shared_state = Arc::new(RwLock::new(DebuggerSharedState::new(step_control_init)));
    let shared_state_emu = shared_state.clone();

    //let dim = [dmg::RESOLUTION_X as f32, dmg::RESOLUTION_Y as f32];
    let (debug, dim) = if args.debugger {
        let debug_app_rom = ROM::new(&debug_rom_data);
        (
            Some(Debug::new(shared_state, debug_app_rom)),
            [850.0, 700.0],
        )
    } else {
        (None, [256.0 + 20.0, 256.0 + 20.0]) // TODO get rid of the border in non-debug mode!
    };

    spawn_async(async move {
        let timer = Time {
            ref_time: Instant::now(),
            now: std_now,
        };

        let rom = ROM::new(&rom_data);
        let mut dmg = DMG::init(rom, timer);

        {
            let mut debug_state_init = shared_state_emu.write().unwrap();
            debug_state_init.register = dmg.sm83.reg.clone();
        }

        loop {
            let step_control = step_control(&shared_state_emu);
            if step_control == StepControl::Break {
                sleep(30).await;
                continue;
            }
            let r = dmg.step();
            if step_control == StepControl::NextStep {
                let mut debug_control = shared_state_emu.write().unwrap();
                debug_control.step_control = StepControl::Break;
            }
            if let Ok(step_result) = r {
                sleep(step_result.wait_time_millis).await;

                // update framebuffer
                let mut fb = frame_buffer_emu.write().unwrap();
                dmg.fb_rgb(&mut fb.buffer);
                fb.needs_update = true; // TODO determine the 'needs_update' in the step() function

                // update debug info
                let mut shared_state = shared_state_emu.write().unwrap();
                shared_state.register = dmg.sm83.reg.clone();
            } else {
                println!("ERR: {}", r.err().unwrap());
            }
        }
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
        Box::new(|cc| Ok(Box::new(SomaApp::new(cc, frame_buffer, debug)))),
    )
}

fn step_control(debug_control: &Arc<RwLock<DebuggerSharedState>>) -> StepControl {
    let ctrl = debug_control.read().unwrap();
    ctrl.step_control
}

fn std_now(ref_time: &Instant) -> f64 {
    ref_time.elapsed().as_millis_f64()
}
