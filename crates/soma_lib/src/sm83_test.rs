use crate::ROM;
use crate::io::IO;
use crate::memory::MemoryController;
use crate::sm83::{RegBuilder, Register, SM83};

#[test]
fn test_err() -> Result<(), &'static str> {
    let cases = [(
        [psy::arch::sm83::INSTR_INVALID.op_code],
        "invalid instruction",
    )];

    for (mem, err) in cases {
        let rom = ROM::new(&mem);
        let r = exec(IO::init(), Register::zero(), rom);
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
        let rom = ROM::new(&mem);
        let (sm83, _) = exec(IO::init(), Register::zero(), rom)?;
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
fn test_jr() -> Result<(), &'static str> {
    let cases = [
        (
            "(jr #c 0xF9)",
            RegBuilder::new().pc(7).f_c(1).reg(),
            [
                0x0,
                0x0,
                0x0,
                0x0,
                0x0,
                0x0,
                0x0,
                psy::arch::sm83::INSTR_JR_IF_C.op_code,
                0xF9,
            ],
            2,
        ),
        (
            "(jr #c 0xF9)",
            RegBuilder::new().pc(7).f_c(0).reg(),
            [
                0x0,
                0x0,
                0x0,
                0x0,
                0x0,
                0x0,
                0x0,
                psy::arch::sm83::INSTR_JR_IF_C.op_code,
                0xF9,
            ],
            9,
        ),
    ];

    for (exp, reg_init, mem, pc) in cases {
        let rom = ROM::new(&mem);
        let (sm83, _) = exec(IO::init(), reg_init, rom)?;
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
    let cases: [(&str, IO, Register, &[u8], u16, Register, &[(u16, u8)]); 9] = [
        (
            "(ld %a 1)",
            IO::init(),
            Register::zero(),
            &[psy::arch::sm83::INSTR_LD_TO_A_FROM_IMMEDIATE.op_code, 1],
            2,
            RegBuilder::new().a(1).reg(),
            &[],
        ),
        (
            "(ld ('label) %a)",
            IO::init(),
            RegBuilder::new().a(0xAB).reg(),
            &[
                psy::arch::sm83::INSTR_LD_TO_DEREF_LABEL_FROM_A.op_code,
                0x26, // IO-Port Address
                0xFF,
            ],
            3,
            RegBuilder::new().a(0xAB).reg(), // reg a stays unchanged
            &[(0xFF26, 0xAB)],
        ),
        (
            "(ld (%hl +) %a)",
            IO::init(),
            RegBuilder::new().a(0xAB).hl(0xFF26).reg(),
            &[psy::arch::sm83::INSTR_LD_TO_DEREF_HL_INC_FROM_A.op_code],
            1,
            RegBuilder::new().a(0xAB).hl(0xFF27).reg(), // reg a stays unchanged
            &[(0xFF26, 0xAB)],
        ),
        (
            "(ld %a ('label))",
            IO::init_with_value(0xFF44, 23),
            Register::zero(),
            &[
                psy::arch::sm83::INSTR_LD_TO_A_FROM_DEREF_LABEL.op_code,
                0x44,
                0xFF,
            ],
            3,
            RegBuilder::new().a(23).reg(),
            &[],
        ),
        (
            "(ld %a (%de))",
            IO::init(),
            RegBuilder::new().d(0x00).e(0x04).reg(),
            &[
                psy::arch::sm83::INSTR_LD_TO_A_FROM_DEREF_DE.op_code,
                0x00, //0x01
                0x00, //0x02
                0x00, //0x03
                42,   //0x04
            ],
            1,
            RegBuilder::new().d(0x00).e(0x04).a(42).reg(),
            &[],
        ),
        (
            "(ld %a (%hl +))",
            IO::init(),
            RegBuilder::new().h(0x00).l(0x05).reg(),
            &[
                psy::arch::sm83::INSTR_LD_TO_A_FROM_DEREF_HL_INC.op_code,
                0x00, //0x01
                0x00, //0x02
                0x00, //0x03
                0x00, //0x04
                32,   //0x05
            ],
            1,
            RegBuilder::new().h(0x00).l(0x06).a(32).reg(),
            &[],
        ),
        (
            "(ld %de 0x8F01)",
            IO::init(),
            Register::zero(),
            &[
                psy::arch::sm83::INSTR_LD_TO_DE_FROM_IMMEDIATE.op_code,
                0x8F,
                0x01,
            ],
            3,
            RegBuilder::new().d(0x01).e(0x8F).reg(),
            &[],
        ),
        (
            "(ld %hl 0x9000)",
            IO::init(),
            Register::zero(),
            &[
                psy::arch::sm83::INSTR_LD_TO_HL_FROM_IMMEDIATE.op_code,
                0x90,
                0x00,
            ],
            3,
            RegBuilder::new().h(0x00).l(0x90).reg(),
            &[],
        ),
        (
            "(ld %bc 0x6004)",
            IO::init(),
            Register::zero(),
            &[
                psy::arch::sm83::INSTR_LD_TO_BC_FROM_IMMEDIATE.op_code,
                0x60,
                0x04,
            ],
            3,
            RegBuilder::new().b(0x04).c(0x60).reg(),
            &[],
        ),
    ];

    for (exp, io, reg_start, mem, pc_at, reg_after, mem_checks) in cases {
        let rom = ROM::new(mem);
        let (sm83, mc) = exec(io, reg_start, rom)?;
        assert_eq!(
            sm83.pc(),
            pc_at,
            "expected pc at 0x{:x}, was at 0x{:x} for {}",
            pc_at,
            sm83.pc(),
            exp,
        );
        assert_equal_v_regs(&sm83.reg, &reg_after, exp);

        for check in mem_checks {
            assert_eq!(mc.read(check.0), check.1);
        }
    }
    Ok(())
}

#[test]
fn test_cp() -> Result<(), &'static str> {
    let cases: [(&str, Register, &[u8], Register); 2] = [
        (
            "(cp 0x90) with a = 1 (not equal)",
            RegBuilder::new().a(1).reg(),
            &[psy::arch::sm83::INSTR_CP_IMMEDIATE.op_code, 0x90],
            RegBuilder::new().a(1).f_z(1).f_n(1).f_h(1).f_c(1).reg(),
        ),
        (
            "(cp 0x90) with a = 0x90 (equal)",
            RegBuilder::new().a(0x90).reg(),
            &[psy::arch::sm83::INSTR_CP_IMMEDIATE.op_code, 0x90],
            RegBuilder::new().a(0x90).f_z(0).f_n(1).f_h(0).f_c(0).reg(),
        ),
    ];

    for (exp, reg_start, mem, reg_after) in cases {
        let rom = ROM::new(mem);
        let (sm83, _) = exec(IO::init(), reg_start, rom)?;
        assert_eq!(sm83.pc(), mem.len() as u16);
        assert_equal_v_regs(&sm83.reg, &reg_after, exp);
    }
    Ok(())
}

