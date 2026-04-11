use crate::ROM;
use crate::sm83::{Register, SM83};

#[test]
fn test_err() -> Result<(), &'static str> {
    let cases = [(
        [psy::arch::sm83::INSTR_INVALID.op_code],
        "invalid instruction",
    )];

    for (mem, err) in cases {
        let r = exec_mem(&mem);
        assert!(r.is_err(), "expected error '{}', but got Ok", err);
        match r {
            Ok(_) => assert!(false, "error expected"),
            Err(e) => assert_eq!(e, err),
        }
    }
    Ok(())
}

#[test]
fn test_jp() -> Result<(), &'static str> {
    let cases = [(
        "(jp 0x150)",
        [psy::arch::sm83::INSTR_JP.op_code, 0xAA, 0xFF],
        0xFFAA,
    )];

    for (exp, mem, pc) in cases {
        let sm83 = exec_mem(&mem)?;
        assert_eq!(
            sm83.pc(),
            pc,
            "{}, want pc 0x{:x}, got 0x{:x}",
            exp,
            pc,
            sm83.pc()
        );
    }
    Ok(())
}

#[test]
fn test_ld() -> Result<(), &'static str> {
    let cases = [(
        "(ld %a 1)",
        [psy::arch::sm83::INSTR_LD_TO_A_FROM_IMMEDIATE.op_code, 1],
        Register::a(1),
    )];

    for (exp, mem, reg) in cases {
        let sm83 = exec_mem(&mem)?;
        assert_eq!(sm83.pc(), 2);
        assert_equal_v_regs(&sm83.reg, &reg, exp);
    }
    Ok(())
}

// helper

/// conly compares the value register a to l, without pc and sp.
fn assert_equal_v_regs(l: &Register, r: &Register, exp: &str) {
    assert_eq!(l.a, r.a, "reg a: {}", exp);
    assert_eq!(l.b, r.b, "reg b: {}", exp);
    assert_eq!(l.c, r.c, "reg c: {}", exp);
    assert_eq!(l.d, r.d, "reg d: {}", exp);
    assert_eq!(l.e, r.e, "reg e: {}", exp);
    assert_eq!(l.f, r.f, "reg f: {}", exp);
    assert_eq!(l.h, r.h, "reg h: {}", exp);
    assert_eq!(l.l, r.l, "reg l: {}", exp);
}

fn exec_mem(mem: &[u8]) -> Result<SM83, &'static str> {
    let mut sm83 = SM83::init();
    sm83.execute(&ROM::new(mem))?;
    Ok(sm83)
}
