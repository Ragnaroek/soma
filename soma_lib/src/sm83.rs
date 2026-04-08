use psy::arch::sm83::{self, Sm83Instr};

use crate::ROM;

/// SM83 CPU emulator

pub struct SM83 {
    debugger: Option<Debugger>,
    halted: bool,
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

pub struct Debugger {
    debug: fn(&Sm83Instr, &mut SM83),
}

impl Debugger {
    pub fn new(debug: fn(&Sm83Instr, &mut SM83)) -> Debugger {
        Debugger { debug }
    }
}

impl SM83 {
    pub fn init() -> SM83 {
        SM83 {
            debugger: None,
            halted: false,
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

    pub fn execute(&mut self, rom: &ROM) {
        let instr = sm83::decode(rom.value_at(self.pc() as usize));

        if instr.op_code == sm83::INSTR_JP.op_code {
            let addr = rom.read_u16((self.pc() + 1) as usize);
            self.set_pc(addr);
        }

        if let Some(debugger) = &self.debugger {
            (debugger.debug)(instr, self);
        }
    }

    pub fn halted(&self) -> bool {
        self.halted
    }

    pub fn set_pc(&mut self, pc: u16) {
        self.reg.pc = pc;
    }

    pub fn pc(&self) -> u16 {
        self.reg.pc
    }

    pub fn attach_debugger(&mut self, debugger: Debugger) {
        self.debugger = Some(debugger)
    }
}
