//! `Shader::IR::IREmitter`, ported from `legacy/gpu/shadps4/shader_recompiler/ir/ir_emitter.h`
//! and `ir_emitter.cpp`.
//!
//! Every method appends one instruction before the insertion point and hands back its result.
//! The C++ spells the result type as the template argument `Inst<T>`; here the helpers are
//! generic in the value type they return (`inst1`, `inst2`, ... plus the `Flags` forms with an
//! `f` suffix), which reads the same at the call sites. Only the five `BitCast` combinations the
//! C++ defines exist, so they are named methods rather than overloads. The argument counts
//! are the C++ ones.
#![allow(clippy::too_many_arguments)]

use crate::gpu::amdgpu::pixel_format::NumberFormat;

use super::attribute::Attribute;
use super::basic_block::{prepend_new_inst, BlockRef};
use super::condition::Condition;
use super::debug_print::{DebugPrintFlags, DEBUGPRINT_NUM_FORMAT_ARGS};
use super::microinstruction::InstRef;
use super::opcodes::Opcode;
use super::patch::Patch;
use super::r#type::Type;
use super::reg::{
    BufferInstInfo, ScalarReg, TextureInstInfo, VectorReg, VirtualReg, NUM_SCALAR_REGS,
    NUM_VECTOR_REGS,
};
use super::value::{
    TypedValue, Value, F16, F16F32F64, F32, F32F64, F64, U1, U16, U32, U32F32, U32U64, U64, U8,
    U8U16U32U64,
};

/// The C++'s `T{Value{&*it}}`: a value that wraps a freshly made instruction.
pub trait FromInst {
    fn from_inst(inst: InstRef) -> Self;
}

impl FromInst for Value {
    fn from_inst(inst: InstRef) -> Self {
        Value::from_inst(inst)
    }
}

impl<const MASK: u32> FromInst for TypedValue<MASK> {
    fn from_inst(inst: InstRef) -> Self {
        TypedValue::from_inst(inst)
    }
}

/// `[[noreturn]] void ThrowInvalidType(Type, source_location)`.
#[track_caller]
fn throw_invalid_type(ty: Type) -> ! {
    let location = std::panic::Location::caller();
    panic!(
        "Invalid type = {}, functionName = {}, line = {}",
        ty.bits(),
        location.file(),
        location.line()
    );
}

pub struct IrEmitter {
    block: BlockRef,
    /// `Block::iterator`: `None` is the C++'s `end()`.
    insertion_point: Option<usize>,
}

impl IrEmitter {
    /// `IREmitter(Block&)`: inserts at the end.
    pub fn new(block: &BlockRef) -> Self {
        IrEmitter {
            block: block.clone(),
            insertion_point: None,
        }
    }

    /// `IREmitter(Block&, Block::iterator)`.
    pub fn at(block: &BlockRef, insertion_point: usize) -> Self {
        IrEmitter {
            block: block.clone(),
            insertion_point: Some(insertion_point),
        }
    }

    /// `Block::PrependNewInst`. The C++'s iterator keeps pointing at the instruction it pointed
    /// at before the insert, so the port's index steps over the one just written.
    fn make_inst(&mut self, op: Opcode, flags: u64, args: &[Value]) -> InstRef {
        let at = match self.insertion_point {
            None => self.block.borrow().instructions.len(),
            Some(point) => {
                self.insertion_point = Some(point + 1);
                point
            }
        };
        let position = prepend_new_inst(&self.block, at, op, args, flags);
        self.block.borrow().instructions[position].clone()
    }

    fn emit0(&mut self, op: Opcode) {
        self.make_inst(op, 0, &[]);
    }

    fn emit1(&mut self, op: Opcode, a: impl Into<Value>) {
        self.make_inst(op, 0, &[a.into()]);
    }

    fn emit2<A: Into<Value>, B: Into<Value>>(&mut self, op: Opcode, a: A, b: B) {
        self.make_inst(op, 0, &[a.into(), b.into()]);
    }

    fn emit3<A: Into<Value>, B: Into<Value>, C: Into<Value>>(
        &mut self,
        op: Opcode,
        a: A,
        b: B,
        c: C,
    ) {
        self.make_inst(op, 0, &[a.into(), b.into(), c.into()]);
    }

    fn inst0<T: FromInst>(&mut self, op: Opcode) -> T {
        T::from_inst(self.make_inst(op, 0, &[]))
    }

    fn inst1<T: FromInst>(&mut self, op: Opcode, a: impl Into<Value>) -> T {
        T::from_inst(self.make_inst(op, 0, &[a.into()]))
    }

    fn inst2<T: FromInst, A: Into<Value>, B: Into<Value>>(&mut self, op: Opcode, a: A, b: B) -> T {
        T::from_inst(self.make_inst(op, 0, &[a.into(), b.into()]))
    }

    fn inst3<T: FromInst, A: Into<Value>, B: Into<Value>, C: Into<Value>>(
        &mut self,
        op: Opcode,
        a: A,
        b: B,
        c: C,
    ) -> T {
        T::from_inst(self.make_inst(op, 0, &[a.into(), b.into(), c.into()]))
    }

    fn inst4<T: FromInst, A: Into<Value>, B: Into<Value>, C: Into<Value>, D: Into<Value>>(
        &mut self,
        op: Opcode,
        a: A,
        b: B,
        c: C,
        d: D,
    ) -> T {
        T::from_inst(self.make_inst(op, 0, &[a.into(), b.into(), c.into(), d.into()]))
    }

    fn inst5<
        T: FromInst,
        A: Into<Value>,
        B: Into<Value>,
        C: Into<Value>,
        D: Into<Value>,
        E: Into<Value>,
    >(
        &mut self,
        op: Opcode,
        a: A,
        b: B,
        c: C,
        d: D,
        e: E,
    ) -> T {
        T::from_inst(self.make_inst(op, 0, &[a.into(), b.into(), c.into(), d.into(), e.into()]))
    }

    fn inst6<
        T: FromInst,
        A: Into<Value>,
        B: Into<Value>,
        C: Into<Value>,
        D: Into<Value>,
        E: Into<Value>,
        F: Into<Value>,
    >(
        &mut self,
        op: Opcode,
        a: A,
        b: B,
        c: C,
        d: D,
        e: E,
        f: F,
    ) -> T {
        T::from_inst(self.make_inst(
            op,
            0,
            &[a.into(), b.into(), c.into(), d.into(), e.into(), f.into()],
        ))
    }

    fn emit2f<A: Into<Value>, B: Into<Value>>(&mut self, op: Opcode, flags: u64, a: A, b: B) {
        self.make_inst(op, flags, &[a.into(), b.into()]);
    }

    fn emit3f<A: Into<Value>, B: Into<Value>, C: Into<Value>>(
        &mut self,
        op: Opcode,
        flags: u64,
        a: A,
        b: B,
        c: C,
    ) {
        self.make_inst(op, flags, &[a.into(), b.into(), c.into()]);
    }

    fn inst1f<T: FromInst>(&mut self, op: Opcode, flags: u64, a: impl Into<Value>) -> T {
        T::from_inst(self.make_inst(op, flags, &[a.into()]))
    }

    fn inst2f<T: FromInst, A: Into<Value>, B: Into<Value>>(
        &mut self,
        op: Opcode,
        flags: u64,
        a: A,
        b: B,
    ) -> T {
        T::from_inst(self.make_inst(op, flags, &[a.into(), b.into()]))
    }

    fn inst3f<T: FromInst, A: Into<Value>, B: Into<Value>, C: Into<Value>>(
        &mut self,
        op: Opcode,
        flags: u64,
        a: A,
        b: B,
        c: C,
    ) -> T {
        T::from_inst(self.make_inst(op, flags, &[a.into(), b.into(), c.into()]))
    }

    fn inst4f<T: FromInst, A: Into<Value>, B: Into<Value>, C: Into<Value>, D: Into<Value>>(
        &mut self,
        op: Opcode,
        flags: u64,
        a: A,
        b: B,
        c: C,
        d: D,
    ) -> T {
        T::from_inst(self.make_inst(op, flags, &[a.into(), b.into(), c.into(), d.into()]))
    }

    // Immediates.

    pub fn imm1(&self, value: bool) -> U1 {
        U1::new(Value::from_u1(value))
    }

    pub fn imm8(&self, value: u8) -> U8 {
        U8::new(Value::from_u8(value))
    }

    pub fn imm16(&self, value: u16) -> U16 {
        U16::new(Value::from_u16(value))
    }

    pub fn imm32(&self, value: u32) -> U32 {
        U32::new(Value::from_u32(value))
    }

    pub fn imm32_signed(&self, value: i32) -> U32 {
        U32::new(Value::from_u32(value as u32))
    }

    pub fn imm32_float(&self, value: f32) -> F32 {
        F32::new(Value::from_f32(value))
    }

    pub fn imm64(&self, value: u64) -> U64 {
        U64::new(Value::from_u64(value))
    }

    pub fn imm64_signed(&self, value: i64) -> U64 {
        U64::new(Value::from_u64(value as u64))
    }

    pub fn imm64_float(&self, value: f64) -> F64 {
        F64::new(Value::from_f64(value))
    }

    // The five `BitCast` specializations the C++ defines, and no others.

    pub fn bit_cast_u32_u1(&mut self, value: &U1) -> U32 {
        let one = self.imm32(1).into_value();
        let zero = self.imm32(0).into_value();
        U32::new(self.select(value, &one, &zero))
    }

    pub fn bit_cast_u32_f32(&mut self, value: &F32) -> U32 {
        self.inst1(Opcode::BitCastU32F32, value)
    }

    pub fn bit_cast_f32_u32(&mut self, value: &U32) -> F32 {
        self.inst1(Opcode::BitCastF32U32, value)
    }

    pub fn bit_cast_u16_f16(&mut self, value: &F16) -> U16 {
        self.inst1(Opcode::BitCastU16F16, value)
    }

    pub fn bit_cast_f16_u16(&mut self, value: &U16) -> F16 {
        self.inst1(Opcode::BitCastF16U16, value)
    }

    pub fn condition_ref(&mut self, value: &U1) -> U1 {
        self.inst1(Opcode::ConditionRef, value)
    }

    pub fn prologue(&mut self) {
        self.emit0(Opcode::Prologue);
    }

    pub fn epilogue(&mut self) {
        self.emit0(Opcode::Epilogue);
    }

    pub fn discard(&mut self) {
        self.emit0(Opcode::Discard);
    }

    pub fn discard_cond(&mut self, cond: &U1) {
        self.emit1(Opcode::DiscardCond, cond);
    }

    pub fn barrier(&mut self) {
        self.emit0(Opcode::Barrier);
    }

    pub fn workgroup_memory_barrier(&mut self) {
        self.emit0(Opcode::WorkgroupMemoryBarrier);
    }

    pub fn device_memory_barrier(&mut self) {
        self.emit0(Opcode::DeviceMemoryBarrier);
    }

    fn emit5f<A: Into<Value>, B: Into<Value>, C: Into<Value>, D: Into<Value>, E: Into<Value>>(
        &mut self,
        op: Opcode,
        flags: u64,
        a: A,
        b: B,
        c: C,
        d: D,
        e: E,
    ) {
        self.make_inst(
            op,
            flags,
            &[a.into(), b.into(), c.into(), d.into(), e.into()],
        );
    }

    fn inst5f<
        T: FromInst,
        A: Into<Value>,
        B: Into<Value>,
        C: Into<Value>,
        D: Into<Value>,
        E: Into<Value>,
    >(
        &mut self,
        op: Opcode,
        flags: u64,
        a: A,
        b: B,
        c: C,
        d: D,
        e: E,
    ) -> T {
        T::from_inst(self.make_inst(
            op,
            flags,
            &[a.into(), b.into(), c.into(), d.into(), e.into()],
        ))
    }

    fn inst6f<
        T: FromInst,
        A: Into<Value>,
        B: Into<Value>,
        C: Into<Value>,
        D: Into<Value>,
        E: Into<Value>,
        F: Into<Value>,
    >(
        &mut self,
        op: Opcode,
        flags: u64,
        a: A,
        b: B,
        c: C,
        d: D,
        e: E,
        f: F,
    ) -> T {
        T::from_inst(self.make_inst(
            op,
            flags,
            &[a.into(), b.into(), c.into(), d.into(), e.into(), f.into()],
        ))
    }

    pub fn get_user_data(&mut self, reg: ScalarReg) -> U32 {
        assert!((reg as usize) < NUM_SCALAR_REGS);
        self.inst1(Opcode::GetUserData, Value::from_scalar_reg(reg))
    }

    pub fn get_scalar_reg(&mut self, reg: ScalarReg) -> U32 {
        assert!((reg as usize) < NUM_SCALAR_REGS);
        self.inst1(Opcode::GetScalarRegister, Value::from_scalar_reg(reg))
    }

    pub fn get_scalar_reg_f32(&mut self, reg: ScalarReg) -> F32 {
        let value = self.get_scalar_reg(reg);
        self.bit_cast_f32_u32(&value)
    }

    pub fn get_vector_reg(&mut self, reg: VectorReg) -> U32 {
        assert!((reg as usize) < NUM_VECTOR_REGS);
        self.inst1(Opcode::GetVectorRegister, Value::from_vector_reg(reg))
    }

    pub fn get_vector_reg_f32(&mut self, reg: VectorReg) -> F32 {
        let value = self.get_vector_reg(reg);
        self.bit_cast_f32_u32(&value)
    }

    fn typed_reg_value(&mut self, value: &U32F32) -> Value {
        if value.ty() == Type::F32 {
            let value = F32::new(value.value().clone());
            self.bit_cast_u32_f32(&value).into_value()
        } else {
            U32::new(value.value().clone()).into_value()
        }
    }

    pub fn set_scalar_reg(&mut self, reg: ScalarReg, value: &U32F32) {
        assert!((reg as usize) < NUM_SCALAR_REGS);
        let value_typed = self.typed_reg_value(value);
        self.emit2(
            Opcode::SetScalarRegister,
            Value::from_scalar_reg(reg),
            value_typed,
        );
    }

    pub fn set_vector_reg(&mut self, reg: VectorReg, value: &U32F32) {
        assert!((reg as usize) < NUM_VECTOR_REGS);
        let value_typed = self.typed_reg_value(value);
        self.emit2(
            Opcode::SetVectorRegister,
            Value::from_vector_reg(reg),
            value_typed,
        );
    }

    pub fn get_virtual_reg(&mut self, reg: VirtualReg) -> Value {
        self.inst1(Opcode::GetVirtualRegister, Value::from_virtual_reg(reg))
    }

