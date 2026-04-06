use clap::{Arg, Command};
use std::fs;

use libsoma::{ROM, dmg::DMG};

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
    let dmg = DMG::init(rom);
    dmg.run();
    println!("run terminated");
}
