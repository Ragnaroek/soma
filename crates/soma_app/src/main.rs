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
            [1000.0, 850.0],
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
        let mut dmg = DMG::init(rom, timer, debug_print);

        update_shared_state(&shared_state_emu, &dmg);
        loop {
            let step_control = step_control(&shared_state_emu);
            if step_control == StepControl::Break {
                sleep(30.0).await;
                continue;
            }
            if let StepControl::BreakAt(break_at) = step_control {
                if dmg.sm83.reg.pc == break_at {
                    sleep(30.0).await;
                    continue;
                }
            }
            let r = dmg.step();
            if step_control == StepControl::NextStep {
                let mut debug_control = shared_state_emu.write().unwrap();
                debug_control.step_control = StepControl::Break;
            }
            if let Ok(step_result) = r {
                if step_result.wait_time_millis != 0.0 {
                    sleep(step_result.wait_time_millis).await;
                }

                if step_result.fb_refresh {
                    // update framebuffer
                    let mut fb = frame_buffer_emu.write().unwrap();
                    dmg.fb_rgb(&mut fb.buffer);
                    fb.needs_update = true;
                }

                // update debug info
                update_shared_state(&shared_state_emu, &dmg);
            } else {
                println!("ERR: {}", r.err().unwrap());
            }
        }
    });

    fn update_shared_state<T>(shared_state_emu: &Arc<RwLock<DebuggerSharedState>>, dmg: &DMG<T>) {
        let mut shared_state = shared_state_emu.write().unwrap();
        shared_state.register = dmg.sm83.reg.clone();

        for i in 0..(32 * 32) {
            shared_state.tile_map_1[i] = dmg.mc.read(0x9800 + i as u16);
        }
    }

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

fn debug_print(txt: &str, v: u16) {
    println!("{} = 0x{:x}", txt, v)
}
