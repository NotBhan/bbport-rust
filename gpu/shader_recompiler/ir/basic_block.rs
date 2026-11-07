//! Basic blocks, ported from `legacy/gpu/shadps4/shader_recompiler/ir/basic_block.h` and
//! `basic_block.cpp`.
//!
//! The C++ holds its instructions in a Boost intrusive list and takes them from an
//! `ObjectPool<Inst>`; the port uses a `Vec` of handles and lets the instruction's own `Rc` own
//! it, so there is no pool to thread through. The GCN link (`cfg_block`) belongs to the
//! unported frontend and is left out until `Shader::Gcn` lands; nothing in the IR reads it.

use super::microinstruction::Inst;
use super::opcodes::{arg_type_of, type_of, Opcode};
use super::r#type::{are_types_compatible, Type};
use super::reg::reg_index;
use super::reg::{ScalarReg, VectorReg};
use super::ssa::{RegTag, RegTagRegister, RegType, SsaState};
use super::value::InstRef;
use super::value::{Value, U1};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// A shared handle to a block, the port's stand-in for `IR::Block*`.
pub type BlockRef = Rc<RefCell<Block>>;

/// `Shader::IR::BlockList`.
pub type BlockList = Vec<BlockRef>;

/// `Shader::IR::Block`.
#[derive(Debug)]
pub struct Block {
    pub ssa_state: SsaState,
    pub branch_cond: U1,
    pub instructions: Vec<InstRef>,
    pub immediate_dominator: Option<BlockRef>,
    pub dominance_frontiers: Vec<BlockRef>,
    pub po_index: u32,
    pub imm_predecessors: Vec<BlockRef>,
    pub imm_successors: Vec<BlockRef>,
    pub definition: u32,
}

impl Block {
    /// The C++'s `Block(Common::ObjectPool<Inst>&)`: the pool is not part of the port's shape.
    pub fn new() -> Self {
        Self {
            ssa_state: SsaState::default(),
            branch_cond: U1::empty(),
            instructions: Vec::new(),
            immediate_dominator: None,
            dominance_frontiers: Vec::new(),
            po_index: 0,
            imm_predecessors: Vec::new(),
            imm_successors: Vec::new(),
            definition: 0,
        }
    }

    /// A fresh block behind a shared handle, which is how every block is used.
    pub fn new_ref() -> BlockRef {
        Rc::new(RefCell::new(Block::new()))
    }

    /// `Block::Instructions`.
    pub fn instructions(&self) -> &[InstRef] {
        &self.instructions
    }

    /// `Block::ImmPredecessors`.
    pub fn imm_predecessors(&self) -> &[BlockRef] {
        &self.imm_predecessors
    }

    /// `Block::ImmSuccessors`.
    pub fn imm_successors(&self) -> &[BlockRef] {
        &self.imm_successors
    }

    pub fn is_empty(&self) -> bool {
        self.instructions.is_empty()
    }

    pub fn len(&self) -> usize {
        self.instructions.len()
    }

    pub fn front(&self) -> Option<InstRef> {
        self.instructions.first().cloned()
    }

    pub fn back(&self) -> Option<InstRef> {
        self.instructions.last().cloned()
    }

    pub fn iter(&self) -> impl Iterator<Item = &InstRef> {
        self.instructions.iter()
    }

    /// `SetDefinition<T>` / `Definition<T>`, over the port's 32-bit word.
    pub fn definition_bits(&self) -> u32 {
        self.definition
    }

    pub fn set_definition_bits(&mut self, definition: u32) {
        self.definition = definition;
    }
}

impl Default for Block {
    fn default() -> Self {
        Self::new()
    }
}

/// `Block::AppendNewInst`.
pub fn append_new_inst(this: &BlockRef, op: Opcode, args: &[Value]) -> usize {
    let point = this.borrow().instructions.len();
    prepend_new_inst(this, point, op, args, 0)
}

/// `Block::PrependNewInst(iterator, Opcode, args, flags)`: the port returns the position the
/// instruction landed at, where the C++ returns the list iterator. The parent link is set from
/// the handle, which is why this is a free function.
pub fn prepend_new_inst(
    this: &BlockRef,
    insertion_point: usize,
    op: Opcode,
    args: &[Value],
    flags: u64,
) -> usize {
    let inst = Rc::new(RefCell::new(Inst::new(op, flags)));
    inst.borrow_mut().set_parent(Rc::downgrade(this));
    assert_eq!(
        inst.borrow().num_args(),
        args.len(),
        "Invalid number of arguments {} in {}",
        args.len(),
        op.name()
    );
    for (index, arg) in args.iter().enumerate() {
        super::microinstruction::set_arg(&inst, index, arg.clone());
    }
    let mut block = this.borrow_mut();
    let point = insertion_point.min(block.instructions.len());
    block.instructions.insert(point, inst);
    point
}

