use crate::io::IO;
use crate::memory::MemoryController;
use crate::rom::ROM;
use crate::sm83::{C, ExecErr, H, N, RegBuilder, Register, SM83, Z};

#[test]
fn test_err() -> Result<(), ExecErr> {
    let cases = [(
        [psy::arch::sm83::INSTR_INVALID.op_code],
        ExecErr::InvalidInstruction(psy::arch::sm83::INSTR_INVALID.op_code, 0),
    )];

    for (mem, err) in cases {
        let rom = ROM::new_copy_from_slice(&mem);
        let r = exec(IO::init(), Register::zero(), rom);
        assert!(r.is_err(), "expected error '{:?}', but got Ok", err);
        match r {
            Ok(_) => assert!(false, "error expected"),
            Err(e) => assert_eq!(e, err),
        }
    }
    Ok(())
}

#[test]
fn test_nop() -> Result<(), ExecErr> {
    let cases = [([psy::arch::sm83::INSTR_NOP.op_code])];

    for mem in cases {
        let rom = ROM::new_copy_from_slice(&mem);
        let (sm83, _) = exec(IO::init(), Register::zero(), rom)?;
        assert_eq!(sm83.pc(), 1, "nop");
        assert_equal_v_regs(&sm83.reg, &Register::zero(), "nop");
    }
    Ok(())
}

#[test]
fn test_interrupt_enablement() -> Result<(), ExecErr> {
    let cases = [
        ("(ei)", [psy::arch::sm83::INSTR_EI.op_code], true),
        ("(di)", [psy::arch::sm83::INSTR_DI.op_code], false),
    ];

    for (exp, mem, ime_flag) in cases {
        let rom = ROM::new_copy_from_slice(&mem);
        let (sm83, _) = exec(IO::init(), Register::zero(), rom)?;
        assert_eq!(
            sm83.reg.ime, ime_flag,
            "{}, want ime {}, got {}",
            exp, ime_flag, sm83.reg.ime
        );
        assert_eq!(sm83.pc(), 1);
    }
    Ok(())
}

#[test]
fn test_jp() -> Result<(), ExecErr> {
    let cases = [
        (
            "(jp 0xFFAA)",
            [psy::arch::sm83::INSTR_JP.op_code, 0xAA, 0xFF],
            Register::zero(),
            RegBuilder::new().pc(0xFFAA).reg(),
        ),
        (
            "(jp %hl)",
            [psy::arch::sm83::INSTR_JP_HL.op_code, 0x00, 0x00],
            RegBuilder::new().hl(0x168).reg(),
            RegBuilder::new().hl(0x168).pc(0x168).reg(),
        ),
        (
            "(jp #z 0x666) if #z is zero",
            [psy::arch::sm83::INSTR_JP_IF_Z.op_code, 0x66, 0x06],
            RegBuilder::new().f_z(1).reg(),
            RegBuilder::new().f_z(1).pc(0x666).reg(),
        ),
        (
            "(jp #z 0x666) if #z is not zero",
            [psy::arch::sm83::INSTR_JP_IF_Z.op_code, 0x66, 0x06],
            RegBuilder::new().f_z(0).reg(),
            RegBuilder::new().f_z(0).pc(0x3).reg(),
        ),
    ];

    for (exp, mem, reg_before, reg_after) in cases {
        let rom = ROM::new_copy_from_slice(&mem);
        let (sm83, _) = exec(IO::init(), reg_before, rom)?;

        assert_equal_v_regs(&sm83.reg, &reg_after, exp);
        assert_eq!(sm83.reg.pc, reg_after.pc);
    }
    Ok(())
}

