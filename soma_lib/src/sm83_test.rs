use crate::ROM;
use crate::sm83::SM83;

#[test]
fn test_jp() {
    let cases = [(
        "(jp 0x150)",
        [psy::arch::sm83::INSTR_JP.op_code, 0xAA, 0xFF],
        0xFFAA,
    )];

    for (exp, mem, pc) in cases {
        let sm83 = exec_mem(&mem);
        assert_eq!(
            sm83.pc(),
            pc,
            "{}, want pc 0x{:x}, got 0x{:x}",
            exp,
            pc,
            sm83.pc()
        );
    }
}

// helper

fn exec_mem(mem: &[u8]) -> SM83 {
    let mut sm83 = SM83::init();
    sm83.execute(&ROM::new(mem));
    sm83
}
