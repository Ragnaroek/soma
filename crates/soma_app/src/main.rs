#![feature(duration_millis_float)]

mod app;
mod util;

use clap::{Arg, Command};
use psy::arch::sm83::Sm83Instr;
use std::{fs, time::Instant};

use libsoma::{
    ROM,
    dmg::{DMG, Time},
    sm83::{Debugger, SM83},
};

use crate::app::SomaApp;
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

    spawn_async(async move {
        let rom = ROM::new(&rom_data);

        let timer = Time {
            ref_time: Instant::now(),
            now: std_now,
        };

        let mut dmg = DMG::init(rom, timer);
        dmg.attach_debugger(Debugger::new(cli_debug));
        loop {
            let r = dmg.step();
            if let Ok(wait_millis) = r {
                sleep(wait_millis).await;
            } else {
                println!("ERR: {}", r.unwrap_err());
            }
        }
    });

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_min_inner_size([300.0, 220.0]),
        ..Default::default()
    };
    eframe::run_native(
        "eframe template",
        native_options,
        Box::new(|cc| Ok(Box::new(SomaApp::new(cc)))),
    )
}

fn cli_debug(instr: &Sm83Instr, _sm83: &mut SM83) {
    println!("executed: {:?}", instr.mnemonic);
}

fn std_now(ref_time: &Instant) -> f64 {
    ref_time.elapsed().as_millis_f64()
}
