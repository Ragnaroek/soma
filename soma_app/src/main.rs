#![feature(duration_millis_float)]

use clap::{Arg, Command};
use psy::arch::sm83::Sm83Instr;
use std::{fs, time::Instant};

use libsoma::{
    ROM,
    dmg::{DMG, Time},
    sm83::{Debugger, SM83},
};

fn main() {
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
    let rom = ROM::new(&rom_data);

    let timer = Time {
        ref_time: Instant::now(),
        now: std_now,
    };

    let mut dmg = DMG::init(timer);
    dmg.attach_debugger(Debugger::new(cli_debug));
    let r = dmg.run(rom);
    if r.is_ok() {
        println!("HALT");
    } else {
        println!("ERR: {}", r.unwrap_err());
    }
}

fn cli_debug(instr: &Sm83Instr, _sm83: &mut SM83) {
    println!("executed: {:?}", instr.mnemonic);
}

fn std_now(ref_time: &Instant) -> f64 {
    ref_time.elapsed().as_millis_f64()
}
