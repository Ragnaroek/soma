#[cfg(test)]
#[path = "./sm83_test.rs"]
mod sm83_test;

use psy::arch::sm83::{self, Sm83Instr};

use crate::ROM;
use crate::io::IO;

/// SM83 CPU emulator

pub struct SM83 {
    debugger: Option<Debugger>,
    halted: bool,
    reg: Register,
    io: IO,
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

impl Register {
    /// Returns a register bank with all registers set to 0.
    pub fn zero() -> Register {
        Register {
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
        }
    }

    /// Returns a register bank with register a set to the supplied value.
    /// All other registers are set to 0.
    pub fn a(v: u8) -> Register {
        let mut reg = Register::zero();
        reg.a = v;
        reg
    }
}

pub struct Debugger {
    debug: fn(&Sm83Instr, &mut SM83),
}

impl Debugger {
    pub fn new(debug: fn(&Sm83Instr, &mut SM83)) -> Debugger {
        Debugger { debug }
    }
}

// mem space definition (inclusive intervals)
const IO_START: u16 = 0xFF00;
const IO_END: u16 = 0xFFFF;

impl SM83 {
    pub fn init() -> SM83 {
        SM83 {
            debugger: None,
            halted: false,
            reg: Register::zero(),
            io: IO::init(),
        }
    }

    pub fn execute(&mut self, rom: &ROM) -> Result<(), &'static str> {
        let instr = sm83::decode(rom.read_u8(self.pc() as usize));

        if instr.op_code == sm83::INSTR_JP.op_code {
            let addr = rom.read_u16((self.pc() + 1) as usize);
            self.set_pc(addr);
        } else if instr.op_code == sm83::INSTR_LD_TO_A_FROM_IMMEDIATE.op_code {
            let val = rom.read_u8((self.pc() + 1) as usize);
            self.reg.a = val;
            self.inc_pc(2);
        } else if instr.op_code == sm83::INSTR_LD_TO_DEREF_LABEL_FROM_A.op_code {
            let addr = rom.read_u16((self.pc() + 1) as usize);
            self.mem_write(addr, self.reg.a);
            self.inc_pc(3);
        } else if instr.op_code == sm83::INSTR_LD_TO_A_FROM_DEREF_LABEL.op_code {
            let addr = rom.read_u16((self.pc() + 1) as usize);
            let v = self.mem_read(addr);
            self.reg.a = v;
            self.inc_pc(3);
        } else {
            return Err("invalid instruction");
        }

        if let Some(debugger) = &self.debugger {
            (debugger.debug)(instr, self);
        }
        Ok(())
    }

    fn mem_write(&mut self, addr: u16, v: u8) {
        if addr >= IO_START && addr <= IO_END {
            self.io.write(addr, v);
        } else {
            panic!("mem write error");
        }
    }

    fn mem_read(&mut self, addr: u16) -> u8 {
        if addr >= IO_START && addr <= IO_END {
            self.io.read(addr)
        } else {
            panic!("mem read error");
        }
    }

    pub fn halted(&self) -> bool {
        self.halted
    }

    pub fn set_pc(&mut self, pc: u16) {
        self.reg.pc = pc;
    }

    pub fn inc_pc(&mut self, inc: u16) {
        self.reg.pc += inc;
    }

    pub fn pc(&self) -> u16 {
        self.reg.pc
    }

    pub fn attach_debugger(&mut self, debugger: Debugger) {
        self.debugger = Some(debugger)
    }
}
