//! Resource discovery data, ported from
//! `legacy/gpu/shadps4/shader_recompiler/ir/passes/resource_pass.h`. The discovery pass fills
//! this list and the patching pass consumes it. The C++ `PostOpData` union keeps its two
//! members side by side here, since which one is read depends on the resource kind.

use crate::gpu::shader_recompiler::ir::microinstruction::InstRef;
use crate::gpu::shader_recompiler::ir::opcodes::Opcode;
use crate::gpu::shader_recompiler::ir::value::Value;
use crate::gpu::shader_recompiler::resource::SharpFetchPostOp;

/// `PostOpData`.
#[derive(Debug, Clone, Default)]
pub struct PostOpData {
    pub dw1_mask: u32,
    pub lod_prod: Value,
}

/// `SharpReference`.
#[derive(Debug, Clone)]
pub struct SharpReference {
    pub dwords: [Value; 8],
    pub num_dwords: u32,
    pub post_op: SharpFetchPostOp,
    pub post_op_data: PostOpData,
}

impl Default for SharpReference {
    fn default() -> Self {
        SharpReference {
            dwords: std::array::from_fn(|_| Value::empty()),
            num_dwords: 0,
            post_op: SharpFetchPostOp::default(),
            post_op_data: PostOpData::default(),
        }
    }
}

/// `ResourceDiscovery`.
#[derive(Debug, Clone)]
pub struct ResourceDiscovery {
    pub user: InstRef,
    pub sharps: [SharpReference; 2],
}

impl ResourceDiscovery {
    /// A discovery entry for `user`, with no sharp words found yet.
    pub fn new(user: InstRef) -> Self {
        ResourceDiscovery {
            user,
            sharps: Default::default(),
        }
    }
}

/// `ResourceDiscoveryList`. The C++ caps the small vector at 32; the port keeps a `Vec`.
pub type ResourceDiscoveryList = Vec<ResourceDiscovery>;

/// `IsBufferAtomic`.
pub fn is_buffer_atomic(inst: &InstRef) -> bool {
    matches!(
        inst.borrow().get_opcode(),
        Opcode::BufferAtomicIAdd32
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
    )
}

/// `IsBufferStore`.
pub fn is_buffer_store(inst: &InstRef) -> bool {
    matches!(
        inst.borrow().get_opcode(),
        Opcode::StoreBufferU8
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
    ) || is_buffer_atomic(inst)
}

/// `IsDataRingInstruction`: the shared-memory forms only count when the GDS flag is set.
pub fn is_data_ring_instruction(inst: &InstRef) -> bool {
    let is_gds = || inst.borrow().flags() & 1 != 0;
    match inst.borrow().get_opcode() {
        Opcode::DataAppend | Opcode::DataConsume => true,
        Opcode::LoadSharedU16
        | Opcode::LoadSharedU32
        | Opcode::LoadSharedU64
        | Opcode::WriteSharedU16
        | Opcode::WriteSharedU32
        | Opcode::WriteSharedU64
        | Opcode::SharedAtomicIAdd32
        | Opcode::SharedAtomicIAdd64
        | Opcode::SharedAtomicUMin32
        | Opcode::SharedAtomicUMin64
        | Opcode::SharedAtomicSMin32
        | Opcode::SharedAtomicSMin64
        | Opcode::SharedAtomicUMax32
        | Opcode::SharedAtomicUMax64
        | Opcode::SharedAtomicSMax32
        | Opcode::SharedAtomicSMax64
        | Opcode::SharedAtomicAnd32
        | Opcode::SharedAtomicAnd64
        | Opcode::SharedAtomicOr32
        | Opcode::SharedAtomicOr64
        | Opcode::SharedAtomicXor32
        | Opcode::SharedAtomicXor64
        | Opcode::SharedAtomicISub32
        | Opcode::SharedAtomicISub64
        | Opcode::SharedAtomicInc32
        | Opcode::SharedAtomicInc64
        | Opcode::SharedAtomicDec32
        | Opcode::SharedAtomicDec64
        | Opcode::SharedAtomicCmpSwap32
        | Opcode::SharedAtomicCmpSwap64 => is_gds(),
        _ => false,
    }
}

/// `IsBufferInstruction`.
pub fn is_buffer_instruction(inst: &InstRef) -> bool {
    matches!(
        inst.borrow().get_opcode(),
        Opcode::LoadBufferU8
            | Opcode::LoadBufferU16
            | Opcode::LoadBufferU32
            | Opcode::LoadBufferU32x2
            | Opcode::LoadBufferU32x3
            | Opcode::LoadBufferU32x4
            | Opcode::LoadBufferU64
            | Opcode::LoadBufferF32
            | Opcode::LoadBufferF32x2
            | Opcode::LoadBufferF32x3
            | Opcode::LoadBufferF32x4
            | Opcode::LoadBufferFormatF32
            | Opcode::ReadConstBuffer
    ) || is_buffer_store(inst)
}

