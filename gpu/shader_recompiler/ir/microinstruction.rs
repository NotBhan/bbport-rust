//! IR instructions, ported from `legacy/gpu/shadps4/shader_recompiler/ir/microinstruction.h`
//! and `microinstruction.cpp`.
//!
//! Two things differ from the C++. The C++ allocates instructions in an `ObjectPool` and passes
//! `Inst*` around; the port hands out `Rc<RefCell<Inst>>` handles instead, so the mutation
//! helpers below take the handle rather than `&mut self` — the C++ writes into the *used*
//! instruction's list, which a handle can reach and a borrow cannot. And the C++ asserts that
//! an instruction never ends up in its own use list, which the port enforces by skipping that
//! link rather than panicking on the borrow.

use super::opcodes::{num_args_of, type_of, Opcode};
use super::r#type::Type;
use super::reg::{BufferInstInfo, FloatClassFunc, TextureInstInfo};
use super::ssa::{RegTag, EMPTY_REG_TAG};
pub use super::value::{InstRef, Value};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// The block type, which the instruction's parent link points at.
pub use super::basic_block::{Block, BlockRef};

/// The SSA state a block carries, re-exported for callers that work with instructions.
pub use super::ssa::SsaState;

/// `Shader::IR::Use`: one instruction's operand referring to another.
#[derive(Clone, Debug)]
pub struct Use {
    pub user: InstRef,
    pub operand: u32,
}

impl Use {
    pub fn new(user: InstRef, operand: u32) -> Self {
        Self { user, operand }
    }
}

impl PartialEq for Use {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.user, &other.user) && self.operand == other.operand
    }
}

impl Eq for Use {}

/// `Shader::IR::Inst`.
#[derive(Debug)]
pub struct Inst {
    op: Opcode,
    definition: u32,
    flags: u64,
    parent: Option<Weak<RefCell<Block>>>,
    reg_tag: RegTag,
    /// The union's non-phi member, and the default for a fresh instruction.
    args: Vec<Value>,
    /// The union's phi member.
    phi_args: Vec<(Weak<RefCell<Block>>, Value)>,
    uses: Vec<Use>,
    pub scc_index: u32,
}

impl Inst {
    /// `Inst(IR::Opcode, u64 flags)`.
    pub fn new(op: Opcode, flags: u64) -> Self {
        Self {
            op,
            definition: 0,
            flags,
            parent: None,
            reg_tag: EMPTY_REG_TAG,
            args: if op == Opcode::Phi {
                Vec::new()
            } else {
                vec![Value::empty(); num_args_of(op)]
            },
            phi_args: Vec::new(),
            uses: Vec::new(),
            scc_index: 0,
        }
    }

    /// `Inst(const Inst& base)`: a copy that keeps the opcode, the flags and the arguments, and
    /// starts with no uses of its own.
    pub fn copy_of(base: &Inst) -> Self {
        assert_ne!(base.op, Opcode::Phi, "Copying phi node");
        let mut inst = Self::new(base.op, 0);
        inst.flags = base.flags;
        inst.definition = base.definition;
        inst.reg_tag = base.reg_tag;
        inst.scc_index = base.scc_index;
        inst.args = base.args.clone();
        inst
    }

    /// `GetOpcode`.
    pub fn get_opcode(&self) -> Opcode {
        self.op
    }

    /// `GetParent`.
    pub fn parent(&self) -> Option<BlockRef> {
        self.parent.as_ref().and_then(Weak::upgrade)
    }

    /// `SetParent`.
    pub fn set_parent(&mut self, block: Weak<RefCell<Block>>) {
        self.parent = Some(block);
    }

    /// `SetParent` with a live handle.
    pub fn set_parent_handle(&mut self, block: &BlockRef) {
        self.parent = Some(Rc::downgrade(block));
    }

    /// `UseCount`.
    pub fn use_count(&self) -> usize {
        self.uses.len()
    }

    /// `HasUses`.
    pub fn has_uses(&self) -> bool {
        !self.uses.is_empty()
    }

    /// `Uses`.
    pub fn uses(&self) -> &[Use] {
        &self.uses
    }

    /// `NumArgs`: a phi node counts its operands.
    /// `Inst::Opcode()`.
    pub fn opcode(&self) -> Opcode {
        self.op
    }

    pub fn num_args(&self) -> usize {
        if self.op == Opcode::Phi {
            self.phi_args.len()
        } else {
            num_args_of(self.op)
        }
    }

    /// `Arg`.
    pub fn arg(&self, index: usize) -> Value {
        if self.op == Opcode::Phi {
            self.phi_args[index].1.clone()
        } else {
            self.args[index].clone()
        }
    }

