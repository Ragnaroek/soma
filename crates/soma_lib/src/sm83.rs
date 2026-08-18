#[cfg(test)]
#[path = "./sm83_test.rs"]
mod sm83_test;

use psy::arch::sm83::{self, Sm83Instr};

use crate::memory::MemoryController;

pub const Z: u8 = 1 << 7;
pub const N: u8 = 1 << 6;
pub const H: u8 = 1 << 5;
pub const C: u8 = 1 << 4;

/// SM83 CPU emulator
pub struct SM83 {
    halted: bool,
    pub reg: Register,
}

#[derive(Debug, PartialEq)]
pub enum ExecErr {
    // values: first one: op_code, second one: address
    InvalidInstruction(u8, u16),
    GeneralError(&'static str),
}

#[derive(Clone, Copy)]
pub struct Register {
    pub pc: u16,
    pub sp: u16,

    /// interrupt enable
    pub ie: u8,
    /// interrupt master enable
    pub ime: bool,

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
            ie: 0,
            ime: true,
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

    pub fn bc(&self) -> u16 {
        ((self.b as u16) << 8) | (self.c as u16)
    }

    pub fn de(&self) -> u16 {
        ((self.d as u16) << 8) | (self.e as u16)
    }

    pub fn hl(&self) -> u16 {
        ((self.h as u16) << 8) | (self.l as u16)
    }

    pub fn set_bc(&mut self, v: u16) {
        self.b = (v >> 8) as u8;
        self.c = v as u8;
    }

    pub fn set_de(&mut self, v: u16) {
        self.d = (v >> 8) as u8;
        self.e = v as u8;
    }

    pub fn set_hl(&mut self, v: u16) {
        self.h = (v >> 8) as u8;
        self.l = v as u8;
    }

    fn set_flag(&mut self, flag: u8, v: u8) {
        self.f = if v == 0 {
            self.f & !flag
        } else {
            self.f | flag
        };
    }

    fn get_flag(&self, flag: u8) -> u8 {
        if (self.f & flag) == 0 { 0 } else { 1 }
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
        self.reg.set_flag(Z, v);
        self
    }

    pub fn f_n(mut self, v: u8) -> RegBuilder {
        self.reg.set_flag(N, v);
        self
    }

    pub fn f_h(mut self, v: u8) -> RegBuilder {
        self.reg.set_flag(H, v);
        self
    }

    pub fn f_c(mut self, v: u8) -> RegBuilder {
        self.reg.set_flag(C, v);
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

    pub fn bc(mut self, v: u16) -> RegBuilder {
        self.reg.set_bc(v);
        self
    }

    pub fn de(mut self, v: u16) -> RegBuilder {
        self.reg.set_de(v);
        self
    }

    pub fn hl(mut self, v: u16) -> RegBuilder {
        self.reg.set_hl(v);
        self
    }

    pub fn pc(mut self, v: u16) -> RegBuilder {
        self.reg.pc = v;
        self
    }

    pub fn sp(mut self, v: u16) -> RegBuilder {
        self.reg.sp = v;
        self
    }
}

impl SM83 {
    pub fn init() -> SM83 {
        SM83 {
            halted: false,
            reg: Register::zero(),
        }
    }

    pub fn execute(&mut self, mc: &mut MemoryController) -> Result<&'static Sm83Instr, ExecErr> {
        let instr = sm83::decode(mc.read(self.pc())?);
        EXEC_TABLE[instr.op_code as usize](self, mc)?;
        Ok(instr)
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

    pub fn dec_sp(&mut self, dec: u16) {
        self.reg.sp -= dec;
    }

    pub fn inc_sp(&mut self, inc: u16) {
        self.reg.sp += inc;
    }

