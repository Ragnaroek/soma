#[cfg(test)]
#[path = "./gdb_test.rs"]
mod gdb_test;

use std::fmt::Write as FmtWrite;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Arc;
use std::sync::RwLock;

use crate::Emulation;

#[derive(Debug, PartialEq)]
enum Command {
    Unknown(String),
    Plus,
    /// ?
    StopQuery,
    /// Get register values
    G,
    /// Read register x
    P,
    /// Read memory
    M(MemoryRange),
    QSupported(GDBFeatures),
    /// query current thread
    QC,
    QAttached,
    QTStatus,
    QFThreadInfo,
    VContQ,
    VMustReplyEmpty,
    /// Set thread for sub-sequent operation
    H(HParams),
}

#[derive(Debug, PartialEq)]
struct GDBFeatures {}

#[derive(Debug, PartialEq)]
struct HParams {}

#[derive(Debug, PartialEq)]
struct MemoryRange {
    addr: usize,
    length: usize,
}

struct Packet {
    content: String,
    /// This message needs to prepend the ack first
    prepend_ack: bool,
}

impl Packet {
    /// A regular message that prepends a ack (+) in the reply.
    fn ack(content: &str) -> Self {
        Self {
            content: content.to_string(),
            prepend_ack: true,
        }
    }

    fn nack(content: &str) -> Self {
        Self {
            content: content.to_string(),
            prepend_ack: false,
        }
    }

    /// Constructs the packet string in the format that it can be send
    /// back to gdb: (+)$...#xx
    fn packet_string(&self) -> String {
        let ack = if self.prepend_ack { "+" } else { "" };

        let mut checksum: u8 = 0;
        for b in self.content.bytes() {
            checksum = checksum.wrapping_add(b);
        }

        format!("{}${}#{:02x}", ack, self.content, checksum)
    }
}

enum Reply {
    Nothing,
    Packet(Packet),
    /// Special reply at the beginning of the protocol
    Plus,
}

pub fn gdb_serve(emulation_lock: Arc<RwLock<Emulation>>) -> Result<(), String> {
    let listener = TcpListener::bind("127.0.0.1:1234").expect("tcp bind");
    println!("Server listening on 127.0.0.1:1234");
    let (mut stream, _) = listener.accept().expect("tcp accept");
    println!("connection accepted!");

    let mut buf = [0; 4096];
    let mut pending_command = false;
    loop {
        match stream.read(&mut buf) {
            Ok(0) => {
                // connection closed
                break;
            }
            Ok(n) => {
                let cmd = String::from_utf8_lossy(&buf[..n]);
                if pending_command {
                    print!("-> {} ", cmd);
                    match cmd.as_str() {
                        "+" => {
                            println!("(ack)")
                        }
                        "-" => {
                            println!("(!! nack)")
                        }
                        _ => {
                            println!("(!! no ack/nack)")
                        }
                    }
                    pending_command = false;
                } else {
                    println!("-> {}", cmd);
                    let mut remaining_cmd: &str = &cmd;
                    while !remaining_cmd.is_empty() {
                        let (reply, next_remaining) =
                            handle_next_command(remaining_cmd, &emulation_lock)?;
                        remaining_cmd = next_remaining;
                        match reply {
                            Reply::Packet(packet) => {
                                let packet_str = packet.packet_string();
                                println!("<- {}", packet_str);
                                stream.write_all(packet_str.as_bytes()).expect("write");
                                stream.flush().expect("flush");
                                pending_command = true;
                            }
                            Reply::Plus => {
                                stream.write_all("+".as_bytes()).expect("write");
                                stream.flush().expect("flush");
                                pending_command = false;
                            }
                            Reply::Nothing => {}
                        }
                    }
                }
            }
            Err(e) => {
                println!("gdb: err read {}", e);
                break;
            }
        }
    }
    Ok(())
}