    pub fn set_virtual_reg(&mut self, reg: VirtualReg, value: &Value) {
        self.emit2(
            Opcode::SetVirtualRegister,
            Value::from_virtual_reg(reg),
            value.clone(),
        );
    }

    pub fn get_goto_variable(&mut self, id: u32) -> U1 {
        let id = self.imm32(id).into_value();
        self.inst1(Opcode::GetGotoVariable, id)
    }

    pub fn set_goto_variable(&mut self, id: u32, value: &U1) {
        let id = self.imm32(id).into_value();
        self.emit2(Opcode::SetGotoVariable, id, value.value().clone());
    }

    fn vcc_ballot(&mut self) -> U64 {
        let lo = self.get_vcc_lo();
        let hi = self.get_vcc_hi();
        let vector = self.composite_construct(lo, hi);
        self.pack_uint_2x32(&vector)
    }

    pub fn condition(&mut self, cond: Condition) -> U1 {
        match cond {
            Condition::False => self.imm1(false),
            Condition::True => self.imm1(true),
            Condition::Scc0 => {
                let scc = self.get_scc();
                self.logical_not(&scc)
            }
            Condition::Scc1 => self.get_scc(),
            Condition::Vccz => {
                let ballot = self.vcc_ballot();
                let ballot = self.inverse_ballot(&ballot);
                self.logical_not(&ballot)
            }
            Condition::Vccnz => {
                let ballot = self.vcc_ballot();
                self.inverse_ballot(&ballot)
            }
            Condition::Execz => {
                let exec = self.get_exec();
                self.logical_not(&exec)
            }
            Condition::Execnz => self.get_exec(),
        }
    }

    pub fn get_scc(&mut self) -> U1 {
        self.inst0(Opcode::GetScc)
    }

    pub fn get_exec(&mut self) -> U1 {
        self.inst0(Opcode::GetExec)
    }

    pub fn get_vcc_lo(&mut self) -> U32 {
        self.inst0(Opcode::GetVccLo)
    }

    pub fn get_vcc_hi(&mut self) -> U32 {
        self.inst0(Opcode::GetVccHi)
    }

    pub fn get_m0(&mut self) -> U32 {
        self.inst0(Opcode::GetM0)
    }

    pub fn set_scc(&mut self, value: &U1) {
        self.emit1(Opcode::SetScc, value.value().clone());
    }

    pub fn set_exec(&mut self, value: &U1) {
        self.emit1(Opcode::SetExec, value.value().clone());
    }

    pub fn set_vcc_lo(&mut self, value: &U32) {
        self.emit1(Opcode::SetVccLo, value.value().clone());
    }

    pub fn set_vcc_hi(&mut self, value: &U32) {
        self.emit1(Opcode::SetVccHi, value.value().clone());
    }

    pub fn set_m0(&mut self, value: &U32) {
        self.emit1(Opcode::SetM0, value.value().clone());
    }

    pub fn get_attribute(&mut self, attribute: Attribute, comp: u32, index: u32) -> F32 {
        let comp = self.imm32(comp).into_value();
        let index = self.imm32(index).into_value();
        self.inst3(
            Opcode::GetAttribute,
            Value::from_attribute(attribute),
            comp,
            index,
        )
    }

    pub fn get_attribute_u1(&mut self, attribute: Attribute, comp: u32) -> U1 {
        let comp = self.imm32(comp).into_value();
        self.inst2(
            Opcode::GetAttributeU1,
            Value::from_attribute(attribute),
            comp,
        )
    }

    pub fn get_attribute_u32(&mut self, attribute: Attribute, comp: u32) -> U32 {
        let comp = self.imm32(comp).into_value();
        self.inst2(
            Opcode::GetAttributeU32,
            Value::from_attribute(attribute),
            comp,
        )
    }

    pub fn set_attribute(&mut self, attribute: Attribute, value: &F32, comp: u32) {
        let comp = self.imm32(comp).into_value();
        self.emit3(
            Opcode::SetAttribute,
            Value::from_attribute(attribute),
            value.value().clone(),
            comp,
        );
    }

    pub fn get_tess_generic_attribute(
        &mut self,
        vertex_index: &U32,
        attr_index: &U32,
        comp_index: &U32,
    ) -> F32 {
        self.inst3(
            Opcode::GetTessGenericAttribute,
            vertex_index.value().clone(),
            attr_index.value().clone(),
            comp_index.value().clone(),
        )
    }

    pub fn set_tcs_generic_attribute(&mut self, value: &F32, attr_index: &U32, comp_index: &U32) {
        self.emit3(
            Opcode::SetTcsGenericAttribute,
            value.value().clone(),
            attr_index.value().clone(),
            comp_index.value().clone(),
        );
    }

    pub fn read_tcs_generic_output_attribute(
        &mut self,
        vertex_index: &U32,
        attr_index: &U32,
        comp_index: &U32,
    ) -> F32 {
        self.inst3(
            Opcode::ReadTcsGenericOuputAttribute,
            vertex_index.value().clone(),
            attr_index.value().clone(),
            comp_index.value().clone(),
        )
    }

    pub fn get_pc_lo(&mut self, pc: &U32) -> U32 {
        self.inst1(Opcode::GetPcLo, pc.value().clone())
    }

    pub fn get_patch(&mut self, patch: Patch) -> F32 {
        self.inst1(Opcode::GetPatch, Value::from_patch(patch))
    }

    pub fn set_patch(&mut self, patch: Patch, value: &F32) {
        self.emit2(
            Opcode::SetPatch,
            Value::from_patch(patch),
            value.value().clone(),
        );
    }

    pub fn load_shared(
        &mut self,
        bit_size: i32,
        is_signed: bool,
        offset: &U32,
        is_gds: bool,
    ) -> Value {
        let _ = is_signed;
        let offset = offset.value().clone();
        match bit_size {
            16 => self.inst1f(Opcode::LoadSharedU16, u64::from(is_gds), offset),
            32 => self.inst1f(Opcode::LoadSharedU32, u64::from(is_gds), offset),
            64 => self.inst1f(Opcode::LoadSharedU64, u64::from(is_gds), offset),
            _ => unreachable!("Invalid bit size {}", bit_size),
        }
    }

    pub fn write_shared(&mut self, bit_size: i32, value: &Value, offset: &U32, is_gds: bool) {
        let args = (offset.value().clone(), value.clone());
        match bit_size {
            16 => self.emit2f(Opcode::WriteSharedU16, u64::from(is_gds), args.0, args.1),
            32 => self.emit2f(Opcode::WriteSharedU32, u64::from(is_gds), args.0, args.1),
            64 => self.emit2f(Opcode::WriteSharedU64, u64::from(is_gds), args.0, args.1),
            _ => unreachable!("Invalid bit size {}", bit_size),
        }
    }

    fn shared_atomic3(
        &mut self,
        op32: Opcode,
        op64: Opcode,
        address: &U32,
        data: &U32U64,
        is_gds: bool,
    ) -> U32U64 {
        match data.ty() {
            Type::U32 => self.inst2f(
                op32,
                u64::from(is_gds),
                address.value().clone(),
                data.value().clone(),
            ),
            Type::U64 => self.inst2f(
                op64,
                u64::from(is_gds),
                address.value().clone(),
                data.value().clone(),
            ),
            _ => throw_invalid_type(data.ty()),
        }
    }

    pub fn shared_atomic_i_add(&mut self, address: &U32, data: &U32U64, is_gds: bool) -> U32U64 {
        self.shared_atomic3(
            Opcode::SharedAtomicIAdd32,
            Opcode::SharedAtomicIAdd64,
            address,
            data,
            is_gds,
        )
    }

    pub fn shared_atomic_i_sub(&mut self, address: &U32, data: &U32U64, is_gds: bool) -> U32U64 {
        self.shared_atomic3(
            Opcode::SharedAtomicISub32,
            Opcode::SharedAtomicISub64,
            address,
            data,
            is_gds,
        )
    }

    pub fn shared_atomic_i_min(
        &mut self,
        address: &U32,
        data: &U32U64,
        is_signed: bool,
        is_gds: bool,
    ) -> U32U64 {
        self.shared_atomic3(
            if is_signed {
                Opcode::SharedAtomicSMin32
            } else {
                Opcode::SharedAtomicUMin32
            },
            if is_signed {
                Opcode::SharedAtomicSMin64
            } else {
                Opcode::SharedAtomicUMin64
            },
            address,
            data,
            is_gds,
        )
    }

    pub fn shared_atomic_i_max(
        &mut self,
        address: &U32,
        data: &U32U64,
        is_signed: bool,
        is_gds: bool,
    ) -> U32U64 {
        self.shared_atomic3(
            if is_signed {
                Opcode::SharedAtomicSMax32
            } else {
                Opcode::SharedAtomicUMax32
            },
            if is_signed {
                Opcode::SharedAtomicSMax64
            } else {
                Opcode::SharedAtomicUMax64
            },
            address,
            data,
            is_gds,
        )
    }

    pub fn shared_atomic_and(&mut self, address: &U32, data: &U32U64, is_gds: bool) -> U32U64 {
        self.shared_atomic3(
            Opcode::SharedAtomicAnd32,
            Opcode::SharedAtomicAnd64,
            address,
            data,
            is_gds,
        )
    }

    pub fn shared_atomic_or(&mut self, address: &U32, data: &U32U64, is_gds: bool) -> U32U64 {
        self.shared_atomic3(
            Opcode::SharedAtomicOr32,
            Opcode::SharedAtomicOr64,
            address,
            data,
            is_gds,
        )
    }

    pub fn shared_atomic_xor(&mut self, address: &U32, data: &U32U64, is_gds: bool) -> U32U64 {
        self.shared_atomic3(
            Opcode::SharedAtomicXor32,
            Opcode::SharedAtomicXor64,
            address,
            data,
            is_gds,
        )
    }

    pub fn shared_atomic_cmp_swap(
        &mut self,
        address: &U32,
        value: &U32U64,
        cmp_value: &U32U64,
        is_gds: bool,
    ) -> U32U64 {
        let args = (
            address.value().clone(),
            value.value().clone(),
            cmp_value.value().clone(),
        );
        match value.ty() {
            Type::U32 => self.inst3f(
                Opcode::SharedAtomicCmpSwap32,
                u64::from(is_gds),
                args.0,
                args.1,
                args.2,
            ),
            Type::U64 => self.inst3f(
                Opcode::SharedAtomicCmpSwap64,
                u64::from(is_gds),
                args.0,
                args.1,
                args.2,
            ),
            _ => throw_invalid_type(value.ty()),
        }
    }

    pub fn shared_atomic_inc(&mut self, address: &U32, is_gds: bool) -> U32 {
        self.inst1f(
            Opcode::SharedAtomicInc32,
            u64::from(is_gds),
            address.value().clone(),
        )
    }

    pub fn shared_atomic_inc64(&mut self, address: &U32, is_gds: bool) -> U64 {
        self.inst1f(
            Opcode::SharedAtomicInc64,
            u64::from(is_gds),
            address.value().clone(),
        )
    }

    pub fn shared_atomic_dec(&mut self, address: &U32, is_gds: bool) -> U32 {
        self.inst1f(
            Opcode::SharedAtomicDec32,
            u64::from(is_gds),
            address.value().clone(),
        )
    }

    pub fn shared_atomic_dec64(&mut self, address: &U32, is_gds: bool) -> U64 {
        self.inst1f(
            Opcode::SharedAtomicDec64,
            u64::from(is_gds),
            address.value().clone(),
        )
    }

    pub fn read_const(&mut self, base: &Value, offset: &U32) -> U32 {
        self.inst2(Opcode::ReadConst, base.clone(), offset.value().clone())
    }

    pub fn read_const_buffer(&mut self, handle: &Value, index: &U32, info: BufferInstInfo) -> U32 {
        self.inst2f(
            Opcode::ReadConstBuffer,
            info.raw(),
            handle.clone(),
            index.value().clone(),
        )
    }

    pub fn load_buffer_u8(&mut self, handle: &Value, address: &Value, info: BufferInstInfo) -> U8 {
        self.inst2f(
            Opcode::LoadBufferU8,
            info.raw(),
            handle.clone(),
            address.clone(),
        )
    }

    pub fn load_buffer_u16(
        &mut self,
        handle: &Value,
        address: &Value,
        info: BufferInstInfo,
    ) -> U16 {
        self.inst2f(
            Opcode::LoadBufferU16,
            info.raw(),
            handle.clone(),
            address.clone(),
        )
    }

    pub fn load_buffer_u32(
        &mut self,
        num_dwords: i32,
        handle: &Value,
        address: &Value,
        info: BufferInstInfo,
    ) -> Value {
        let args = (handle.clone(), address.clone());
        match num_dwords {
            1 => self.inst2f(Opcode::LoadBufferU32, info.raw(), args.0, args.1),
            2 => self.inst2f(Opcode::LoadBufferU32x2, info.raw(), args.0, args.1),
            3 => self.inst2f(Opcode::LoadBufferU32x3, info.raw(), args.0, args.1),
            4 => self.inst2f(Opcode::LoadBufferU32x4, info.raw(), args.0, args.1),
            _ => unreachable!("Invalid number of dwords {}", num_dwords),
        }
    }

    pub fn load_buffer_u64(
        &mut self,
        handle: &Value,
        address: &Value,
        info: BufferInstInfo,
    ) -> U64 {
        self.inst2f(
            Opcode::LoadBufferU64,
            info.raw(),
            handle.clone(),
            address.clone(),
        )
    }

    pub fn load_buffer_f32(
        &mut self,
        num_dwords: i32,
        handle: &Value,
        address: &Value,
        info: BufferInstInfo,
    ) -> Value {
        let args = (handle.clone(), address.clone());
        match num_dwords {
            1 => self.inst2f(Opcode::LoadBufferF32, info.raw(), args.0, args.1),
            2 => self.inst2f(Opcode::LoadBufferF32x2, info.raw(), args.0, args.1),
            3 => self.inst2f(Opcode::LoadBufferF32x3, info.raw(), args.0, args.1),
            4 => self.inst2f(Opcode::LoadBufferF32x4, info.raw(), args.0, args.1),
            _ => unreachable!("Invalid number of dwords {}", num_dwords),
        }
    }

    pub fn load_buffer_format(
        &mut self,
        handle: &Value,
        address: &Value,
        info: BufferInstInfo,
    ) -> Value {
        self.inst2f(
            Opcode::LoadBufferFormatF32,
            info.raw(),
            handle.clone(),
            address.clone(),
        )
    }

