//TODO Test
//$qSupported:multiprocess+;swbreak+;hwbreak+;qRelocInsn+;fork-events+;vfork-events+;exec-events+;vContSupported+;QThreadEvents+;QThreadOptions+;no-resumed+;memory-tagging+;xmlRegisters=i386;error-message+#14

use crate::gdb::{Command, GDBFeatures, parse_command};

#[test]
fn test_parse_command() -> Result<(), String> {
    let cases = [
        ("+", Some(Command::Plus)),
        (
            "$qSupported:multiprocess+;swbreak+;hwbreak+;qRelocInsn+;fork-events+;vfork-events+;exec-events+;vContSupported+;QThreadEvents+;QThreadOptions+;no-resumed+;memory-tagging+;xmlRegisters=i386;error-message+#14",
            Some(Command::QSupported(GDBFeatures {})),
        ),
    ];

    for (input, cmd) in cases {
        let cmd_parsed = parse_command(input)?;
        assert_eq!(cmd_parsed, cmd);
    }
    Ok(())
}
