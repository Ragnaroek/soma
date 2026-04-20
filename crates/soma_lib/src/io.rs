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
                value: 0,
                effect: no_op,
            }; 0xFF],
        }
    }

    /// For testing, init IO with a predefined value
    pub fn init_with_value(addr: u16, v: u8) -> IO {
        let mut io = IO::init();
        io.write(addr, v);
        io
    }

    /// addrs = absolute address. Must be in IO address space.
    pub fn write(&mut self, addr: u16, v: u8) {
        let me = &mut self.mem_effect[(addr - 0xFF00) as usize];
        me.value = v;
        (me.effect)();
    }

    pub fn read(&self, addr: u16) -> u8 {
        self.mem_effect[(addr - 0xFF00) as usize].value
    }
}

fn no_op() {}