/// `Block::PrependNewInst(iterator, const Inst&)`: inserts a copy of an existing instruction.
pub fn prepend_copy(this: &BlockRef, insertion_point: usize, base_inst: &Inst) -> usize {
    let inst = Rc::new(RefCell::new(Inst::copy_of(base_inst)));
    inst.borrow_mut().set_parent(Rc::downgrade(this));
    let mut block = this.borrow_mut();
    let point = insertion_point.min(block.instructions.len());
    block.instructions.insert(point, inst);
    point
}

/// `Block::AddBranch`: a successor here is a predecessor there, and neither may repeat.
pub fn add_branch(this: &BlockRef, block: &BlockRef) {
    assert!(
        !this
            .borrow()
            .imm_successors
            .iter()
            .any(|successor| Rc::ptr_eq(successor, block)),
        "Successor already inserted"
    );
    assert!(
        !block
            .borrow()
            .imm_predecessors
            .iter()
            .any(|predecessor| Rc::ptr_eq(predecessor, this)),
        "Predecessor already inserted"
    );
    this.borrow_mut().imm_successors.push(block.clone());
    block.borrow_mut().imm_predecessors.push(this.clone());
}

/// `BlockToIndex`.
fn block_index_name(block_to_index: &HashMap<usize, usize>, block: &BlockRef) -> String {
    match block_to_index.get(&(Rc::as_ptr(block) as usize)) {
        Some(index) => format!("{{Block ${index}}}"),
        None => format!("$<unknown block {:016x}>", Rc::as_ptr(block) as usize),
    }
}

/// `InstIndex`.
fn inst_index(
    inst_to_index: &mut HashMap<usize, usize>,
    inst_index_counter: &mut usize,
    inst: &InstRef,
) -> usize {
    let key = Rc::as_ptr(inst) as usize;
    if let Some(index) = inst_to_index.get(&key) {
        return *index;
    }
    *inst_index_counter += 1;
    inst_to_index.insert(key, *inst_index_counter);
    *inst_index_counter
}

/// `ArgToIndex`.
fn arg_to_index(
    inst_to_index: &mut HashMap<usize, usize>,
    inst_index_counter: &mut usize,
    arg: &Value,
) -> String {
    if arg.is_empty() {
        return "<null>".to_string();
    }
    if !arg.is_immediate() {
        return format!(
            "%{}",
            inst_index(inst_to_index, inst_index_counter, &arg.inst())
        );
    }
    match arg.stored_type() {
        Type::U1 => format!("#{}", if arg.u1() { "true" } else { "false" }),
        Type::U8 => format!("#{}", arg.u8()),
        Type::U16 => format!("#{}", arg.u16()),
        Type::U32 => format!("#{}", arg.u32()),
        Type::U64 => format!("#{}", arg.u64()),
        Type::F32 => format!("#{}", arg.f32()),
        other if other == Type::ScalarReg => scalar_name(arg.scalar_reg()),
        other if other == Type::VectorReg => vector_name(arg.vector_reg()),
        other if other == Type::VirtualReg => virtual_name(arg.virtual_reg()),
        other if other == Type::Attribute => attribute_name(arg.attribute()),
        other if other == Type::Patch => patch_name(arg.patch()),
        _ => "<unknown immediate type>".to_string(),
    }
}

fn scalar_name(reg: ScalarReg) -> String {
    format!("SGPR{}", reg_index(reg))
}

fn vector_name(reg: VectorReg) -> String {
    format!("VGPR{}", reg_index(reg))
}

fn virtual_name(reg: super::reg::VirtualReg) -> String {
    format!("REG{}_{}", reg.index, reg.ty)
}

fn attribute_name(attribute: super::attribute::Attribute) -> String {
    super::attribute::name_of(attribute)
}

fn patch_name(patch: super::patch::Patch) -> String {
    patch.name()
}

/// `RegTagInfo`.
fn reg_tag_info(tag: RegTag) -> String {
    match tag.ty {
        RegType::ScalarReg => format!("{}", tag.raw_index()),
        RegType::VectorReg => format!("{}", tag.raw_index()),
        RegType::VirtualReg => match tag.register() {
            RegTagRegister::Virtual(reg) => format!("{}", reg.index),
            _ => format!("{}", tag.raw_index()),
        },
        RegType::GotoVariable => format!("GOTO{}", tag.raw_index()),
        RegType::VccLo => "VCC_LO".to_string(),
        RegType::VccHi => "VCC_HI".to_string(),
        RegType::Exec => "EXEC".to_string(),
        RegType::Scc => "SCC".to_string(),
        RegType::M0 => "M0".to_string(),
        _ => panic!("Unreachable"),
    }
}