    pub fn store_buffer_u8(
        &mut self,
        handle: &Value,
        address: &Value,
        data: &U8,
        info: BufferInstInfo,
    ) {
        let args = (handle.clone(), address.clone(), data.value().clone());
        self.emit3f(Opcode::StoreBufferU8, info.raw(), args.0, args.1, args.2);
    }

    pub fn store_buffer_u16(
        &mut self,
        handle: &Value,
        address: &Value,
        data: &U16,
        info: BufferInstInfo,
    ) {
        let args = (handle.clone(), address.clone(), data.value().clone());
        self.emit3f(Opcode::StoreBufferU16, info.raw(), args.0, args.1, args.2);
    }

    pub fn store_buffer_u32(
        &mut self,
        num_dwords: i32,
        handle: &Value,
        address: &Value,
        data: &Value,
        info: BufferInstInfo,
    ) {
        let args = (handle.clone(), address.clone(), data.clone());
        match num_dwords {
            1 => self.emit3f(Opcode::StoreBufferU32, info.raw(), args.0, args.1, args.2),
            2 => self.emit3f(Opcode::StoreBufferU32x2, info.raw(), args.0, args.1, args.2),
            3 => self.emit3f(Opcode::StoreBufferU32x3, info.raw(), args.0, args.1, args.2),
            4 => self.emit3f(Opcode::StoreBufferU32x4, info.raw(), args.0, args.1, args.2),
            _ => unreachable!("Invalid number of dwords {}", num_dwords),
        }
    }

    pub fn store_buffer_u64(
        &mut self,
        handle: &Value,
        address: &Value,
        data: &U64,
        info: BufferInstInfo,
    ) {
        let args = (handle.clone(), address.clone(), data.value().clone());
        self.emit3f(Opcode::StoreBufferU64, info.raw(), args.0, args.1, args.2);
    }

    pub fn store_buffer_f32(
        &mut self,
        num_dwords: i32,
        handle: &Value,
        address: &Value,
        data: &Value,
        info: BufferInstInfo,
    ) {
        let args = (handle.clone(), address.clone(), data.clone());
        match num_dwords {
            1 => self.emit3f(Opcode::StoreBufferF32, info.raw(), args.0, args.1, args.2),
            2 => self.emit3f(Opcode::StoreBufferF32x2, info.raw(), args.0, args.1, args.2),
            3 => self.emit3f(Opcode::StoreBufferF32x3, info.raw(), args.0, args.1, args.2),
            4 => self.emit3f(Opcode::StoreBufferF32x4, info.raw(), args.0, args.1, args.2),
            _ => unreachable!("Invalid number of dwords {}", num_dwords),
        }
    }

    pub fn store_buffer_format(
        &mut self,
        handle: &Value,
        address: &Value,
        data: &Value,
        info: BufferInstInfo,
    ) {
        let args = (handle.clone(), address.clone(), data.clone());
        self.emit3f(
            Opcode::StoreBufferFormatF32,
            info.raw(),
            args.0,
            args.1,
            args.2,
        );
    }

    fn buffer_atomic3(
        &mut self,
        op: Opcode,
        handle: &Value,
        address: &Value,
        value: &Value,
        info: BufferInstInfo,
    ) -> Value {
        let args = (handle.clone(), address.clone(), value.clone());
        self.inst3f(op, info.raw(), args.0, args.1, args.2)
    }

    fn buffer_atomic2(
        &mut self,
        op: Opcode,
        handle: &Value,
        address: &Value,
        info: BufferInstInfo,
    ) -> Value {
        self.inst2f(op, info.raw(), handle.clone(), address.clone())
    }

    pub fn buffer_atomic_i_add(
        &mut self,
        handle: &Value,
        address: &Value,
        value: &Value,
        info: BufferInstInfo,
    ) -> Value {
        match value.ty() {
            Type::U32 => {
                self.buffer_atomic3(Opcode::BufferAtomicIAdd32, handle, address, value, info)
            }
            Type::U64 => {
                self.buffer_atomic3(Opcode::BufferAtomicIAdd64, handle, address, value, info)
            }
            _ => throw_invalid_type(value.ty()),
        }
    }

    pub fn buffer_atomic_i_sub(
        &mut self,
        handle: &Value,
        address: &Value,
        value: &Value,
        info: BufferInstInfo,
    ) -> Value {
        self.buffer_atomic3(Opcode::BufferAtomicISub32, handle, address, value, info)
    }

    pub fn buffer_atomic_i_min(
        &mut self,
        handle: &Value,
        address: &Value,
        value: &Value,
        is_signed: bool,
        info: BufferInstInfo,
    ) -> Value {
        match value.ty() {
            Type::U32 => self.buffer_atomic3(
                if is_signed {
                    Opcode::BufferAtomicSMin32
                } else {
                    Opcode::BufferAtomicUMin32
                },
                handle,
                address,
                value,
                info,
            ),
            Type::U64 => self.buffer_atomic3(
                if is_signed {
                    Opcode::BufferAtomicSMin64
                } else {
                    Opcode::BufferAtomicUMin64
                },
                handle,
                address,
                value,
                info,
            ),
            _ => throw_invalid_type(value.ty()),
        }
    }

    pub fn buffer_atomic_f_min(
        &mut self,
        handle: &Value,
        address: &Value,
        value: &Value,
        info: BufferInstInfo,
    ) -> Value {
        self.buffer_atomic3(Opcode::BufferAtomicFMin32, handle, address, value, info)
    }

    pub fn buffer_atomic_i_max(
        &mut self,
        handle: &Value,
        address: &Value,
        value: &Value,
        is_signed: bool,
        info: BufferInstInfo,
    ) -> Value {
        match value.ty() {
            Type::U32 => self.buffer_atomic3(
                if is_signed {
                    Opcode::BufferAtomicSMax32
                } else {
                    Opcode::BufferAtomicUMax32
                },
                handle,
                address,
                value,
                info,
            ),
            Type::U64 => self.buffer_atomic3(
                if is_signed {
                    Opcode::BufferAtomicSMax64
                } else {
                    Opcode::BufferAtomicUMax64
                },
                handle,
                address,
                value,
                info,
            ),
            _ => throw_invalid_type(value.ty()),
        }
    }

    pub fn buffer_atomic_f_max(
        &mut self,
        handle: &Value,
        address: &Value,
        value: &Value,
        info: BufferInstInfo,
    ) -> Value {
        self.buffer_atomic3(Opcode::BufferAtomicFMax32, handle, address, value, info)
    }

    pub fn buffer_atomic_inc(
        &mut self,
        handle: &Value,
        address: &Value,
        info: BufferInstInfo,
    ) -> Value {
        self.buffer_atomic2(Opcode::BufferAtomicInc32, handle, address, info)
    }

    pub fn buffer_atomic_dec(
        &mut self,
        handle: &Value,
        address: &Value,
        info: BufferInstInfo,
    ) -> Value {
        self.buffer_atomic2(Opcode::BufferAtomicDec32, handle, address, info)
    }

    pub fn buffer_atomic_and(
        &mut self,
        handle: &Value,
        address: &Value,
        value: &Value,
        info: BufferInstInfo,
    ) -> Value {
        self.buffer_atomic3(Opcode::BufferAtomicAnd32, handle, address, value, info)
    }

    pub fn buffer_atomic_or(
        &mut self,
        handle: &Value,
        address: &Value,
        value: &Value,
        info: BufferInstInfo,
    ) -> Value {
        self.buffer_atomic3(Opcode::BufferAtomicOr32, handle, address, value, info)
    }

    pub fn buffer_atomic_xor(
        &mut self,
        handle: &Value,
        address: &Value,
        value: &Value,
        info: BufferInstInfo,
    ) -> Value {
        self.buffer_atomic3(Opcode::BufferAtomicXor32, handle, address, value, info)
    }

    pub fn buffer_atomic_swap(
        &mut self,
        handle: &Value,
        address: &Value,
        value: &Value,
        info: BufferInstInfo,
    ) -> Value {
        self.buffer_atomic3(Opcode::BufferAtomicSwap32, handle, address, value, info)
    }

    pub fn buffer_atomic_cmp_swap(
        &mut self,
        handle: &Value,
        address: &Value,
        vdata: &Value,
        cmp_value: &Value,
        info: BufferInstInfo,
    ) -> Value {
        let args = (
            handle.clone(),
            address.clone(),
            vdata.clone(),
            cmp_value.clone(),
        );
        self.inst4f(
            Opcode::BufferAtomicCmpSwap32,
            info.raw(),
            args.0,
            args.1,
            args.2,
            args.3,
        )
    }

    pub fn buffer_atomic_f_cmp_swap(
        &mut self,
        handle: &Value,
        address: &Value,
        vdata: &Value,
        cmp_value: &Value,
        info: BufferInstInfo,
    ) -> Value {
        let args = (
            handle.clone(),
            address.clone(),
            vdata.clone(),
            cmp_value.clone(),
        );
        self.inst4f(
            Opcode::BufferAtomicFCmpSwap32,
            info.raw(),
            args.0,
            args.1,
            args.2,
            args.3,
        )
    }

    pub fn data_append(&mut self, gds_dw_offset: &U32) -> U32 {
        let exec = self.get_exec();
        self.inst2(
            Opcode::DataAppend,
            gds_dw_offset.value().clone(),
            exec.value().clone(),
        )
    }

    pub fn data_consume(&mut self, gds_dw_offset: &U32) -> U32 {
        let exec = self.get_exec();
        self.inst2(
            Opcode::DataConsume,
            gds_dw_offset.value().clone(),
            exec.value().clone(),
        )
    }

    pub fn lane_id(&mut self) -> U32 {
        self.inst0(Opcode::LaneId)
    }

    pub fn warp_id(&mut self) -> U32 {
        self.inst0(Opcode::WarpId)
    }

    pub fn quad_broadcast(&mut self, value: &U32, index: &U32) -> U32 {
        self.inst2(
            Opcode::QuadBroadcast,
            value.value().clone(),
            index.value().clone(),
        )
    }

    pub fn shuffle(&mut self, value: &U32, index: &U32) -> U32 {
        self.inst2(
            Opcode::Shuffle,
            value.value().clone(),
            index.value().clone(),
        )
    }

    pub fn shuffle_xor(&mut self, value: &U32, mask: &U32) -> U32 {
        self.inst2(
            Opcode::ShuffleXor,
            value.value().clone(),
            mask.value().clone(),
        )
    }

    pub fn read_first_lane(&mut self, value: &U32) -> U32 {
        self.inst1(Opcode::ReadFirstLane, value.value().clone())
    }

    pub fn read_lane(&mut self, value: &U32, lane: &U32) -> U32 {
        self.inst2(
            Opcode::ReadLane,
            value.value().clone(),
            lane.value().clone(),
        )
    }

    pub fn write_lane(&mut self, value: &U32, write_value: &U32, lane: &U32) -> U32 {
        self.inst3(
            Opcode::WriteLane,
            value.value().clone(),
            write_value.value().clone(),
            lane.value().clone(),
        )
    }

    pub fn ballot(&mut self, bit: &U1) -> U64 {
        self.inst1(Opcode::Ballot, bit.value().clone())
    }

    pub fn ballot_find_lsb(&mut self, mask: &U64) -> U32 {
        self.inst1(Opcode::BallotFindLsb, mask.value().clone())
    }

    pub fn inverse_ballot(&mut self, mask: &U64) -> U1 {
        self.inst1(Opcode::InverseBallot, mask.value().clone())
    }

    pub fn group_any(&mut self, bit: &U1) -> U1 {
        self.inst1(Opcode::GroupAny, bit.value().clone())
    }

    fn composite2(&mut self, op: Opcode, e1: &Value, e2: &Value) -> Value {
        if e1.ty() != e2.ty() {
            unreachable!("Mismatching types {} and {}", e1.ty(), e2.ty());
        }
        self.inst2(op, e1.clone(), e2.clone())
    }

    pub fn composite_construct(&mut self, e1: impl Into<Value>, e2: impl Into<Value>) -> Value {
        let e1: Value = e1.into();
        let e2: Value = e2.into();
        let op = match e1.ty() {
            Type::U32 => Opcode::CompositeConstructU32x2,
            Type::U32x2 => Opcode::CompositeConstructU32x2x2,
            Type::F32 => Opcode::CompositeConstructF32x2,
            Type::F32x2 => Opcode::CompositeConstructF32x2x2,
            _ => throw_invalid_type(e1.ty()),
        };
        self.composite2(op, &e1, &e2)
    }

    pub fn composite_construct3(
        &mut self,
        e1: impl Into<Value>,
        e2: impl Into<Value>,
        e3: impl Into<Value>,
    ) -> Value {
        let e1: Value = e1.into();
        let e2: Value = e2.into();
        let e3: Value = e3.into();
        if e1.ty() != e2.ty() || e1.ty() != e3.ty() {
            unreachable!(
                "Mismatching types {}, {}, and {}",
                e1.ty(),
                e2.ty(),
                e3.ty()
            );
        }
        let op = match e1.ty() {
            Type::U32 => Opcode::CompositeConstructU32x3,
            Type::F32 => Opcode::CompositeConstructF32x3,
            _ => throw_invalid_type(e1.ty()),
        };
        self.inst3(op, e1, e2, e3)
    }

    pub fn composite_construct4(
        &mut self,
        e1: impl Into<Value>,
        e2: impl Into<Value>,
        e3: impl Into<Value>,
        e4: impl Into<Value>,
    ) -> Value {
        let e1: Value = e1.into();
        let e2: Value = e2.into();
        let e3: Value = e3.into();
        let e4: Value = e4.into();
        if e1.ty() != e2.ty() || e1.ty() != e3.ty() || e1.ty() != e4.ty() {
            unreachable!(
                "Mismatching types {}, {}, {}, and {}",
                e1.ty(),
                e2.ty(),
                e3.ty(),
                e4.ty()
            );
        }
        let op = match e1.ty() {
            Type::U32 => Opcode::CompositeConstructU32x4,
            Type::F32 => Opcode::CompositeConstructF32x4,
            _ => throw_invalid_type(e1.ty()),
        };
        self.inst4(op, e1, e2, e3, e4)
    }

    pub fn composite_construct_slice(&mut self, elements: &[Value]) -> Value {
        match elements.len() {
            2 => self.composite_construct(elements[0].clone(), elements[1].clone()),
            3 => self.composite_construct3(
                elements[0].clone(),
                elements[1].clone(),
                elements[2].clone(),
            ),
            4 => self.composite_construct4(
                elements[0].clone(),
                elements[1].clone(),
                elements[2].clone(),
                elements[3].clone(),
            ),
            _ => unreachable!(
                "Composite construct with {} elements, only 2-4 are supported",
                elements.len()
            ),
        }
    }