/// `IsImageAtomicInstruction`.
pub fn is_image_atomic_instruction(inst: &InstRef) -> bool {
    matches!(
        inst.borrow().get_opcode(),
        Opcode::ImageAtomicIAdd32
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
    )
}

/// `IsImageInstruction`.
pub fn is_image_instruction(inst: &InstRef) -> bool {
    matches!(
        inst.borrow().get_opcode(),
        Opcode::ImageRead
            | Opcode::ImageWrite
            | Opcode::ImageQueryDimensions
            | Opcode::ImageQueryLod
            | Opcode::ImageSampleRaw
    ) || is_image_atomic_instruction(inst)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::shader_recompiler::ir::microinstruction::Inst;
    use std::cell::RefCell;
    use std::rc::Rc;

    fn inst(op: Opcode, flags: u64) -> InstRef {
        Rc::new(RefCell::new(Inst::new(op, flags)))
    }

    #[test]
    fn buffer_instructions_cover_loads_stores_and_atomics() {
        assert!(is_buffer_instruction(&inst(Opcode::LoadBufferU32, 0)));
        assert!(is_buffer_instruction(&inst(Opcode::LoadBufferFormatF32, 0)));
        assert!(is_buffer_instruction(&inst(Opcode::StoreBufferF32x4, 0)));
        assert!(is_buffer_instruction(&inst(Opcode::BufferAtomicIAdd64, 0)));
        assert!(is_buffer_instruction(&inst(Opcode::ReadConstBuffer, 0)));
        assert!(!is_buffer_instruction(&inst(Opcode::Barrier, 0)));
        assert!(!is_buffer_instruction(&inst(Opcode::ImageRead, 0)));
        assert!(is_buffer_store(&inst(Opcode::StoreBufferFormatF32, 0)));
        assert!(is_buffer_store(&inst(Opcode::BufferAtomicFMax32, 0)));
        assert!(!is_buffer_store(&inst(Opcode::LoadBufferU32, 0)));
    }

    #[test]
    fn image_instructions_cover_the_query_and_atomic_forms() {
        for op in [
            Opcode::ImageRead,
            Opcode::ImageWrite,
            Opcode::ImageQueryDimensions,
            Opcode::ImageQueryLod,
            Opcode::ImageSampleRaw,
            Opcode::ImageAtomicExchange32,
            Opcode::ImageAtomicCmpSwap32,
        ] {
            assert!(is_image_instruction(&inst(op, 0)), "{op:?}");
        }
        assert!(!is_image_instruction(&inst(Opcode::LoadBufferU32, 0)));
        assert!(!is_image_instruction(&inst(Opcode::Barrier, 0)));
    }

    #[test]
    fn the_data_ring_requires_the_gds_flag() {
        assert!(is_data_ring_instruction(&inst(Opcode::DataAppend, 0)));
        assert!(is_data_ring_instruction(&inst(Opcode::DataConsume, 0)));
        assert!(is_data_ring_instruction(&inst(Opcode::LoadSharedU32, 1)));
        assert!(is_data_ring_instruction(&inst(Opcode::WriteSharedU64, 1)));
        assert!(is_data_ring_instruction(&inst(
            Opcode::SharedAtomicIAdd64,
            1
        )));
        assert!(!is_data_ring_instruction(&inst(Opcode::LoadSharedU32, 0)));
        assert!(!is_data_ring_instruction(&inst(
            Opcode::SharedAtomicIAdd64,
            0
        )));
        assert!(!is_data_ring_instruction(&inst(Opcode::Barrier, 1)));
    }

    #[test]
    fn a_fresh_sharp_reference_is_empty() {
        let sharp = SharpReference::default();
        assert_eq!(sharp.num_dwords, 0);
        assert_eq!(sharp.post_op, SharpFetchPostOp::None);
        assert_eq!(sharp.post_op_data.dw1_mask, 0);
        assert!(sharp.post_op_data.lod_prod.is_empty());
        assert!(sharp.dwords.iter().all(Value::is_empty));
    }

    #[test]
    fn a_discovery_entry_keeps_its_user_and_two_sharps() {
        let user = inst(Opcode::ImageSampleRaw, 0);
        let discovery = ResourceDiscovery::new(user.clone());
        assert!(Rc::ptr_eq(&discovery.user, &user));
        assert_eq!(discovery.sharps.len(), 2);
        assert!(discovery.sharps.iter().all(|sharp| sharp.num_dwords == 0));
    }
}
