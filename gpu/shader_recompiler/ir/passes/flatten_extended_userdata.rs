//! Extended-userdata flattening, ported from
//! `legacy/gpu/shadps4/shader_recompiler/ir/passes/flatten_extended_userdata_pass.cpp`.
//!
//! The C++ builds an SRT walker with Xbyak and installs a signal handler that patches faulting
//! loads. The port has neither a JIT nor that handler, and its walker slot (`srt.rs`) is filled
//! by the offline builder instead, so the port keeps the IR half: the constant-buffer address
//! addition simplification, the pointer-use graph of the read-constants, their depth-first
//! flattened-buffer offsets, and the flattened buffer size. `RegisterWalkerCode`, the signal
//! handler and the walker dump are not ported.

use crate::gpu::shader_recompiler::info::Info;
use crate::gpu::shader_recompiler::ir::basic_block::BlockRef;
use crate::gpu::shader_recompiler::ir::ir_emitter::IrEmitter;
use crate::gpu::shader_recompiler::ir::microinstruction::{set_arg, InstRef};
use crate::gpu::shader_recompiler::ir::opcodes::Opcode;
use crate::gpu::shader_recompiler::ir::program::Program;
use crate::gpu::shader_recompiler::ir::srt_gvn_table::SrtGvnTable;
use crate::gpu::shader_recompiler::ir::value::{Value, U32, U32U64};
use crate::gpu::shader_recompiler::resource::NUM_USER_DATA_REGS;
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::rc::Rc;

fn position(block: &BlockRef, inst: &InstRef) -> Option<usize> {
    block
        .borrow()
        .instructions
        .iter()
        .position(|candidate| Rc::ptr_eq(candidate, inst))
}

fn key(inst: &InstRef) -> usize {
    Rc::as_ptr(inst) as usize
}

/// `PointerListCompare`: immediates first, sorted by value; instructions after them, sorted by
/// block and then by position in the block.
fn compare_values(a: &Value, b: &Value) -> Ordering {
    if a == b {
        return Ordering::Equal;
    }
    match (a.is_immediate(), b.is_immediate()) {
        (true, true) => a.u32().cmp(&b.u32()),
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        (false, false) => {
            let a_inst = a.inst();
            let b_inst = b.inst();
            let a_block = a_inst
                .borrow()
                .parent()
                .expect("the value has a parent block");
            let b_block = b_inst
                .borrow()
                .parent()
                .expect("the value has a parent block");
            let a_ptr = Rc::as_ptr(&a_block) as usize;
            let b_ptr = Rc::as_ptr(&b_block) as usize;
            if a_ptr != b_ptr {
                return a_ptr.cmp(&b_ptr);
            }
            let a_index = position(&a_block, &a_inst).expect("the value is in its block");
            let b_index = position(&a_block, &b_inst).expect("the value is in its block");
            a_index.cmp(&b_index)
        }
    }
}

/// `PassInfo::PtrUserList`: the offsets a pointer is used with, in the C++ flat map's order.
#[derive(Clone, Default)]
struct PtrUserList {
    entries: Vec<(Value, InstRef)>,
}

impl PtrUserList {
    fn insert(&mut self, key: Value, value: InstRef) {
        match self
            .entries
            .binary_search_by(|(existing, _)| compare_values(existing, &key))
        {
            Ok(index) => self.entries[index].1 = value,
            Err(index) => self.entries.insert(index, (key, value)),
        }
    }

    fn iter(&self) -> std::slice::Iter<'_, (Value, InstRef)> {
        self.entries.iter()
    }
}

/// `PassInfo`.
#[derive(Default)]
struct PassInfo {
    gvn_table: SrtGvnTable,
    pointer_uses: HashMap<usize, PtrUserList>,
    srt_roots: BTreeMap<u32, InstRef>,
    vn_to_inst: HashMap<u32, InstRef>,
    dst_off_dw: u16,
}

impl PassInfo {
    fn uses_as_pointer(&self, inst: &InstRef) -> Option<&PtrUserList> {
        self.pointer_uses.get(&key(inst))
    }