fn handle_next_command<'a>(
    input: &'a str,
    emulation_lock: &Arc<RwLock<Emulation>>,
) -> Result<(Reply, &'a str), String> {
    let (may_cmd, remaining_input) = parse_next_command(input)?;

    if let Some(cmd) = may_cmd {
        let packet = match cmd {
            Command::Plus => return Ok((Reply::Plus, "")),
            Command::StopQuery => Packet::ack("S00"),
            Command::QSupported(_) => {
                Packet::nack("qSupported:hwbreak+;vContSupported+;QThreadEvents+")
            }
            Command::QC => Packet::ack("QC1"),
            Command::QAttached => Packet::ack("1"), //Message::new_ack("+$1#31"), // process already exists, gdb attached to it
            Command::QTStatus => Packet::ack(""), //Message::new_ack("+$#00"), // no tracing going on
            Command::QFThreadInfo => Packet::ack("l"), // Message::new_ack("+$l#6c"), // no thread support
            Command::VContQ => Packet::ack("vcont;c;s;t"),
            Command::VMustReplyEmpty => Packet::ack(""),
            Command::H(_) => Packet::ack("OK"), //Message::new_ack("+$OK#9a"), // only one thread in soma, nothing to prepare. just ack
            Command::G => {
                let emu = emulation_lock.read().unwrap();
                let pc = emu.dmg.sm83.reg.pc.to_le_bytes();
                let sp = emu.dmg.sm83.reg.sp.to_le_bytes();
                Packet::ack(&format!(
                    "0000000000000000{:02X}{:02X}{:02X}{:02X}0000000000000000000000000000",
                    //AF BC  DE  HL  SP    PC
                    sp[0],
                    sp[1],
                    pc[0],
                    pc[1]
                ))
            }
            Command::M(range) => read_memory(emulation_lock, range), // Packet::ack(&("0".repeat(range.length * 2))),
            Command::P => Packet::ack("00000000"),
            _ => return Err(format!("unkown command: {:?}", cmd).to_string()),
        };
        Ok((Reply::Packet(packet), remaining_input))
    } else {
        Ok((Reply::Nothing, remaining_input))
    }
}

fn read_memory(emulation_lock: &Arc<RwLock<Emulation>>, range: MemoryRange) -> Packet {
    let emu = emulation_lock.read().unwrap();
    let mut result = String::new();
    for p in range.addr..(range.addr + range.length) {
        println!("!!! p = {:x}", p);
        if p >= 0xA000 && p < 0xFF00 {
            // RAM, sprite table and echo RAM not readable yet
            write!(&mut result, "{:02x}", 0).unwrap();
        } else if p < 0xFFFF {
            let byte = emu.dmg.mc.read(p as u16);
            write!(&mut result, "{:02x}", byte).unwrap();
        } else {
            write!(&mut result, "{:02x}", 0).unwrap();
        }
    }
    Packet::ack(&result)
}

fn parse_next_command(input: &str) -> Result<(Option<Command>, &str), String> {
    if input.starts_with("+") {
        return Ok((Some(Command::Plus), input.get(1..).unwrap_or("")));
    }

    let may_end_ix = input.find('#');
    if may_end_ix.is_none() {
        // command not yet complete, wait for next try with more input
        return Ok((None, input));
    }
    let end_ix = may_end_ix.unwrap();

    if !input.starts_with('$') {
        return Err(format!("invalid command: {}", input));
    }

    // extract the command part
    let input = &input[1..end_ix];

    let (command, params) = if let Some(destruct) = input.split_once(':') {
        destruct
    } else {
        (input, "")
    };

    let cmd = if command == "?" {
        Command::StopQuery
    } else if command.starts_with("p") {
        Command::P // TODO parse register number
    } else if command.starts_with("m") {
        if let Some(mem_range) = command.get(1..) {
            Command::M(parse_memory_range(mem_range)?)
        } else {
            return Err(format!("m: invalid memory range: {}", command));
        }
    } else if command == "g" {
        Command::G
    } else if command.starts_with('H') {
        Command::H(parse_h_params(command)?)
    } else if command == "qSupported" {
        Command::QSupported(parse_gdb_features(params)?)
    } else if command == "qC" {
        Command::QC
    } else if command == "qAttached" {
        Command::QAttached
    } else if command == "qTStatus" {
        Command::QTStatus
    } else if command == "qfThreadInfo" {
        Command::QFThreadInfo
    } else if command == "vCont?" {
        Command::VContQ
    } else if command == "vMustReplyEmpty" {
        Command::VMustReplyEmpty
    } else {
        Command::Unknown(command.to_string())
    };

    Ok((Some(cmd), input.get(end_ix..).unwrap_or("")))
}

fn parse_gdb_features(param_str: &str) -> Result<GDBFeatures, String> {
    // TODO actually parse the GDBFeatures
    Ok(GDBFeatures {})
}

fn parse_h_params(h_command: &str) -> Result<HParams, String> {
    // TODO actually parse the HParams
    Ok(HParams {})
}

fn parse_memory_range(input: &str) -> Result<MemoryRange, String> {
    if let Some((addr_str, len_str)) = input.split_once(',') {
        let addr = usize::from_str_radix(addr_str, 16).map_err(|e| e.to_string())?;
        let len = usize::from_str_radix(len_str, 16).map_err(|e| e.to_string())?;
        Ok(MemoryRange {
            addr: addr,
            length: len,
        })
    } else {
        Err(format!("invalid memory range: {}", input))
    }
}
