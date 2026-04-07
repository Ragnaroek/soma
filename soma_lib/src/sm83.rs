/// SM83 CPU emulator

pub struct SM83 {
    reg: Register,
}

pub struct Register {
    pub pc: u16,
    pub sp: u16,

    pub a: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub f: u8,
}

impl SM83 {
    pub fn init() -> SM83 {
        SM83 {
            reg: Register {
                pc: 0,
                sp: 0,
                a: 0,
                b: 0,
                c: 0,
                d: 0,
                e: 0,
                h: 0,
                l: 0,
                f: 0,
            },
        }
    }

    pub fn set_pc(&mut self, pc: u16) {
        self.reg.pc = pc;
    }

    pub fn pc(&self) -> u16 {
        self.reg.pc
    }
}