    /// `DeduplicateInstruction`: the first instruction found for a value number.
    fn deduplicate_instruction(&mut self, inst: &InstRef) -> InstRef {
        let number = self.gvn_table.get_value_number_inst(inst);
        self.vn_to_inst
            .entry(number)
            .or_insert_with(|| inst.clone())
            .clone()
    }
}

/// `GetFlatbufOffset`.
fn flatbuf_offset(inst: &InstRef) -> u16 {
    let opcode = inst.borrow().get_opcode();
    match opcode {
        Opcode::ReadConstBuffer => inst.borrow().buffer_info().flatbuf_off_dw() as u16,
        Opcode::ReadConst => inst.borrow().flags() as u16,
        _ => panic!("Instruction not supported"),
    }
}

/// `SetFlatbufOffset`.
fn set_flatbuf_offset(inst: &InstRef, offset: u16) {
    let opcode = inst.borrow().get_opcode();
    match opcode {
        Opcode::ReadConstBuffer => {
            let mut info = inst.borrow().buffer_info();
            info.set_flatbuf_off_dw(u64::from(offset));
            inst.borrow_mut().set_flags(info.raw());
        }
        Opcode::ReadConst => inst.borrow_mut().set_flags(u64::from(offset)),
        _ => panic!("Instruction not supported"),
    }
}

/// `IsAllowedOffsetInstruction`.
fn is_allowed_offset_instruction(inst: &InstRef) -> bool {
    matches!(
        inst.borrow().get_opcode(),
        Opcode::GetUserData
            | Opcode::ReadConst
            | Opcode::ReadConstBuffer
            | Opcode::IAdd32
            | Opcode::ISub32
            | Opcode::IMul32
            | Opcode::ShiftLeftLogical32
            | Opcode::ShiftRightLogical32
            | Opcode::BitwiseAnd32
            | Opcode::BitwiseOr32
            | Opcode::BitwiseXor32
            | Opcode::BitwiseNot32
            | Opcode::UMin32
            | Opcode::UMax32
            | Opcode::BitFieldUExtract
    )
}

/// `IsReadConstSource`.
fn is_read_const_source(value: &Value) -> bool {
    let Some(inst) = value.try_inst() else {
        return false;
    };
    let opcode = inst.borrow().get_opcode();
    matches!(
        opcode,
        Opcode::GetUserData | Opcode::ReadConst | Opcode::ReadConstBuffer
    )
}