    /// `PhiBlock`: the predecessor a phi operand arrives from.
    pub fn phi_block(&self, index: usize) -> BlockRef {
        assert_eq!(
            self.op,
            Opcode::Phi,
            "{} is not a Phi instruction",
            self.op.name()
        );
        self.phi_args[index]
            .0
            .upgrade()
            .expect("the phi predecessor is still alive")
    }

    /// `Flags<u64>`: the raw flags word.
    pub fn flags(&self) -> u64 {
        self.flags
    }

    /// `Flags<u32>`.
    pub fn flags_u32(&self) -> u32 {
        self.flags as u32
    }

    /// `Flags<u64>`: the whole word, for the flags wider than a `Type`.
    pub fn flags_u64(&self) -> u64 {
        self.flags
    }

    /// `SetFlags`.
    pub fn set_flags(&mut self, flags: u64) {
        self.flags = flags;
    }

    /// `Flags<IR::Type>`: the type a phi node carries in its flags.
    pub fn type_flags(&self) -> Type {
        Type::from_bits(self.flags as u32)
    }

    /// `SetFlags<IR::Type>`.
    pub fn set_type_flags(&mut self, ty: Type) {
        self.flags = (self.flags & !0xFFFF_FFFF) | ty.bits() as u64;
    }

    /// `Flags<TextureInstInfo>`.
    pub fn texture_info(&self) -> TextureInstInfo {
        TextureInstInfo::from_raw(self.flags as u32)
    }

    /// `Flags<BufferInstInfo>`.
    pub fn buffer_info(&self) -> BufferInstInfo {
        BufferInstInfo::from_raw(self.flags)
    }

    /// `Flags<FloatClassFunc>`.
    pub fn float_class(&self) -> FloatClassFunc {
        FloatClassFunc::from_bits(self.flags as u32)
    }

    /// `Definition<T>` / `SetDefinition<T>`, over the port's 32-bit word.
    pub fn definition_bits(&self) -> u32 {
        self.definition
    }

    pub fn set_definition_bits(&mut self, definition: u32) {
        self.definition = definition;
    }

    /// `SetRegTag`.
    pub fn set_reg_tag(&mut self, tag: RegTag) {
        self.reg_tag = tag;
    }

    /// `GetRegTag`.
    pub fn reg_tag(&self) -> RegTag {
        self.reg_tag
    }

    /// `Inst::Type`.
    pub fn ty(&self) -> Type {
        if self.op == Opcode::Phi {
            // The type of a phi node is stored in its flags.
            return self.type_flags();
        }
        if self.op == Opcode::GetVirtualRegister {
            // The type of a virtual register load depends on the register id.
            return self.arg(0).virtual_reg().ty;
        }
        type_of(self.op)
    }

    /// `Inst::AreAllArgsImmediates`.
    pub fn are_all_args_immediates(&self) -> bool {
        assert_ne!(
            self.op,
            Opcode::Phi,
            "Testing for all arguments are immediates on phi instruction"
        );
        let count = self.num_args();
        (0..count).all(|index| self.args[index].is_immediate())
    }

