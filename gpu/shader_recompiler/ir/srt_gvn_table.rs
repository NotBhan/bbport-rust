//! Global value numbering for SRT loads, ported from
//! `legacy/gpu/shadps4/shader_recompiler/ir/srt_gvn_table.h`. Only the instructions that can
//! take part in an offset computation are hashed structurally; everything else gets a fresh
//! number. The C++ combines the instruction vector with `Common::HashCombine`; the port uses
//! the standard `Vec<u32>` hash, which only changes probing, not the numbering.

use crate::gpu::shader_recompiler::ir::microinstruction::InstRef;
use crate::gpu::shader_recompiler::ir::opcodes::Opcode;
use crate::gpu::shader_recompiler::ir::value::Value;
use std::collections::HashMap;

/// `SrtGvnTable`.
#[derive(Default)]
pub struct SrtGvnTable {
    value_numbers: HashMap<Value, u32>,
    inst_vectors: HashMap<Vec<u32>, u32>,
    next_num: u32,
}

impl SrtGvnTable {
    pub fn new() -> Self {
        Self::default()
    }

    /// `GetValueNumber(IR::Inst*)`.
    pub fn get_value_number_inst(&mut self, inst: &InstRef) -> u32 {
        self.get_value_number(&Value::from_inst(inst.clone()))
    }

    /// `GetValueNumber(IR::Value)`.
    pub fn get_value_number(&mut self, value: &Value) -> u32 {
        if let Some(number) = self.value_numbers.get(value) {
            return *number;
        }
        if let Some(inst) = value.try_inst() {
            return self.compute_inst_value_number(&inst);
        }
        self.next_value_number(value)
    }

    /// `IsArgHashInst`: the opcodes whose arguments are hashed instead of the instruction.
    fn is_arg_hash_inst(inst: &InstRef) -> bool {
        let hash = matches!(
            inst.borrow().get_opcode(),
            Opcode::GetUserData
                | Opcode::CompositeConstructU32x2
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
        );
        if hash {
            assert!(!inst.borrow().may_have_side_effects());
        }
        hash
    }

    /// `ComputeInstValueNumber`.
    fn compute_inst_value_number(&mut self, inst: &InstRef) -> u32 {
        assert!(
            !self
                .value_numbers
                .contains_key(&Value::from_inst(inst.clone())),
            "the instruction already has a value number"
        );
        if Self::is_arg_hash_inst(inst) {
            let vector = self.make_inst_vector(inst);
            if let Some(number) = self.inst_vectors.get(&vector) {
                self.value_numbers
                    .insert(Value::from_inst(inst.clone()), *number);
                *number
            } else {
                let number = self.next_value_number(&Value::from_inst(inst.clone()));
                self.inst_vectors.insert(vector, number);
                number
            }
        } else {
            self.next_value_number(&Value::from_inst(inst.clone()))
        }
    }

    /// `NextValueNumber`.
    fn next_value_number(&mut self, value: &Value) -> u32 {
        let number = self.next_num;
        self.next_num += 1;
        self.value_numbers.insert(value.clone(), number);
        number
    }

