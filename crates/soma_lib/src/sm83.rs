#[cfg(test)]
#[path = "./sm83_test.rs"]
mod sm83_test;

use psy::arch::sm83::{self, Sm83Instr};

use crate::memory::MemoryController;

const Z: u8 = 1 << 7;
const N: u8 = 1 << 6;
const H: u8 = 1 << 5;
const C: u8 = 1 << 4;

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
    pub f: u8, // z n h c flags in lower half
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

    pub fn de(&self) -> u16 {
        ((self.d as u16) << 8) | (self.e as u16)
    }

    pub fn hl(&self) -> u16 {
        ((self.h as u16) << 8) | (self.l as u16)
    }

    pub fn set_hl(&mut self, v: u16) {
        self.h = (v >> 8) as u8;
        self.l = v as u8;
    }
}

/// Mostly useful in tests
pub struct RegBuilder {
    reg: Register,
}

impl RegBuilder {
    pub fn new() -> RegBuilder {
        RegBuilder {
            reg: Register::zero(),
        }
    }

    pub fn reg(self) -> Register {
        self.reg
    }

    /// Returns a register bank with register a set to the supplied value.
    /// All other registers are set to 0.
    pub fn a(mut self, v: u8) -> RegBuilder {
        self.reg.a = v;
        self
    }

    pub fn b(mut self, v: u8) -> RegBuilder {
        self.reg.b = v;
        self
    }

    pub fn c(mut self, v: u8) -> RegBuilder {
        self.reg.c = v;
        self
    }

    pub fn d(mut self, v: u8) -> RegBuilder {
        self.reg.d = v;
        self
    }

    pub fn e(mut self, v: u8) -> RegBuilder {
        self.reg.e = v;
        self
    }

    pub fn f(mut self, v: u8) -> RegBuilder {
        self.reg.f = v;
        self
    }

    pub fn f_z(mut self, v: u8) -> RegBuilder {
        self.reg.f = set_flag(self.reg.f, Z, v);
        self
    }

    pub fn f_n(mut self, v: u8) -> RegBuilder {
        self.reg.f = set_flag(self.reg.f, N, v);
        self
    }

    pub fn f_h(mut self, v: u8) -> RegBuilder {
        self.reg.f = set_flag(self.reg.f, H, v);
        self
    }

    pub fn f_c(mut self, v: u8) -> RegBuilder {
        self.reg.f = set_flag(self.reg.f, C, v);
        self
    }

    pub fn h(mut self, v: u8) -> RegBuilder {
        self.reg.h = v;
        self
    }

    pub fn l(mut self, v: u8) -> RegBuilder {
        self.reg.l = v;
        self
    }

    pub fn pc(mut self, v: u16) -> RegBuilder {
        self.reg.pc = v;
        self
    }

    pub fn hl(mut self, v: u16) -> RegBuilder {
        self.reg.set_hl(v);
        self
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

impl SM83 {
    pub fn init() -> SM83 {
        SM83 {
            debugger: None,
            halted: false,
            reg: Register::zero(),
        }
    }

    pub fn execute(&mut self, mc: &mut MemoryController) -> Result<(), &'static str> {
        let instr = sm83::decode(mc.read(self.pc()));

        EXEC_TABLE[instr.op_code as usize](self, mc)?;

        if let Some(debugger) = &self.debugger {
            (debugger.debug)(instr, self);
        }
        Ok(())
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

type Sm83Exec = fn(&mut SM83, &mut MemoryController) -> Result<(), &'static str>;

fn exec_invalid(_: &mut SM83, _: &mut MemoryController) -> Result<(), &'static str> {
    return Err("invalid instruction");
}

fn exec_cp_immediate(sm83: &mut SM83, mc: &mut MemoryController) -> Result<(), &'static str> {
    let v = mc.read(sm83.pc() + 1);
    let (z, carry) = sm83.reg.a.overflowing_sub(v);
    let mut f = set_flag(sm83.reg.f, Z, z);
    f = set_flag(f, N, 1);
    f = set_flag(f, H, z & H);
    f = set_flag(f, C, carry as u8);
    sm83.reg.f = f;
    sm83.inc_pc(2);
    Ok(())
}

fn exec_jp(sm83: &mut SM83, mc: &mut MemoryController) -> Result<(), &'static str> {
    let addr = mc.read_u16(sm83.pc() + 1);
    sm83.set_pc(addr);
    Ok(())
}

fn exec_jp_if_c(sm83: &mut SM83, mc: &mut MemoryController) -> Result<(), &'static str> {
    let rel = mc.read(sm83.pc() + 1) as i8;
    sm83.inc_pc(2); // relative jump is computed after the instruction
    if (sm83.reg.f & C) != 0 {
        sm83.set_pc(sm83.pc().saturating_add_signed(rel as i16));
    }
    Ok(())
}

