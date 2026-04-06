extern crate clap;
extern crate soma;

use clap::{Arg, Command};
use std::fs;

use soma::gameboy;
use soma::sm83;

fn main() {
    let matches = Command::new("soma_g")
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
    let rom = fs::read(rom_file).unwrap();

    let state = gameboy::gameboy_init(rom);
    let term = sm83::start(state);
    println!("terminated: {:?}", term);
}
