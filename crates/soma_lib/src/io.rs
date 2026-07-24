use crate::sm83::ExecErr;

type IOEffect = fn();

#[derive(Copy, Clone)]
struct MemEffect {
    value: u8,
    effect: IOEffect,
}

pub struct IO {
    mem_effect: [MemEffect; 0xFF],
}

impl IO {
    pub fn init() -> IO {
        IO {
            mem_effect: [MemEffect {
                value: 0x00,
                effect: no_op,
            }; 0xFF],
        }
    }

    /// For testing, init IO with a predefined value
    pub fn init_with_value(addr: u16, v: u8) -> Result<IO, ExecErr> {
        let mut io = IO::init();
        io.write(addr, v)?;
        Ok(io)
    }

    /// addrs = absolute address. Must be in IO address space.
    pub fn write(&mut self, addr: u16, v: u8) -> Result<(), ExecErr> {
        let offset = (addr - 0xFF00) as usize;
        if offset >= self.mem_effect.len() {
            return Err(ExecErr::GeneralError("illegal IO address"));
        }
        let me = &mut self.mem_effect[offset];
        me.value = v;
        (me.effect)();
        Ok(())
    }

    pub fn read(&self, addr: u16) -> u8 {
        let v = self.mem_effect[(addr - 0xFF00) as usize].value;
        //panic!("mc read = {:x}, v = {:x}", addr, v);
        v
    }
}

fn no_op() {}
