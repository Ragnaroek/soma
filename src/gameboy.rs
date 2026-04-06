use super::sm83;

pub fn gameboy_init(mem: Vec<u8>) -> sm83::State {
    sm83::initial_state(mem, 127, 0x100)
}
