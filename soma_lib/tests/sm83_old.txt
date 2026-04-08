extern crate libsoma;

use libsoma::sm83_old;

#[test]
fn test_nop() {
    let s = exec(sm83_old::nop);
    assert_eq!(s, state_no_mem());
}

#[test]
fn test_inc_a() {
    let mut s = exec(sm83_old::inc_a);
    assert_eq!(s.reg.a, 1);
    sm83_old::inc_a(&mut s);
    assert_eq!(s.reg.a, 2);

    s.reg.a = 0;
    assert_eq!(s, state_no_mem());
}

#[test]
fn test_inc_l() {
    let mut s = exec(sm83_old::inc_l);
    assert_eq!(s.reg.l, 1);
    sm83_old::inc_l(&mut s);
    assert_eq!(s.reg.l, 2);

    s.reg.l = 0;
    assert_eq!(s, state_no_mem());
}

#[test]
fn test_ld_bc_a() {
    let mut mem = vec![0; 0x3000];
    mem[0x205F] = 0x3F;
    let mut s = state_mem(mem);
    s.reg.b = 0x20;
    s.reg.c = 0x5F;
    sm83_old::ld_bc_a(&mut s);

    assert_eq!(s.reg.a, 0x3F);
    assert_eq!(s.reg.pc, 2);
}

#[test]
fn test_ld_de() {
    let mem = [0x11, 0x3A, 0x5B].to_vec();
    let s = exec_mem(mem, sm83_old::ld_de);

    assert_eq!(s.reg.pc, 0x02);
    assert_eq!(s.reg.d, 0x3A);
    assert_eq!(s.reg.e, 0x5B);
}

#[test]
fn test_call() {
    let mut mem = vec![0; 0x3000];
    mem[0x2000] = 0xCD;
    mem[0x2001] = 0x34;
    mem[0x2002] = 0x12;
    let mut s = state_mem(mem);
    s.reg.pc = 0x2000;

    sm83_old::call(&mut s);
    assert_eq!(s.reg.sp, 125);
    assert_eq!(s.reg.pc, 0x1234);
    assert_eq!(s.stack[126], 0x20);
    assert_eq!(s.stack[125], 0x03);
}

#[test]
fn test_rst_38() {
    let mut s = state_no_mem();
    s.reg.pc = 0x2000;

    sm83_old::rst_38(&mut s);
    assert_eq!(s.reg.sp, 125);
    assert_eq!(s.reg.pc, 0x38);
    assert_eq!(s.stack[126], 0x20);
    assert_eq!(s.stack[125], 0x03);
}

#[test]
fn test_or_d() {
    let mut s = state_no_mem();
    s.reg.a = 0b10101010;
    s.reg.d = 0b01010101;
    sm83_old::or_d(&mut s);

    assert_eq!(s.reg.a, 0xFF);
}

#[test]
fn test_sub_byte_non_zero() {
    let mem = [0xD6, 0x0F].to_vec();
    let mut s = state_mem(mem);
    s.reg.a = 0x3E;
    sm83_old::sub_byte(&mut s);

    assert_eq!(s.reg.pc, 0x01);
    assert_eq!(s.reg.a, 0x2F);
    assert_eq!(s.reg.zero_flag(), false);
}

#[test]
fn test_sub_byte_zero() {
    let mem = [0xD6, 0x3E].to_vec();
    let mut s = state_mem(mem);
    s.reg.a = 0x3E;
    sm83_old::sub_byte(&mut s);

    assert_eq!(s.reg.pc, 0x01);
    assert_eq!(s.reg.a, 0x0);
    assert_zchn(&s, true, false, false, true);
}

#[test]
fn test_sub_byte_carry() {
    let mem = [0xD6, 0x40].to_vec();
    let mut s = state_mem(mem);
    s.reg.a = 0x3E;
    sm83_old::sub_byte(&mut s);

    assert_eq!(s.reg.pc, 0x01);
    assert_eq!(s.reg.a, 0xFE);
    assert_zchn(&s, false, true, false, true);
}

#[test]
fn test_sub_byte_half_carry() {
    let mem = [0xD6, 0x0F].to_vec();
    let mut s = state_mem(mem);
    s.reg.a = 0x3E;
    sm83_old::sub_byte(&mut s);

    assert_eq!(s.reg.pc, 0x01);
    assert_eq!(s.reg.a, 0x2F);
    assert_zchn(&s, false, false, true, true);
}

#[test]
fn test_djnz_non_zero_result() {
    let mem = [0x10, 0xFF].to_vec();
    let mut s = state_mem(mem);
    s.reg.b = 0xFF;
    sm83_old::djnz(&mut s);

    assert_eq!(s.reg.pc, 0xFF);
    assert_eq!(s.reg.b, 0xFE);
}

#[test]
fn test_djnz_non_zero_result_overflow() {
    let mem = [0x10, 0xFF].to_vec();
    let mut s = state_mem(mem);
    s.reg.b = 0x0;
    sm83_old::djnz(&mut s);

    assert_eq!(s.reg.pc, 0xFF);
    assert_eq!(s.reg.b, 0xFF);
}

#[test]
fn test_djnz_zero_result() {
    let mem = [0x10, 0xFF].to_vec();
    let mut s = state_mem(mem);
    s.reg.b = 0x01;
    sm83_old::djnz(&mut s);

    assert_eq!(s.reg.pc, 0x00);
    assert_eq!(s.reg.b, 0x00);
}