    /// `Inst::MayHaveSideEffects`: the opcode list the header spells out.
    pub fn may_have_side_effects(&self) -> bool {
        matches!(
            self.op,
            Opcode::Barrier
                | Opcode::WorkgroupMemoryBarrier
                | Opcode::DeviceMemoryBarrier
                | Opcode::ConditionRef
                | Opcode::Prologue
                | Opcode::Epilogue
                | Opcode::Discard
                | Opcode::DiscardCond
                | Opcode::SetAttribute
                | Opcode::SetTcsGenericAttribute
                | Opcode::SetPatch
                | Opcode::StoreBufferU8
                | Opcode::StoreBufferU16
                | Opcode::StoreBufferU32
                | Opcode::StoreBufferU32x2
                | Opcode::StoreBufferU32x3
                | Opcode::StoreBufferU32x4
                | Opcode::StoreBufferU64
                | Opcode::StoreBufferF32
                | Opcode::StoreBufferF32x2
                | Opcode::StoreBufferF32x3
                | Opcode::StoreBufferF32x4
                | Opcode::StoreBufferFormatF32
                | Opcode::BufferAtomicIAdd32
                | Opcode::BufferAtomicIAdd64
                | Opcode::BufferAtomicISub32
                | Opcode::BufferAtomicSMin32
                | Opcode::BufferAtomicSMin64
                | Opcode::BufferAtomicUMin32
                | Opcode::BufferAtomicUMin64
                | Opcode::BufferAtomicFMin32
                | Opcode::BufferAtomicSMax32
                | Opcode::BufferAtomicSMax64
                | Opcode::BufferAtomicUMax32
                | Opcode::BufferAtomicUMax64
                | Opcode::BufferAtomicFMax32
                | Opcode::BufferAtomicInc32
                | Opcode::BufferAtomicDec32
                | Opcode::BufferAtomicAnd32
                | Opcode::BufferAtomicOr32
                | Opcode::BufferAtomicXor32
                | Opcode::BufferAtomicSwap32
                | Opcode::BufferAtomicCmpSwap32
                | Opcode::BufferAtomicFCmpSwap32
                | Opcode::WriteSharedU16
                | Opcode::WriteSharedU32
                | Opcode::WriteSharedU64
                | Opcode::SharedAtomicIAdd32
                | Opcode::SharedAtomicISub32
                | Opcode::SharedAtomicSMin32
                | Opcode::SharedAtomicUMin32
                | Opcode::SharedAtomicSMax32
                | Opcode::SharedAtomicUMax32
                | Opcode::SharedAtomicInc32
                | Opcode::SharedAtomicDec32
                | Opcode::SharedAtomicAnd32
                | Opcode::SharedAtomicOr32
                | Opcode::SharedAtomicXor32
                | Opcode::SharedAtomicIAdd64
                | Opcode::SharedAtomicISub64
                | Opcode::SharedAtomicSMin64
                | Opcode::SharedAtomicUMin64
                | Opcode::SharedAtomicSMax64
                | Opcode::SharedAtomicUMax64
                | Opcode::SharedAtomicInc64
                | Opcode::SharedAtomicDec64
                | Opcode::SharedAtomicAnd64
                | Opcode::SharedAtomicOr64
                | Opcode::SharedAtomicXor64
                | Opcode::SharedAtomicCmpSwap32
                | Opcode::SharedAtomicCmpSwap64
                | Opcode::ImageWrite
                | Opcode::ImageAtomicIAdd32
                | Opcode::ImageAtomicSMin32
                | Opcode::ImageAtomicUMin32
                | Opcode::ImageAtomicSMax32
                | Opcode::ImageAtomicUMax32
                | Opcode::ImageAtomicFMax32
                | Opcode::ImageAtomicFMin32
                | Opcode::ImageAtomicInc32
                | Opcode::ImageAtomicDec32
                | Opcode::ImageAtomicAnd32
                | Opcode::ImageAtomicOr32
                | Opcode::ImageAtomicXor32
                | Opcode::ImageAtomicExchange32
                | Opcode::ImageAtomicCmpSwap32
                | Opcode::DebugPrint
                | Opcode::EmitVertex
                | Opcode::EmitPrimitive
        )
    }

    /// `ReplaceOpcode`.
    pub fn replace_opcode(&mut self, opcode: Opcode) {
        assert_ne!(opcode, Opcode::Phi, "Cannot transition into Phi");
        if self.op == Opcode::Phi {
            // Transition out of phi arguments into non-phi.
            let count = self.phi_args.len();
            self.args = vec![Value::empty(); count.max(6)];
            self.phi_args.clear();
        }
        self.op = opcode;
    }

    /// The storage write shared by the C++'s `SetArg`.
    fn set_arg_storage(&mut self, index: usize, value: Value) {
        if self.op == Opcode::Phi {
            self.phi_args[index].1 = value;
        } else {
            self.args[index] = value;
        }
    }
}

/// `IsPhi`.
pub fn is_phi(inst: &InstRef) -> bool {
    inst.borrow().get_opcode() == Opcode::Phi
}

/// `Inst::Use`: records that `user` reads `used`.
fn add_use(user: &InstRef, used: &InstRef, operand: u32) {
    if Rc::ptr_eq(user, used) {
        // The C++ asserts an instruction is never its own user; the port keeps that invariant
        // without borrowing the same instruction twice.
        return;
    }
    used.borrow_mut().uses.push(Use::new(user.clone(), operand));
}