#[test]
fn test_jr() -> Result<(), ExecErr> {
    let cases: [(&str, Register, &[u8], u16); 9] = [
        (
            "(jr 0xFE)", //self-jump
            RegBuilder::new().pc(1).reg(),
            &[0x0, psy::arch::sm83::INSTR_JR.op_code, 0xFE],
            1,
        ),
        (
            "(jr 0xF9)",
            RegBuilder::new().pc(7).reg(),
            &[
                0x0,
                0x0,
                0x0,
                0x0,
                0x0,
                0x0,
                0x0,
                psy::arch::sm83::INSTR_JR.op_code,
                0xF9,
            ],
            2,
        ),
        (
            "(jr 0x02)",
            RegBuilder::new().pc(0).reg(),
            &[psy::arch::sm83::INSTR_JR.op_code, 0x02],
            4,
        ),
        (
            "(jr #c 0xF9)",
            RegBuilder::new().pc(7).f_c(1).reg(),
            &[
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
            &[
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
        (
            "(jr #nz 0xF8) if #z zero",
            RegBuilder::new().pc(8).f_z(1).reg(),
            &[
                0x0,
                0x0,
                0x0,
                0x0,
                0x0,
                0x0,
                0x0,
                0x0,
                psy::arch::sm83::INSTR_JR_IF_NZ.op_code,
                0xF8, // -7 jump
            ],
            10, // don't jump, as z is zero
        ),
        (
            "(jr #nz 0xF8) if #z not zero",
            RegBuilder::new().pc(8).f_z(0).reg(),
            &[
                0x0,
                0x0,
                0x0,
                0x0,
                0x0,
                0x0,
                0x0,
                0x0,
                psy::arch::sm83::INSTR_JR_IF_NZ.op_code,
                0xF8, // -7 jump
            ],
            2, // jump back to pc=2, as z is not zero
        ),
        (
            "(jr #z 0x6) if #z zero",
            RegBuilder::new().pc(0).f_z(1).reg(),
            &[psy::arch::sm83::INSTR_JR_IF_Z.op_code, 0x6],
            8, // jump 6 instructions forward
        ),
        (
            "(jr #z 0x06) if #z is not zero",
            RegBuilder::new().pc(0).f_z(0).reg(),
            &[psy::arch::sm83::INSTR_JR_IF_Z.op_code, 0x06, 0x0],
            2, // don't jump and go to next instruction
        ),
    ];

    for (exp, reg_init, mem, pc) in cases {
        let rom = ROM::new_copy_from_slice(mem);
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
fn test_ld() -> Result<(), ExecErr> {
    let cases: [(&str, IO, Register, &[u8], u16, Register, &[(u16, u8)]); 25] = [
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
            "(ld %b 65)",
            IO::init(),
            Register::zero(),
            &[psy::arch::sm83::INSTR_LD_TO_B_FROM_IMMEDIATE.op_code, 65],
            2,
            RegBuilder::new().b(65).reg(),
            &[],
        ),
        (
            "(ld %c 42)",
            IO::init(),
            Register::zero(),
            &[psy::arch::sm83::INSTR_LD_TO_C_FROM_IMMEDIATE.op_code, 42],
            2,
            RegBuilder::new().c(42).reg(),
            &[],
        ),
        (
            "(ld %d 32)",
            IO::init(),
            Register::zero(),
            &[psy::arch::sm83::INSTR_LD_TO_D_FROM_IMMEDIATE.op_code, 32],
            2,
            RegBuilder::new().d(32).reg(),
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
            "(ld (%de) %a)",
            IO::init(),
            RegBuilder::new().a(0x5).de(0xFF16).reg(),
            &[psy::arch::sm83::INSTR_LD_TO_DEREF_DE_FROM_A.op_code],
            1,
            RegBuilder::new().a(0x5).de(0xFF16).reg(), // reg a stays unchanged
            &[(0xFF16, 0x5)],                          // address stores register value
        ),
        (
            "(ld (%hl +) %a)",
            IO::init(),
            RegBuilder::new().a(0xAB).hl(0xFF26).reg(),
            &[psy::arch::sm83::INSTR_LD_TO_DEREF_HL_INC_FROM_A.op_code],
            1,
            RegBuilder::new().a(0xAB).hl(0xFF27).reg(), // reg a stays unchanged
            &[(0xFF26, 0xAB)], // address before increment stores register value
        ),
        (
            "(ld (%hl -) %a)",
            IO::init(),
            RegBuilder::new().a(0xAB).hl(0xFF26).reg(),
            &[psy::arch::sm83::INSTR_LD_TO_DEREF_HL_DEC_FROM_A.op_code],
            1,
            RegBuilder::new().a(0xAB).hl(0xFF25).reg(), // reg a stays unchanged
            &[(0xFF26, 0xAB)], // address after increment stores register value
        ),
        (
            "(ld %a ('label))",
            IO::init_with_value(0xFF44, 23)?,
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
            "(ld %a (%hl))",
            IO::init(),
            RegBuilder::new().hl(0x0004).reg(),
            &[
                psy::arch::sm83::INSTR_LD_TO_A_FROM_DEREF_HL.op_code,
                0x00, //0x01
                0x00, //0x02
                0x00, //0x03
                0x66, //0x04
            ],
            1,
            RegBuilder::new().hl(0x0004).a(0x66).reg(),
            &[],
        ),
        (
            "(ld %d (%hl))",
            IO::init(),
            RegBuilder::new().hl(0x0004).reg(),
            &[
                psy::arch::sm83::INSTR_LD_TO_D_FROM_DEREF_HL.op_code,
                0x00, //0x01
                0x00, //0x02
                0x00, //0x03
                42,   //0x04
            ],
            1,
            RegBuilder::new().hl(0x0004).d(42).reg(),
            &[],
        ),
        (
            "(ld %e (%hl))",
            IO::init(),
            RegBuilder::new().hl(0x0004).reg(),
            &[
                psy::arch::sm83::INSTR_LD_TO_E_FROM_DEREF_HL.op_code,
                0x00, //0x01
                0x00, //0x02
                0x00, //0x03
                42,   //0x04
            ],
            1,
            RegBuilder::new().hl(0x0004).e(42).reg(),
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
            "(ld (%hl) 0x11)",
            IO::init(),
            RegBuilder::new().h(0xC0).l(0x10).reg(),
            &[
                psy::arch::sm83::INSTR_LD_TO_DEREF_HL_FROM_IMMEDIATE.op_code,
                0x11,
            ],
            2,
            RegBuilder::new().h(0xC0).l(0x10).reg(),
            &[(0xC010, 0x11)],
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
        (
            "(ld %sp 0xFF03)",
            IO::init(),
            Register::zero(),
            &[
                psy::arch::sm83::INSTR_LD_TO_SP_FROM_IMMEDIATE.op_code,
                0xFF,
                0x03,
            ],
            3,
            RegBuilder::new().sp(0xFF03).reg(),
            &[],
        ),
        (
            "(ld %a %b)",
            IO::init(),
            RegBuilder::new().a(0x01).b(0x66).reg(),
            &[psy::arch::sm83::INSTR_LD_TO_A_FROM_B.op_code],
            1,
            RegBuilder::new().a(0x66).b(0x66).reg(),
            &[],
        ),
        (
            "(ld %a %c)",
            IO::init(),
            RegBuilder::new().a(0x01).c(0x66).reg(),
            &[psy::arch::sm83::INSTR_LD_TO_A_FROM_C.op_code],
            1,
            RegBuilder::new().a(0x66).c(0x66).reg(),
            &[],
        ),
        (
            "(ld %a %h)",
            IO::init(),
            RegBuilder::new().a(0x01).h(0xFF).reg(),
            &[psy::arch::sm83::INSTR_LD_TO_A_FROM_H.op_code],
            1,
            RegBuilder::new().a(0xFF).h(0xFF).reg(),
            &[],
        ),
        (
            "(ld %b %a)",
            IO::init(),
            RegBuilder::new().a(0x01).b(0x66).reg(),
            &[psy::arch::sm83::INSTR_LD_TO_B_FROM_A.op_code],
            1,
            RegBuilder::new().a(0x01).b(0x01).reg(),
            &[],
        ),
        (
            "(ld %c %a)",
            IO::init(),
            RegBuilder::new().c(0x01).a(0x66).reg(),
            &[psy::arch::sm83::INSTR_LD_TO_C_FROM_A.op_code],
            1,
            RegBuilder::new().c(0x66).a(0x66).reg(),
            &[],
        ),
        (
            "(ld %e %a)",
            IO::init(),
            RegBuilder::new().e(0x01).a(0x66).reg(),
            &[psy::arch::sm83::INSTR_LD_TO_E_FROM_A.op_code],
            1,
            RegBuilder::new().e(0x66).a(0x66).reg(),
            &[],
        ),
    ];

    for (exp, io, reg_start, mem, pc_at, reg_after, mem_checks) in cases {
        let rom = ROM::new_copy_from_slice(mem);
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

        check_mem(mem_checks, &mc)?;
    }
    Ok(())
}

#[test]
fn test_ldh() -> Result<(), ExecErr> {
    let cases: [(
        &str,
        IO,
        Register,
        &[u8],
        u16,
        Register,
        &[(u16, u8)],
        &[(u16, u8)],
    ); 3] = [
        (
            "(ldh (0xE0) %a)",
            IO::init(),
            RegBuilder::new().a(0x42).reg(),
            &[psy::arch::sm83::INSTR_LDH_TO_IMMEDIATE_FROM_A.op_code, 0xE0],
            2,
            RegBuilder::new().a(0x42).reg(),
            &[],
            &[(0xFFE0, 0x42)],
        ),
        (
            "(ldh %a (0xE0))",
            IO::init(),
            RegBuilder::new().a(0x0).reg(),
            &[psy::arch::sm83::INSTR_LDH_TO_A_FROM_IMMEDIATE.op_code, 0xE0],
            2,
            RegBuilder::new().a(0x66).reg(),
            &[(0xFFE0, 0x66)],
            &[(0xFFE0, 0x66)],
        ),
        (
            "(ldh (%c) %a)",
            IO::init(),
            RegBuilder::new().c(0xE0).a(0x66).reg(),
            &[psy::arch::sm83::INSTR_LDH_TO_DEREF_C_FROM_A.op_code],
            1,
            RegBuilder::new().c(0xE0).a(0x66).reg(),
            &[],
            &[(0xFFE0, 0x66)],
        ),
    ];

    for (exp, io, reg_start, mem, pc_at, reg_after, mem_prep, mem_checks) in cases {
        let rom = ROM::new_copy_from_slice(mem);

        let mut mc = MemoryController::new(io, rom);
        for prep in mem_prep {
            mc.write(prep.0, prep.1)?;
        }

        let (sm83, mc) = exec_with_mc(mc, reg_start)?;

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
            let mem_value = mc.read(check.0)?;
            assert_eq!(
                mem_value, check.1,
                "expected memory location 0x{:x} to have value 0x{:x}. But was 0x{:x}",
                check.0, check.1, mem_value
            );
        }
    }

    Ok(())
}

#[test]
fn test_cp() -> Result<(), ExecErr> {
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
        let rom = ROM::new_copy_from_slice(mem);
        let (sm83, _) = exec(IO::init(), reg_start, rom)?;
        assert_eq!(sm83.pc(), mem.len() as u16);
        assert_equal_v_regs(&sm83.reg, &reg_after, exp);
    }
    Ok(())
}

#[test]
fn test_add() -> Result<(), ExecErr> {
    let cases = [
        (
            "(add %a %a), zero result",
            RegBuilder::new().a(0x00).f_z(0).f_n(1).f_h(1).f_c(1).reg(),
            &[psy::arch::sm83::INSTR_ADD_A_A.op_code],
            RegBuilder::new().a(0x00).f_z(1).f_n(0).f_h(0).f_c(0).reg(),
        ),
        (
            "(add %a %a), half-carry but not carry",
            RegBuilder::new().a(0x08).f_z(0).f_n(1).f_h(0).f_c(1).reg(),
            &[psy::arch::sm83::INSTR_ADD_A_A.op_code],
            RegBuilder::new().a(0x10).f_z(0).f_n(0).f_h(1).f_c(0).reg(),
        ),
        (
            "(add %a %a), not half-carry but carry with zero result overflow",
            RegBuilder::new().a(0x80).f_z(0).f_n(1).f_h(1).f_c(0).reg(),
            &[psy::arch::sm83::INSTR_ADD_A_A.op_code],
            RegBuilder::new().a(0x00).f_z(1).f_n(0).f_h(0).f_c(1).reg(),
        ),
        (
            "(add %a %a), not half-carry but carry with non-zero overflow",
            RegBuilder::new().a(0x90).f_z(1).f_n(1).f_h(1).f_c(0).reg(),
            &[psy::arch::sm83::INSTR_ADD_A_A.op_code],
            RegBuilder::new().a(0x20).f_z(0).f_n(0).f_h(0).f_c(1).reg(),
        ),
        (
            "(add %a %a), half-carry and carry",
            RegBuilder::new().a(0x88).f_z(1).f_n(1).f_h(0).f_c(0).reg(),
            &[psy::arch::sm83::INSTR_ADD_A_A.op_code],
            RegBuilder::new().a(0x10).f_z(0).f_n(0).f_h(1).f_c(1).reg(),
        ),
        // add %hl %de
        (
            "(add %hl %de), zero result",
            RegBuilder::new()
                .hl(0x00)
                .de(0x00)
                .f_z(0)
                .f_n(1)
                .f_h(1)
                .f_c(1)
                .reg(),
            &[psy::arch::sm83::INSTR_ADD_HL_DE.op_code],
            RegBuilder::new()
                .hl(0x00)
                .de(0x00)
                .f_z(0) // zero flag not set by add %hl!
                .f_n(0)
                .f_h(0)
                .f_c(0)
                .reg(),
        ),
        (
            "(add %hl %de), half-carry but not carry",
            RegBuilder::new()
                .hl(0x0080)
                .de(0x0080)
                .f_z(0)
                .f_n(1)
                .f_h(0)
                .f_c(1)
                .reg(),
            &[psy::arch::sm83::INSTR_ADD_HL_DE.op_code],
            RegBuilder::new()
                .hl(0x0100)
                .de(0x0080)
                .f_z(0)
                .f_n(0)
                .f_h(1)
                .f_c(0)
                .reg(),
        ),
        (
            "(add %hl %de), not half-carry but carry with zero result overflow",
            RegBuilder::new()
                .hl(0x8000)
                .de(0x8000)
                .f_z(1)
                .f_n(1)
                .f_h(1)
                .f_c(0)
                .reg(),
            &[psy::arch::sm83::INSTR_ADD_HL_DE.op_code],
            RegBuilder::new()
                .hl(0x00)
                .de(0x8000)
                .f_z(1)
                .f_n(0)
                .f_h(0)
                .f_c(1)
                .reg(),
        ),
        (
            "(add %hl %de), not half-carry but carry with non-zero overflow",
            RegBuilder::new()
                .hl(0x9000)
                .de(0x9000)
                .f_z(1)
                .f_n(1)
                .f_h(1)
                .f_c(0)
                .reg(),
            &[psy::arch::sm83::INSTR_ADD_HL_DE.op_code],
            RegBuilder::new()
                .hl(0x2000)
                .de(0x9000)
                .f_z(1)
                .f_n(0)
                .f_h(0)
                .f_c(1)
                .reg(),
        ),
        (
            "(add %hl %de), half-carry and carry",
            RegBuilder::new()
                .hl(0x8080)
                .de(0x8080)
                .f_z(1)
                .f_n(1)
                .f_h(0)
                .f_c(0)
                .reg(),
            &[psy::arch::sm83::INSTR_ADD_HL_DE.op_code],
            RegBuilder::new()
                .hl(0x0100)
                .de(0x8080)
                .f_z(1)
                .f_n(0)
                .f_h(1)
                .f_c(1)
                .reg(),
        ),
    ];

    for (exp, reg_init, mem, reg_after) in cases {
        let rom = ROM::new_copy_from_slice(mem);
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

#[test]
fn test_inc() -> Result<(), ExecErr> {
    let cases = [
        (
            "(inc %c), zero result",
            RegBuilder::new().c(0xFF).f_z(0).f_n(1).f_h(1).reg(),
            &[psy::arch::sm83::INSTR_INC_C.op_code],
            RegBuilder::new().c(0x00).f_z(1).f_n(0).f_h(1).reg(),
        ),
        (
            "(inc %c), non-zero result",
            RegBuilder::new().c(0x00).f_z(1).f_n(1).f_h(1).reg(),
            &[psy::arch::sm83::INSTR_INC_C.op_code],
            RegBuilder::new().c(0x01).f_z(0).f_n(0).f_h(0).reg(),
        ),
        (
            "(inc %c), half-carry",
            RegBuilder::new().c(0x0F).f_z(1).f_n(1).f_h(0).reg(),
            &[psy::arch::sm83::INSTR_INC_C.op_code],
            RegBuilder::new().c(0x10).f_z(0).f_n(0).f_h(1).reg(),
        ),
        //(inc %l)
        (
            "(inc %l), zero result",
            RegBuilder::new().l(0xFF).f_z(0).f_n(1).f_h(1).reg(),
            &[psy::arch::sm83::INSTR_INC_L.op_code],
            RegBuilder::new().l(0x00).f_z(1).f_n(0).f_h(1).reg(),
        ),
        (
            "(inc %l), non-zero result",
            RegBuilder::new().l(0x00).f_z(1).f_n(1).f_h(1).reg(),
            &[psy::arch::sm83::INSTR_INC_L.op_code],
            RegBuilder::new().l(0x01).f_z(0).f_n(0).f_h(0).reg(),
        ),
        (
            "(inc %l), half-carry",
            RegBuilder::new().l(0x0F).f_z(1).f_n(1).f_h(0).reg(),
            &[psy::arch::sm83::INSTR_INC_L.op_code],
            RegBuilder::new().l(0x10).f_z(0).f_n(0).f_h(1).reg(),
        ),
        //(inc %de)
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
        (
            "(inc %de) with overflow",
            RegBuilder::new().de(0xFFFF).reg(),
            &[psy::arch::sm83::INSTR_INC_DE.op_code],
            RegBuilder::new().de(0x0).reg(),
        ),
        //(inc %hl)
        (
            "(inc %hl) with zero %hl",
            RegBuilder::new().hl(0x00).reg(),
            &[psy::arch::sm83::INSTR_INC_HL.op_code],
            RegBuilder::new().hl(0x01).reg(),
        ),
        (
            "(inc %hl) with non-zero %hl",
            RegBuilder::new().hl(0x666).reg(),
            &[psy::arch::sm83::INSTR_INC_HL.op_code],
            RegBuilder::new().hl(0x667).reg(),
        ),
        (
            "(inc %hl) with overflow",
            RegBuilder::new().hl(0xFFFF).reg(),
            &[psy::arch::sm83::INSTR_INC_HL.op_code],
            RegBuilder::new().hl(0x0).reg(),
        ),
    ];

    for (exp, reg_init, mem, reg_after) in cases {
        let rom = ROM::new_copy_from_slice(mem);
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

#[test]
fn test_dec() -> Result<(), ExecErr> {
    let cases = [
        (
            "(dec %b) with 1 %b",
            RegBuilder::new().b(0x01).f_z(0).f_h(0).reg(),
            &[psy::arch::sm83::INSTR_DEC_B.op_code],
            RegBuilder::new().b(0x00).f_z(1).f_n(1).f_h(0).reg(),
        ),
        (
            "(dec %b) with 0 %b",
            RegBuilder::new().b(0x0).f_z(1).f_h(0).reg(),
            &[psy::arch::sm83::INSTR_DEC_B.op_code],
            RegBuilder::new().b(0xFF).f_z(0).f_n(1).f_h(1).reg(),
        ),
        (
            "(dec %b) no half carry",
            RegBuilder::new().b(0x15).f_z(1).f_h(1).reg(),
            &[psy::arch::sm83::INSTR_DEC_B.op_code],
            RegBuilder::new().b(0x14).f_z(0).f_n(1).f_h(0).reg(),
        ),
        (
            "(dec %b) half carry",
            RegBuilder::new().b(0x10).f_z(1).f_h(0).reg(),
            &[psy::arch::sm83::INSTR_DEC_B.op_code],
            RegBuilder::new().b(0x0F).f_z(0).f_n(1).f_h(1).reg(),
        ),
        (
            "(dec %c) with 1 %c",
            RegBuilder::new().c(0x01).f_z(0).f_h(0).reg(),
            &[psy::arch::sm83::INSTR_DEC_C.op_code],
            RegBuilder::new().c(0x00).f_z(1).f_n(1).f_h(0).reg(),
        ),
        (
            "(dec %c) with 0 %c",
            RegBuilder::new().c(0x0).f_z(1).f_h(0).reg(),
            &[psy::arch::sm83::INSTR_DEC_C.op_code],
            RegBuilder::new().c(0xFF).f_z(0).f_n(1).f_h(1).reg(),
        ),
        (
            "(dec %c) no half carry",
            RegBuilder::new().c(0x15).f_z(1).f_h(1).reg(),
            &[psy::arch::sm83::INSTR_DEC_C.op_code],
            RegBuilder::new().c(0x14).f_z(0).f_n(1).f_h(0).reg(),
        ),
        (
            "(dec %c) half carry",
            RegBuilder::new().c(0x10).f_z(1).f_h(0).reg(),
            &[psy::arch::sm83::INSTR_DEC_C.op_code],
            RegBuilder::new().c(0x0F).f_z(0).f_n(1).f_h(1).reg(),
        ),
        (
            "(dec %bc) with 1 %bc",
            RegBuilder::new().bc(0x01).reg(),
            &[psy::arch::sm83::INSTR_DEC_BC.op_code],
            RegBuilder::new().bc(0x00).reg(),
        ),
        (
            "(dec %bc) with 0 %bc",
            RegBuilder::new().bc(0x0).reg(),
            &[psy::arch::sm83::INSTR_DEC_BC.op_code],
            RegBuilder::new().bc(0xFFFF).reg(),
        ),
    ];

    for (exp, reg_init, mem, reg_after) in cases {
        let rom = ROM::new_copy_from_slice(mem);
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

#[test]
fn test_or() -> Result<(), ExecErr> {
    let cases = [
        (
            "(or %a %b) non-zero result",
            RegBuilder::new()
                .a(0x01)
                .b(0x10)
                .f_z(1)
                .f_n(1)
                .f_h(1)
                .f_c(1)
                .reg(),
            &[psy::arch::sm83::INSTR_OR_A_B.op_code],
            RegBuilder::new()
                .a(0x11)
                .b(0x10)
                .f_z(0)
                .f_n(0)
                .f_h(0)
                .f_c(0)
                .reg(),
        ),
        (
            "(or %a %b) zero result",
            RegBuilder::new()
                .a(0x00)
                .c(0x00)
                .f_z(0)
                .f_n(1)
                .f_h(1)
                .f_c(1)
                .reg(),
            &[psy::arch::sm83::INSTR_OR_A_B.op_code],
            RegBuilder::new()
                .a(0x00)
                .c(0x00)
                .f_z(1)
                .f_n(0)
                .f_h(0)
                .f_c(0)
                .reg(),
        ),
        (
            "(or %a %c) non-zero result",
            RegBuilder::new()
                .a(0x01)
                .c(0x10)
                .f_z(1)
                .f_n(1)
                .f_h(1)
                .f_c(1)
                .reg(),
            &[psy::arch::sm83::INSTR_OR_A_C.op_code],
            RegBuilder::new()
                .a(0x11)
                .c(0x10)
                .f_z(0)
                .f_n(0)
                .f_h(0)
                .f_c(0)
                .reg(),
        ),
        (
            "(or %a %c) zero result",
            RegBuilder::new()
                .a(0x00)
                .c(0x00)
                .f_z(0)
                .f_n(1)
                .f_h(1)
                .f_c(1)
                .reg(),
            &[psy::arch::sm83::INSTR_OR_A_C.op_code],
            RegBuilder::new()
                .a(0x00)
                .c(0x00)
                .f_z(1)
                .f_n(0)
                .f_h(0)
                .f_c(0)
                .reg(),
        ),
    ];

    for (exp, reg_init, mem, reg_after) in cases {
        let rom = ROM::new_copy_from_slice(mem);
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

#[test]
fn test_and() -> Result<(), ExecErr> {
    let cases: [(&str, Register, &[u8], Register, u16); 6] = [
        (
            "(and %a 0x43) zero result",
            RegBuilder::new().a(0x00).f_z(0).f_n(1).f_h(0).f_c(1).reg(),
            &[psy::arch::sm83::INSTR_AND_A_IMMEDIATE.op_code, 0x43],
            RegBuilder::new().a(0x00).f_z(1).f_n(0).f_h(1).f_c(0).reg(),
            2,
        ),
        (
            "(and %a 0b00001010) non-zero result",
            RegBuilder::new()
                .a(0b11111000)
                .f_z(1)
                .f_n(1)
                .f_h(0)
                .f_c(1)
                .reg(),
            &[psy::arch::sm83::INSTR_AND_A_IMMEDIATE.op_code, 0b00001010],
            RegBuilder::new()
                .a(0b00001000)
                .f_z(0)
                .f_n(0)
                .f_h(1)
                .f_c(0)
                .reg(),
            2,
        ),
        (
            "(and %a %c) zero result",
            RegBuilder::new()
                .a(0b11110000)
                .c(0b00001111)
                .f_z(0)
                .f_n(1)
                .f_h(0)
                .f_c(1)
                .reg(),
            &[psy::arch::sm83::INSTR_AND_A_C.op_code],
            RegBuilder::new()
                .a(0x00)
                .c(0b00001111)
                .f_z(1)
                .f_n(0)
                .f_h(1)
                .f_c(0)
                .reg(),
            1,
        ),
        (
            "(and %a %c) non-zero result",
            RegBuilder::new()
                .a(0b11111000)
                .c(0b00001010)
                .f_z(1)
                .f_n(1)
                .f_h(0)
                .f_c(1)
                .reg(),
            &[psy::arch::sm83::INSTR_AND_A_C.op_code],
            RegBuilder::new()
                .a(0b00001000)
                .c(0b00001010)
                .f_z(0)
                .f_n(0)
                .f_h(1)
                .f_c(0)
                .reg(),
            1,
        ),
        (
            "(and %a %a) zero result",
            RegBuilder::new().a(0x00).f_z(0).f_n(1).f_h(0).f_c(1).reg(),
            &[psy::arch::sm83::INSTR_AND_A_A.op_code],
            RegBuilder::new().a(0x00).f_z(1).f_n(0).f_h(1).f_c(0).reg(),
            1,
        ),
        (
            "(and %a %a) non-zero result",
            RegBuilder::new().a(0x66).f_z(1).f_n(1).f_h(0).f_c(1).reg(),
            &[psy::arch::sm83::INSTR_AND_A_A.op_code],
            RegBuilder::new().a(0x66).f_z(0).f_n(0).f_h(1).f_c(0).reg(),
            1,
        ),
    ];

    for (exp, reg_init, mem, reg_after, pc) in cases {
        let rom = ROM::new_copy_from_slice(mem);
        let (sm83, _) = exec(IO::init(), reg_init, rom)?;
        assert_eq!(
            sm83.pc(),
            pc,
            "{}, want pc 0x{:x}, got 0x{:x}",
            exp,
            pc,
            sm83.pc()
        );
        assert_equal_v_regs(&sm83.reg, &reg_after, exp);
    }
    Ok(())
}

#[test]
fn test_xor() -> Result<(), ExecErr> {
    let cases = [
        (
            "(xor %a %a) - with zero reg",
            RegBuilder::new().a(0x00).f_z(1).f_n(1).f_h(1).f_c(1).reg(), // flags are all reset
            &[psy::arch::sm83::INSTR_XOR_A_A.op_code],
            RegBuilder::new().a(0x00).f_z(0).f_n(0).f_h(0).f_c(0).reg(),
        ),
        (
            "(xor %a %a) - with non-zero reg",
            RegBuilder::new().a(0x10).f_z(1).f_n(1).f_h(1).f_c(1).reg(),
            &[psy::arch::sm83::INSTR_XOR_A_A.op_code],
            RegBuilder::new().a(0x00).f_z(0).f_n(0).f_h(0).f_c(0).reg(),
        ),
        (
            "(xor %a %c) - with zero result",
            RegBuilder::new()
                .a(0x11)
                .c(0x11)
                .f_z(0)
                .f_n(1)
                .f_h(1)
                .f_c(1)
                .reg(), // flags are all reset
            &[psy::arch::sm83::INSTR_XOR_A_C.op_code],
            RegBuilder::new()
                .a(0x00)
                .c(0x11)
                .f_z(1)
                .f_n(0)
                .f_h(0)
                .f_c(0)
                .reg(),
        ),
        (
            "(xor %a %c) - with non-zero result",
            RegBuilder::new()
                .a(0x11)
                .c(0x10)
                .f_z(1)
                .f_n(1)
                .f_h(1)
                .f_c(1)
                .reg(),
            &[psy::arch::sm83::INSTR_XOR_A_C.op_code],
            RegBuilder::new()
                .a(0x01)
                .c(0x10)
                .f_z(0)
                .f_n(0)
                .f_h(0)
                .f_c(0)
                .reg(),
        ),
    ];

    for (exp, reg_init, mem, reg_after) in cases {
        let rom = ROM::new_copy_from_slice(mem);
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

#[test]
fn test_cpl() -> Result<(), ExecErr> {
    let cases = [
        (
            "(cpl) zero",
            RegBuilder::new().a(0x00).f_n(0).f_h(1).reg(),
            &[psy::arch::sm83::INSTR_CPL.op_code],
            RegBuilder::new().a(0xFF).f_n(1).f_h(1).reg(),
        ),
        (
            "(cpl) FF",
            RegBuilder::new().a(0xFF).f_n(0).f_h(1).reg(),
            &[psy::arch::sm83::INSTR_CPL.op_code],
            RegBuilder::new().a(0x00).f_n(1).f_h(1).reg(),
        ),
        (
            "(cpl) mixed",
            RegBuilder::new().a(0b01010101).f_n(0).f_h(1).reg(),
            &[psy::arch::sm83::INSTR_CPL.op_code],
            RegBuilder::new().a(0b10101010).f_n(1).f_h(1).reg(),
        ),
    ];

    for (exp, reg_init, mem, reg_after) in cases {
        let rom = ROM::new_copy_from_slice(mem);
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

#[test]
fn test_rotate_circular() -> Result<(), ExecErr> {
    let cases = [
        (
            "(RRCA) carry set",
            RegBuilder::new()
                .a(0b01010101)
                .f_z(1)
                .f_n(1)
                .f_h(1)
                .f_c(0)
                .reg(),
            &[psy::arch::sm83::INSTR_RRCA.op_code],
            RegBuilder::new()
                .a(0b10101010)
                .f_z(0)
                .f_n(0)
                .f_h(0)
                .f_c(1)
                .reg(),
        ),
        (
            "(RRCA) carry not set",
            RegBuilder::new()
                .a(0b10101010)
                .f_z(1)
                .f_n(1)
                .f_h(1)
                .f_c(1)
                .reg(),
            &[psy::arch::sm83::INSTR_RRCA.op_code],
            RegBuilder::new()
                .a(0b01010101)
                .f_z(0)
                .f_n(0)
                .f_h(0)
                .f_c(0)
                .reg(),
        ),
    ];

    for (exp, reg_init, mem, reg_after) in cases {
        let rom = ROM::new_copy_from_slice(mem);
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

#[test]
fn test_call() -> Result<(), ExecErr> {
    let mut mem = [psy::arch::sm83::INSTR_INVALID.op_code; 0x166 + 4];
    mem[0x167] = psy::arch::sm83::INSTR_CALL.op_code;
    mem[0x168] = 0x50;
    mem[0x169] = 0x01;
    let cases = [("(call 0x150)", mem, 0x167, 0x150)];

    for (exp, mem, pc_start, pc_after) in cases {
        let rom = ROM::new_copy_from_slice(&mem);
        let io = IO::init();
        let regs = RegBuilder::new().pc(pc_start).sp(0xFFFE).reg();
        let (sm83, mc) = exec(io, regs, rom)?;

        assert_eq!(
            sm83.pc(),
            pc_after,
            "{}, want pc 0x{:x}, got 0x{:x}",
            exp,
            pc_after,
            sm83.pc()
        );
        assert_eq!(sm83.reg.sp, 0xFFFC);
        assert_eq!(
            mc.read(0xFFFC)?,
            0x6A,
            "got 0x{:x}, want 0x6A",
            mc.read(0xFFFC)?
        );
        assert_eq!(
            mc.read(0xFFFD)?,
            0x01,
            "got 0x{:x}, want 0x01",
            mc.read(0xFFFD)?
        );
    }
    Ok(())
}

#[test]
fn test_ret() -> Result<(), ExecErr> {
    let cases = [(
        "(ret)",
        [psy::arch::sm83::INSTR_RET.op_code],
        0xFFFC,
        0xFFFE,
        0,
        0x168,
        0x68,
        0x01,
    )];

    for (exp, mem, sp_start, sp_after, pc_start, pc_after, sp_low, sp_high) in cases {
        let rom = ROM::new_copy_from_slice(&mem);
        let mut mc = MemoryController::new(IO::init(), rom);
        mc.write(sp_start, sp_low)?;
        mc.write(sp_start + 1, sp_high)?;
        let (sm83, _) = exec_with_mc(mc, RegBuilder::new().pc(pc_start).sp(sp_start).reg())?;
        assert_eq!(
            sm83.reg.sp, sp_after,
            "{}, want sp 0x{:x}, got 0x{:x}",
            exp, sp_after, sm83.reg.sp
        );
        assert_eq!(
            sm83.pc(),
            pc_after,
            "{}, want pc 0x{:x}, got 0x{:x}",
            exp,
            pc_after,
            sm83.pc()
        );
    }
    Ok(())
}

#[test]
fn test_pop() -> Result<(), ExecErr> {
    let cases = [
        (
            "(pop %af)",
            [psy::arch::sm83::INSTR_POP_AF.op_code],
            0xFFFC,
            0xFFFE,
            0,
            1,
            0xFF,
            0xAA,
            RegBuilder::new().af(0xAAFF).reg(),
        ),
        (
            "(pop %bc)",
            [psy::arch::sm83::INSTR_POP_BC.op_code],
            0xFFFC,
            0xFFFE,
            0,
            1,
            0x23,
            0x76,
            RegBuilder::new().bc(0x7623).reg(),
        ),
        (
            "(pop %de)",
            [psy::arch::sm83::INSTR_POP_DE.op_code],
            0xFFFC,
            0xFFFE,
            0,
            1,
            0x68,
            0x01,
            RegBuilder::new().de(0x168).reg(),
        ),
        (
            "(pop %hl)",
            [psy::arch::sm83::INSTR_POP_HL.op_code],
            0xFFFC,
            0xFFFE,
            0,
            1,
            0x68,
            0x01,
            RegBuilder::new().hl(0x168).reg(),
        ),
    ];

    for (exp, mem, sp_start, sp_after, pc_start, pc_after, sp_low, sp_high, reg_after) in cases {
        let rom = ROM::new_copy_from_slice(&mem);
        let mut mc = MemoryController::new(IO::init(), rom);
        mc.write(sp_start, sp_low)?;
        mc.write(sp_start + 1, sp_high)?;
        let (sm83, _) = exec_with_mc(mc, RegBuilder::new().pc(pc_start).sp(sp_start).reg())?;
        assert_eq!(
            sm83.reg.sp, sp_after,
            "{}, want sp 0x{:x}, got 0x{:x}",
            exp, sp_after, sm83.reg.sp
        );
        assert_eq!(
            sm83.pc(),
            pc_after,
            "{}, want pc 0x{:x}, got 0x{:x}",
            exp,
            pc_after,
            sm83.pc()
        );
        assert_equal_v_regs(&sm83.reg, &reg_after, exp);
    }
    Ok(())
}

#[test]
fn test_push() -> Result<(), ExecErr> {
    let cases = [
        (
            "(push %af)",
            [psy::arch::sm83::INSTR_PUSH_AF.op_code],
            RegBuilder::new().af(0x43FF).pc(0).sp(0xFFFE).reg(),
            0xFFFC,
            1,
            0xFF,
            0x43,
            RegBuilder::new().af(0x43FF).reg(),
        ),
        (
            "(push %bc)",
            [psy::arch::sm83::INSTR_PUSH_BC.op_code],
            RegBuilder::new().bc(0x1188).pc(0).sp(0xFFFE).reg(),
            0xFFFC,
            1,
            0x88,
            0x11,
            RegBuilder::new().bc(0x1188).reg(),
        ),
        (
            "(push %de)",
            [psy::arch::sm83::INSTR_PUSH_DE.op_code],
            RegBuilder::new().de(0x168).pc(0).sp(0xFFFE).reg(),
            0xFFFC,
            1,
            0x68,
            0x01,
            RegBuilder::new().de(0x168).reg(),
        ),
        (
            "(push %hl)",
            [psy::arch::sm83::INSTR_PUSH_HL.op_code],
            RegBuilder::new().hl(0x168).pc(0).sp(0xFFFE).reg(),
            0xFFFC,
            1,
            0x68,
            0x01,
            RegBuilder::new().hl(0x168).reg(),
        ),
    ];

    for (exp, mem, reg_before, sp_after, pc_after, sp_low, sp_high, reg_after) in cases {
        let rom = ROM::new_copy_from_slice(&mem);
        let mc = MemoryController::new(IO::init(), rom);
        let (sm83, mc) = exec_with_mc(mc, reg_before)?;

        assert_eq!(mc.read(sp_after)?, sp_low);
        assert_eq!(mc.read(sp_after + 1)?, sp_high);

        assert_eq!(
            sm83.reg.sp, sp_after,
            "{}, want sp 0x{:x}, got 0x{:x}",
            exp, sp_after, sm83.reg.sp
        );
        assert_eq!(
            sm83.pc(),
            pc_after,
            "{}, want pc 0x{:x}, got 0x{:x}",
            exp,
            pc_after,
            sm83.pc()
        );
        assert_equal_v_regs(&sm83.reg, &reg_after, exp);
    }
    Ok(())
}

#[test]
fn test_rst() -> Result<(), ExecErr> {
    let mut mem = [psy::arch::sm83::INSTR_INVALID.op_code; 0x150 + 1];
    let cases = [(
        "(rst 0x28)",
        psy::arch::sm83::INSTR_RST_28.op_code,
        0x150,
        0x28,
    )];

    for (exp, rst_op, pc_start, pc_after) in cases {
        mem[0x150] = rst_op;
        let rom = ROM::new_copy_from_slice(&mem);
        let io = IO::init();
        let regs = RegBuilder::new().pc(pc_start).sp(0xFFFE).reg();
        let (sm83, mc) = exec(io, regs, rom)?;

        assert_eq!(
            sm83.pc(),
            pc_after,
            "{}, want pc 0x{:x}, got 0x{:x}",
            exp,
            pc_after,
            sm83.pc()
        );
        assert_eq!(sm83.reg.sp, 0xFFFC);
        assert_eq!(
            mc.read(0xFFFC)?,
            0x51,
            "got 0x{:x}, want 0x51",
            mc.read(0xFFFC)?
        );
        assert_eq!(
            mc.read(0xFFFD)?,
            0x01,
            "got 0x{:x}, want 0x01",
            mc.read(0xFFFD)?
        );
    }
    Ok(())
}

#[test]
fn test_prefix_swap() -> Result<(), ExecErr> {
    let cases = [
        (
            "(swap %a) zero",
            RegBuilder::new().a(0x00).f_z(0).f_n(1).f_h(1).f_c(1).reg(),
            &[
                psy::arch::sm83::INSTR_PREFIX.op_code,
                psy::arch::sm83::INSTR_PREFIX_SWAP_A.op_code,
            ],
            RegBuilder::new().a(0x00).f_z(1).f_n(0).f_h(0).f_c(0).reg(),
        ),
        (
            "(swap %a) non-zero",
            RegBuilder::new().a(0xFA).f_z(1).f_n(1).f_h(1).f_c(1).reg(),
            &[
                psy::arch::sm83::INSTR_PREFIX.op_code,
                psy::arch::sm83::INSTR_PREFIX_SWAP_A.op_code,
            ],
            RegBuilder::new().a(0xAF).f_z(0).f_n(0).f_h(0).f_c(0).reg(),
        ),
    ];

    for (exp, reg_init, mem, reg_after) in cases {
        let rom = ROM::new_copy_from_slice(mem);
        let (sm83, _) = exec(IO::init(), reg_init, rom)?;
        assert_eq!(
            sm83.pc(),
            2,
            "{}, want pc 0x{:x}, got 0x{:x}",
            exp,
            2,
            sm83.pc()
        );
        assert_equal_v_regs(&sm83.reg, &reg_after, exp);
    }
    Ok(())
}

#[test]
fn test_prefix_reset() -> Result<(), ExecErr> {
    let cases: [(&str, IO, Register, &[u8], Register, &[(u16, u8)]); 2] = [
        (
            "(res 0 %a)",
            IO::init(),
            RegBuilder::new().a(0b11111111).reg(),
            &[
                psy::arch::sm83::INSTR_PREFIX.op_code,
                psy::arch::sm83::INSTR_PREFIX_RST_0_A.op_code,
            ],
            RegBuilder::new().a(0b11111110).reg(),
            &[],
        ),
        (
            "(res 7 (%hl))",
            IO::init_with_value(0xFF44, 0b11111111)?,
            RegBuilder::new().hl(0xFF44).reg(),
            &[
                psy::arch::sm83::INSTR_PREFIX.op_code,
                psy::arch::sm83::INSTR_PREFIX_RST_7_DEREF_HL.op_code,
            ],
            RegBuilder::new().hl(0xFF44).reg(),
            &[(0xFF44, 0b01111111)],
        ),
    ];

    for (exp, io, reg_init, mem, reg_after, mem_checks) in cases {
        let rom = ROM::new_copy_from_slice(mem);
        let (sm83, mc) = exec(io, reg_init, rom)?;
        assert_eq!(
            sm83.pc(),
            2,
            "{}, want pc 0x{:x}, got 0x{:x}",
            exp,
            2,
            sm83.pc()
        );
        assert_equal_v_regs(&sm83.reg, &reg_after, exp);

        check_mem(mem_checks, &mc)?
    }
    Ok(())
}

// helper

fn check_mem(mem_checks: &[(u16, u8)], mc: &MemoryController) -> Result<(), ExecErr> {
    for check in mem_checks {
        let mem_value = mc.read(check.0)?;
        assert_eq!(
            mem_value, check.1,
            "expected memory location 0x{:x} to have value 0x{:x}. But was 0x{:x}",
            check.0, check.1, mem_value
        );
    }
    Ok(())
}

/// conly compares the value register a to l, without pc and sp.
fn assert_equal_v_regs(l: &Register, r: &Register, exp: &str) {
    assert_eq!(
        l.a, r.a,
        "reg a: left 0x{:x} right 0x{:x}, exp: {}",
        l.a, r.a, exp
    );
    assert_eq!(l.b, r.b, "reg b: {}", exp);
    assert_eq!(l.c, r.c, "reg c: {}", exp);
    assert_eq!(l.d, r.d, "reg d: {}", exp);
    assert_eq!(l.e, r.e, "reg e: {}", exp);
    assert_eq!(l.h, r.h, "reg h: {}", exp);
    assert_eq!(l.l, r.l, "reg l: {}", exp);
    // flag check
    assert_eq!(l.get_flag(Z), r.get_flag(Z), "flag z: {}", exp);
    assert_eq!(l.get_flag(N), r.get_flag(N), "flag n: {}", exp);
    assert_eq!(l.get_flag(H), r.get_flag(H), "flag h: {}", exp);
    assert_eq!(l.get_flag(C), r.get_flag(C), "flag c: {}", exp);
}

fn exec(io: IO, reg: Register, rom: ROM) -> Result<(SM83, MemoryController), ExecErr> {
    exec_with_mc(MemoryController::new(io, rom), reg)
}

fn exec_with_mc(
    mut mc: MemoryController,
    reg: Register,
) -> Result<(SM83, MemoryController), ExecErr> {
    let mut sm83 = SM83::init();
    sm83.reg = reg;
    sm83.execute(&mut mc)?;
    Ok((sm83, mc))
}