#[test]
fn test_add_a_hl_non_zero() {
    let mut mem = vec![0; 0x3000];
    mem[0x205F] = 0x3F;
    let mut s = state_mem(mem);
    s.reg.a = 0x05;
    s.reg.h = 0x20;
    s.reg.l = 0x5F;
    sm83_old::add_a_hl(&mut s);

    assert_eq!(s.reg.a, 0x3F + 0x05);
    assert_zchn(&s, false, false, true, false);
}

#[test]
fn test_add_a_hl_zero() {
    let mut mem = vec![0; 0x3000];
    mem[0x205F] = 0x3F;
    let mut s = state_mem(mem);
    s.reg.a = 0xC1;
    s.reg.h = 0x20;
    s.reg.l = 0x5F;
    sm83_old::add_a_hl(&mut s);

    assert_eq!(s.reg.a, 0x0);
    assert_zchn(&s, true, true, true, false);
}

#[test]
fn test_add_a_hl_carry() {
    let mut mem = vec![0; 0x3000];
    mem[0x205F] = 0x02;
    let mut s = state_mem(mem);
    s.reg.a = 0xFF;
    s.reg.h = 0x20;
    s.reg.l = 0x5F;
    sm83_old::add_a_hl(&mut s);

    assert_eq!(s.reg.a, 0x01);
    assert_zchn(&s, false, true, true, false);
}

#[test]
fn test_add_a_hl_half_carry() {
    let mut mem = vec![0; 0x3000];
    mem[0x205F] = 0xC6;
    let mut s = state_mem(mem);
    s.reg.a = 0x3A;
    s.reg.h = 0x20;
    s.reg.l = 0x5F;
    sm83_old::add_a_hl(&mut s);

    assert_eq!(s.reg.a, 0x0);
    assert_zchn(&s, true, true, true, false);
}

#[test]
fn test_add_hl_hl_non_zero_and_carry() {
    let mut s = state_no_mem();
    s.reg.h = 0x8A;
    s.reg.l = 0x23;
    sm83_old::add_hl_hl(&mut s);

    assert_eq!(s.reg.h, 0x14);
    assert_eq!(s.reg.l, 0x46);

    assert_zchn(&s, false, true, true, false);
}

#[test]
fn test_add_hl_hl_zero() {
    let mut s = state_no_mem();
    s.reg.h = 0x0;
    s.reg.l = 0x0;
    sm83_old::add_hl_hl(&mut s);

    assert_eq!(s.reg.h, 0x0);
    assert_eq!(s.reg.l, 0x0);

    assert_zchn(&s, true, false, false, false);
}

//template reg tests
//xxx_non_zero, xxx_zero, xxx_carry, xxx_half_carry

//register helpers

fn assert_zchn(s: &sm83_old::State, z: bool, c: bool, h: bool, n: bool) {
    assert_eq!(z, s.reg.zero_flag(), "zero flag");
    assert_eq!(c, s.reg.carry_flag(), "carry flag");
    assert_eq!(h, s.reg.half_carry_flag(), "half carry flag");
    assert_eq!(n, s.reg.n_flag(), "n flag");
}

#[test]
fn test_zero_flag() {
    let mut s = state_no_mem();
    s.reg.set_zero_flag(true);
    assert_eq!(true, s.reg.zero_flag());

    s.reg.set_zero_flag(false);
    assert_eq!(false, s.reg.zero_flag());
}

#[test]
fn test_n_flag() {
    let mut s = state_no_mem();
    s.reg.set_n_flag(true);
    assert_eq!(true, s.reg.n_flag());

    s.reg.set_n_flag(false);
    assert_eq!(false, s.reg.n_flag());
}

#[test]
fn test_half_carry_flag() {
    let mut s = state_no_mem();
    s.reg.set_half_carry_flag(true);
    assert_eq!(true, s.reg.half_carry_flag());

    s.reg.set_half_carry_flag(false);
    assert_eq!(false, s.reg.half_carry_flag());
}

#[test]
fn test_carry_flag() {
    let mut s = state_no_mem();
    s.reg.set_carry_flag(true);
    assert_eq!(true, s.reg.carry_flag());

    s.reg.set_carry_flag(false);
    assert_eq!(false, s.reg.carry_flag());
}

//read mem methods

#[test]
fn test_read_u16_le() {
    let mem = [0x3C, 0x50, 0x01];
    let result = sm83_old::read_u16_le(0, &mem);
    assert_eq!(result, 0x0150);
}

#[test]
fn test_read_u8() {
    let mem = [0xD6, 0x66];
    let result = sm83_old::read_u8(0, &mem);
    assert_eq!(result, 0x66);
}

#[test]
fn test_u16_reg() {
    let result = sm83_old::u16_reg(0x20, 0x5F);
    assert_eq!(result, 0x205F);
}

//helper methods

fn state_no_mem() -> sm83_old::State {
    sm83_old::initial_state(Vec::new(), 127, 0x0)
}

fn state_mem(mem: Vec<u8>) -> sm83_old::State {
    sm83_old::initial_state(mem, 127, 0x0)
}

fn exec(effect: fn(&mut sm83_old::State)) -> sm83_old::State {
    let mut s = state_no_mem();
    (effect)(&mut s);
    return s;
}

fn exec_mem(mem: Vec<u8>, effect: fn(&mut sm83_old::State)) -> sm83_old::State {
    let mut s = sm83_old::initial_state(mem, 127, 0x0);
    (effect)(&mut s);
    return s;
}
