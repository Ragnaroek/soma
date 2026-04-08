use crate::ROM;
use crate::sm83::SM83;

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

// helper

fn exec_mem(mem: &[u8]) -> Result<SM83, &'static str> {
    let mut sm83 = SM83::init();
    sm83.execute(&ROM::new(mem))?;
    Ok(sm83)
}
