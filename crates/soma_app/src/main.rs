#![feature(duration_millis_float)]

mod app;
mod util;

use clap::{Arg, Command};
use psy::arch::sm83::Sm83Instr;
use std::sync::{Arc, RwLock};
use std::{fs, time::Instant};

use libsoma::{
    ROM,
    dmg::{self, DMG, Time},
    sm83::{Debugger, SM83},
};

use crate::app::{FrameBuffer, SomaApp};
use crate::util::{sleep, spawn_async};

fn main() -> eframe::Result {
    let matches = Command::new("soma")
        .version("0.0.1")
        .about("gameboy emulator")
        .arg(
            Arg::new("ROMFILE")
                .help("ROM file input")
                .required(true)
                .index(1),
        )
        .get_matches();

    let rom_file = matches.get_one::<String>("ROMFILE").unwrap();
    let rom_data = fs::read(rom_file).unwrap();

    let frame_buffer = Arc::new(RwLock::new(FrameBuffer {
        buffer: vec![0u8; dmg::RESOLUTION_X * dmg::RESOLUTION_Y * 3],
        needs_update: true,
    }));
    let frame_buffer_emu = frame_buffer.clone();

    spawn_async(async move {
        let rom = ROM::new(&rom_data);

        let timer = Time {
            ref_time: Instant::now(),
            now: std_now,
        };

        let mut dmg = DMG::init(rom, timer);
        dmg.attach_debugger(Debugger::new(cli_debug));

        let mut v = 0;
        loop {
            let r = dmg.step();
            if let Ok(wait_millis) = r {
                sleep(wait_millis).await;

                let mut fb = frame_buffer_emu.write().unwrap();
                for i in 0..(dmg::RESOLUTION_X * dmg::RESOLUTION_Y) {
                    let p = i * 3;
                    fb.buffer[p] = v;
                    fb.buffer[p + 1] = v;
                    fb.buffer[p + 2] = v;
                }
                fb.needs_update = true; // TODO determine the 'needs_update' in the step() function
            } else {
                println!("ERR: {}", r.unwrap_err());
            }

            v = v.wrapping_add(1);
        }
    });

    let dim = [dmg::RESOLUTION_X as f32, dmg::RESOLUTION_Y as f32];

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(dim)
            .with_min_inner_size(dim),
        ..Default::default()
    };
    eframe::run_native(
        "Soma",
        native_options,
        Box::new(|cc| Ok(Box::new(SomaApp::new(cc, frame_buffer)))),
    )
}

fn cli_debug(instr: &Sm83Instr, _sm83: &mut SM83) {
    println!("executed: {:?}", instr.mnemonic);
}

fn std_now(ref_time: &Instant) -> f64 {
    ref_time.elapsed().as_millis_f64()
}