fn exec_ld_to_a_from_immediate(
    sm83: &mut SM83,
    mc: &mut MemoryController,
) -> Result<(), &'static str> {
    let val = mc.read(sm83.pc() + 1);
    sm83.reg.a = val;
    sm83.inc_pc(2);
    Ok(())
}

fn exec_ld_to_a_from_deref_de(
    sm83: &mut SM83,
    mc: &mut MemoryController,
) -> Result<(), &'static str> {
    let addr = sm83.reg.de();
    let v = mc.read(addr);
    sm83.reg.a = v;
    sm83.inc_pc(1);
    Ok(())
}

fn exec_ld_to_de_from_immediate(
    sm83: &mut SM83,
    mc: &mut MemoryController,
) -> Result<(), &'static str> {
    let lsb = mc.read(sm83.pc() + 1);
    let msb = mc.read(sm83.pc() + 2);
    sm83.reg.d = msb;
    sm83.reg.e = lsb;
    sm83.inc_pc(3);
    Ok(())
}

fn exec_ld_to_hl_from_immediate(
    sm83: &mut SM83,
    mc: &mut MemoryController,
) -> Result<(), &'static str> {
    let lsb = mc.read(sm83.pc() + 1);
    let msb = mc.read(sm83.pc() + 2);
    sm83.reg.h = msb;
    sm83.reg.l = lsb;
    sm83.inc_pc(3);
    Ok(())
}

fn exec_ld_to_bc_from_immediate(
    sm83: &mut SM83,
    mc: &mut MemoryController,
) -> Result<(), &'static str> {
    let lsb = mc.read(sm83.pc() + 1);
    let msb = mc.read(sm83.pc() + 2);
    sm83.reg.b = msb;
    sm83.reg.c = lsb;
    sm83.inc_pc(3);
    Ok(())
}

fn exec_ld_to_deref_label_from_a(
    sm83: &mut SM83,
    mc: &mut MemoryController,
) -> Result<(), &'static str> {
    let addr = mc.read_u16(sm83.pc() + 1);
    mc.write(addr, sm83.reg.a);
    sm83.inc_pc(3);
    Ok(())
}

fn exec_ld_to_deref_hl_inc_from_a(
    sm83: &mut SM83,
    mc: &mut MemoryController,
) -> Result<(), &'static str> {
    let addr = sm83.reg.hl();
    mc.write(addr, sm83.reg.a);
    sm83.reg.set_hl(addr + 1);
    sm83.inc_pc(1);
    Ok(())
}

fn exec_ld_to_a_from_deref_label(
    sm83: &mut SM83,
    mc: &mut MemoryController,
) -> Result<(), &'static str> {
    let addr = mc.read_u16(sm83.pc() + 1);
    let v = mc.read(addr);
    sm83.reg.a = v;
    sm83.inc_pc(3);
    Ok(())
}

fn exec_ld_to_a_from_deref_hl_inc(
    sm83: &mut SM83,
    mc: &mut MemoryController,
) -> Result<(), &'static str> {
    let addr = sm83.reg.hl();
    let v = mc.read(addr);
    sm83.reg.a = v;
    sm83.reg.set_hl(addr + 1);
    sm83.inc_pc(1);
    Ok(())
}