    fn composite_read(
        &mut self,
        op: Opcode,
        vector: &Value,
        element: usize,
        limit: usize,
    ) -> Value {
        if element >= limit {
            unreachable!("Out of bounds element {}", element);
        }
        let element = self.imm32(element as u32).into_value();
        self.inst2(op, vector.clone(), element)
    }

    pub fn composite_extract(&mut self, vector: &Value, element: usize) -> Value {
        match vector.ty() {
            Type::U32x2 => self.composite_read(Opcode::CompositeExtractU32x2, vector, element, 2),
            Type::U32x3 => self.composite_read(Opcode::CompositeExtractU32x3, vector, element, 3),
            Type::U32x4 => self.composite_read(Opcode::CompositeExtractU32x4, vector, element, 4),
            Type::F32x2 => self.composite_read(Opcode::CompositeExtractF32x2, vector, element, 2),
            Type::F32x3 => self.composite_read(Opcode::CompositeExtractF32x3, vector, element, 3),
            Type::F32x4 => self.composite_read(Opcode::CompositeExtractF32x4, vector, element, 4),
            _ => throw_invalid_type(vector.ty()),
        }
    }

    fn composite_write(
        &mut self,
        op: Opcode,
        vector: &Value,
        object: &Value,
        element: usize,
        limit: usize,
    ) -> Value {
        if element >= limit {
            unreachable!("Out of bounds element {}", element);
        }
        let element = self.imm32(element as u32).into_value();
        self.inst3(op, vector.clone(), object.clone(), element)
    }

    pub fn composite_insert(&mut self, vector: &Value, object: &Value, element: usize) -> Value {
        match vector.ty() {
            Type::U32x2 => {
                self.composite_write(Opcode::CompositeInsertU32x2, vector, object, element, 2)
            }
            Type::U32x3 => {
                self.composite_write(Opcode::CompositeInsertU32x3, vector, object, element, 3)
            }
            Type::U32x4 => {
                self.composite_write(Opcode::CompositeInsertU32x4, vector, object, element, 4)
            }
            Type::F32x2 => {
                self.composite_write(Opcode::CompositeInsertF32x2, vector, object, element, 2)
            }
            Type::F32x3 => {
                self.composite_write(Opcode::CompositeInsertF32x3, vector, object, element, 3)
            }
            Type::F32x4 => {
                self.composite_write(Opcode::CompositeInsertF32x4, vector, object, element, 4)
            }
            _ => throw_invalid_type(vector.ty()),
        }
    }

    pub fn composite_shuffle(
        &mut self,
        vector1: &Value,
        vector2: &Value,
        comp0: usize,
        comp1: usize,
    ) -> Value {
        if vector1.ty() != vector2.ty() {
            unreachable!("Mismatching types {} and {}", vector1.ty(), vector2.ty());
        }
        if comp0 >= 4 || comp1 >= 4 {
            unreachable!("One or more out of bounds elements {}, {}", comp0, comp1);
        }
        let args = (
            vector1.clone(),
            vector2.clone(),
            self.imm32(comp0 as u32).into_value(),
            self.imm32(comp1 as u32).into_value(),
        );
        let op = match vector1.ty() {
            Type::U32x4 => Opcode::CompositeShuffleU32x2,
            Type::F32x4 => Opcode::CompositeShuffleF32x2,
            _ => throw_invalid_type(vector1.ty()),
        };
        self.inst4(op, args.0, args.1, args.2, args.3)
    }

    pub fn composite_shuffle3(
        &mut self,
        vector1: &Value,
        vector2: &Value,
        comp0: usize,
        comp1: usize,
        comp2: usize,
    ) -> Value {
        if vector1.ty() != vector2.ty() {
            unreachable!("Mismatching types {} and {}", vector1.ty(), vector2.ty());
        }
        if comp0 >= 6 || comp1 >= 6 || comp2 >= 6 {
            unreachable!(
                "One or more out of bounds elements {}, {}, {}",
                comp0, comp1, comp2
            );
        }
        let args = (
            vector1.clone(),
            vector2.clone(),
            self.imm32(comp0 as u32).into_value(),
            self.imm32(comp1 as u32).into_value(),
            self.imm32(comp2 as u32).into_value(),
        );
        let op = match vector1.ty() {
            Type::U32x4 => Opcode::CompositeShuffleU32x3,
            Type::F32x4 => Opcode::CompositeShuffleF32x3,
            _ => throw_invalid_type(vector1.ty()),
        };
        self.inst5(op, args.0, args.1, args.2, args.3, args.4)
    }

    pub fn composite_shuffle4(
        &mut self,
        vector1: &Value,
        vector2: &Value,
        comp0: usize,
        comp1: usize,
        comp2: usize,
        comp3: usize,
    ) -> Value {
        if vector1.ty() != vector2.ty() {
            unreachable!("Mismatching types {} and {}", vector1.ty(), vector2.ty());
        }
        if comp0 >= 8 || comp1 >= 8 || comp2 >= 8 || comp3 >= 8 {
            unreachable!(
                "One or more out of bounds elements {}, {}, {}, {}",
                comp0, comp1, comp2, comp3
            );
        }
        let args = (
            vector1.clone(),
            vector2.clone(),
            self.imm32(comp0 as u32).into_value(),
            self.imm32(comp1 as u32).into_value(),
            self.imm32(comp2 as u32).into_value(),
            self.imm32(comp3 as u32).into_value(),
        );
        let op = match vector1.ty() {
            Type::U32x4 => Opcode::CompositeShuffleU32x4,
            Type::F32x4 => Opcode::CompositeShuffleF32x4,
            _ => throw_invalid_type(vector1.ty()),
        };
        self.inst6(op, args.0, args.1, args.2, args.3, args.4, args.5)
    }

    pub fn select(&mut self, condition: &U1, true_value: &Value, false_value: &Value) -> Value {
        if true_value.ty() != false_value.ty() {
            unreachable!(
                "Mismatching types {} and {}",
                true_value.ty(),
                false_value.ty()
            );
        }
        let op = match true_value.ty() {
            Type::U1 => Opcode::SelectU1,
            Type::U32 => Opcode::SelectU32,
            Type::F32 => Opcode::SelectF32,
            Type::U64 => Opcode::SelectU64,
            _ => unreachable!("Invalid type {}", true_value.ty()),
        };
        self.inst3(
            op,
            condition.value().clone(),
            true_value.clone(),
            false_value.clone(),
        )
    }

    pub fn pack_uint_2x32(&mut self, vector: &Value) -> U64 {
        self.inst1(Opcode::PackUint2x32, vector.clone())
    }

    pub fn unpack_uint_2x32(&mut self, value: &U64) -> Value {
        self.inst1(Opcode::UnpackUint2x32, value.value().clone())
    }

    pub fn pack_double_2x32(&mut self, vector: &Value) -> F64 {
        self.inst1(Opcode::PackDouble2x32, vector.clone())
    }

    pub fn unpack_double_2x32(&mut self, value: &F64) -> Value {
        self.inst1(Opcode::UnpackDouble2x32, value.value().clone())
    }

    pub fn pack_2x16(&mut self, number_format: NumberFormat, vector: &Value) -> U32 {
        let op = match number_format {
            NumberFormat::Unorm => Opcode::PackUnorm2x16,
            NumberFormat::Snorm => Opcode::PackSnorm2x16,
            NumberFormat::Uint => Opcode::PackUint2x16,
            NumberFormat::Sint => Opcode::PackSint2x16,
            NumberFormat::Float => Opcode::PackHalf2x16,
            _ => unreachable!("Unsupported 2x16 number format: {:?}", number_format),
        };
        self.inst1(op, vector.clone())
    }

    pub fn unpack_2x16(&mut self, number_format: NumberFormat, value: &U32) -> Value {
        let op = match number_format {
            NumberFormat::Unorm => Opcode::UnpackUnorm2x16,
            NumberFormat::Snorm => Opcode::UnpackSnorm2x16,
            NumberFormat::Uint => Opcode::UnpackUint2x16,
            NumberFormat::Sint => Opcode::UnpackSint2x16,
            NumberFormat::Float => Opcode::UnpackHalf2x16,
            _ => unreachable!("Unsupported 2x16 number format: {:?}", number_format),
        };
        self.inst1(op, value.value().clone())
    }

    pub fn pack_4x8(&mut self, number_format: NumberFormat, vector: &Value) -> U32 {
        let op = match number_format {
            NumberFormat::Unorm => Opcode::PackUnorm4x8,
            NumberFormat::Snorm => Opcode::PackSnorm4x8,
            NumberFormat::Uint => Opcode::PackUint4x8,
            NumberFormat::Sint => Opcode::PackSint4x8,
            _ => unreachable!("Unsupported 4x8 number format: {:?}", number_format),
        };
        self.inst1(op, vector.clone())
    }

    pub fn unpack_4x8(&mut self, number_format: NumberFormat, value: &U32) -> Value {
        let op = match number_format {
            NumberFormat::Unorm => Opcode::UnpackUnorm4x8,
            NumberFormat::Snorm => Opcode::UnpackSnorm4x8,
            NumberFormat::Uint => Opcode::UnpackUint4x8,
            NumberFormat::Sint => Opcode::UnpackSint4x8,
            _ => unreachable!("Unsupported 4x8 number format: {:?}", number_format),
        };
        self.inst1(op, value.value().clone())
    }

    pub fn pack_10_11_11(&mut self, number_format: NumberFormat, vector: &Value) -> U32 {
        match number_format {
            NumberFormat::Float => self.inst1(Opcode::PackUfloat10_11_11, vector.clone()),
            _ => unreachable!("Unsupported 10_11_11 number format: {:?}", number_format),
        }
    }

    pub fn pack_2_10_10_10(&mut self, number_format: NumberFormat, vector: &Value) -> U32 {
        let op = match number_format {
            NumberFormat::Unorm => Opcode::PackUnorm2_10_10_10,
            NumberFormat::Snorm => Opcode::PackSnorm2_10_10_10,
            NumberFormat::Uint => Opcode::PackUint2_10_10_10,
            NumberFormat::Sint => Opcode::PackSint2_10_10_10,
            _ => unreachable!("Unsupported 2_10_10_10 number format: {:?}", number_format),
        };
        self.inst1(op, vector.clone())
    }

    pub fn unpack_2_10_10_10(&mut self, number_format: NumberFormat, value: &U32) -> Value {
        let op = match number_format {
            NumberFormat::Unorm => Opcode::UnpackUnorm2_10_10_10,
            NumberFormat::Snorm => Opcode::UnpackSnorm2_10_10_10,
            NumberFormat::Uint => Opcode::UnpackUint2_10_10_10,
            NumberFormat::Sint => Opcode::UnpackSint2_10_10_10,
            _ => unreachable!("Unsupported 2_10_10_10 number format: {:?}", number_format),
        };
        self.inst1(op, value.value().clone())
    }

    pub fn unpack_10_11_11(&mut self, number_format: NumberFormat, value: &U32) -> Value {
        match number_format {
            NumberFormat::Float => self.inst1(Opcode::UnpackUfloat10_11_11, value.value().clone()),
            _ => unreachable!("Unsupported 10_11_11 number format: {:?}", number_format),
        }
    }

    fn fp_binary(&mut self, op32: Opcode, op64: Option<Opcode>, a: &F32F64, b: &F32F64) -> F32F64 {
        if a.ty() != b.ty() {
            unreachable!("Mismatching types {} and {}", a.ty(), b.ty());
        }
        match a.ty() {
            Type::F32 => self.inst2(op32, a.value().clone(), b.value().clone()),
            Type::F64 => match op64 {
                Some(op) => self.inst2(op, a.value().clone(), b.value().clone()),
                None => throw_invalid_type(a.ty()),
            },
            _ => throw_invalid_type(a.ty()),
        }
    }

    fn fp_unary(&mut self, op32: Opcode, op64: Opcode, value: &F32F64) -> F32F64 {
        match value.ty() {
            Type::F32 => self.inst1(op32, value.value().clone()),
            Type::F64 => self.inst1(op64, value.value().clone()),
            _ => throw_invalid_type(value.ty()),
        }
    }

    pub fn fp_add(&mut self, a: &F32F64, b: &F32F64) -> F32F64 {
        self.fp_binary(Opcode::FPAdd32, Some(Opcode::FPAdd64), a, b)
    }

    pub fn fp_sub(&mut self, a: &F32F64, b: &F32F64) -> F32F64 {
        self.fp_binary(Opcode::FPSub32, None, a, b)
    }

    pub fn fp_mul(&mut self, a: &F32F64, b: &F32F64) -> F32F64 {
        self.fp_binary(Opcode::FPMul32, Some(Opcode::FPMul64), a, b)
    }

    pub fn fp_div(&mut self, a: &F32F64, b: &F32F64) -> F32F64 {
        self.fp_binary(Opcode::FPDiv32, Some(Opcode::FPDiv64), a, b)
    }

    pub fn fp_fma(&mut self, a: &F32F64, b: &F32F64, c: &F32F64) -> F32F64 {
        if a.ty() != b.ty() || a.ty() != c.ty() {
            unreachable!("Mismatching types {}, {}, and {}", a.ty(), b.ty(), c.ty());
        }
        match a.ty() {
            Type::F32 => self.inst3(
                Opcode::FPFma32,
                a.value().clone(),
                b.value().clone(),
                c.value().clone(),
            ),
            Type::F64 => self.inst3(
                Opcode::FPFma64,
                a.value().clone(),
                b.value().clone(),
                c.value().clone(),
            ),
            _ => throw_invalid_type(a.ty()),
        }
    }

    pub fn fp_abs(&mut self, value: &F32F64) -> F32F64 {
        self.fp_unary(Opcode::FPAbs32, Opcode::FPAbs64, value)
    }

    pub fn fp_neg(&mut self, value: &F32F64) -> F32F64 {
        self.fp_unary(Opcode::FPNeg32, Opcode::FPNeg64, value)
    }

    pub fn fp_abs_neg(&mut self, value: &F32F64, abs: bool, neg: bool) -> F32F64 {
        let mut result = value.clone();
        if abs {
            result = self.fp_abs(&result);
        }
        if neg {
            result = self.fp_neg(&result);
        }
        result
    }

    pub fn fp_cos(&mut self, value: &F32) -> F32 {
        self.inst1(Opcode::FPCos, value.value().clone())
    }

    pub fn fp_sin(&mut self, value: &F32) -> F32 {
        self.inst1(Opcode::FPSin, value.value().clone())
    }

    pub fn fp_exp2(&mut self, value: &F32) -> F32 {
        self.inst1(Opcode::FPExp2, value.value().clone())
    }