/// `Shader::IR::DumpBlock`.
pub fn dump_block(block: &BlockRef) -> String {
    let mut inst_to_index = HashMap::new();
    let mut inst_index_counter = 0;
    dump_block_with(
        block,
        &HashMap::new(),
        &mut inst_to_index,
        &mut inst_index_counter,
    )
}

/// `Shader::IR::DumpBlock` with the maps a caller keeps between blocks.
pub fn dump_block_with(
    block: &BlockRef,
    block_to_index: &HashMap<usize, usize>,
    inst_to_index: &mut HashMap<usize, usize>,
    inst_index_counter: &mut usize,
) -> String {
    let self_key = Rc::as_ptr(block) as usize;
    let block = &block.borrow();
    let mut ret = String::from("Block");
    if let Some(index) = block_to_index.get(&self_key) {
        ret.push_str(&format!(" ${index}"));
    }
    ret.push_str(" predecessors [");
    let predecessors = block.imm_predecessors.clone();
    for predecessor in &predecessors {
        let index = block_to_index
            .get(&(Rc::as_ptr(predecessor) as usize))
            .expect("a predecessor has an index");
        ret.push_str(&format!("${index}, "));
    }
    if !predecessors.is_empty() {
        ret.pop();
        ret.pop();
    }
    ret.push_str("]\n");

    for inst in &block.instructions {
        let inst = inst.clone();
        let borrowed = inst.borrow();
        let op = borrowed.get_opcode();
        ret.push_str(&format!("[{:016x}] ", Rc::as_ptr(&inst) as usize));
        if type_of(op) != Type::Void {
            ret.push_str(&format!(
                "%{:<5} = {}",
                inst_index(inst_to_index, inst_index_counter, &inst),
                op.name()
            ));
        } else {
            ret.push_str(&format!("         {}", op.name()));
        }

        if op == Opcode::ReadConst || op == Opcode::ImageSampleRaw {
            ret.push_str(&format!(" (flags={:#x}) ", borrowed.flags_u32()));
        } else if op == Opcode::ReadConstBuffer {
            let info = borrowed.buffer_info();
            ret.push_str(&format!(
                " (sharp_source={}, flatbuf_off_dw={:#x}) ",
                info.sharp_source() != 0,
                info.flatbuf_off_dw()
            ));
        }
        let arg_count = borrowed.num_args();
        for arg_index in 0..arg_count {
            let arg = borrowed.arg(arg_index);
            let arg_str = arg_to_index(inst_to_index, inst_index_counter, &arg);
            ret.push_str(if arg_index != 0 { ", " } else { " " });
            if op == Opcode::Phi {
                ret.push_str(&format!(
                    "[ {}, {} ]",
                    arg_str,
                    block_index_name(block_to_index, &borrowed.phi_block(arg_index))
                ));
            } else {
                ret.push_str(&arg_str);
            }
            if op != Opcode::Phi {
                let actual = arg.ty();
                let expected = arg_type_of(op, arg_index);
                if !are_types_compatible(actual, expected) {
                    ret.push_str(&format!("<type error: {} != {}>", actual, expected));
                }
            }
        }
        if type_of(op) != Type::Void {
            ret.push_str(&format!(" (uses: {})", borrowed.use_count()));
            let tag = borrowed.reg_tag();
            if tag.is_set() {
                ret.push_str(&format!(" (tag: {})", reg_tag_info(tag)));
            }
            if op == Opcode::Phi {
                ret.push_str(&format!(" (scc{})", borrowed.scc_index));
            }
        }
        ret.push('\n');
    }
    ret
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instructions_are_appended_and_owned_by_their_block() {
        let block = Block::new_ref();
        assert!(block.borrow().is_empty());
        let first = append_new_inst(
            &block,
            Opcode::ReadConst,
            &[Value::from_u32(0), Value::from_u32(0)],
        );
        let second = append_new_inst(
            &block,
            Opcode::IAdd32,
            &[Value::from_u32(1), Value::from_u32(2)],
        );
        assert_eq!(first, 0);
        assert_eq!(second, 1);
        assert_eq!(block.borrow().len(), 2);
        assert_eq!(
            block.borrow().front().expect("front").borrow().get_opcode(),
            Opcode::ReadConst
        );
        assert_eq!(
            block.borrow().back().expect("back").borrow().get_opcode(),
            Opcode::IAdd32
        );
        assert_eq!(block.borrow().iter().count(), 2);

        // An instruction inserted at a point lands there.
        let middle = prepend_new_inst(&block, 1, Opcode::Barrier, &[], 0);
        assert_eq!(middle, 1);
        assert_eq!(
            block.borrow().instructions[1].borrow().get_opcode(),
            Opcode::Barrier
        );
        // The parent link points back at the block.
        let inst = block.borrow().front().expect("front");
        assert!(Rc::ptr_eq(
            &inst.borrow().parent().expect("a parent"),
            &block
        ));

        // A copied instruction takes the opcode and arguments, and no uses.
        let copy_point = prepend_copy(&block, 0, &inst.borrow());
        assert_eq!(copy_point, 0);
        assert_eq!(block.borrow().len(), 4);
        assert_eq!(block.borrow().instructions[0].borrow().use_count(), 0);
    }

    #[test]
    fn a_wrong_argument_count_is_refused() {
        let block = Block::new_ref();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            append_new_inst(&block, Opcode::ReadConst, &[Value::from_u32(1)]);
        }));
        assert!(result.is_err(), "ReadConst takes two arguments");
    }

    #[test]
    fn branches_record_both_sides_and_refuse_repeats() {
        let entry = Block::new_ref();
        let left = Block::new_ref();
        let right = Block::new_ref();
        add_branch(&entry, &left);
        add_branch(&entry, &right);
        assert_eq!(entry.borrow().imm_successors().len(), 2);
        assert_eq!(left.borrow().imm_predecessors().len(), 1);
        assert_eq!(right.borrow().imm_predecessors().len(), 1);
        assert!(Rc::ptr_eq(&left.borrow().imm_predecessors()[0], &entry));

        let repeat = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            add_branch(&entry, &left);
        }));
        assert!(repeat.is_err(), "a successor is recorded once");

        // A diamond closes back on the join block.
        let join = Block::new_ref();
        add_branch(&left, &join);
        add_branch(&right, &join);
        assert_eq!(join.borrow().imm_predecessors().len(), 2);
        assert_eq!(join.borrow().imm_successors().len(), 0);
    }

    #[test]
    fn a_block_dumps_its_instructions_and_arguments() {
        let block = Block::new_ref();
        let producer = Rc::new(RefCell::new(Inst::new(Opcode::ReadConst, 0)));
        producer.borrow_mut().set_parent_handle(&block);
        block.borrow_mut().instructions.push(producer.clone());
        let consumer = Rc::new(RefCell::new(Inst::new(Opcode::IAdd32, 0)));
        consumer.borrow_mut().set_parent_handle(&block);
        super::super::microinstruction::set_arg(&consumer, 0, Value::from_inst(producer.clone()));
        super::super::microinstruction::set_arg(&consumer, 1, Value::from_u32(7));
        block.borrow_mut().instructions.push(consumer);
        append_new_inst(&block, Opcode::Barrier, &[]);

        let index = HashMap::from([(Rc::as_ptr(&block) as usize, 3usize)]);
        let mut inst_to_index = HashMap::new();
        let mut counter = 0;
        let text = dump_block_with(&block, &index, &mut inst_to_index, &mut counter);
        assert!(text.starts_with("Block $3 predecessors []\n"), "{text}");
        assert!(text.contains("ReadConst"), "{text}");
        assert!(text.contains("(uses: 1)"), "{text}");
        assert!(text.contains(", #7"), "{text}");
        assert!(text.contains("Barrier"), "{text}");
        // The producer was left with empty arguments, which the C++ reports as a type error
        // against ReadConst's two argument types.
        assert!(text.contains("<type error: Void != U32x2>"), "{text}");
        let add_line = text
            .lines()
            .find(|line| line.contains("IAdd32"))
            .expect("the add line");
        assert!(!add_line.contains("type error"), "{add_line}");
        assert!(dump_block(&block).starts_with("Block predecessors ["));
        assert!(counter >= 2);

        // A phi argument prints the block it comes from.
        let other = Block::new_ref();
        let phi = Rc::new(RefCell::new(Inst::new(Opcode::Phi, 0)));
        phi.borrow_mut().set_type_flags(Type::F32);
        super::super::microinstruction::add_phi_operand(&phi, &other, Value::from_u32(1));
        block.borrow_mut().instructions.push(phi);
        let index = HashMap::from([
            (Rc::as_ptr(&block) as usize, 0usize),
            (Rc::as_ptr(&other) as usize, 1usize),
        ]);
        let mut counter = 0;
        let text = dump_block_with(&block, &index, &mut inst_to_index, &mut counter);
        assert!(text.contains("[ #1, {Block $1} ]"), "{text}");
        assert!(text.contains("(scc0)"), "{text}");
    }
}