    /// `MakeInstVector`: the opcode, the flags word and the arguments' value numbers.
    fn make_inst_vector(&mut self, inst: &InstRef) -> Vec<u32> {
        let (opcode, flags, num_args) = {
            let inst = inst.borrow();
            (inst.get_opcode(), inst.flags_u32(), inst.num_args())
        };
        let mut vector = Vec::with_capacity(2 + num_args);
        vector.push(opcode as u32);
        vector.push(flags);
        for index in 0..num_args {
            let arg = inst.borrow().arg(index);
            vector.push(self.get_value_number(&arg));
        }
        vector
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::shader_recompiler::ir::basic_block::Block;
    use crate::gpu::shader_recompiler::ir::microinstruction::Inst;
    use crate::gpu::shader_recompiler::ir::reg::ScalarReg;
    use std::cell::RefCell;
    use std::rc::Rc;

    fn inst(op: Opcode, flags: u64, args: &[Value]) -> InstRef {
        let inst = Rc::new(RefCell::new(Inst::new(op, flags)));
        for (index, arg) in args.iter().enumerate() {
            crate::gpu::shader_recompiler::ir::microinstruction::set_arg(&inst, index, arg.clone());
        }
        inst
    }

    #[test]
    fn identical_offset_instructions_share_a_number() {
        let mut table = SrtGvnTable::new();
        let first = inst(Opcode::IAdd32, 0, &[Value::from_u32(4), Value::from_u32(8)]);
        let second = inst(Opcode::IAdd32, 0, &[Value::from_u32(4), Value::from_u32(8)]);
        let different = inst(Opcode::IAdd32, 0, &[Value::from_u32(4), Value::from_u32(9)]);
        assert_eq!(
            table.get_value_number_inst(&first),
            table.get_value_number_inst(&second)
        );
        assert_ne!(
            table.get_value_number_inst(&first),
            table.get_value_number_inst(&different)
        );
    }

    #[test]
    fn opcodes_flags_and_arguments_all_participate() {
        let mut table = SrtGvnTable::new();
        let add = inst(Opcode::IAdd32, 0, &[Value::from_u32(1), Value::from_u32(2)]);
        let sub = inst(Opcode::ISub32, 0, &[Value::from_u32(1), Value::from_u32(2)]);
        assert_ne!(
            table.get_value_number_inst(&add),
            table.get_value_number_inst(&sub)
        );

        let read_a = inst(
            Opcode::ReadConst,
            3,
            &[Value::from_u32(1), Value::from_u32(0)],
        );
        let read_b = inst(
            Opcode::ReadConst,
            4,
            &[Value::from_u32(1), Value::from_u32(0)],
        );
        assert_ne!(
            table.get_value_number_inst(&read_a),
            table.get_value_number_inst(&read_b)
        );

        let ud_a = inst(
            Opcode::GetUserData,
            0,
            &[Value::from_scalar_reg(ScalarReg::S2)],
        );
        let ud_b = inst(
            Opcode::GetUserData,
            0,
            &[Value::from_scalar_reg(ScalarReg::S2)],
        );
        let ud_c = inst(
            Opcode::GetUserData,
            0,
            &[Value::from_scalar_reg(ScalarReg::S3)],
        );
        assert_eq!(
            table.get_value_number_inst(&ud_a),
            table.get_value_number_inst(&ud_b)
        );
        assert_ne!(
            table.get_value_number_inst(&ud_a),
            table.get_value_number_inst(&ud_c)
        );
    }

    #[test]
    fn other_instructions_get_a_fresh_number_every_time() {
        let mut table = SrtGvnTable::new();
        let first = inst(Opcode::GetScc, 0, &[]);
        let second = inst(Opcode::GetScc, 0, &[]);
        assert_ne!(
            table.get_value_number_inst(&first),
            table.get_value_number_inst(&second)
        );
        assert_eq!(table.get_value_number_inst(&first), 0);
        assert_eq!(table.get_value_number_inst(&second), 1);
        assert_eq!(
            table.get_value_number(&Value::from_u32(5)),
            table.get_value_number(&Value::from_u32(5)),
            "equal immediates share a number"
        );
        assert_ne!(
            table.get_value_number(&Value::from_u32(5)),
            table.get_value_number(&Value::from_u32(6))
        );
    }

    #[test]
    fn repeated_lookups_share_one_number() {
        // The parents are not needed for hashing, only for the pointer map ordering.
        let _ = Block::new_ref();
        let mut table = SrtGvnTable::new();
        let a = inst(Opcode::IAdd32, 0, &[Value::from_u32(1), Value::from_u32(2)]);
        let number = table.get_value_number_inst(&a);
        assert_eq!(table.get_value_number_inst(&a), number);
        assert_eq!(number, 2, "the two immediates are numbered first");
    }
}
