use crate::gdb::{
    Command, GDBFeatures, MemoryRange, Packet, parse_memory_range, parse_next_command,
};

#[test]
fn test_parse_command() -> Result<(), String> {
    let cases = [
        ("+", Some(Command::Plus), ""),
        ("+$#00", Some(Command::Plus), "$#00"),
        (
            "$qSupported:multiprocess+;swbreak+;hwbreak+;qRelocInsn+;fork-events+;vfork-events+;exec-events+;vContSupported+;QThreadEvents+;QThreadOptions+;no-resumed+;memory-tagging+;xmlRegisters=i386;error-message+#14",
            Some(Command::QSupported(GDBFeatures {})),
            "",
        ),
    ];

    for (input, cmd, remaining_input) in cases {
        let (cmd_parsed, remaining_input_parsed) = parse_next_command(input)?;
        assert_eq!(cmd_parsed, cmd);
        assert_eq!(remaining_input_parsed, remaining_input);
    }
    Ok(())
}

#[test]
fn test_parse_memory_range() -> Result<(), String> {
    let cases = [
        ("00,00", MemoryRange { addr: 0, length: 0 }),
        (
            "ffc0,1a",
            MemoryRange {
                addr: 0xFFC0,
                length: 0x1A,
            },
        ),
    ];

    for (input, range) in cases {
        let range_parsed = parse_memory_range(input)?;
        assert_eq!(range_parsed, range);
    }
    Ok(())
}

#[test]
fn test_packet_string() -> Result<(), String> {
    let cases = [
        (Packet::ack("S00"), "+$S00#b3"),
        (Packet::ack("QC1"), "+$QC1#c5"),
        (
            Packet::nack("qSupported:swbreak+;vContSupported+;QThreadEvents+"),
            "$qSupported:swbreak+;vContSupported+;QThreadEvents+#45",
        ),
        (Packet::ack("vcont;c;s;t"), "+$vcont;c;s;t#25"),
        (Packet::ack(""), "+$#00"),
    ];

    for (packet, packet_str_want) in cases {
        let str_got = packet.packet_string();
        assert_eq!(packet_str_want, str_got);
    }
    Ok(())
}
