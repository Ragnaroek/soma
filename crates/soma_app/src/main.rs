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

use crate::app::{DebuggerSharedState, FrameBuffer, SomaApp, StepControl};
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

    let frame_buffer = Arc::new(RwLock::new(FrameBuffer {
        buffer: vec![0u8; 32 * 32 * 64 * 3], //vec![0u8; dmg::RESOLUTION_X * dmg::RESOLUTION_Y * 3],
        needs_update: true,
    }));
    let frame_buffer_emu = frame_buffer.clone();

    let debug_rom_data = rom_data.clone();
    let debug_app_rom = ROM::new(&debug_rom_data);
    let debug_state = Arc::new(RwLock::new(DebuggerSharedState::new()));
    let debug_state_emu = debug_state.clone();

    spawn_async(async move {
        let timer = Time {
            ref_time: Instant::now(),
            now: std_now,
        };

        let rom = ROM::new(&rom_data);
        let mut dmg = DMG::init(rom, timer);

        {
            let mut debug_state_init = debug_state_emu.write().unwrap();
            debug_state_init.register = dmg.sm83.reg.clone();
        }

        let mut v = 0;
        loop {
            let step_control = step_control(&debug_state_emu);
            if step_control == StepControl::Break {
                sleep(30).await;
                continue;
            }
            let r = dmg.step();
            if step_control == StepControl::NextStep {
                let mut debug_control = debug_state_emu.write().unwrap();
                debug_control.step_control = StepControl::Break;
            }
            if let Ok(step_result) = r {
                sleep(step_result.wait_time_millis).await;

                // debug
                println!("executed: {:?}", step_result.instr.mnemonic);

                // update framebuffer
                let mut fb = frame_buffer_emu.write().unwrap();
                /*
                for i in 0..(dmg::RESOLUTION_X * dmg::RESOLUTION_Y) {
                    let p = i * 3;
                    fb.buffer[p] = v;
                    fb.buffer[p + 1] = v;
                    fb.buffer[p + 2] = v;
                }
                */
                dmg.fb_rgb(&mut fb.buffer);
                fb.needs_update = true; // TODO determine the 'needs_update' in the step() function

                // update debug info
                let mut debug_control = debug_state_emu.write().unwrap();
                debug_control.register = dmg.sm83.reg.clone();
            } else {
                println!("ERR: {}", r.err().unwrap());
            }

            //v = v.wrapping_add(1);
        }
    });

    //let dim = [dmg::RESOLUTION_X as f32, dmg::RESOLUTION_Y as f32];
    let dim = [850.0, 700.0];

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(dim)
            .with_min_inner_size(dim),
        ..Default::default()
    };
    eframe::run_native(
        "Soma",
        native_options,
        Box::new(|cc| {
            Ok(Box::new(SomaApp::new(
                cc,
                frame_buffer,
                debug_state,
                debug_app_rom,
            )))
        }),
    )
}

fn step_control(debug_control: &Arc<RwLock<DebuggerSharedState>>) -> StepControl {
    let ctrl = debug_control.read().unwrap();
    ctrl.step_control
}

fn std_now(ref_time: &Instant) -> f64 {
    ref_time.elapsed().as_millis_f64()
}
