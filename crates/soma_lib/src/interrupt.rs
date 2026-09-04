use crate::{
    memory::MemoryController,
    sm83::{ExecErr, SM83, call_to_addr},
};

const INTERRUPT_VBLANK_MASK: u8 = 0b00000001;

pub fn vblank_interrupt(sm83: &mut SM83, mc: &mut MemoryController) -> Result<(), ExecErr> {
    if !sm83.reg.ime {
        return Ok(());
    }
    if (sm83.reg.ie & INTERRUPT_VBLANK_MASK) == 0 {
        return Ok(());
    }

    call_to_addr(sm83, mc, 0x40, 0)
}