pub static EXEC_TABLE: [Sm83Exec; psy::arch::sm83::SM83_NUM_INSTRUCTIONS] = [
    /*0x00*/ exec_invalid,
    /*0x01*/ exec_ld_to_bc_from_immediate,
    /*0x02*/ exec_invalid,
    /*0x03*/ exec_invalid,
    /*0x04*/ exec_invalid,
    /*0x05*/ exec_invalid,
    /*0x06*/ exec_invalid,
    /*0x07*/ exec_invalid,
    /*0x08*/ exec_invalid,
    /*0x09*/ exec_invalid,
    /*0x0A*/ exec_invalid,
    /*0x0B*/ exec_invalid,
    /*0x0C*/ exec_invalid,
    /*0x0D*/ exec_invalid,
    /*0x0E*/ exec_invalid,
    /*0x0F*/ exec_invalid,
    /*0x10*/ exec_invalid,
    /*0x11*/ exec_ld_to_de_from_immediate,
    /*0x12*/ exec_invalid,
    /*0x13*/ exec_invalid,
    /*0x14*/ exec_invalid,
    /*0x15*/ exec_invalid,
    /*0x16*/ exec_invalid,
    /*0x17*/ exec_invalid,
    /*0x18*/ exec_invalid,
    /*0x19*/ exec_invalid,
    /*0x1A*/ exec_ld_to_a_from_deref_de,
    /*0x1B*/ exec_invalid,
    /*0x1C*/ exec_invalid,
    /*0x1D*/ exec_invalid,
    /*0x1E*/ exec_invalid,
    /*0x1F*/ exec_invalid,
    /*0x20*/ exec_invalid,
    /*0x21*/ exec_ld_to_hl_from_immediate,
    /*0x22*/ exec_ld_to_deref_hl_inc_from_a,
    /*0x23*/ exec_invalid,
    /*0x24*/ exec_invalid,
    /*0x25*/ exec_invalid,
    /*0x26*/ exec_invalid,
    /*0x27*/ exec_invalid,
    /*0x28*/ exec_invalid,
    /*0x29*/ exec_invalid,
    /*0x2A*/ exec_ld_to_a_from_deref_hl_inc,
    /*0x2B*/ exec_invalid,
    /*0x2C*/ exec_invalid,
    /*0x2D*/ exec_invalid,
    /*0x2E*/ exec_invalid,
    /*0x2F*/ exec_invalid,
    /*0x30*/ exec_invalid,
    /*0x31*/ exec_invalid,
    /*0x32*/ exec_invalid,
    /*0x33*/ exec_invalid,
    /*0x34*/ exec_invalid,
    /*0x35*/ exec_invalid,
    /*0x36*/ exec_invalid,
    /*0x37*/ exec_invalid,
    /*0x38*/ exec_jp_if_c,
    /*0x39*/ exec_invalid,
    /*0x3A*/ exec_invalid,
    /*0x3B*/ exec_invalid,
    /*0x3C*/ exec_invalid,
    /*0x3D*/ exec_invalid,
    /*0x3E*/ exec_ld_to_a_from_immediate,
    /*0x3F*/ exec_invalid,
    /*0x40*/ exec_invalid,
    /*0x41*/ exec_invalid,
    /*0x42*/ exec_invalid,
    /*0x43*/ exec_invalid,
    /*0x44*/ exec_invalid,
    /*0x45*/ exec_invalid,
    /*0x46*/ exec_invalid,
    /*0x47*/ exec_invalid,
    /*0x48*/ exec_invalid,
    /*0x49*/ exec_invalid,
    /*0x4A*/ exec_invalid,
    /*0x4B*/ exec_invalid,
    /*0x4C*/ exec_invalid,
    /*0x4D*/ exec_invalid,
    /*0x4E*/ exec_invalid,
    /*0x4F*/ exec_invalid,
    /*0x50*/ exec_invalid,
    /*0x51*/ exec_invalid,
    /*0x52*/ exec_invalid,
    /*0x53*/ exec_invalid,
    /*0x54*/ exec_invalid,
    /*0x55*/ exec_invalid,
    /*0x56*/ exec_invalid,
    /*0x57*/ exec_invalid,
    /*0x58*/ exec_invalid,
    /*0x59*/ exec_invalid,
    /*0x5A*/ exec_invalid,
    /*0x5B*/ exec_invalid,
    /*0x5C*/ exec_invalid,
    /*0x5D*/ exec_invalid,
    /*0x5E*/ exec_invalid,
    /*0x5F*/ exec_invalid,
    /*0x60*/ exec_invalid,
    /*0x61*/ exec_invalid,
    /*0x62*/ exec_invalid,
    /*0x63*/ exec_invalid,
    /*0x64*/ exec_invalid,
    /*0x65*/ exec_invalid,
    /*0x66*/ exec_invalid,
    /*0x67*/ exec_invalid,
    /*0x68*/ exec_invalid,
    /*0x69*/ exec_invalid,
    /*0x6A*/ exec_invalid,
    /*0x6B*/ exec_invalid,
    /*0x6C*/ exec_invalid,
    /*0x6D*/ exec_invalid,
    /*0x6E*/ exec_invalid,
    /*0x6F*/ exec_invalid,
    /*0x70*/ exec_invalid,
    /*0x71*/ exec_invalid,
    /*0x72*/ exec_invalid,
    /*0x73*/ exec_invalid,
    /*0x74*/ exec_invalid,
    /*0x75*/ exec_invalid,
    /*0x76*/ exec_invalid,
    /*0x77*/ exec_invalid,
    /*0x78*/ exec_invalid,
    /*0x79*/ exec_invalid,
    /*0x7A*/ exec_invalid,
    /*0x7B*/ exec_invalid,
    /*0x7C*/ exec_invalid,
    /*0x7D*/ exec_invalid,
    /*0x7E*/ exec_invalid,
    /*0x7F*/ exec_invalid,
    exec_invalid,
    exec_invalid,
    exec_invalid,
    exec_invalid,
    exec_invalid,
    exec_invalid,
    exec_invalid,
    exec_invalid,
    exec_invalid,
    exec_invalid,
    exec_invalid,
    exec_invalid,
    exec_invalid,
    exec_invalid,
    exec_invalid,
    exec_invalid,
    exec_invalid,
    exec_invalid,
    exec_invalid,
    exec_invalid,
    exec_invalid,
    exec_invalid,
    exec_invalid,
    exec_invalid,
    exec_invalid,
    exec_invalid,
    exec_invalid,
    exec_invalid,
    exec_invalid,
    exec_invalid,
    exec_invalid,
    exec_invalid,
    exec_invalid,
    exec_invalid,
    exec_invalid,
    exec_invalid,
    exec_invalid,
    exec_invalid,
    exec_invalid,
    exec_invalid,
    exec_invalid,
    exec_invalid,
    exec_invalid,
    exec_invalid,
    exec_invalid,
    exec_invalid,
    exec_invalid,
    exec_invalid,
    exec_invalid,
    /*0xB1*/ exec_invalid,
    /*0xB2*/ exec_invalid,
    /*0xB3*/ exec_invalid,
    /*0xB4*/ exec_invalid,
    /*0xB5*/ exec_invalid,
    /*0xB6*/ exec_invalid,
    /*0xB7*/ exec_invalid,
    /*0xB8*/ exec_invalid,
    /*0xB9*/ exec_invalid,
    /*0xBA*/ exec_invalid,
    /*0xBB*/ exec_invalid,
    /*0xBC*/ exec_invalid,
    /*0xBD*/ exec_invalid,
    /*0xBE*/ exec_invalid,
    /*0xBF*/ exec_invalid,
    /*0xC0*/ exec_invalid,
    /*0xC1*/ exec_invalid,
    /*0xC2*/ exec_invalid,
    /*0xC3*/ exec_jp,
    /*0xC4*/ exec_invalid,
    /*0xC5*/ exec_invalid,
    /*0xC6*/ exec_invalid,
    /*0xC7*/ exec_invalid,
    /*0xC8*/ exec_invalid,
    /*0xC9*/ exec_invalid,
    /*0xCA*/ exec_invalid,
    /*0xCB*/ exec_invalid,
    /*0xCC*/ exec_invalid,
    /*0xCD*/ exec_invalid,
    /*0xCE*/ exec_invalid,
    /*0xCF*/ exec_invalid,
    /*0xD0*/ exec_invalid,
    /*0xD1*/ exec_invalid,
    /*0xD2*/ exec_invalid,
    /*0xD3*/ exec_invalid,
    /*0xD4*/ exec_invalid,
    /*0xD5*/ exec_invalid,
    /*0xD6*/ exec_invalid,
    /*0xD7*/ exec_invalid,
    /*0xD8*/ exec_invalid,
    /*0xD9*/ exec_invalid,
    /*0xDA*/ exec_invalid,
    /*0xDB*/ exec_invalid,
    /*0xDC*/ exec_invalid,
    /*0xDD*/ exec_invalid,
    /*0xDE*/ exec_invalid,
    /*0xDF*/ exec_invalid,
    /*0xE9*/ exec_invalid,
    /*0xE1*/ exec_invalid,
    /*0xE2*/ exec_invalid,
    /*0xE3*/ exec_invalid,
    /*0xE4*/ exec_invalid,
    /*0xE5*/ exec_invalid,
    /*0xE6*/ exec_invalid,
    /*0xE7*/ exec_invalid,
    /*0xE8*/ exec_invalid,
    /*0xE9*/ exec_invalid,
    /*0xEA*/ exec_ld_to_deref_label_from_a,
    /*0xEB*/ exec_invalid,
    /*0xEC*/ exec_invalid,
    /*0xED*/ exec_invalid,
    /*0xEE*/ exec_invalid,
    /*0xEF*/ exec_invalid,
    /*0xF0*/ exec_invalid,
    /*0xF1*/ exec_invalid,
    /*0xF2*/ exec_invalid,
    /*0xF3*/ exec_invalid,
    /*0xF4*/ exec_invalid,
    /*0xF5*/ exec_invalid,
    /*0xF6*/ exec_invalid,
    /*0xF7*/ exec_invalid,
    /*0xF8*/ exec_invalid,
    /*0xF9*/ exec_invalid,
    /*0xFA*/ exec_ld_to_a_from_deref_label,
    /*0xFB*/ exec_invalid,
    /*0xFC*/ exec_invalid,
    /*0xFD*/ exec_invalid,
    /*0xFE*/ exec_cp_immediate,
    /*0xFF*/ exec_invalid,
];

// Helper

fn set_flag(reg: u8, flag: u8, v: u8) -> u8 {
    if v == 0 { reg & !flag } else { reg | flag }
}