#[test]
fn test_inc() -> Result<(), &'static str> {
    let cases = [
        (
            "(inc %de) with zero %de",
            RegBuilder::new().de(0x00).reg(),
            &[psy::arch::sm83::INSTR_INC_DE.op_code],
            RegBuilder::new().de(0x01).reg(),
        ),
        (
            "(inc %de) with non-zero %de",
            RegBuilder::new().de(0x666).reg(),
            &[psy::arch::sm83::INSTR_INC_DE.op_code],
            RegBuilder::new().de(0x667).reg(),
        ),
    ];

    for (exp, reg_init, mem, reg_after) in cases {
        let rom = ROM::new(mem);
        let (sm83, _) = exec(IO::init(), reg_init, rom)?;
        assert_eq!(
            sm83.pc(),
            1,
            "{}, want pc 0x{:x}, got 0x{:x}",
            exp,
            1,
            sm83.pc()
        );
        assert_equal_v_regs(&sm83.reg, &reg_after, exp);
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

fn exec(io: IO, reg: Register, rom: ROM) -> Result<(SM83, MemoryController), &'static str> {
    let mut mc = MemoryController {
        io: io,
        rom: Some(rom),
        vram: [0; 8192],
    };
    let mut sm83 = SM83::init();
    sm83.reg = reg;
    sm83.execute(&mut mc)?;
    Ok((sm83, mc))
}