    pub fn pc(&self) -> u16 {
        self.reg.pc
    }
}

type Sm83Exec = fn(&mut SM83, &mut MemoryController) -> Result<(), ExecErr>;
type Sm83PrefixExec = fn(&mut SM83) -> Result<(), ExecErr>;

fn exec_invalid(sm83: &mut SM83, mc: &mut MemoryController) -> Result<(), ExecErr> {
    let pc = sm83.pc();
    let op = mc.read(pc)?;
    Err(ExecErr::InvalidInstruction(op, pc))
}

fn exec_ei(sm83: &mut SM83, _: &mut MemoryController) -> Result<(), ExecErr> {
    sm83.reg.ime = true;
    sm83.inc_pc(1);
    Ok(())
}

fn exec_di(sm83: &mut SM83, _: &mut MemoryController) -> Result<(), ExecErr> {
    sm83.reg.ime = false;
    sm83.inc_pc(1);
    Ok(())
}

// CP

fn exec_cp_immediate(sm83: &mut SM83, mc: &mut MemoryController) -> Result<(), ExecErr> {
    let v = mc.read(sm83.pc() + 1)?;
    let (z, carry) = sm83.reg.a.overflowing_sub(v);
    sm83.reg.set_flag(Z, z);
    sm83.reg.set_flag(N, 1);
    sm83.reg.set_flag(H, z & H);
    sm83.reg.set_flag(C, carry as u8);
    sm83.inc_pc(2);
    Ok(())
}

// ADD
fn exec_add_a_a(sm83: &mut SM83, _mc: &mut MemoryController) -> Result<(), ExecErr> {
    let (a_val, carry) = sm83.reg.a.overflowing_add(sm83.reg.a);
    let half_carry = (sm83.reg.a & 0x0F) >= 0x8;
    sm83.reg.a = a_val;
    sm83.reg.set_flag(Z, (sm83.reg.a == 0) as u8);
    sm83.reg.set_flag(N, 0);
    sm83.reg.set_flag(H, half_carry as u8);
    sm83.reg.set_flag(C, carry as u8);
    sm83.inc_pc(1);
    Ok(())
}

// JP

fn exec_jp(sm83: &mut SM83, mc: &mut MemoryController) -> Result<(), ExecErr> {
    let addr = mc.read_u16(sm83.pc() + 1)?;
    sm83.set_pc(addr);
    Ok(())
}

fn exec_jr(sm83: &mut SM83, mc: &mut MemoryController) -> Result<(), ExecErr> {
    let rel = mc.read(sm83.pc() + 1)? as i8;
    sm83.inc_pc(2); // relative jump is computed after the instruction
    sm83.set_pc(sm83.pc().saturating_add_signed(rel as i16));
    Ok(())
}

fn exec_jr_if_c(sm83: &mut SM83, mc: &mut MemoryController) -> Result<(), ExecErr> {
    let rel = mc.read(sm83.pc() + 1)? as i8;
    sm83.inc_pc(2); // relative jump is computed after the instruction
    if (sm83.reg.f & C) != 0 {
        sm83.set_pc(sm83.pc().saturating_add_signed(rel as i16));
    }
    Ok(())
}

fn exec_jr_if_nz(sm83: &mut SM83, mc: &mut MemoryController) -> Result<(), ExecErr> {
    let rel = mc.read(sm83.pc() + 1)? as i8;
    sm83.inc_pc(2); // relative jump is computed after the instruction
    if (sm83.reg.f & Z) == 0 {
        sm83.set_pc(sm83.pc().saturating_add_signed(rel as i16));
    }
    Ok(())
}

fn exec_ld_to_a_from_immediate(sm83: &mut SM83, mc: &mut MemoryController) -> Result<(), ExecErr> {
    let val = mc.read(sm83.pc() + 1)?;
    sm83.reg.a = val;
    sm83.inc_pc(2);
    Ok(())
}

fn exec_ld_to_b_from_immediate(sm83: &mut SM83, mc: &mut MemoryController) -> Result<(), ExecErr> {
    let val = mc.read(sm83.pc() + 1)?;
    sm83.reg.b = val;
    sm83.inc_pc(2);
    Ok(())
}

fn exec_ld_to_c_from_immediate(sm83: &mut SM83, mc: &mut MemoryController) -> Result<(), ExecErr> {
    let val = mc.read(sm83.pc() + 1)?;
    sm83.reg.c = val;
    sm83.inc_pc(2);
    Ok(())
}

fn exec_ld_to_d_from_immediate(sm83: &mut SM83, mc: &mut MemoryController) -> Result<(), ExecErr> {
    let val = mc.read(sm83.pc() + 1)?;
    sm83.reg.d = val;
    sm83.inc_pc(2);
    Ok(())
}

fn exec_ld_to_a_from_deref_de(sm83: &mut SM83, mc: &mut MemoryController) -> Result<(), ExecErr> {
    let addr = sm83.reg.de();
    let v = mc.read(addr)?;
    sm83.reg.a = v;
    sm83.inc_pc(1);
    Ok(())
}

fn exec_nop(sm83: &mut SM83, _: &mut MemoryController) -> Result<(), ExecErr> {
    sm83.inc_pc(1);
    Ok(())
}

fn exec_ld_to_de_from_immediate(sm83: &mut SM83, mc: &mut MemoryController) -> Result<(), ExecErr> {
    let lsb = mc.read(sm83.pc() + 1)?;
    let msb = mc.read(sm83.pc() + 2)?;
    sm83.reg.d = msb;
    sm83.reg.e = lsb;
    sm83.inc_pc(3);
    Ok(())
}

fn exec_ld_to_hl_from_immediate(sm83: &mut SM83, mc: &mut MemoryController) -> Result<(), ExecErr> {
    let lsb = mc.read(sm83.pc() + 1)?;
    let msb = mc.read(sm83.pc() + 2)?;
    sm83.reg.h = msb;
    sm83.reg.l = lsb;
    sm83.inc_pc(3);
    Ok(())
}

fn exec_ld_to_bc_from_immediate(sm83: &mut SM83, mc: &mut MemoryController) -> Result<(), ExecErr> {
    let lsb = mc.read(sm83.pc() + 1)?;
    let msb = mc.read(sm83.pc() + 2)?;
    sm83.reg.b = msb;
    sm83.reg.c = lsb;
    sm83.inc_pc(3);
    Ok(())
}

fn exec_ld_to_sp_from_immediate(sm83: &mut SM83, mc: &mut MemoryController) -> Result<(), ExecErr> {
    let v = mc.read_u16(sm83.pc() + 1)?;
    sm83.reg.sp = v;
    sm83.inc_pc(3);
    Ok(())
}

fn exec_ld_to_deref_label_from_a(
    sm83: &mut SM83,
    mc: &mut MemoryController,
) -> Result<(), ExecErr> {
    let addr = mc.read_u16(sm83.pc() + 1)?;
    mc.write(addr, sm83.reg.a)?;
    sm83.inc_pc(3);
    Ok(())
}

fn exec_ld_to_deref_hl_from_immediate(
    sm83: &mut SM83,
    mc: &mut MemoryController,
) -> Result<(), ExecErr> {
    let addr = sm83.reg.hl();
    let v = mc.read(sm83.pc() + 1)?;
    mc.write(addr, v)?;
    sm83.inc_pc(2);
    Ok(())
}

fn exec_ld_to_deref_hl_dec_from_a(
    sm83: &mut SM83,
    mc: &mut MemoryController,
) -> Result<(), ExecErr> {
    let addr = sm83.reg.hl();
    mc.write(addr, sm83.reg.a)?;
    sm83.reg.set_hl(addr - 1);
    sm83.inc_pc(1);
    Ok(())
}

fn exec_ld_to_deref_hl_inc_from_a(
    sm83: &mut SM83,
    mc: &mut MemoryController,
) -> Result<(), ExecErr> {
    let addr = sm83.reg.hl();
    mc.write(addr, sm83.reg.a)?;
    sm83.reg.set_hl(addr + 1);
    sm83.inc_pc(1);
    Ok(())
}

fn exec_ld_to_a_from_deref_label(
    sm83: &mut SM83,
    mc: &mut MemoryController,
) -> Result<(), ExecErr> {
    let addr = mc.read_u16(sm83.pc() + 1)?;
    let v = mc.read(addr)?;
    sm83.reg.a = v;
    sm83.inc_pc(3);
    Ok(())
}

fn exec_ld_to_a_from_deref_hl_inc(
    sm83: &mut SM83,
    mc: &mut MemoryController,
) -> Result<(), ExecErr> {
    let addr = sm83.reg.hl();
    let v = mc.read(addr)?;
    sm83.reg.a = v;
    sm83.reg.set_hl(addr + 1);
    sm83.inc_pc(1);
    Ok(())
}

fn exec_ld_to_a_from_b(sm83: &mut SM83, _mc: &mut MemoryController) -> Result<(), ExecErr> {
    sm83.reg.a = sm83.reg.b;
    sm83.inc_pc(1);
    Ok(())
}

fn exec_ld_to_a_from_c(sm83: &mut SM83, _mc: &mut MemoryController) -> Result<(), ExecErr> {
    sm83.reg.a = sm83.reg.c;
    sm83.inc_pc(1);
    Ok(())
}

fn exec_ld_to_b_from_a(sm83: &mut SM83, _mc: &mut MemoryController) -> Result<(), ExecErr> {
    sm83.reg.b = sm83.reg.a;
    sm83.inc_pc(1);
    Ok(())
}

fn exec_ld_to_c_from_a(sm83: &mut SM83, _mc: &mut MemoryController) -> Result<(), ExecErr> {
    sm83.reg.c = sm83.reg.a;
    sm83.inc_pc(1);
    Ok(())
}

fn exec_ld_to_e_from_a(sm83: &mut SM83, _mc: &mut MemoryController) -> Result<(), ExecErr> {
    sm83.reg.e = sm83.reg.a;
    sm83.inc_pc(1);
    Ok(())
}

fn exec_ldh_to_immediate_from_a(sm83: &mut SM83, mc: &mut MemoryController) -> Result<(), ExecErr> {
    let im = mc.read(sm83.pc() + 1)? as u16;
    let addr = 0xFF00 | im;
    write_high_mem(sm83, mc, addr, sm83.reg.a)?;
    sm83.inc_pc(2);
    Ok(())
}

fn exec_ldh_to_a_from_immediate(sm83: &mut SM83, mc: &mut MemoryController) -> Result<(), ExecErr> {
    let im = mc.read(sm83.pc() + 1)? as u16;
    let addr = 0xFF00 | im;
    sm83.reg.a = read_high_mem(sm83, mc, addr)?;
    sm83.inc_pc(2);
    Ok(())
}

fn exec_ldh_to_deref_c_from_a(sm83: &mut SM83, mc: &mut MemoryController) -> Result<(), ExecErr> {
    let addr = 0xFF00 | sm83.reg.c as u16;
    write_high_mem(sm83, mc, addr, sm83.reg.a)?;
    sm83.inc_pc(1);
    Ok(())
}

fn write_high_mem(
    sm83: &mut SM83,
    mc: &mut MemoryController,
    addr: u16,
    v: u8,
) -> Result<(), ExecErr> {
    if addr == 0xFFFF {
        sm83.reg.ie = v;
    } else {
        mc.write(addr, v)?;
    }
    Ok(())
}

fn read_high_mem(sm83: &mut SM83, mc: &mut MemoryController, addr: u16) -> Result<u8, ExecErr> {
    if addr == 0xFFFF {
        Ok(sm83.reg.ie)
    } else {
        mc.read(addr)
    }
}

// INC
fn exec_inc_c(sm83: &mut SM83, _: &mut MemoryController) -> Result<(), ExecErr> {
    let (c_inc, _) = sm83.reg.c.overflowing_add(1);
    let half_carry = half_carry_inc(sm83.reg.c);
    sm83.reg.c = c_inc;
    sm83.reg.set_flag(Z, (c_inc == 0) as u8);
    sm83.reg.set_flag(N, 0);
    sm83.reg.set_flag(H, half_carry);
    sm83.inc_pc(1);
    Ok(())
}
fn exec_inc_de(sm83: &mut SM83, _: &mut MemoryController) -> Result<(), ExecErr> {
    let de = sm83.reg.de();
    let (de_inc, _) = de.overflowing_add(1);
    sm83.reg.set_de(de_inc);
    sm83.inc_pc(1);
    Ok(())
}

// DEC
fn exec_dec_bc(sm83: &mut SM83, _mc: &mut MemoryController) -> Result<(), ExecErr> {
    let bc = sm83.reg.bc();
    let (bc_dec, _) = bc.overflowing_sub(1);
    sm83.reg.set_bc(bc_dec);
    sm83.inc_pc(1);
    Ok(())
}

fn exec_dec_b(sm83: &mut SM83, _mc: &mut MemoryController) -> Result<(), ExecErr> {
    let (b_dec, _) = sm83.reg.b.overflowing_sub(1);
    sm83.reg.b = b_dec;
    sm83.reg.set_flag(Z, (b_dec == 0) as u8);
    sm83.reg.set_flag(N, 1);
    sm83.reg.set_flag(H, half_carry_dec(b_dec));
    sm83.inc_pc(1);
    Ok(())
}

fn exec_dec_c(sm83: &mut SM83, _mc: &mut MemoryController) -> Result<(), ExecErr> {
    let (c_dec, _) = sm83.reg.c.overflowing_sub(1);
    sm83.reg.c = c_dec;
    sm83.reg.set_flag(Z, (c_dec == 0) as u8);
    sm83.reg.set_flag(N, 1);
    sm83.reg.set_flag(H, half_carry_dec(c_dec));
    sm83.inc_pc(1);
    Ok(())
}

fn exec_or_a_b(sm83: &mut SM83, _mc: &mut MemoryController) -> Result<(), ExecErr> {
    sm83.reg.a = sm83.reg.a | sm83.reg.b;
    sm83.reg.set_flag(Z, (sm83.reg.a == 0) as u8);
    sm83.reg.set_flag(N, 0);
    sm83.reg.set_flag(H, 0);
    sm83.reg.set_flag(C, 0);
    sm83.inc_pc(1);
    Ok(())
}

fn exec_or_a_c(sm83: &mut SM83, _mc: &mut MemoryController) -> Result<(), ExecErr> {
    sm83.reg.a = sm83.reg.a | sm83.reg.c;
    sm83.reg.set_flag(Z, (sm83.reg.a == 0) as u8);
    sm83.reg.set_flag(N, 0);
    sm83.reg.set_flag(H, 0);
    sm83.reg.set_flag(C, 0);
    sm83.inc_pc(1);
    Ok(())
}

// AND

fn exec_and_immediate(sm83: &mut SM83, mc: &mut MemoryController) -> Result<(), ExecErr> {
    let im = mc.read(sm83.pc() + 1)?;
    sm83.reg.a = sm83.reg.a & im;
    sm83.reg.set_flag(Z, (sm83.reg.a == 0) as u8);
    sm83.reg.set_flag(N, 0);
    sm83.reg.set_flag(H, 1);
    sm83.reg.set_flag(C, 0);
    sm83.inc_pc(2);
    Ok(())
}

fn exec_and_c(sm83: &mut SM83, _mc: &mut MemoryController) -> Result<(), ExecErr> {
    sm83.reg.a = sm83.reg.a & sm83.reg.c;
    sm83.reg.set_flag(Z, (sm83.reg.a == 0) as u8);
    sm83.reg.set_flag(N, 0);
    sm83.reg.set_flag(H, 1);
    sm83.reg.set_flag(C, 0);
    sm83.inc_pc(1);
    Ok(())
}

// XOR

fn exec_xor_a_a(sm83: &mut SM83, _mc: &mut MemoryController) -> Result<(), ExecErr> {
    sm83.reg.a = 0;
    sm83.reg.set_flag(Z, 0);
    sm83.reg.set_flag(N, 0);
    sm83.reg.set_flag(H, 0);
    sm83.reg.set_flag(C, 0);
    sm83.inc_pc(1);
    Ok(())
}

fn exec_xor_a_c(sm83: &mut SM83, _mc: &mut MemoryController) -> Result<(), ExecErr> {
    sm83.reg.a = sm83.reg.a ^ sm83.reg.c;
    sm83.reg.set_flag(Z, (sm83.reg.a == 0) as u8);
    sm83.reg.set_flag(N, 0);
    sm83.reg.set_flag(H, 0);
    sm83.reg.set_flag(C, 0);
    sm83.inc_pc(1);
    Ok(())
}

fn exec_cpl(sm83: &mut SM83, _mc: &mut MemoryController) -> Result<(), ExecErr> {
    sm83.reg.a = !sm83.reg.a;
    sm83.reg.set_flag(N, 1);
    sm83.reg.set_flag(H, 1);
    sm83.inc_pc(1);
    Ok(())
}

fn exec_rrca(sm83: &mut SM83, _mc: &mut MemoryController) -> Result<(), ExecErr> {
    let c = sm83.reg.a & 0x01;
    sm83.reg.a = sm83.reg.a.rotate_right(1);
    sm83.reg.set_flag(Z, 0);
    sm83.reg.set_flag(N, 0);
    sm83.reg.set_flag(H, 0);
    sm83.reg.set_flag(C, c);
    sm83.inc_pc(1);
    Ok(())
}

fn exec_call(sm83: &mut SM83, mc: &mut MemoryController) -> Result<(), ExecErr> {
    let addr = mc.read_u16(sm83.pc() + 1)?;
    internal_call_to_addr(sm83, mc, addr, 3)
}

fn exec_ret(sm83: &mut SM83, mc: &mut MemoryController) -> Result<(), ExecErr> {
    let lsb = mc.read(sm83.reg.sp)?;
    sm83.inc_sp(1);
    let msb = mc.read(sm83.reg.sp)?;
    sm83.inc_sp(1);

    let addr = u16::from_le_bytes([lsb, msb]);
    sm83.set_pc(addr);
    Ok(())
}

fn exec_rst_28(sm83: &mut SM83, mc: &mut MemoryController) -> Result<(), ExecErr> {
    internal_call_to_addr(sm83, mc, 0x28, 1)
}

fn exec_pop_hl(sm83: &mut SM83, mc: &mut MemoryController) -> Result<(), ExecErr> {
    let lsb = mc.read(sm83.reg.sp)?;
    sm83.inc_sp(1);
    let msb = mc.read(sm83.reg.sp)?;
    sm83.inc_sp(1);

    let addr = u16::from_le_bytes([lsb, msb]);
    sm83.reg.set_hl(addr);
    sm83.inc_pc(1);
    Ok(())
}

fn internal_call_to_addr(
    sm83: &mut SM83,
    mc: &mut MemoryController,
    addr: u16,
    op_size: u16,
) -> Result<(), ExecErr> {
    let pc = (sm83.pc() + op_size).to_le_bytes();
    sm83.dec_sp(1);
    mc.write(sm83.reg.sp, pc[1])?; // MSB first, as stack is _decreased_
    sm83.dec_sp(1);
    mc.write(sm83.reg.sp, pc[0])?;

    sm83.set_pc(addr);
    Ok(())
}

fn exec_prefix(sm83: &mut SM83, mc: &mut MemoryController) -> Result<(), ExecErr> {
    let op_code = mc.read(sm83.pc() + 1)?;
    EXEC_PREFIX_TABLE[op_code as usize](sm83)?;
    sm83.inc_pc(2);
    Ok(())
}

fn exec_prefix_invalid(_sm83: &mut SM83) -> Result<(), ExecErr> {
    Err(ExecErr::GeneralError("invalid prefix instruction"))
}

fn exec_prefix_swap_a(sm83: &mut SM83) -> Result<(), ExecErr> {
    let h = sm83.reg.a & 0xF0;
    let l = sm83.reg.a & 0x0F;
    sm83.reg.a = (l << 4) | (h >> 4);

    sm83.reg.set_flag(Z, (sm83.reg.a == 0) as u8);
    sm83.reg.set_flag(N, 0);
    sm83.reg.set_flag(H, 0);
    sm83.reg.set_flag(C, 0);
    Ok(())
}

pub static EXEC_TABLE: [Sm83Exec; psy::arch::sm83::SM83_NUM_INSTRUCTIONS] = [
    /*0x00*/ exec_nop,
    /*0x01*/ exec_ld_to_bc_from_immediate,
    /*0x02*/ exec_invalid,
    /*0x03*/ exec_invalid,
    /*0x04*/ exec_invalid,
    /*0x05*/ exec_dec_b,
    /*0x06*/ exec_ld_to_b_from_immediate,
    /*0x07*/ exec_invalid,
    /*0x08*/ exec_invalid,
    /*0x09*/ exec_invalid,
    /*0x0A*/ exec_invalid,
    /*0x0B*/ exec_dec_bc,
    /*0x0C*/ exec_inc_c,
    /*0x0D*/ exec_dec_c,
    /*0x0E*/ exec_ld_to_c_from_immediate,
    /*0x0F*/ exec_rrca,
    /*0x10*/ exec_invalid,
    /*0x11*/ exec_ld_to_de_from_immediate,
    /*0x12*/ exec_invalid,
    /*0x13*/ exec_inc_de,
    /*0x14*/ exec_invalid,
    /*0x15*/ exec_invalid,
    /*0x16*/ exec_ld_to_d_from_immediate,
    /*0x17*/ exec_invalid,
    /*0x18*/ exec_jr,
    /*0x19*/ exec_invalid,
    /*0x1A*/ exec_ld_to_a_from_deref_de,
    /*0x1B*/ exec_invalid,
    /*0x1C*/ exec_invalid,
    /*0x1D*/ exec_invalid,
    /*0x1E*/ exec_invalid,
    /*0x1F*/ exec_invalid,
    /*0x20*/ exec_jr_if_nz,
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
    /*0x2F*/ exec_cpl,
    /*0x30*/ exec_invalid,
    /*0x31*/ exec_ld_to_sp_from_immediate,
    /*0x32*/ exec_ld_to_deref_hl_dec_from_a,
    /*0x33*/ exec_invalid,
    /*0x34*/ exec_invalid,
    /*0x35*/ exec_invalid,
    /*0x36*/ exec_ld_to_deref_hl_from_immediate,
    /*0x37*/ exec_invalid,
    /*0x38*/ exec_jr_if_c,
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
    /*0x47*/ exec_ld_to_b_from_a,
    /*0x48*/ exec_invalid,
    /*0x49*/ exec_invalid,
    /*0x4A*/ exec_invalid,
    /*0x4B*/ exec_invalid,
    /*0x4C*/ exec_invalid,
    /*0x4D*/ exec_invalid,
    /*0x4E*/ exec_invalid,
    /*0x4F*/ exec_ld_to_c_from_a,
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
    /*0x5F*/ exec_ld_to_e_from_a,
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
    /*0x78*/ exec_ld_to_a_from_b,
    /*0x79*/ exec_ld_to_a_from_c,
    /*0x7A*/ exec_invalid,
    /*0x7B*/ exec_invalid,
    /*0x7C*/ exec_invalid,
    /*0x7D*/ exec_invalid,
    /*0x7E*/ exec_invalid,
    /*0x7F*/ exec_invalid,
    /*0x80*/ exec_invalid,
    /*0x81*/ exec_invalid,
    /*0x82*/ exec_invalid,
    /*0x83*/ exec_invalid,
    /*0x84*/ exec_invalid,
    /*0x85*/ exec_invalid,
    /*0x86*/ exec_invalid,
    /*0x87*/ exec_add_a_a,
    /*0x88*/ exec_invalid,
    /*0x89*/ exec_invalid,
    /*0x8A*/ exec_invalid,
    /*0x8B*/ exec_invalid,
    /*0x8C*/ exec_invalid,
    /*0x8D*/ exec_invalid,
    /*0x8E*/ exec_invalid,
    /*0x8F*/ exec_invalid,
    /*0x90*/ exec_invalid,
    /*0x91*/ exec_invalid,
    /*0x92*/ exec_invalid,
    /*0x93*/ exec_invalid,
    /*0x94*/ exec_invalid,
    /*0x95*/ exec_invalid,
    /*0x96*/ exec_invalid,
    /*0x97*/ exec_invalid,
    /*0x98*/ exec_invalid,
    /*0x99*/ exec_invalid,
    /*0x9A*/ exec_invalid,
    /*0x9B*/ exec_invalid,
    /*0x9C*/ exec_invalid,
    /*0x9D*/ exec_invalid,
    /*0x9E*/ exec_invalid,
    /*0x9F*/ exec_invalid,
    /*0xA0*/ exec_invalid,
    /*0xA1*/ exec_and_c,
    /*0xA2*/ exec_invalid,
    /*0xA3*/ exec_invalid,
    /*0xA4*/ exec_invalid,
    /*0xA5*/ exec_invalid,
    /*0xA6*/ exec_invalid,
    /*0xA7*/ exec_invalid,
    /*0xA8*/ exec_invalid,
    /*0xA9*/ exec_xor_a_c,
    /*0xAA*/ exec_invalid,
    /*0xAB*/ exec_invalid,
    /*0xAC*/ exec_invalid,
    /*0xAD*/ exec_invalid,
    /*0xAE*/ exec_invalid,
    /*0xAF*/ exec_xor_a_a,
    /*0xB0*/ exec_or_a_b,
    /*0xB1*/ exec_or_a_c,
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
    /*0xC9*/ exec_ret,
    /*0xCA*/ exec_invalid,
    /*0xCB*/ exec_prefix,
    /*0xCC*/ exec_invalid,
    /*0xCD*/ exec_call,
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
    /*0xE0*/ exec_ldh_to_immediate_from_a,
    /*0xE1*/ exec_pop_hl,
    /*0xE2*/ exec_ldh_to_deref_c_from_a,
    /*0xE3*/ exec_invalid,
    /*0xE4*/ exec_invalid,
    /*0xE5*/ exec_invalid,
    /*0xE6*/ exec_and_immediate,
    /*0xE7*/ exec_invalid,
    /*0xE8*/ exec_invalid,
    /*0xE9*/ exec_invalid,
    /*0xEA*/ exec_ld_to_deref_label_from_a,
    /*0xEB*/ exec_invalid,
    /*0xEC*/ exec_invalid,
    /*0xED*/ exec_invalid,
    /*0xEE*/ exec_invalid,
    /*0xEF*/ exec_rst_28,
    /*0xF0*/ exec_ldh_to_a_from_immediate,
    /*0xF1*/ exec_invalid,
    /*0xF2*/ exec_invalid,
    /*0xF3*/ exec_di,
    /*0xF4*/ exec_invalid,
    /*0xF5*/ exec_invalid,
    /*0xF6*/ exec_invalid,
    /*0xF7*/ exec_invalid,
    /*0xF8*/ exec_invalid,
    /*0xF9*/ exec_invalid,
    /*0xFA*/ exec_ld_to_a_from_deref_label,
    /*0xFB*/ exec_ei,
    /*0xFC*/ exec_invalid,
    /*0xFD*/ exec_invalid,
    /*0xFE*/ exec_cp_immediate,
    /*0xFF*/ exec_invalid,
];

pub static EXEC_PREFIX_TABLE: [Sm83PrefixExec; psy::arch::sm83::SM83_NUM_PREFIX_INSTRUCTIONS] = [
    /*0x00*/ exec_prefix_invalid,
    /*0x01*/ exec_prefix_invalid,
    /*0x02*/ exec_prefix_invalid,
    /*0x03*/ exec_prefix_invalid,
    /*0x04*/ exec_prefix_invalid,
    /*0x05*/ exec_prefix_invalid,
    /*0x06*/ exec_prefix_invalid,
    /*0x07*/ exec_prefix_invalid,
    /*0x08*/ exec_prefix_invalid,
    /*0x09*/ exec_prefix_invalid,
    /*0x0A*/ exec_prefix_invalid,
    /*0x0B*/ exec_prefix_invalid,
    /*0x0C*/ exec_prefix_invalid,
    /*0x0D*/ exec_prefix_invalid,
    /*0x0E*/ exec_prefix_invalid,
    /*0x0F*/ exec_prefix_invalid,
    /*0x10*/ exec_prefix_invalid,
    /*0x11*/ exec_prefix_invalid,
    /*0x12*/ exec_prefix_invalid,
    /*0x13*/ exec_prefix_invalid,
    /*0x14*/ exec_prefix_invalid,
    /*0x15*/ exec_prefix_invalid,
    /*0x16*/ exec_prefix_invalid,
    /*0x17*/ exec_prefix_invalid,
    /*0x18*/ exec_prefix_invalid,
    /*0x19*/ exec_prefix_invalid,
    /*0x1A*/ exec_prefix_invalid,
    /*0x1B*/ exec_prefix_invalid,
    /*0x1C*/ exec_prefix_invalid,
    /*0x1D*/ exec_prefix_invalid,
    /*0x1E*/ exec_prefix_invalid,
    /*0x1F*/ exec_prefix_invalid,
    /*0x20*/ exec_prefix_invalid,
    /*0x21*/ exec_prefix_invalid,
    /*0x22*/ exec_prefix_invalid,
    /*0x23*/ exec_prefix_invalid,
    /*0x24*/ exec_prefix_invalid,
    /*0x25*/ exec_prefix_invalid,
    /*0x26*/ exec_prefix_invalid,
    /*0x27*/ exec_prefix_invalid,
    /*0x28*/ exec_prefix_invalid,
    /*0x29*/ exec_prefix_invalid,
    /*0x2A*/ exec_prefix_invalid,
    /*0x2B*/ exec_prefix_invalid,
    /*0x2C*/ exec_prefix_invalid,
    /*0x2D*/ exec_prefix_invalid,
    /*0x2E*/ exec_prefix_invalid,
    /*0x2F*/ exec_prefix_invalid,
    /*0x30*/ exec_prefix_invalid,
    /*0x31*/ exec_prefix_invalid,
    /*0x32*/ exec_prefix_invalid,
    /*0x33*/ exec_prefix_invalid,
    /*0x34*/ exec_prefix_invalid,
    /*0x35*/ exec_prefix_invalid,
    /*0x36*/ exec_prefix_invalid,
    /*0x37*/ exec_prefix_swap_a,
    /*0x38*/ exec_prefix_invalid,
    /*0x39*/ exec_prefix_invalid,
    /*0x3A*/ exec_prefix_invalid,
    /*0x3B*/ exec_prefix_invalid,
    /*0x3C*/ exec_prefix_invalid,
    /*0x3D*/ exec_prefix_invalid,
    /*0x3E*/ exec_prefix_invalid,
    /*0x3F*/ exec_prefix_invalid,
    /*0x40*/ exec_prefix_invalid,
    /*0x41*/ exec_prefix_invalid,
    /*0x42*/ exec_prefix_invalid,
    /*0x43*/ exec_prefix_invalid,
    /*0x44*/ exec_prefix_invalid,
    /*0x45*/ exec_prefix_invalid,
    /*0x46*/ exec_prefix_invalid,
    /*0x47*/ exec_prefix_invalid,
    /*0x48*/ exec_prefix_invalid,
    /*0x49*/ exec_prefix_invalid,
    /*0x4A*/ exec_prefix_invalid,
    /*0x4B*/ exec_prefix_invalid,
    /*0x4C*/ exec_prefix_invalid,
    /*0x4D*/ exec_prefix_invalid,
    /*0x4E*/ exec_prefix_invalid,
    /*0x4F*/ exec_prefix_invalid,
    /*0x50*/ exec_prefix_invalid,
    /*0x51*/ exec_prefix_invalid,
    /*0x52*/ exec_prefix_invalid,
    /*0x53*/ exec_prefix_invalid,
    /*0x54*/ exec_prefix_invalid,
    /*0x55*/ exec_prefix_invalid,
    /*0x56*/ exec_prefix_invalid,
    /*0x57*/ exec_prefix_invalid,
    /*0x58*/ exec_prefix_invalid,
    /*0x59*/ exec_prefix_invalid,
    /*0x5A*/ exec_prefix_invalid,
    /*0x5B*/ exec_prefix_invalid,
    /*0x5C*/ exec_prefix_invalid,
    /*0x5D*/ exec_prefix_invalid,
    /*0x5E*/ exec_prefix_invalid,
    /*0x5F*/ exec_prefix_invalid,
    /*0x60*/ exec_prefix_invalid,
    /*0x61*/ exec_prefix_invalid,
    /*0x62*/ exec_prefix_invalid,
    /*0x63*/ exec_prefix_invalid,
    /*0x64*/ exec_prefix_invalid,
    /*0x65*/ exec_prefix_invalid,
    /*0x66*/ exec_prefix_invalid,
    /*0x67*/ exec_prefix_invalid,
    /*0x68*/ exec_prefix_invalid,
    /*0x69*/ exec_prefix_invalid,
    /*0x6A*/ exec_prefix_invalid,
    /*0x6B*/ exec_prefix_invalid,
    /*0x6C*/ exec_prefix_invalid,
    /*0x6D*/ exec_prefix_invalid,
    /*0x6E*/ exec_prefix_invalid,
    /*0x6F*/ exec_prefix_invalid,
    /*0x70*/ exec_prefix_invalid,
    /*0x71*/ exec_prefix_invalid,
    /*0x72*/ exec_prefix_invalid,
    /*0x73*/ exec_prefix_invalid,
    /*0x74*/ exec_prefix_invalid,
    /*0x75*/ exec_prefix_invalid,
    /*0x76*/ exec_prefix_invalid,
    /*0x77*/ exec_prefix_invalid,
    /*0x78*/ exec_prefix_invalid,
    /*0x79*/ exec_prefix_invalid,
    /*0x7A*/ exec_prefix_invalid,
    /*0x7B*/ exec_prefix_invalid,
    /*0x7C*/ exec_prefix_invalid,
    /*0x7D*/ exec_prefix_invalid,
    /*0x7E*/ exec_prefix_invalid,
    /*0x7F*/ exec_prefix_invalid,
    /*0x80*/ exec_prefix_invalid,
    /*0x81*/ exec_prefix_invalid,
    /*0x82*/ exec_prefix_invalid,
    /*0x83*/ exec_prefix_invalid,
    /*0x84*/ exec_prefix_invalid,
    /*0x85*/ exec_prefix_invalid,
    /*0x86*/ exec_prefix_invalid,
    /*0x87*/ exec_prefix_invalid,
    /*0x88*/ exec_prefix_invalid,
    /*0x89*/ exec_prefix_invalid,
    /*0x8A*/ exec_prefix_invalid,
    /*0x8B*/ exec_prefix_invalid,
    /*0x8C*/ exec_prefix_invalid,
    /*0x8D*/ exec_prefix_invalid,
    /*0x8E*/ exec_prefix_invalid,
    /*0x8F*/ exec_prefix_invalid,
    /*0x90*/ exec_prefix_invalid,
    /*0x91*/ exec_prefix_invalid,
    /*0x92*/ exec_prefix_invalid,
    /*0x93*/ exec_prefix_invalid,
    /*0x94*/ exec_prefix_invalid,
    /*0x95*/ exec_prefix_invalid,
    /*0x96*/ exec_prefix_invalid,
    /*0x97*/ exec_prefix_invalid,
    /*0x98*/ exec_prefix_invalid,
    /*0x99*/ exec_prefix_invalid,
    /*0x9A*/ exec_prefix_invalid,
    /*0x9B*/ exec_prefix_invalid,
    /*0x9C*/ exec_prefix_invalid,
    /*0x9D*/ exec_prefix_invalid,
    /*0x9E*/ exec_prefix_invalid,
    /*0x9F*/ exec_prefix_invalid,
    /*0xA0*/ exec_prefix_invalid,
    /*0xA1*/ exec_prefix_invalid,
    /*0xA2*/ exec_prefix_invalid,
    /*0xA3*/ exec_prefix_invalid,
    /*0xA4*/ exec_prefix_invalid,
    /*0xA5*/ exec_prefix_invalid,
    /*0xA6*/ exec_prefix_invalid,
    /*0xA7*/ exec_prefix_invalid,
    /*0xA8*/ exec_prefix_invalid,
    /*0xA9*/ exec_prefix_invalid,
    /*0xAA*/ exec_prefix_invalid,
    /*0xAB*/ exec_prefix_invalid,
    /*0xAC*/ exec_prefix_invalid,
    /*0xAD*/ exec_prefix_invalid,
    /*0xAE*/ exec_prefix_invalid,
    /*0xAF*/ exec_prefix_invalid,
    /*0xB0*/ exec_prefix_invalid,
    /*0xB1*/ exec_prefix_invalid,
    /*0xB2*/ exec_prefix_invalid,
    /*0xB3*/ exec_prefix_invalid,
    /*0xB4*/ exec_prefix_invalid,
    /*0xB5*/ exec_prefix_invalid,
    /*0xB6*/ exec_prefix_invalid,
    /*0xB7*/ exec_prefix_invalid,
    /*0xB8*/ exec_prefix_invalid,
    /*0xB9*/ exec_prefix_invalid,
    /*0xBA*/ exec_prefix_invalid,
    /*0xBB*/ exec_prefix_invalid,
    /*0xBC*/ exec_prefix_invalid,
    /*0xBD*/ exec_prefix_invalid,
    /*0xBE*/ exec_prefix_invalid,
    /*0xBF*/ exec_prefix_invalid,
    /*0xC0*/ exec_prefix_invalid,
    /*0xC1*/ exec_prefix_invalid,
    /*0xC2*/ exec_prefix_invalid,
    /*0xC3*/ exec_prefix_invalid,
    /*0xC4*/ exec_prefix_invalid,
    /*0xC5*/ exec_prefix_invalid,
    /*0xC6*/ exec_prefix_invalid,
    /*0xC7*/ exec_prefix_invalid,
    /*0xC8*/ exec_prefix_invalid,
    /*0xC9*/ exec_prefix_invalid,
    /*0xCA*/ exec_prefix_invalid,
    /*0xCB*/ exec_prefix_invalid,
    /*0xCC*/ exec_prefix_invalid,
    /*0xCD*/ exec_prefix_invalid,
    /*0xCE*/ exec_prefix_invalid,
    /*0xCF*/ exec_prefix_invalid,
    /*0xD0*/ exec_prefix_invalid,
    /*0xD1*/ exec_prefix_invalid,
    /*0xD2*/ exec_prefix_invalid,
    /*0xD3*/ exec_prefix_invalid,
    /*0xD4*/ exec_prefix_invalid,
    /*0xD5*/ exec_prefix_invalid,
    /*0xD6*/ exec_prefix_invalid,
    /*0xD7*/ exec_prefix_invalid,
    /*0xD8*/ exec_prefix_invalid,
    /*0xD9*/ exec_prefix_invalid,
    /*0xDA*/ exec_prefix_invalid,
    /*0xDB*/ exec_prefix_invalid,
    /*0xDC*/ exec_prefix_invalid,
    /*0xDD*/ exec_prefix_invalid,
    /*0xDE*/ exec_prefix_invalid,
    /*0xDF*/ exec_prefix_invalid,
    /*0xE0*/ exec_prefix_invalid,
    /*0xE1*/ exec_prefix_invalid,
    /*0xE2*/ exec_prefix_invalid,
    /*0xE3*/ exec_prefix_invalid,
    /*0xE4*/ exec_prefix_invalid,
    /*0xE5*/ exec_prefix_invalid,
    /*0xE6*/ exec_prefix_invalid,
    /*0xE7*/ exec_prefix_invalid,
    /*0xE8*/ exec_prefix_invalid,
    /*0xE9*/ exec_prefix_invalid,
    /*0xEA*/ exec_prefix_invalid,
    /*0xEB*/ exec_prefix_invalid,
    /*0xEC*/ exec_prefix_invalid,
    /*0xED*/ exec_prefix_invalid,
    /*0xEE*/ exec_prefix_invalid,
    /*0xEF*/ exec_prefix_invalid,
    /*0xF0*/ exec_prefix_invalid,
    /*0xF1*/ exec_prefix_invalid,
    /*0xF2*/ exec_prefix_invalid,
    /*0xF3*/ exec_prefix_invalid,
    /*0xF4*/ exec_prefix_invalid,
    /*0xF5*/ exec_prefix_invalid,
    /*0xF6*/ exec_prefix_invalid,
    /*0xF7*/ exec_prefix_invalid,
    /*0xF8*/ exec_prefix_invalid,
    /*0xF9*/ exec_prefix_invalid,
    /*0xFA*/ exec_prefix_invalid,
    /*0xFB*/ exec_prefix_invalid,
    /*0xFC*/ exec_prefix_invalid,
    /*0xFD*/ exec_prefix_invalid,
    /*0xFE*/ exec_prefix_invalid,
    /*0xFF*/ exec_prefix_invalid,
];

// helper

fn half_carry_dec(v: u8) -> u8 {
    if v & 0x0F == 0x0F { 1 } else { 0 }
}

fn half_carry_inc(v_before_inc: u8) -> u8 {
    ((v_before_inc & 0x0F) + 1 > 0x0F) as u8
}