/// `SimplifyReadConstAddressAdd`.
fn simplify_read_const_address_add(inst: &InstRef) {
    if inst.borrow().get_opcode() != Opcode::ReadConst {
        return;
    }
    let addr = inst.borrow().arg(0).inst();
    let hi = addr.borrow().arg(1).try_inst();
    let lo = addr.borrow().arg(0).try_inst();
    let (Some(hi), Some(lo)) = (hi, lo) else {
        return;
    };
    if lo.borrow().get_opcode() != Opcode::IAdd32 || hi.borrow().get_opcode() != Opcode::IAdd32 {
        return;
    }

    let (base, add_offset) = {
        let lo_0 = lo.borrow().arg(0);
        if is_read_const_source(&lo_0) {
            (lo_0, lo.borrow().arg(1))
        } else {
            let lo_1 = lo.borrow().arg(1);
            if is_read_const_source(&lo_1) {
                (lo_1, lo.borrow().arg(0))
            } else {
                return;
            }
        }
    };

    let sel = hi.borrow().arg(1).try_inst();
    let Some(sel) = sel else {
        return;
    };
    if sel.borrow().get_opcode() != Opcode::SelectU32 {
        return;
    }
    let sel_true = sel.borrow().arg(1);
    let sel_false = sel.borrow().arg(2);
    if !sel_true.is_immediate()
        || !sel_false.is_immediate()
        || sel_true.u32() != 1
        || sel_false.u32() != 0
    {
        return;
    }

    let hi_arg_0 = hi.borrow().arg(0);
    set_arg(&addr, 0, base);
    set_arg(&addr, 1, hi_arg_0);

    let uses = addr.borrow().uses().to_vec();
    for r#use in uses {
        assert_eq!(
            r#use.user.borrow().get_opcode(),
            Opcode::ReadConst,
            "the SRT address is only used by constant reads"
        );
        assert_eq!(r#use.operand, 0, "the address is the first argument");
        let block = r#use
            .user
            .borrow()
            .parent()
            .expect("the use has a parent block");
        let index = position(&block, &r#use.user).expect("the use is in its block");
        let mut ir = IrEmitter::at(&block, index);
        let offset = U32::new(r#use.user.borrow().arg(1));
        let dw_add_offset = if add_offset.is_immediate() {
            ir.imm32(add_offset.u32() >> 2)
        } else {
            let address = U32::new(add_offset.clone());
            let two = ir.imm32(2u32);
            U32::new(
                ir.shift_right_logical(&U32U64::new(address.into_value()), &two)
                    .into_value(),
            )
        };
        if offset.value().is_immediate() && dw_add_offset.value().is_immediate() {
            let sum = offset.value().u32() + dw_add_offset.value().u32();
            let sum = ir.imm32(sum);
            set_arg(&r#use.user, 1, sum.into_value());
        } else {
            let sum = U32::new(
                ir.i_add(
                    &U32U64::new(offset.into_value()),
                    &U32U64::new(dw_add_offset.into_value()),
                )
                .into_value(),
            );
            set_arg(&r#use.user, 1, sum.into_value());
        }
    }
}

/// `VisitPointer`: assigns the contiguous offsets level by level, then descends into pointers.
fn visit_pointer(subtree: &InstRef, pass_info: &mut PassInfo) {
    let opcode = subtree.borrow().get_opcode();
    if opcode == Opcode::ReadConst && subtree.borrow().flags() as u16 == 0 {
        return;
    }
    if opcode == Opcode::ReadConstBuffer && subtree.borrow().buffer_info().flatbuf_off_dw() == 0 {
        return;
    }
    let Some(use_list) = pass_info.pointer_uses.get(&key(subtree)).cloned() else {
        return;
    };
    for (_, use_) in use_list.iter() {
        let offset = pass_info.dst_off_dw;
        set_flatbuf_offset(use_, offset);
        pass_info.dst_off_dw += 1;
    }
    for (_, use_) in use_list.iter() {
        if pass_info.uses_as_pointer(use_).is_some() {
            visit_pointer(use_, pass_info);
        }
    }
}

/// `GenerateSrtProgram` without the walker: the offsets are assigned in the same depth-first
/// order, and the buffer size is recorded. The walker itself comes from the offline builder.
fn generate_srt_program(info: &mut Info, pass_info: &mut PassInfo) {
    if pass_info.srt_roots.is_empty() {
        return;
    }
    pass_info.dst_off_dw = NUM_USER_DATA_REGS as u16;
    assert_eq!(
        u32::from(pass_info.dst_off_dw),
        info.srt_info.flattened_bufsize_dw,
        "the flattened buffer starts at the user-data registers"
    );
    let roots: Vec<InstRef> = pass_info.srt_roots.values().cloned().collect();
    for root in &roots {
        visit_pointer(root, pass_info);
    }
    info.srt_info.flattened_bufsize_dw = u32::from(pass_info.dst_off_dw);
}

/// `FlattenExtendedUserdataPass`.
pub fn flatten_extended_userdata_pass(program: &mut Program) {
    let post_order = program.post_order_blocks.clone();
    let mut pass_info = PassInfo::default();
    let mut all_readconsts: Vec<InstRef> = Vec::new();
    let mut visited: Vec<InstRef> = Vec::new();

    for block in post_order.iter().rev() {
        let instructions = block.borrow().instructions.clone();
        for inst in &instructions {
            let opcode = inst.borrow().get_opcode();
            if !matches!(opcode, Opcode::ReadConst | Opcode::ReadConstBuffer) {
                continue;
            }
            if opcode == Opcode::ReadConstBuffer && inst.borrow().buffer_info().sharp_source() == 0
            {
                continue;
            }

            simplify_read_const_address_add(inst);
            all_readconsts.push(inst.clone());

            let offset = inst.borrow().arg(1);
            if offset.is_immediate() {
                continue;
            }
            let mut queue = VecDeque::new();
            queue.push_back(offset.inst());
            while let Some(current) = queue.pop_front() {
                let current_opcode = current.borrow().get_opcode();
                if current_opcode == Opcode::ReadConstBuffer {
                    if current.borrow().buffer_info().sharp_source() == 0 {
                        all_readconsts.push(current.clone());
                        let offset = current.borrow().arg(1);
                        if !offset.is_immediate() {
                            let arg_inst = offset.inst();
                            if !visited
                                .iter()
                                .any(|candidate| Rc::ptr_eq(candidate, &arg_inst))
                            {
                                visited.push(arg_inst.clone());
                                queue.push_back(arg_inst);
                            }
                        }
                    }
                    continue;
                }
                if matches!(current_opcode, Opcode::GetUserData | Opcode::ReadConst)
                    || !is_allowed_offset_instruction(&current)
                {
                    continue;
                }
                let num_args = current.borrow().num_args();
                for arg_index in (0..num_args).rev() {
                    let arg = current.borrow().arg(arg_index);
                    if arg.is_immediate() {
                        continue;
                    }
                    let arg_inst = arg.inst();
                    if !visited
                        .iter()
                        .any(|candidate| Rc::ptr_eq(candidate, &arg_inst))
                    {
                        visited.push(arg_inst.clone());
                        queue.push_back(arg_inst);
                    }
                }
            }
        }
    }

    for inst in &all_readconsts {
        if !Rc::ptr_eq(&pass_info.deduplicate_instruction(inst), inst) {
            continue;
        }
        let base = inst.borrow().arg(0).inst();
        if let Some(inner) = base.borrow().arg(0).try_inst() {
            if inner.borrow().get_opcode() == Opcode::ReadFirstLane {
                continue;
            }
        }
        let base_low = base.borrow().arg(0);
        let base_high = base.borrow().arg(1);
        assert!(
            is_read_const_source(&base_low),
            "ReadConst base low not from constant memory"
        );
        assert!(
            is_read_const_source(&base_high),
            "ReadConst base high not from constant memory"
        );

        let ptr_lo = pass_info.deduplicate_instruction(&base_low.inst());
        let offset = inst.borrow().arg(1);
        {
            let user_list = pass_info.pointer_uses.entry(key(&ptr_lo)).or_default();
            user_list.insert(offset, inst.clone());
        }
        if ptr_lo.borrow().get_opcode() == Opcode::GetUserData {
            let ud_reg = ptr_lo.borrow().arg(0).scalar_reg() as u32;
            pass_info.srt_roots.insert(ud_reg, ptr_lo.clone());
        }
    }

    let info_rc = program.info.clone();
    let mut info = info_rc.borrow_mut();
    generate_srt_program(&mut info, &mut pass_info);

    for readconst in &all_readconsts {
        let number = pass_info.gvn_table.get_value_number_inst(readconst);
        assert!(
            pass_info.vn_to_inst.contains_key(&number),
            "the readconst has not been deduplicated"
        );
        let original = pass_info.deduplicate_instruction(readconst);
        let offset = flatbuf_offset(&original);
        set_flatbuf_offset(readconst, offset);
    }
    info.refresh_flat_buf();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::shader_recompiler::ir::basic_block::{append_new_inst, Block};
    use crate::gpu::shader_recompiler::ir::reg::ScalarReg;
    use crate::gpu::shader_recompiler::params::ShaderParams;
    use crate::gpu::shader_recompiler::runtime_info::{HwStage, SwStage};
    use std::cell::RefCell;

    fn program(block: &BlockRef) -> Program {
        let info = Rc::new(RefCell::new(Info::new(
            HwStage::Fragment,
            SwStage::Fragment,
            ShaderParams::new([0; 16], &[], 0),
        )));
        let mut program = Program::new(info);
        program.post_order_blocks = vec![block.clone()];
        program
    }

    fn append(block: &BlockRef, op: Opcode, args: &[Value]) -> InstRef {
        let index = append_new_inst(block, op, args);
        block.borrow().instructions[index].clone()
    }

    fn user_data(block: &BlockRef, reg: ScalarReg) -> InstRef {
        append(block, Opcode::GetUserData, &[Value::from_scalar_reg(reg)])
    }

    fn address(block: &BlockRef, lo: &InstRef, hi: &InstRef) -> InstRef {
        append(
            block,
            Opcode::CompositeConstructU32x2,
            &[Value::from_inst(lo.clone()), Value::from_inst(hi.clone())],
        )
    }

    fn read_const(block: &BlockRef, base: &InstRef, offset: Value) -> InstRef {
        append(
            block,
            Opcode::ReadConst,
            &[Value::from_inst(base.clone()), offset],
        )
    }

    #[test]
    fn an_address_add_is_folded_into_the_constant_offset() {
        let block = Block::new_ref();
        let ud = user_data(&block, ScalarReg::S2);
        let ud_hi = user_data(&block, ScalarReg::S3);
        let lo = append(
            &block,
            Opcode::IAdd32,
            &[Value::from_inst(ud.clone()), Value::from_u32(0x40)],
        );
        let cond = append(&block, Opcode::GetScc, &[]);
        let select = append(
            &block,
            Opcode::SelectU32,
            &[
                Value::from_inst(cond),
                Value::from_u32(1),
                Value::from_u32(0),
            ],
        );
        let hi = append(
            &block,
            Opcode::IAdd32,
            &[Value::from_inst(ud_hi.clone()), Value::from_inst(select)],
        );
        let base = address(&block, &lo, &hi);
        let read = read_const(&block, &base, Value::from_u32(0));

        simplify_read_const_address_add(&read);

        assert!(Rc::ptr_eq(&base.borrow().arg(0).inst(), &ud));
        assert!(Rc::ptr_eq(&base.borrow().arg(1).inst(), &ud_hi));
        assert_eq!(read.borrow().arg(1).u32(), 0x10);
    }

    #[test]
    fn a_read_const_gets_the_next_flattened_offset() {
        let block = Block::new_ref();
        let ud = user_data(&block, ScalarReg::S0);
        let ud_hi = user_data(&block, ScalarReg::S1);
        let base = address(&block, &ud, &ud_hi);
        let read = read_const(&block, &base, Value::from_u32(0x10));

        let mut program = program(&block);
        flatten_extended_userdata_pass(&mut program);

        assert_eq!(read.borrow().flags() as u16, 16);
        assert_eq!(program.info.borrow().srt_info.flattened_bufsize_dw, 17);
        assert_eq!(program.info.borrow().flattened_ud_buf.len(), 17);
    }

    #[test]
    fn the_offsets_are_assigned_in_offset_order() {
        let block = Block::new_ref();
        let ud = user_data(&block, ScalarReg::S0);
        let ud_hi = user_data(&block, ScalarReg::S1);
        let base = address(&block, &ud, &ud_hi);
        let first = read_const(&block, &base, Value::from_u32(0x10));
        let second = read_const(&block, &base, Value::from_u32(0x20));

        let mut program = program(&block);
        flatten_extended_userdata_pass(&mut program);

        assert_eq!(first.borrow().flags() as u16, 16);
        assert_eq!(second.borrow().flags() as u16, 17);
        assert_eq!(program.info.borrow().srt_info.flattened_bufsize_dw, 18);
    }

    #[test]
    fn a_nested_pointer_is_assigned_after_its_parent() {
        let block = Block::new_ref();
        let ud = user_data(&block, ScalarReg::S0);
        let ud_hi = user_data(&block, ScalarReg::S1);
        let base = address(&block, &ud, &ud_hi);
        let parent = read_const(&block, &base, Value::from_u32(0x10));

        let next_hi = user_data(&block, ScalarReg::S2);
        let nested_base = address(&block, &parent, &next_hi);
        let child = read_const(&block, &nested_base, Value::from_u32(0x20));

        let mut program = program(&block);
        flatten_extended_userdata_pass(&mut program);

        assert_eq!(parent.borrow().flags() as u16, 16);
        assert_eq!(child.borrow().flags() as u16, 17);
        assert_eq!(program.info.borrow().srt_info.flattened_bufsize_dw, 18);
    }
}