    pub fn fp_ldexp(&mut self, value: &F32, exp: &U32) -> F32 {
        self.inst2(Opcode::FPLdexp, value.value().clone(), exp.value().clone())
    }

    pub fn fp_log2(&mut self, value: &F32) -> F32 {
        self.inst1(Opcode::FPLog2, value.value().clone())
    }

    pub fn fp_pow(&mut self, x: &F32, y: &F32) -> F32 {
        self.inst2(Opcode::FPPow, x.value().clone(), y.value().clone())
    }

    pub fn fp_recip(&mut self, value: &F32F64) -> F32F64 {
        self.fp_unary(Opcode::FPRecip32, Opcode::FPRecip64, value)
    }

    pub fn fp_recip_sqrt(&mut self, value: &F32F64) -> F32F64 {
        self.fp_unary(Opcode::FPRecipSqrt32, Opcode::FPRecipSqrt64, value)
    }

    pub fn fp_sqrt(&mut self, value: &F32) -> F32 {
        self.inst1(Opcode::FPSqrt, value.value().clone())
    }

    pub fn fp_saturate(&mut self, value: &F32F64) -> F32F64 {
        self.fp_unary(Opcode::FPSaturate32, Opcode::FPSaturate64, value)
    }

    pub fn fp_clamp(&mut self, value: &F32F64, min_value: &F32F64, max_value: &F32F64) -> F32F64 {
        if value.ty() != min_value.ty() || value.ty() != max_value.ty() {
            unreachable!(
                "Mismatching types {}, {}, and {}",
                value.ty(),
                min_value.ty(),
                max_value.ty()
            );
        }
        match value.ty() {
            Type::F32 => self.inst3(
                Opcode::FPClamp32,
                value.value().clone(),
                min_value.value().clone(),
                max_value.value().clone(),
            ),
            Type::F64 => self.inst3(
                Opcode::FPClamp64,
                value.value().clone(),
                min_value.value().clone(),
                max_value.value().clone(),
            ),
            _ => throw_invalid_type(value.ty()),
        }
    }

    pub fn fp_round_even(&mut self, value: &F32F64) -> F32F64 {
        self.fp_unary(Opcode::FPRoundEven32, Opcode::FPRoundEven64, value)
    }

    pub fn fp_floor(&mut self, value: &F32F64) -> F32F64 {
        self.fp_unary(Opcode::FPFloor32, Opcode::FPFloor64, value)
    }

    pub fn fp_ceil(&mut self, value: &F32F64) -> F32F64 {
        self.fp_unary(Opcode::FPCeil32, Opcode::FPCeil64, value)
    }

    pub fn fp_trunc(&mut self, value: &F32F64) -> F32F64 {
        self.fp_unary(Opcode::FPTrunc32, Opcode::FPTrunc64, value)
    }

    pub fn fp_fract(&mut self, value: &F32F64) -> F32F64 {
        self.fp_unary(Opcode::FPFract32, Opcode::FPFract64, value)
    }

    pub fn fp_frexp_sig(&mut self, value: &F32F64) -> F32F64 {
        self.fp_unary(Opcode::FPFrexpSig32, Opcode::FPFrexpSig64, value)
    }