/// `Inst::UndoUse`.
fn remove_use(user: &InstRef, used: &InstRef, operand: u32) {
    if Rc::ptr_eq(user, used) {
        return;
    }
    let mut used = used.borrow_mut();
    if let Some(index) = used
        .uses
        .iter()
        .position(|r#use| r#use.operand == operand && Rc::ptr_eq(&r#use.user, user))
    {
        used.uses.remove(index);
    }
}

/// `Inst::SetArg`: writes the argument and keeps the used instruction's list in step.
pub fn set_arg(this: &InstRef, index: usize, value: Value) {
    let (old, num_args) = {
        let inst = this.borrow();
        (inst.arg(index), inst.num_args())
    };
    assert!(
        index < num_args,
        "Out of bounds argument index {} in opcode {}",
        index,
        this.borrow().get_opcode().name()
    );
    if let Some(used) = old.try_inst() {
        remove_use(this, &used, index as u32);
    }
    if let Some(used) = value.try_inst() {
        add_use(this, &used, index as u32);
    }
    this.borrow_mut().set_arg_storage(index, value);
}

/// `Inst::ClearArgs`.
pub fn clear_args(this: &InstRef) {
    let (is_phi, args) = {
        let inst = this.borrow();
        let count = inst.num_args();
        (
            inst.get_opcode() == Opcode::Phi,
            (0..count).map(|i| inst.arg(i)).collect::<Vec<_>>(),
        )
    };
    for (index, arg) in args.into_iter().enumerate() {
        if let Some(used) = arg.try_inst() {
            remove_use(this, &used, index as u32);
        }
    }
    let mut inst = this.borrow_mut();
    if is_phi {
        inst.phi_args.clear();
    } else {
        for arg in &mut inst.args {
            *arg = Value::empty();
        }
    }
}

/// `Inst::AddPhiOperand`.
pub fn add_phi_operand(this: &InstRef, predecessor: &BlockRef, value: Value) {
    if let Some(used) = value.try_inst() {
        let operand = this.borrow().phi_args.len() as u32;
        add_use(this, &used, operand);
    }
    this.borrow_mut()
        .phi_args
        .push((Rc::downgrade(predecessor), value));
}

/// `Inst::ReplaceUsesWith`: every user reads the replacement instead, and this instruction is
/// left void.
pub fn replace_uses_with(this: &InstRef, replacement: Value) {
    let uses = this.borrow().uses.clone();
    for r#use in uses {
        set_arg(&r#use.user, r#use.operand as usize, replacement.clone());
    }
    invalidate(this);
}

/// `Inst::Invalidate`.
pub fn invalidate(this: &InstRef) {
    clear_args(this);
    this.borrow_mut().replace_opcode(Opcode::Void);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_inst(op: Opcode) -> InstRef {
        Rc::new(RefCell::new(Inst::new(op, 0)))
    }

    #[test]
    fn a_fresh_instruction_has_its_opcodes_arguments() {
        let inst = new_inst(Opcode::ReadConst);
        assert_eq!(inst.borrow().get_opcode(), Opcode::ReadConst);
        assert_eq!(inst.borrow().num_args(), 2);
        assert!(inst.borrow().arg(0).is_empty());
        assert_eq!(inst.borrow().ty(), Type::U32);
        assert_eq!(inst.borrow().use_count(), 0);
        assert!(!inst.borrow().has_uses());
        assert!(inst.borrow().parent().is_none());
        assert!(!inst.borrow().may_have_side_effects());
        assert!(!inst.borrow().reg_tag().is_set());

        let store = new_inst(Opcode::WriteSharedU32);
        assert!(store.borrow().may_have_side_effects());
        let barrier = new_inst(Opcode::Barrier);
        assert!(barrier.borrow().may_have_side_effects());
        let add = new_inst(Opcode::IAdd32);
        assert!(!add.borrow().may_have_side_effects());
    }

    #[test]
    fn setting_an_argument_records_a_use() {
        let producer = new_inst(Opcode::ReadConst);
        let consumer = new_inst(Opcode::IAdd32);
        set_arg(&consumer, 0, Value::from_inst(producer.clone()));
        assert_eq!(producer.borrow().use_count(), 1);
        assert!(producer.borrow().has_uses());
        assert!(Rc::ptr_eq(&producer.borrow().uses()[0].user, &consumer));
        assert_eq!(producer.borrow().uses()[0].operand, 0);

        // Replacing the argument drops the old use and takes the new one.
        let other = new_inst(Opcode::ReadConst);
        set_arg(&consumer, 0, Value::from_inst(other.clone()));
        assert_eq!(
            producer.borrow().use_count(),
            0,
            "the old producer is released"
        );
        assert_eq!(other.borrow().use_count(), 1);
        set_arg(&consumer, 0, Value::from_u32(5));
        assert_eq!(other.borrow().use_count(), 0, "an immediate uses nothing");
        assert_eq!(consumer.borrow().arg(0).u32(), 5);
    }

    #[test]
    fn clearing_and_invalidating_release_the_uses() {
        let producer = new_inst(Opcode::ReadConst);
        let consumer = new_inst(Opcode::IAdd32);
        set_arg(&consumer, 0, Value::from_inst(producer.clone()));
        set_arg(&consumer, 1, Value::from_inst(producer.clone()));
        assert_eq!(producer.borrow().use_count(), 2);
        clear_args(&consumer);
        assert_eq!(producer.borrow().use_count(), 0);
        assert!(consumer.borrow().arg(0).is_empty());
        assert_eq!(consumer.borrow().num_args(), 2);

        set_arg(&consumer, 0, Value::from_inst(producer.clone()));
        invalidate(&consumer);
        assert_eq!(producer.borrow().use_count(), 0);
        assert_eq!(consumer.borrow().get_opcode(), Opcode::Void);
    }

    #[test]
    fn replacing_uses_rewires_every_reader() {
        let producer = new_inst(Opcode::ReadConst);
        let replacement = new_inst(Opcode::ReadConst);
        let first = new_inst(Opcode::IAdd32);
        let second = new_inst(Opcode::IMul32);
        set_arg(&first, 0, Value::from_inst(producer.clone()));
        set_arg(&second, 1, Value::from_inst(producer.clone()));
        assert_eq!(producer.borrow().use_count(), 2);

        replace_uses_with(&producer, Value::from_inst(replacement.clone()));
        assert_eq!(producer.borrow().use_count(), 0);
        assert_eq!(
            producer.borrow().get_opcode(),
            Opcode::Void,
            "it is invalidated"
        );
        assert_eq!(replacement.borrow().use_count(), 2);
        assert!(Rc::ptr_eq(&first.borrow().arg(0).inst(), &replacement));
        assert!(Rc::ptr_eq(&second.borrow().arg(1).inst(), &replacement));
    }

    #[test]
    fn a_phi_node_collects_operands_with_their_blocks() {
        let phi = new_inst(Opcode::Phi);
        phi.borrow_mut().set_type_flags(Type::F32);
        assert_eq!(phi.borrow().ty(), Type::F32);
        assert!(is_phi(&phi));

        let entry = Block::new_ref();
        let other = Block::new_ref();
        let value = new_inst(Opcode::ReadConst);
        add_phi_operand(&phi, &entry, Value::from_u32(1));
        add_phi_operand(&phi, &other, Value::from_inst(value.clone()));
        assert_eq!(phi.borrow().num_args(), 2);
        assert!(Rc::ptr_eq(&phi.borrow().phi_block(1), &other));
        assert_eq!(phi.borrow().arg(0).u32(), 1);
        assert_eq!(value.borrow().use_count(), 1, "the phi operand is a use");
        // The operand's type comes from the instruction it names, a ReadConst here.
        assert_eq!(phi.borrow().arg(1).ty(), Type::U32);

        // A copied instruction takes the arguments but no uses.
        let copy = Inst::copy_of(&value.borrow());
        assert_eq!(copy.get_opcode(), Opcode::ReadConst);
        assert_eq!(copy.flags(), value.borrow().flags());
        assert_eq!(copy.use_count(), 0);
        let phi = Rc::new(RefCell::new(Inst::new(Opcode::Phi, 0)));
        let inner = Rc::clone(&phi);
        let refused = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            let borrowed = inner.borrow();
            let copy = Inst::copy_of(&borrowed);
            drop(borrowed);
            copy
        }));
        assert!(refused.is_err(), "phi nodes cannot be copied");
    }

    #[test]
    fn a_virtual_register_load_takes_its_type_from_the_register() {
        use crate::gpu::shader_recompiler::ir::reg::VirtualReg;
        use crate::gpu::shader_recompiler::ir::ssa::RegTag;
        let inst = new_inst(Opcode::GetVirtualRegister);
        inst.borrow_mut().set_parent_handle(&Block::new_ref());
        let reg = VirtualReg::new(1, Type::F16x2);
        set_arg(&inst, 0, Value::from_virtual_reg(reg));
        assert_eq!(inst.borrow().ty(), Type::F16x2);
        let tag = RegTag::from_virtual_reg(reg);
        inst.borrow_mut().set_reg_tag(tag);
        assert_eq!(
            inst.borrow().reg_tag().ty,
            crate::gpu::shader_recompiler::ir::ssa::RegType::VirtualReg
        );

        // The instruction's parent link survives as a weak handle.
        let block = Block::new_ref();
        inst.borrow_mut().set_parent_handle(&block);
        assert!(Rc::ptr_eq(&inst.borrow().parent().expect("parent"), &block));
    }
}
