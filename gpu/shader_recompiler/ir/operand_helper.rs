//! Operand indices and buffer-address access, ported from
//! `legacy/gpu/shadps4/shader_recompiler/ir/operand_helper.h`. The C++ uses namespaces of
//! `static const size_t`; the port uses modules of constants, and its two `static_assert`s
//! become const assertions.

use super::microinstruction::InstRef;
use super::opcodes::Opcode;
use super::value::U32;

/// `Shader::IR::LoadBufferArgs`.
pub mod load_buffer_args {
    /// `LoadBufferArgs::Handle`.
    pub const HANDLE: usize = 0;
    /// `LoadBufferArgs::Address`.
    pub const ADDRESS: usize = 1;
}

/// `Shader::IR::StoreBufferArgs`.
pub mod store_buffer_args {
    /// `StoreBufferArgs::Handle`.
    pub const HANDLE: usize = 0;
    /// `StoreBufferArgs::Address`.
    pub const ADDRESS: usize = 1;
    /// `StoreBufferArgs::Data`.
    pub const DATA: usize = 2;
}

// The header's two `static_assert`s: the handle and address slots line up between the two.
const _: () = assert!(load_buffer_args::HANDLE == store_buffer_args::HANDLE);
const _: () = assert!(load_buffer_args::ADDRESS == store_buffer_args::ADDRESS);

/// `GetBufferAddressComponent`: the address argument is a `CompositeConstructU32x3` whose
/// components are kept separate. The C++ asserts the opcode; the port checks it too.
pub fn get_buffer_address_component(buffer_inst: &InstRef, comp: usize) -> U32 {
    let address = buffer_inst.borrow().arg(store_buffer_args::ADDRESS).inst();
    assert_eq!(
        address.borrow().get_opcode(),
        Opcode::CompositeConstructU32x3,
        "the buffer address is a u32x3 composite"
    );
    let component = {
        let inst = address.borrow();
        inst.arg(comp)
    };
    U32::new(component)
}

/// `GetBufferIndexArg`: the address's first component.
pub fn get_buffer_index_arg(buffer_inst: &InstRef) -> U32 {
    get_buffer_address_component(buffer_inst, 0)
}

/// `GetBufferVOffsetArg`: the address's second component.
pub fn get_buffer_voffset_arg(buffer_inst: &InstRef) -> U32 {
    get_buffer_address_component(buffer_inst, 1)
}

/// `GetBufferSOffsetArg`: the address's third component.
pub fn get_buffer_soffset_arg(buffer_inst: &InstRef) -> U32 {
    get_buffer_address_component(buffer_inst, 2)
}

#[cfg(test)]
mod tests {
    use super::super::basic_block::Block;
    use super::super::microinstruction::{set_arg, Inst};
    use super::super::value::Value;
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn the_argument_slots_line_up() {
        assert_eq!(load_buffer_args::HANDLE, 0);
        assert_eq!(load_buffer_args::ADDRESS, 1);
        assert_eq!(store_buffer_args::DATA, 2);
        assert_eq!(load_buffer_args::HANDLE, store_buffer_args::HANDLE);
        assert_eq!(load_buffer_args::ADDRESS, store_buffer_args::ADDRESS);
    }

    #[test]
    fn a_buffer_address_hands_back_its_three_components() {
        let handle = Rc::new(RefCell::new(Inst::new(Opcode::ReadConstBuffer, 0)));
        let address = Rc::new(RefCell::new(Inst::new(Opcode::CompositeConstructU32x3, 0)));
        let index = Value::from_u32(0x11);
        let voffset = Value::from_u32(0x22);
        let soffset = Value::from_u32(0x33);
        set_arg(&address, 0, index);
        set_arg(&address, 1, voffset);
        set_arg(&address, 2, soffset);

        let buffer = Rc::new(RefCell::new(Inst::new(Opcode::StoreBufferU32, 0)));
        set_arg(&buffer, store_buffer_args::HANDLE, Value::from_inst(handle));
        set_arg(
            &buffer,
            store_buffer_args::ADDRESS,
            Value::from_inst(address.clone()),
        );
        set_arg(&buffer, store_buffer_args::DATA, Value::from_u32(0x44));

        assert_eq!(get_buffer_index_arg(&buffer).value().u32(), 0x11);
        assert_eq!(get_buffer_voffset_arg(&buffer).value().u32(), 0x22);
        assert_eq!(get_buffer_soffset_arg(&buffer).value().u32(), 0x33);
        assert_eq!(get_buffer_address_component(&buffer, 1).value().u32(), 0x22);
        // The address instruction is a real producer of the buffer instruction.
        assert_eq!(address.borrow().use_count(), 1);
        let _ = Block::new_ref();

        // An address that is not a composite is refused, as the C++'s assert does.
        let plain = Rc::new(RefCell::new(Inst::new(Opcode::ReadConst, 0)));
        let broken = Rc::new(RefCell::new(Inst::new(Opcode::StoreBufferU32, 0)));
        set_arg(&broken, 0, Value::from_u32(0));
        set_arg(&broken, 1, Value::from_inst(plain));
        set_arg(&broken, 2, Value::from_u32(0));
        let refused = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            get_buffer_index_arg(&broken);
        }));
        assert!(refused.is_err(), "the address must be a u32x3 composite");
    }
}
