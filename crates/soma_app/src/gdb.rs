#[cfg(test)]
#[path = "./gdb_test.rs"]
mod gdb_test;

use std::io::{Read, Write};
use std::net::TcpListener;

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

struct Message {
    content: String,
    needs_ack: bool,
}

impl Message {
    /// A regular message that is acknowledged
    fn new_ack(content: &str) -> Message {
        Message {
            content: content.to_string(),
            needs_ack: true,
        }
    }

    fn new_no_ack(content: &str) -> Message {
        Message {
            content: content.to_string(),
            needs_ack: false,
        }
    }
}

enum Reply {
    Nothing,
    Message(Message),
}

pub fn gdb_serve() -> Result<(), String> {
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
                        let (reply, next_remaining) = handle_next_command(remaining_cmd)?;
                        remaining_cmd = next_remaining;
                        if let Reply::Message(message) = reply {
                            println!("<- {}", message.content);
                            stream.write_all(message.content.as_bytes()).expect("write");
                            stream.flush().expect("flush");
                            pending_command = message.needs_ack;
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

fn handle_next_command(input: &str) -> Result<(Reply, &str), String> {
    let (may_cmd, remaining_input) = parse_next_command(input)?;

    if let Some(cmd) = may_cmd {
        let message = match cmd {
            Command::Plus => Message::new_no_ack("+"),
            Command::StopQuery => Message::new_ack("+$S00#b3"),
            Command::QSupported(_) => {
                Message::new_ack("$qSupported:swbreak+;vContSupported+;QThreadEvents+#45")
            }
            Command::QC => Message::new_ack("+$QC1#c5"),
            Command::QAttached => Message::new_ack("+$1#31"), // process already exists, gdb attached to it
            Command::QTStatus => Message::new_ack("+$#00"),   // no tracing going on
            Command::QFThreadInfo => Message::new_ack("+$l#6c"), // no thread support
            Command::VContQ => Message::new_ack("+$vcont;c;s;t#25"),
            Command::VMustReplyEmpty => Message::new_ack("+$#00"),
            Command::H(_) => Message::new_ack("+$OK#9a"), // only one thread in soma, nothing to prepare. just ack
            Command::G => {
                Message::new_ack("+$0000000000000000000000000000000000000000000000000000#c0")
            }
            Command::M(range) => {
                Message::new_ack(&format!("+${}#00", "0".repeat(range.length * 2)))
            }
            Command::P => Message::new_ack("+$00000000#80"),
            _ => return Err(format!("unkown command: {:?}", cmd).to_string()),
        };
        Ok((Reply::Message(message), remaining_input))
    } else {
        Ok((Reply::Nothing, remaining_input))
    }
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