    pub fn fp_frexp_exp(&mut self, value: &F32F64) -> U32 {
        match value.ty() {
            Type::F32 => self.inst1(Opcode::FPFrexpExp32, value.value().clone()),
            Type::F64 => self.inst1(Opcode::FPFrexpExp64, value.value().clone()),
            _ => throw_invalid_type(value.ty()),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn fp_compare(
        &mut self,
        ord32: Opcode,
        unord32: Opcode,
        ord64: Opcode,
        unord64: Opcode,
        lhs: &F32F64,
        rhs: &F32F64,
        ordered: bool,
    ) -> U1 {
        if lhs.ty() != rhs.ty() {
            unreachable!("Mismatching types {} and {}", lhs.ty(), rhs.ty());
        }
        let op = |o: Opcode, u: Opcode| if ordered { o } else { u };
        match lhs.ty() {
            Type::F32 => self.inst2(op(ord32, unord32), lhs.value().clone(), rhs.value().clone()),
            Type::F64 => self.inst2(op(ord64, unord64), lhs.value().clone(), rhs.value().clone()),
            _ => throw_invalid_type(lhs.ty()),
        }
    }

    pub fn fp_equal(&mut self, lhs: &F32F64, rhs: &F32F64, ordered: bool) -> U1 {
        self.fp_compare(
            Opcode::FPOrdEqual32,
            Opcode::FPUnordEqual32,
            Opcode::FPOrdEqual64,
            Opcode::FPUnordEqual64,
            lhs,
            rhs,
            ordered,
        )
    }

    pub fn fp_not_equal(&mut self, lhs: &F32F64, rhs: &F32F64, ordered: bool) -> U1 {
        self.fp_compare(
            Opcode::FPOrdNotEqual32,
            Opcode::FPUnordNotEqual32,
            Opcode::FPOrdNotEqual64,
            Opcode::FPUnordNotEqual64,
            lhs,
            rhs,
            ordered,
        )
    }

    pub fn fp_less_than(&mut self, lhs: &F32F64, rhs: &F32F64, ordered: bool) -> U1 {
        self.fp_compare(
            Opcode::FPOrdLessThan32,
            Opcode::FPUnordLessThan32,
            Opcode::FPOrdLessThan64,
            Opcode::FPUnordLessThan64,
            lhs,
            rhs,
            ordered,
        )
    }

    pub fn fp_greater_than(&mut self, lhs: &F32F64, rhs: &F32F64, ordered: bool) -> U1 {
        self.fp_compare(
            Opcode::FPOrdGreaterThan32,
            Opcode::FPUnordGreaterThan32,
            Opcode::FPOrdGreaterThan64,
            Opcode::FPUnordGreaterThan64,
            lhs,
            rhs,
            ordered,
        )
    }

    pub fn fp_less_than_equal(&mut self, lhs: &F32F64, rhs: &F32F64, ordered: bool) -> U1 {
        self.fp_compare(
            Opcode::FPOrdLessThanEqual32,
            Opcode::FPUnordLessThanEqual32,
            Opcode::FPOrdLessThanEqual64,
            Opcode::FPUnordLessThanEqual64,
            lhs,
            rhs,
            ordered,
        )
    }

    pub fn fp_greater_than_equal(&mut self, lhs: &F32F64, rhs: &F32F64, ordered: bool) -> U1 {
        self.fp_compare(
            Opcode::FPOrdGreaterThanEqual32,
            Opcode::FPUnordGreaterThanEqual32,
            Opcode::FPOrdGreaterThanEqual64,
            Opcode::FPUnordGreaterThanEqual64,
            lhs,
            rhs,
            ordered,
        )
    }

    pub fn fp_is_nan(&mut self, value: &F32F64) -> U1 {
        match value.ty() {
            Type::F32 => self.inst1(Opcode::FPIsNan32, value.value().clone()),
            Type::F64 => self.inst1(Opcode::FPIsNan64, value.value().clone()),
            _ => throw_invalid_type(value.ty()),
        }
    }

    pub fn fp_is_inf(&mut self, value: &F32F64) -> U1 {
        match value.ty() {
            Type::F32 => self.inst1(Opcode::FPIsInf32, value.value().clone()),
            Type::F64 => self.inst1(Opcode::FPIsInf64, value.value().clone()),
            _ => throw_invalid_type(value.ty()),
        }
    }

    pub fn fp_cmp_class32(&mut self, value: &F32, op: &U32) -> U1 {
        self.inst2(
            Opcode::FPCmpClass32,
            value.value().clone(),
            op.value().clone(),
        )
    }

    pub fn fp_ordered(&mut self, lhs: &F32F64, rhs: &F32F64) -> U1 {
        if lhs.ty() != rhs.ty() {
            unreachable!("Mismatching types {} and {}", lhs.ty(), rhs.ty());
        }
        let lhs_nan = self.fp_is_nan(lhs);
        let rhs_nan = self.fp_is_nan(rhs);
        let lhs_not_nan = self.logical_not(&lhs_nan);
        let rhs_not_nan = self.logical_not(&rhs_nan);
        self.logical_and(&lhs_not_nan, &rhs_not_nan)
    }

    pub fn fp_unordered(&mut self, lhs: &F32F64, rhs: &F32F64) -> U1 {
        if lhs.ty() != rhs.ty() {
            unreachable!("Mismatching types {} and {}", lhs.ty(), rhs.ty());
        }
        let lhs_nan = self.fp_is_nan(lhs);
        let rhs_nan = self.fp_is_nan(rhs);
        self.logical_or(&lhs_nan, &rhs_nan)
    }

    pub fn fp_max(&mut self, lhs: &F32F64, rhs: &F32F64) -> F32F64 {
        self.fp_binary(Opcode::FPMax32, Some(Opcode::FPMax64), lhs, rhs)
    }

    pub fn fp_min(&mut self, lhs: &F32F64, rhs: &F32F64) -> F32F64 {
        self.fp_binary(Opcode::FPMin32, Some(Opcode::FPMin64), lhs, rhs)
    }

    pub fn fp_min_tri(&mut self, a: &F32F64, b: &F32F64, c: &F32F64) -> F32F64 {
        self.inst3(
            Opcode::FPMinTri32,
            a.value().clone(),
            b.value().clone(),
            c.value().clone(),
        )
    }

    pub fn fp_max_tri(&mut self, a: &F32F64, b: &F32F64, c: &F32F64) -> F32F64 {
        self.inst3(
            Opcode::FPMaxTri32,
            a.value().clone(),
            b.value().clone(),
            c.value().clone(),
        )
    }

    pub fn fp_med_tri(&mut self, a: &F32F64, b: &F32F64, c: &F32F64) -> F32F64 {
        self.inst3(
            Opcode::FPMedTri32,
            a.value().clone(),
            b.value().clone(),
            c.value().clone(),
        )
    }

    fn int_binary(&mut self, op32: Opcode, op64: Opcode, a: &U32U64, b: &U32U64) -> U32U64 {
        if a.ty() != b.ty() {
            unreachable!("Mismatching types {} and {}", a.ty(), b.ty());
        }
        match a.ty() {
            Type::U32 => self.inst2(op32, a.value().clone(), b.value().clone()),
            Type::U64 => self.inst2(op64, a.value().clone(), b.value().clone()),
            _ => throw_invalid_type(a.ty()),
        }
    }

    fn int_unary(&mut self, op32: Opcode, op64: Opcode, value: &U32U64) -> U32U64 {
        match value.ty() {
            Type::U32 => self.inst1(op32, value.value().clone()),
            Type::U64 => self.inst1(op64, value.value().clone()),
            _ => throw_invalid_type(value.ty()),
        }
    }

    pub fn i_add(&mut self, a: &U32U64, b: &U32U64) -> U32U64 {
        self.int_binary(Opcode::IAdd32, Opcode::IAdd64, a, b)
    }

    pub fn i_add_carry(&mut self, a: &U32, b: &U32) -> Value {
        if a.ty() != b.ty() {
            unreachable!("Mismatching types {} and {}", a.ty(), b.ty());
        }
        match a.ty() {
            Type::U32 => self.inst2(Opcode::IAddCarry32, a.value().clone(), b.value().clone()),
            _ => throw_invalid_type(a.ty()),
        }
    }

    pub fn i_sub(&mut self, a: &U32U64, b: &U32U64) -> U32U64 {
        self.int_binary(Opcode::ISub32, Opcode::ISub64, a, b)
    }

    pub fn i_mul_hi(&mut self, a: &U32, b: &U32, is_signed: bool) -> U32 {
        let op = if is_signed {
            Opcode::SMulHi
        } else {
            Opcode::UMulHi
        };
        self.inst2(op, a.value().clone(), b.value().clone())
    }

    pub fn i_mul(&mut self, a: &U32U64, b: &U32U64) -> U32U64 {
        self.int_binary(Opcode::IMul32, Opcode::IMul64, a, b)
    }

    pub fn i_div(&mut self, a: &U32, b: &U32, is_signed: bool) -> U32 {
        let op = if is_signed {
            Opcode::SDiv32
        } else {
            Opcode::UDiv32
        };
        self.inst2(op, a.value().clone(), b.value().clone())
    }

    pub fn i_mod(&mut self, a: &U32, b: &U32, is_signed: bool) -> U32 {
        let op = if is_signed {
            Opcode::SMod32
        } else {
            Opcode::UMod32
        };
        self.inst2(op, a.value().clone(), b.value().clone())
    }

    pub fn i_neg(&mut self, value: &U32U64) -> U32U64 {
        self.int_unary(Opcode::INeg32, Opcode::INeg64, value)
    }

    pub fn i_abs(&mut self, value: &U32) -> U32 {
        self.inst1(Opcode::IAbs32, value.value().clone())
    }

    fn shift(&mut self, op32: Opcode, op64: Opcode, base: &U32U64, shift: &U32) -> U32U64 {
        if shift.is_immediate() && shift.value().u32() == 0 {
            return base.clone();
        }
        match base.ty() {
            Type::U32 => self.inst2(op32, base.value().clone(), shift.value().clone()),
            Type::U64 => self.inst2(op64, base.value().clone(), shift.value().clone()),
            _ => throw_invalid_type(base.ty()),
        }
    }

    pub fn shift_left_logical(&mut self, base: &U32U64, shift: &U32) -> U32U64 {
        self.shift(
            Opcode::ShiftLeftLogical32,
            Opcode::ShiftLeftLogical64,
            base,
            shift,
        )
    }

    pub fn shift_right_logical(&mut self, base: &U32U64, shift: &U32) -> U32U64 {
        self.shift(
            Opcode::ShiftRightLogical32,
            Opcode::ShiftRightLogical64,
            base,
            shift,
        )
    }

    pub fn shift_right_arithmetic(&mut self, base: &U32U64, shift: &U32) -> U32U64 {
        self.shift(
            Opcode::ShiftRightArithmetic32,
            Opcode::ShiftRightArithmetic64,
            base,
            shift,
        )
    }

    pub fn bitwise_and(&mut self, a: &U32U64, b: &U32U64) -> U32U64 {
        self.int_binary(Opcode::BitwiseAnd32, Opcode::BitwiseAnd64, a, b)
    }

    pub fn bitwise_or(&mut self, a: &U32U64, b: &U32U64) -> U32U64 {
        self.int_binary(Opcode::BitwiseOr32, Opcode::BitwiseOr64, a, b)
    }

    pub fn bitwise_xor(&mut self, a: &U32U64, b: &U32U64) -> U32U64 {
        self.int_binary(Opcode::BitwiseXor32, Opcode::BitwiseXor64, a, b)
    }

    pub fn bit_field_insert(
        &mut self,
        base: &U32U64,
        insert: &U32U64,
        offset: &U32,
        count: &U32,
    ) -> U32U64 {
        if base.ty() != insert.ty() {
            unreachable!("Mismatching types {} and {}", base.ty(), insert.ty());
        }
        let args = (offset.value().clone(), count.value().clone());
        match base.ty() {
            Type::U32 => self.inst4(
                Opcode::BitFieldInsert32,
                base.value().clone(),
                insert.value().clone(),
                args.0,
                args.1,
            ),
            Type::U64 => self.inst4(
                Opcode::BitFieldInsert64,
                base.value().clone(),
                insert.value().clone(),
                args.0,
                args.1,
            ),
            _ => throw_invalid_type(base.ty()),
        }
    }

    pub fn bit_field_extract(
        &mut self,
        base: &U32,
        offset: &U32,
        count: &U32,
        is_signed: bool,
    ) -> U32 {
        let op = if is_signed {
            Opcode::BitFieldSExtract
        } else {
            Opcode::BitFieldUExtract
        };
        self.inst3(
            op,
            base.value().clone(),
            offset.value().clone(),
            count.value().clone(),
        )
    }

    pub fn bit_reverse(&mut self, value: &U32) -> U32 {
        self.inst1(Opcode::BitReverse32, value.value().clone())
    }

    pub fn bit_count(&mut self, value: &U32U64) -> U32 {
        match value.ty() {
            Type::U32 => self.inst1(Opcode::BitCount32, value.value().clone()),
            Type::U64 => self.inst1(Opcode::BitCount64, value.value().clone()),
            _ => throw_invalid_type(value.ty()),
        }
    }

    pub fn bitwise_not(&mut self, value: &U32U64) -> U32U64 {
        self.int_unary(Opcode::BitwiseNot32, Opcode::BitwiseNot64, value)
    }

    pub fn masked_bit_count(&mut self, value: &U32, addend: &U32, hi: bool) -> U32 {
        let hi = self.imm1(hi).into_value();
        self.inst3(
            Opcode::MaskedBitCount32,
            value.value().clone(),
            addend.value().clone(),
            hi,
        )
    }

    pub fn find_s_msb(&mut self, value: &U32) -> U32 {
        self.inst1(Opcode::FindSMsb32, value.value().clone())
    }

    pub fn find_u_msb(&mut self, value: &U32U64) -> U32 {
        match value.ty() {
            Type::U32 => self.inst1(Opcode::FindUMsb32, value.value().clone()),
            Type::U64 => self.inst1(Opcode::FindUMsb64, value.value().clone()),
            _ => throw_invalid_type(value.ty()),
        }
    }

    pub fn find_i_lsb(&mut self, value: &U32U64) -> U32 {
        match value.ty() {
            Type::U32 => self.inst1(Opcode::FindILsb32, value.value().clone()),
            Type::U64 => self.inst1(Opcode::FindILsb64, value.value().clone()),
            _ => throw_invalid_type(value.ty()),
        }
    }

    pub fn s_min(&mut self, a: &U32, b: &U32) -> U32 {
        self.inst2(Opcode::SMin32, a.value().clone(), b.value().clone())
    }

    pub fn u_min(&mut self, a: &U32, b: &U32) -> U32 {
        self.inst2(Opcode::UMin32, a.value().clone(), b.value().clone())
    }

    pub fn i_min(&mut self, a: &U32, b: &U32, is_signed: bool) -> U32 {
        if is_signed {
            self.s_min(a, b)
        } else {
            self.u_min(a, b)
        }
    }

    pub fn s_max(&mut self, a: &U32, b: &U32) -> U32 {
        self.inst2(Opcode::SMax32, a.value().clone(), b.value().clone())
    }

    pub fn u_max(&mut self, a: &U32, b: &U32) -> U32 {
        self.inst2(Opcode::UMax32, a.value().clone(), b.value().clone())
    }

    pub fn i_max(&mut self, a: &U32, b: &U32, is_signed: bool) -> U32 {
        if is_signed {
            self.s_max(a, b)
        } else {
            self.u_max(a, b)
        }
    }

    pub fn s_min_tri(&mut self, a: &U32, b: &U32, c: &U32) -> U32 {
        self.inst3(
            Opcode::SMinTri32,
            a.value().clone(),
            b.value().clone(),
            c.value().clone(),
        )
    }

    pub fn u_min_tri(&mut self, a: &U32, b: &U32, c: &U32) -> U32 {
        self.inst3(
            Opcode::UMinTri32,
            a.value().clone(),
            b.value().clone(),
            c.value().clone(),
        )
    }

    pub fn i_min_tri(&mut self, a: &U32, b: &U32, c: &U32, is_signed: bool) -> U32 {
        if is_signed {
            self.s_min_tri(a, b, c)
        } else {
            self.u_min_tri(a, b, c)
        }
    }

    pub fn s_max_tri(&mut self, a: &U32, b: &U32, c: &U32) -> U32 {
        self.inst3(
            Opcode::SMaxTri32,
            a.value().clone(),
            b.value().clone(),
            c.value().clone(),
        )
    }

    pub fn u_max_tri(&mut self, a: &U32, b: &U32, c: &U32) -> U32 {
        self.inst3(
            Opcode::UMaxTri32,
            a.value().clone(),
            b.value().clone(),
            c.value().clone(),
        )
    }

    pub fn i_max_tri(&mut self, a: &U32, b: &U32, c: &U32, is_signed: bool) -> U32 {
        if is_signed {
            self.s_max_tri(a, b, c)
        } else {
            self.u_max_tri(a, b, c)
        }
    }

    pub fn s_med_tri(&mut self, a: &U32, b: &U32, c: &U32) -> U32 {
        self.inst3(
            Opcode::SMedTri32,
            a.value().clone(),
            b.value().clone(),
            c.value().clone(),
        )
    }

    pub fn u_med_tri(&mut self, a: &U32, b: &U32, c: &U32) -> U32 {
        self.inst3(
            Opcode::UMedTri32,
            a.value().clone(),
            b.value().clone(),
            c.value().clone(),
        )
    }

    pub fn i_med_tri(&mut self, a: &U32, b: &U32, c: &U32, is_signed: bool) -> U32 {
        if is_signed {
            self.s_med_tri(a, b, c)
        } else {
            self.u_med_tri(a, b, c)
        }
    }

    pub fn s_clamp(&mut self, value: &U32, min: &U32, max: &U32) -> U32 {
        self.inst3(
            Opcode::SClamp32,
            value.value().clone(),
            min.value().clone(),
            max.value().clone(),
        )
    }

    pub fn u_clamp(&mut self, value: &U32, min: &U32, max: &U32) -> U32 {
        self.inst3(
            Opcode::UClamp32,
            value.value().clone(),
            min.value().clone(),
            max.value().clone(),
        )
    }

    fn int_compare(&mut self, op32: Opcode, op64: Opcode, lhs: &U32U64, rhs: &U32U64) -> U1 {
        if lhs.ty() != rhs.ty() {
            unreachable!("Mismatching types {} and {}", lhs.ty(), rhs.ty());
        }
        match lhs.ty() {
            Type::U32 => self.inst2(op32, lhs.value().clone(), rhs.value().clone()),
            Type::U64 => self.inst2(op64, lhs.value().clone(), rhs.value().clone()),
            _ => throw_invalid_type(lhs.ty()),
        }
    }

    pub fn i_less_than(&mut self, lhs: &U32U64, rhs: &U32U64, is_signed: bool) -> U1 {
        let (op32, op64) = if is_signed {
            (Opcode::SLessThan32, Opcode::SLessThan64)
        } else {
            (Opcode::ULessThan32, Opcode::ULessThan64)
        };
        self.int_compare(op32, op64, lhs, rhs)
    }

    pub fn i_equal(&mut self, lhs: &U32U64, rhs: &U32U64) -> U1 {
        self.int_compare(Opcode::IEqual32, Opcode::IEqual64, lhs, rhs)
    }

    pub fn i_less_than_equal(&mut self, lhs: &U32U64, rhs: &U32U64, is_signed: bool) -> U1 {
        let (op32, op64) = if is_signed {
            (Opcode::SLessThanEqual32, Opcode::SLessThanEqual64)
        } else {
            (Opcode::ULessThanEqual32, Opcode::ULessThanEqual64)
        };
        self.int_compare(op32, op64, lhs, rhs)
    }

    pub fn i_greater_than(&mut self, lhs: &U32U64, rhs: &U32U64, is_signed: bool) -> U1 {
        let (op32, op64) = if is_signed {
            (Opcode::SGreaterThan32, Opcode::SGreaterThan64)
        } else {
            (Opcode::UGreaterThan32, Opcode::UGreaterThan64)
        };
        self.int_compare(op32, op64, lhs, rhs)
    }

    pub fn i_not_equal(&mut self, lhs: &U32U64, rhs: &U32U64) -> U1 {
        self.int_compare(Opcode::INotEqual32, Opcode::INotEqual64, lhs, rhs)
    }

    pub fn i_greater_than_equal(&mut self, lhs: &U32U64, rhs: &U32U64, is_signed: bool) -> U1 {
        let (op32, op64) = if is_signed {
            (Opcode::SGreaterThanEqual32, Opcode::SGreaterThanEqual64)
        } else {
            (Opcode::UGreaterThanEqual32, Opcode::UGreaterThanEqual64)
        };
        self.int_compare(op32, op64, lhs, rhs)
    }

    pub fn logical_or(&mut self, a: &U1, b: &U1) -> U1 {
        self.inst2(Opcode::LogicalOr, a.value().clone(), b.value().clone())
    }

    pub fn logical_and(&mut self, a: &U1, b: &U1) -> U1 {
        self.inst2(Opcode::LogicalAnd, a.value().clone(), b.value().clone())
    }

    pub fn logical_xor(&mut self, a: &U1, b: &U1) -> U1 {
        self.inst2(Opcode::LogicalXor, a.value().clone(), b.value().clone())
    }

    pub fn logical_not(&mut self, value: &U1) -> U1 {
        self.inst1(Opcode::LogicalNot, value.value().clone())
    }

    pub fn convert_f_to_s(&mut self, bitsize: usize, value: &F32F64) -> U32U64 {
        if bitsize == 32 {
            return match value.ty() {
                Type::F32 => self.inst1(Opcode::ConvertS32F32, value.value().clone()),
                Type::F64 => self.inst1(Opcode::ConvertS32F64, value.value().clone()),
                _ => throw_invalid_type(value.ty()),
            };
        }
        unreachable!("Invalid destination bitsize {}", bitsize)
    }

    pub fn convert_f_to_u(&mut self, bitsize: usize, value: &F32F64) -> U32U64 {
        if bitsize == 32 {
            return match value.ty() {
                Type::F32 => self.inst1(Opcode::ConvertU32F32, value.value().clone()),
                _ => throw_invalid_type(value.ty()),
            };
        }
        unreachable!("Invalid destination bitsize {}", bitsize)
    }

    pub fn convert_f_to_i(&mut self, bitsize: usize, is_signed: bool, value: &F32F64) -> U32U64 {
        if is_signed {
            self.convert_f_to_s(bitsize, value)
        } else {
            self.convert_f_to_u(bitsize, value)
        }
    }

    pub fn convert_s_to_f(
        &mut self,
        dest_bitsize: usize,
        src_bitsize: usize,
        value: &Value,
    ) -> F32F64 {
        match (dest_bitsize, src_bitsize) {
            (32, 32) => self.inst1(Opcode::ConvertF32S32, value.clone()),
            (64, 32) => self.inst1(Opcode::ConvertF64S32, value.clone()),
            _ => unreachable!(
                "Invalid bit size combination dst={} src={}",
                dest_bitsize, src_bitsize
            ),
        }
    }

    pub fn convert_u_to_f(
        &mut self,
        dest_bitsize: usize,
        src_bitsize: usize,
        value: &Value,
    ) -> F32F64 {
        match (dest_bitsize, src_bitsize) {
            (32, 16) => self.inst1(Opcode::ConvertF32U16, value.clone()),
            (32, 32) => self.inst1(Opcode::ConvertF32U32, value.clone()),
            (64, 32) => self.inst1(Opcode::ConvertF64U32, value.clone()),
            _ => unreachable!(
                "Invalid bit size combination dst={} src={}",
                dest_bitsize, src_bitsize
            ),
        }
    }

    pub fn convert_i_to_f(
        &mut self,
        dest_bitsize: usize,
        src_bitsize: usize,
        is_signed: bool,
        value: &Value,
    ) -> F32F64 {
        if is_signed {
            self.convert_s_to_f(dest_bitsize, src_bitsize, value)
        } else {
            self.convert_u_to_f(dest_bitsize, src_bitsize, value)
        }
    }

    pub fn u_convert(&mut self, result_bitsize: usize, value: &U8U16U32U64) -> U8U16U32U64 {
        match (result_bitsize, value.ty()) {
            (8, Type::U32) => self.inst1(Opcode::ConvertU8U32, value.value().clone()),
            (16, Type::U32) => self.inst1(Opcode::ConvertU16U32, value.value().clone()),
            (32, Type::U8) => self.inst1(Opcode::ConvertU32U8, value.value().clone()),
            (32, Type::U16) => self.inst1(Opcode::ConvertU32U16, value.value().clone()),
            (64, Type::U32) => self.inst1(Opcode::ConvertU64U32, value.value().clone()),
            _ => unreachable!("Conversion from {} to {} bits", value.ty(), result_bitsize),
        }
    }

    pub fn s_convert(&mut self, result_bitsize: usize, value: &U8U16U32U64) -> U8U16U32U64 {
        match (result_bitsize, value.ty()) {
            (32, Type::U8) => self.inst1(Opcode::ConvertS32S8, value.value().clone()),
            (32, Type::U16) => self.inst1(Opcode::ConvertS32S16, value.value().clone()),
            _ => unreachable!(
                "Signed Conversion from {} to {} bits",
                value.ty(),
                result_bitsize
            ),
        }
    }

    pub fn fp_convert(&mut self, result_bitsize: usize, value: &F16F32F64) -> F16F32F64 {
        match (result_bitsize, value.ty()) {
            (16, Type::F32) => self.inst1(Opcode::ConvertF16F32, value.value().clone()),
            (32, Type::F16) => self.inst1(Opcode::ConvertF32F16, value.value().clone()),
            (32, Type::F64) => self.inst1(Opcode::ConvertF32F64, value.value().clone()),
            (64, Type::F32) => self.inst1(Opcode::ConvertF64F32, value.value().clone()),
            _ => unreachable!("Conversion from {} to {} bits", value.ty(), result_bitsize),
        }
    }

    fn image_atomic3(
        &mut self,
        op: Opcode,
        handle: &Value,
        coords: &Value,
        value: &Value,
        info: TextureInstInfo,
    ) -> Value {
        let args = (handle.clone(), coords.clone(), value.clone());
        self.inst3f(op, u64::from(info.raw()), args.0, args.1, args.2)
    }

    pub fn image_atomic_i_add(
        &mut self,
        handle: &Value,
        coords: &Value,
        value: &Value,
        info: TextureInstInfo,
    ) -> Value {
        self.image_atomic3(Opcode::ImageAtomicIAdd32, handle, coords, value, info)
    }

    pub fn image_atomic_s_min(
        &mut self,
        handle: &Value,
        coords: &Value,
        value: &Value,
        info: TextureInstInfo,
    ) -> Value {
        self.image_atomic3(Opcode::ImageAtomicSMin32, handle, coords, value, info)
    }

    pub fn image_atomic_u_min(
        &mut self,
        handle: &Value,
        coords: &Value,
        value: &Value,
        info: TextureInstInfo,
    ) -> Value {
        self.image_atomic3(Opcode::ImageAtomicUMin32, handle, coords, value, info)
    }

    pub fn image_atomic_i_min(
        &mut self,
        handle: &Value,
        coords: &Value,
        value: &Value,
        is_signed: bool,
        info: TextureInstInfo,
    ) -> Value {
        if is_signed {
            self.image_atomic_s_min(handle, coords, value, info)
        } else {
            self.image_atomic_u_min(handle, coords, value, info)
        }
    }

    pub fn image_atomic_s_max(
        &mut self,
        handle: &Value,
        coords: &Value,
        value: &Value,
        info: TextureInstInfo,
    ) -> Value {
        self.image_atomic3(Opcode::ImageAtomicSMax32, handle, coords, value, info)
    }

    pub fn image_atomic_u_max(
        &mut self,
        handle: &Value,
        coords: &Value,
        value: &Value,
        info: TextureInstInfo,
    ) -> Value {
        self.image_atomic3(Opcode::ImageAtomicUMax32, handle, coords, value, info)
    }

    pub fn image_atomic_f_max(
        &mut self,
        handle: &Value,
        coords: &Value,
        value: &Value,
        info: TextureInstInfo,
    ) -> Value {
        self.image_atomic3(Opcode::ImageAtomicFMax32, handle, coords, value, info)
    }

    pub fn image_atomic_f_min(
        &mut self,
        handle: &Value,
        coords: &Value,
        value: &Value,
        info: TextureInstInfo,
    ) -> Value {
        self.image_atomic3(Opcode::ImageAtomicFMin32, handle, coords, value, info)
    }

    pub fn image_atomic_i_max(
        &mut self,
        handle: &Value,
        coords: &Value,
        value: &Value,
        is_signed: bool,
        info: TextureInstInfo,
    ) -> Value {
        if is_signed {
            self.image_atomic_s_max(handle, coords, value, info)
        } else {
            self.image_atomic_u_max(handle, coords, value, info)
        }
    }

    pub fn image_atomic_inc(
        &mut self,
        handle: &Value,
        coords: &Value,
        value: &Value,
        info: TextureInstInfo,
    ) -> Value {
        self.image_atomic3(Opcode::ImageAtomicInc32, handle, coords, value, info)
    }

    pub fn image_atomic_dec(
        &mut self,
        handle: &Value,
        coords: &Value,
        value: &Value,
        info: TextureInstInfo,
    ) -> Value {
        self.image_atomic3(Opcode::ImageAtomicDec32, handle, coords, value, info)
    }

    pub fn image_atomic_and(
        &mut self,
        handle: &Value,
        coords: &Value,
        value: &Value,
        info: TextureInstInfo,
    ) -> Value {
        self.image_atomic3(Opcode::ImageAtomicAnd32, handle, coords, value, info)
    }

    pub fn image_atomic_or(
        &mut self,
        handle: &Value,
        coords: &Value,
        value: &Value,
        info: TextureInstInfo,
    ) -> Value {
        self.image_atomic3(Opcode::ImageAtomicOr32, handle, coords, value, info)
    }

    pub fn image_atomic_xor(
        &mut self,
        handle: &Value,
        coords: &Value,
        value: &Value,
        info: TextureInstInfo,
    ) -> Value {
        self.image_atomic3(Opcode::ImageAtomicXor32, handle, coords, value, info)
    }

    pub fn image_atomic_exchange(
        &mut self,
        handle: &Value,
        coords: &Value,
        value: &Value,
        info: TextureInstInfo,
    ) -> Value {
        self.image_atomic3(Opcode::ImageAtomicExchange32, handle, coords, value, info)
    }

    pub fn image_atomic_cmp_swap(
        &mut self,
        handle: &Value,
        coords: &Value,
        value: &Value,
        cmp_value: &Value,
        info: TextureInstInfo,
    ) -> Value {
        let args = (
            handle.clone(),
            coords.clone(),
            value.clone(),
            cmp_value.clone(),
        );
        self.inst4f(
            Opcode::ImageAtomicCmpSwap32,
            u64::from(info.raw()),
            args.0,
            args.1,
            args.2,
            args.3,
        )
    }

    pub fn image_handle(&mut self, tsharp_low: &Value, tsharp_high: &Value) -> Value {
        self.inst2(Opcode::ImageHandle, tsharp_low.clone(), tsharp_high.clone())
    }

    pub fn image_sample_raw(
        &mut self,
        image_handle: &Value,
        sampler_handle: &Value,
        address1: &Value,
        address2: &Value,
        address3: &Value,
        address4: &Value,
        info: TextureInstInfo,
    ) -> Value {
        let args = (
            image_handle.clone(),
            sampler_handle.clone(),
            address1.clone(),
            address2.clone(),
            address3.clone(),
            address4.clone(),
        );
        self.inst6f(
            Opcode::ImageSampleRaw,
            u64::from(info.raw()),
            args.0,
            args.1,
            args.2,
            args.3,
            args.4,
            args.5,
        )
    }

    pub fn image_sample_implicit_lod(
        &mut self,
        handle: &Value,
        body: &Value,
        bias: &F32,
        offset: &Value,
        info: TextureInstInfo,
    ) -> Value {
        let args = (
            handle.clone(),
            body.clone(),
            bias.value().clone(),
            offset.clone(),
        );
        self.inst4f(
            Opcode::ImageSampleImplicitLod,
            u64::from(info.raw()),
            args.0,
            args.1,
            args.2,
            args.3,
        )
    }

    pub fn image_sample_explicit_lod(
        &mut self,
        handle: &Value,
        body: &Value,
        lod: &F32,
        offset: &Value,
        info: TextureInstInfo,
    ) -> Value {
        let args = (
            handle.clone(),
            body.clone(),
            lod.value().clone(),
            offset.clone(),
        );
        self.inst4f(
            Opcode::ImageSampleExplicitLod,
            u64::from(info.raw()),
            args.0,
            args.1,
            args.2,
            args.3,
        )
    }

    pub fn image_sample_dref_implicit_lod(
        &mut self,
        handle: &Value,
        body: &Value,
        dref: &F32,
        bias: &F32,
        offset: &Value,
        info: TextureInstInfo,
    ) -> Value {
        let args = (
            handle.clone(),
            body.clone(),
            dref.value().clone(),
            bias.value().clone(),
            offset.clone(),
        );
        self.inst5f(
            Opcode::ImageSampleDrefImplicitLod,
            u64::from(info.raw()),
            args.0,
            args.1,
            args.2,
            args.3,
            args.4,
        )
    }

    pub fn image_sample_dref_explicit_lod(
        &mut self,
        handle: &Value,
        body: &Value,
        dref: &F32,
        lod: &F32,
        offset: &Value,
        info: TextureInstInfo,
    ) -> Value {
        let args = (
            handle.clone(),
            body.clone(),
            dref.value().clone(),
            lod.value().clone(),
            offset.clone(),
        );
        self.inst5f(
            Opcode::ImageSampleDrefExplicitLod,
            u64::from(info.raw()),
            args.0,
            args.1,
            args.2,
            args.3,
            args.4,
        )
    }

    pub fn image_gather(
        &mut self,
        handle: &Value,
        coords: &Value,
        offset: &Value,
        info: TextureInstInfo,
    ) -> Value {
        let args = (handle.clone(), coords.clone(), offset.clone());
        self.inst3f(
            Opcode::ImageGather,
            u64::from(info.raw()),
            args.0,
            args.1,
            args.2,
        )
    }

    pub fn image_gather_dref(
        &mut self,
        handle: &Value,
        coords: &Value,
        offset: &Value,
        dref: &F32,
        info: TextureInstInfo,
    ) -> Value {
        let args = (
            handle.clone(),
            coords.clone(),
            offset.clone(),
            dref.value().clone(),
        );
        self.inst4f(
            Opcode::ImageGatherDref,
            u64::from(info.raw()),
            args.0,
            args.1,
            args.2,
            args.3,
        )
    }

    pub fn image_query_dimension(
        &mut self,
        handle: &Value,
        lod: &U32,
        skip_mips: &U1,
        info: TextureInstInfo,
    ) -> Value {
        let args = (
            handle.clone(),
            lod.value().clone(),
            skip_mips.value().clone(),
        );
        self.inst3f(
            Opcode::ImageQueryDimensions,
            u64::from(info.raw()),
            args.0,
            args.1,
            args.2,
        )
    }

    pub fn image_query_lod(
        &mut self,
        handle: &Value,
        coords: &Value,
        info: TextureInstInfo,
    ) -> Value {
        self.inst2f(
            Opcode::ImageQueryLod,
            u64::from(info.raw()),
            handle.clone(),
            coords.clone(),
        )
    }

    pub fn image_gradient(
        &mut self,
        handle: &Value,
        coords: &Value,
        derivatives_dx: &Value,
        derivatives_dy: &Value,
        offset: &Value,
        lod_clamp: &F32,
        info: TextureInstInfo,
    ) -> Value {
        let args = (
            handle.clone(),
            coords.clone(),
            derivatives_dx.clone(),
            derivatives_dy.clone(),
            offset.clone(),
            lod_clamp.value().clone(),
        );
        self.inst6f(
            Opcode::ImageGradient,
            u64::from(info.raw()),
            args.0,
            args.1,
            args.2,
            args.3,
            args.4,
            args.5,
        )
    }

    pub fn image_read(
        &mut self,
        handle: &Value,
        coords: &Value,
        lod: &U32,
        multisampling: &U32,
        info: TextureInstInfo,
    ) -> Value {
        let args = (
            handle.clone(),
            coords.clone(),
            lod.value().clone(),
            multisampling.value().clone(),
        );
        self.inst4f(
            Opcode::ImageRead,
            u64::from(info.raw()),
            args.0,
            args.1,
            args.2,
            args.3,
        )
    }

    pub fn image_write(
        &mut self,
        handle: &Value,
        coords: &Value,
        lod: &U32,
        multisampling: &U32,
        color: &Value,
        info: TextureInstInfo,
    ) {
        let args = (
            handle.clone(),
            coords.clone(),
            lod.value().clone(),
            multisampling.value().clone(),
            color.clone(),
        );
        self.emit5f(
            Opcode::ImageWrite,
            u64::from(info.raw()),
            args.0,
            args.1,
            args.2,
            args.3,
            args.4,
        )
    }

    pub fn cube_face_index(&mut self, cube_coords: &Value) -> F32 {
        self.inst1(Opcode::CubeFaceIndex, cube_coords.clone())
    }

    pub fn debug_print(&mut self, fmt: &'static str, format_args: &[Value]) {
        assert!(
            format_args.len() < DEBUGPRINT_NUM_FORMAT_ARGS,
            "DebugPrint only supports up to {} format args",
            DEBUGPRINT_NUM_FORMAT_ARGS
        );

        let mut args: Vec<Value> = (0..DEBUGPRINT_NUM_FORMAT_ARGS)
            .map(|_| Value::empty())
            .collect();
        for (slot, arg) in args.iter_mut().zip(format_args.iter()) {
            *slot = arg.clone();
        }
        for slot in args.iter_mut().skip(format_args.len()) {
            *slot = self.inst0(Opcode::Void);
        }

        let fmt_val = Value::from_string_literal(fmt);
        let flags = DebugPrintFlags::new(format_args.len() as u32);
        self.emit5f(
            Opcode::DebugPrint,
            u64::from(flags.raw()),
            fmt_val,
            args[0].clone(),
            args[1].clone(),
            args[2].clone(),
            args[3].clone(),
        );
    }

    pub fn emit_vertex(&mut self) {
        self.emit0(Opcode::EmitVertex);
    }

    pub fn emit_primitive(&mut self) {
        self.emit0(Opcode::EmitPrimitive);
    }

    pub fn memtime(&mut self) -> U64 {
        self.inst0(Opcode::Memtime)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::shader_recompiler::ir::basic_block::Block;

    fn ops(block: &BlockRef) -> Vec<(Opcode, Vec<Value>)> {
        block
            .borrow()
            .instructions
            .iter()
            .map(|inst| {
                let inst = inst.borrow();
                let args = (0..inst.num_args()).map(|i| inst.arg(i)).collect();
                (inst.opcode(), args)
            })
            .collect()
    }

    fn emitter() -> (BlockRef, IrEmitter) {
        let block = Block::new_ref();
        let emitter = IrEmitter::new(&block);
        (block, emitter)
    }

    #[test]
    fn immediates_carry_their_own_types() {
        let (_, ir) = emitter();
        assert_eq!(ir.imm1(true).ty(), Type::U1);
        assert_eq!(ir.imm8(3).ty(), Type::U8);
        assert_eq!(ir.imm16(3).ty(), Type::U16);
        assert_eq!(ir.imm32(3).ty(), Type::U32);
        assert_eq!(ir.imm32_signed(-1).ty(), Type::U32);
        assert_eq!(ir.imm32_signed(-1).value().u32(), u32::MAX);
        assert_eq!(ir.imm32_float(0.5).ty(), Type::F32);
        assert_eq!(ir.imm32_float(0.5).value().f32(), 0.5);
        assert_eq!(ir.imm64(3).ty(), Type::U64);
        assert_eq!(ir.imm64_signed(-1).value().u64(), u64::MAX);
        assert_eq!(ir.imm64_float(0.5).ty(), Type::F64);
        // Immediates emit nothing.
        assert!(ir.imm32(1).value().is_immediate());
    }

    #[test]
    fn every_instruction_lands_before_the_insertion_point() {
        let (block, _) = emitter();
        let mut ir = IrEmitter::new(&block);
        ir.lane_id();
        ir.warp_id();
        // The second emitter inserts before the second instruction, and then after its own.
        let mut at_one = IrEmitter::at(&block, 1);
        at_one.barrier();
        at_one.workgroup_memory_barrier();
        let emitted: Vec<Opcode> = ops(&block).iter().map(|(op, _)| *op).collect();
        assert_eq!(
            emitted,
            vec![
                Opcode::LaneId,
                Opcode::Barrier,
                Opcode::WorkgroupMemoryBarrier,
                Opcode::WarpId,
            ]
        );
    }

    #[test]
    fn the_conditions_build_their_register_sequences() {
        let (block, _) = emitter();
        let mut ir = IrEmitter::new(&block);
        ir.condition(Condition::False);
        ir.condition(Condition::True);
        ir.condition(Condition::Scc0);
        ir.condition(Condition::Scc1);
        assert_eq!(
            ops(&block).iter().map(|(op, _)| *op).collect::<Vec<_>>(),
            vec![Opcode::GetScc, Opcode::LogicalNot, Opcode::GetScc]
        );

        let (block, _) = emitter();
        let mut ir = IrEmitter::new(&block);
        ir.condition(Condition::Vccz);
        ir.condition(Condition::Vccnz);
        assert_eq!(
            ops(&block).iter().map(|(op, _)| *op).collect::<Vec<_>>(),
            vec![
                Opcode::GetVccLo,
                Opcode::GetVccHi,
                Opcode::CompositeConstructU32x2,
                Opcode::PackUint2x32,
                Opcode::InverseBallot,
                Opcode::LogicalNot,
                Opcode::GetVccLo,
                Opcode::GetVccHi,
                Opcode::CompositeConstructU32x2,
                Opcode::PackUint2x32,
                Opcode::InverseBallot,
            ]
        );

        let (block, _) = emitter();
        let mut ir = IrEmitter::new(&block);
        ir.condition(Condition::Execz);
        ir.condition(Condition::Execnz);
        assert_eq!(
            ops(&block).iter().map(|(op, _)| *op).collect::<Vec<_>>(),
            vec![Opcode::GetExec, Opcode::LogicalNot, Opcode::GetExec]
        );
    }

    #[test]
    fn the_bit_cast_of_a_condition_selects_one_or_zero() {
        let (block, _) = emitter();
        let mut ir = IrEmitter::new(&block);
        let value = ir.get_exec();
        ir.bit_cast_u32_u1(&value);
        let seen = ops(&block);
        assert_eq!(
            seen.iter().map(|(op, _)| *op).collect::<Vec<_>>(),
            vec![Opcode::GetExec, Opcode::SelectU32]
        );
        let args = &seen[1].1;
        assert_eq!(args[1].u32(), 1);
        assert_eq!(args[2].u32(), 0);
    }

    #[test]
    fn storing_a_float_scalar_register_casts_it_first() {
        let (block, _) = emitter();
        let mut ir = IrEmitter::new(&block);
        let value = F32::new(Value::from_f32(1.0));
        let stored = U32F32::new(value.value().clone());
        ir.set_scalar_reg(ScalarReg::S4, &stored);
        let seen = ops(&block);
        assert_eq!(
            seen.iter().map(|(op, _)| *op).collect::<Vec<_>>(),
            vec![Opcode::BitCastU32F32, Opcode::SetScalarRegister]
        );
        assert_eq!(seen[1].1[0], Value::from_scalar_reg(ScalarReg::S4));

        // A u32 goes in as it is.
        let (block, _) = emitter();
        let mut ir = IrEmitter::new(&block);
        let value = U32F32::new(Value::from_u32(7));
        ir.set_scalar_reg(ScalarReg::S4, &value);
        let seen = ops(&block);
        assert_eq!(
            seen.iter().map(|(op, _)| *op).collect::<Vec<_>>(),
            vec![Opcode::SetScalarRegister]
        );
    }

    #[test]
    #[should_panic(expected = "Mismatching types")]
    fn floating_point_operations_refuse_mixed_types() {
        let (_block, mut ir) = emitter();
        let f32_value = F32F64::new(Value::from_f32(1.0));
        let f64_value = F32F64::new(Value::from_f64(1.0));
        ir.fp_add(&f32_value, &f64_value);
    }

    #[test]
    fn the_floating_point_operations_pick_the_width() {
        let (block, _) = emitter();
        let mut ir = IrEmitter::new(&block);
        let wide = F32F64::new(Value::from_f64(1.0));
        let narrow = F32F64::new(Value::from_f32(1.0));
        ir.fp_add(&wide, &wide);
        ir.fp_add(&narrow, &narrow);
        ir.fp_mul(&wide, &wide);
        ir.fp_abs(&wide);
        ir.fp_is_nan(&narrow);
        let seen: Vec<Opcode> = ops(&block).iter().map(|(op, _)| *op).collect();
        assert_eq!(
            seen,
            vec![
                Opcode::FPAdd64,
                Opcode::FPAdd32,
                Opcode::FPMul64,
                Opcode::FPAbs64,
                Opcode::FPIsNan32,
            ]
        );
        // FPSub has no 64-bit form in the C++.
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut ir = IrEmitter::new(&block);
            ir.fp_sub(&wide, &wide);
        }))
        .is_err());
    }

    #[test]
    fn the_integer_operations_pick_the_width_and_signedness() {
        let (block, _) = emitter();
        let mut ir = IrEmitter::new(&block);
        let wide = U32U64::new(Value::from_u64(1));
        let narrow = U32U64::new(Value::from_u32(1));
        let a = U32::new(Value::from_u32(1));
        ir.i_add(&wide, &wide);
        ir.i_add(&narrow, &narrow);
        ir.i_mul_hi(&a, &a, true);
        ir.i_mul_hi(&a, &a, false);
        ir.i_div(&a, &a, true);
        ir.i_div(&a, &a, false);
        ir.i_less_than(&wide, &wide, true);
        ir.i_less_than(&narrow, &narrow, false);
        ir.s_min(&a, &a);
        ir.i_min(&a, &a, false);
        let seen: Vec<Opcode> = ops(&block).iter().map(|(op, _)| *op).collect();
        assert_eq!(
            seen,
            vec![
                Opcode::IAdd64,
                Opcode::IAdd32,
                Opcode::SMulHi,
                Opcode::UMulHi,
                Opcode::SDiv32,
                Opcode::UDiv32,
                Opcode::SLessThan64,
                Opcode::ULessThan32,
                Opcode::SMin32,
                Opcode::UMin32,
            ]
        );
    }

    #[test]
    fn a_zero_shift_returns_the_base_untouched() {
        let (block, _) = emitter();
        let mut ir = IrEmitter::new(&block);
        let base = U32U64::new(Value::from_u32(5));
        let zero = U32::new(Value::from_u32(0));
        let one = U32::new(Value::from_u32(1));
        let unchanged = ir.shift_left_logical(&base, &zero);
        assert_eq!(unchanged, base);
        assert!(ops(&block).is_empty(), "the shortcut emits nothing");
        ir.shift_left_logical(&base, &one);
        assert_eq!(ops(&block)[0].0, Opcode::ShiftLeftLogical32);
    }

    #[test]
    fn composite_operations_check_their_bounds() {
        let (block, _) = emitter();
        let mut ir = IrEmitter::new(&block);
        let four = ir.composite_construct4(ir.imm32(1), ir.imm32(2), ir.imm32(3), ir.imm32(4));
        ir.composite_extract(&four, 3);
        let seen: Vec<Opcode> = ops(&block).iter().map(|(op, _)| *op).collect();
        assert_eq!(
            seen,
            vec![
                Opcode::CompositeConstructU32x4,
                Opcode::CompositeExtractU32x4
            ]
        );
        assert_eq!(ops(&block)[1].1[1].u32(), 3, "the element is an immediate");

        let too_big = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut ir = IrEmitter::new(&block);
            ir.composite_extract(&four, 4);
        }));
        assert!(too_big.is_err(), "an out of bounds element is unreachable");
    }

    #[test]
    fn the_packing_operations_follow_the_number_format() {
        let (block, _) = emitter();
        let mut ir = IrEmitter::new(&block);
        let vector = ir.composite_construct(ir.imm32_float(1.0), ir.imm32_float(2.0));
        let value = U32::new(Value::from_u32(0));
        ir.pack_2x16(NumberFormat::Unorm, &vector);
        ir.pack_2x16(NumberFormat::Snorm, &vector);
        ir.pack_2x16(NumberFormat::Uint, &vector);
        ir.pack_2x16(NumberFormat::Sint, &vector);
        ir.pack_2x16(NumberFormat::Float, &vector);
        ir.unpack_2x16(NumberFormat::Float, &value);
        ir.pack_4x8(NumberFormat::Unorm, &vector);
        ir.pack_10_11_11(NumberFormat::Float, &vector);
        ir.pack_2_10_10_10(NumberFormat::Snorm, &vector);
        ir.unpack_2_10_10_10(NumberFormat::Uint, &value);
        ir.unpack_10_11_11(NumberFormat::Float, &value);
        let seen: Vec<Opcode> = ops(&block).iter().map(|(op, _)| *op).collect();
        assert_eq!(
            seen,
            vec![
                Opcode::CompositeConstructF32x2,
                Opcode::PackUnorm2x16,
                Opcode::PackSnorm2x16,
                Opcode::PackUint2x16,
                Opcode::PackSint2x16,
                Opcode::PackHalf2x16,
                Opcode::UnpackHalf2x16,
                Opcode::PackUnorm4x8,
                Opcode::PackUfloat10_11_11,
                Opcode::PackSnorm2_10_10_10,
                Opcode::UnpackUint2_10_10_10,
                Opcode::UnpackUfloat10_11_11,
            ]
        );

        // The formats the C++ has no packing for are unreachable.
        let unsupported = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut ir = IrEmitter::new(&block);
            ir.pack_4x8(NumberFormat::Float, &vector);
        }));
        assert!(unsupported.is_err());
    }

    #[test]
    fn the_shared_operations_pick_the_width() {
        let (block, _) = emitter();
        let mut ir = IrEmitter::new(&block);
        let offset = U32::new(Value::from_u32(0));
        let wide = U32U64::new(Value::from_u64(1));
        let narrow = U32U64::new(Value::from_u32(1));
        ir.load_shared(16, false, &offset, true);
        ir.load_shared(64, false, &offset, false);
        ir.shared_atomic_i_add(&offset, &wide, true);
        ir.shared_atomic_i_min(&offset, &narrow, true, false);
        ir.shared_atomic_inc(&offset, false);
        let seen = ops(&block);
        assert_eq!(
            seen.iter().map(|(op, _)| *op).collect::<Vec<_>>(),
            vec![
                Opcode::LoadSharedU16,
                Opcode::LoadSharedU64,
                Opcode::SharedAtomicIAdd64,
                Opcode::SharedAtomicSMin32,
                Opcode::SharedAtomicInc32,
            ]
        );
        // `is_gds` travels in the flags word.
        assert_eq!(seen[0].0, Opcode::LoadSharedU16);
        assert_eq!(
            block.borrow().instructions[0].borrow().flags_u32(),
            1,
            "the gds flag is the low bit"
        );
        assert_eq!(block.borrow().instructions[1].borrow().flags_u32(), 0);

        let bad_size = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut ir = IrEmitter::new(&block);
            ir.load_shared(8, false, &offset, false);
        }));
        assert!(bad_size.is_err());
        let bad_type = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut ir = IrEmitter::new(&block);
            let f32_value = U32U64::new(Value::from_f32(1.0));
            ir.shared_atomic_and(&offset, &f32_value, false);
        }));
        assert!(bad_type.is_err());
    }

    #[test]
    fn the_buffer_operations_carry_their_info_in_the_flags() {
        let (block, _) = emitter();
        let mut ir = IrEmitter::new(&block);
        let handle = ir.get_scalar_reg(ScalarReg::S0);
        let address = ir.get_vector_reg(VectorReg::V1);
        let mut info = BufferInstInfo::default();
        info.set_index_enable(1);
        let mut flags = info.raw();
        ir.load_buffer_u32(3, &handle.value().clone(), &address.value().clone(), info);
        ir.store_buffer_f32(
            4,
            &handle.value().clone(),
            &address.value().clone(),
            &Value::from_u32(0),
            info,
        );
        ir.buffer_atomic_i_add(
            &handle.value().clone(),
            &address.value().clone(),
            &Value::from_u32(1),
            info,
        );
        let seen: Vec<Opcode> = ops(&block).iter().map(|(op, _)| *op).collect();
        assert_eq!(
            seen,
            vec![
                Opcode::GetScalarRegister,
                Opcode::GetVectorRegister,
                Opcode::LoadBufferU32x3,
                Opcode::StoreBufferF32x4,
                Opcode::BufferAtomicIAdd32,
            ]
        );
        assert_eq!(block.borrow().instructions[2].borrow().flags_u64(), flags);
        assert_eq!(info.raw(), flags, "the same word, read through the getter");
        let _ = &mut flags;
    }

    #[test]
    fn debug_print_pads_its_arguments_with_void() {
        let (block, _) = emitter();
        let mut ir = IrEmitter::new(&block);
        let value = Value::from_u32(1);
        ir.debug_print("value %u", std::slice::from_ref(&value));
        let seen = ops(&block);
        assert_eq!(DEBUGPRINT_NUM_FORMAT_ARGS, 4);
        // The three unused slots are `Void` instructions, then the print itself.
        assert_eq!(
            seen.iter().map(|(op, _)| *op).collect::<Vec<_>>(),
            vec![Opcode::Void, Opcode::Void, Opcode::Void, Opcode::DebugPrint]
        );
        let print = &seen[3];
        assert_eq!(print.1.len(), 1 + DEBUGPRINT_NUM_FORMAT_ARGS);
        assert_eq!(print.1[0].string_literal(), "value %u");
        assert_eq!(print.1[1], value);
        assert_eq!(
            block.borrow().instructions[3].borrow().flags_u32(),
            1,
            "one format argument was given"
        );
    }

    #[test]
    fn the_vertex_and_primitive_operations_have_no_operands() {
        let (block, _) = emitter();
        let mut ir = IrEmitter::new(&block);
        ir.emit_vertex();
        ir.emit_primitive();
        ir.memtime();
        assert_eq!(
            ops(&block).iter().map(|(op, _)| *op).collect::<Vec<_>>(),
            vec![Opcode::EmitVertex, Opcode::EmitPrimitive, Opcode::Memtime]
        );
    }
}
