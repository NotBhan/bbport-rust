//! The abstract syntax list, ported from
//! `legacy/gpu/shadps4/shader_recompiler/ir/abstract_syntax_list.h` and its `.cpp`.
//!
//! The C++ node is a type tag beside a union of the shapes a node can take. The port keeps the
//! tag and the shape as one enum, and the dump takes the same pointer-keyed maps `DumpBlock`
//! does.

use super::basic_block::BlockRef;
use super::value::U1;
use std::collections::HashMap;
use std::rc::Rc;

/// `AbstractSyntaxNode::Type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub enum NodeType {
    #[default]
    Block,
    If,
    EndIf,
    Loop,
    Repeat,
    Break,
    Return,
    Unreachable,
}

/// The union's members, one per node type that carries data.
#[derive(Debug, Clone, Default)]
pub enum NodeData {
    /// The union's default state, which no node with a shape uses.
    #[default]
    None,
    Block(BlockRef),
    If {
        cond: U1,
        body: BlockRef,
        merge: BlockRef,
    },
    EndIf {
        merge: BlockRef,
    },
    Loop {
        body: BlockRef,
        continue_block: BlockRef,
        merge: BlockRef,
    },
    Repeat {
        cond: U1,
        loop_header: BlockRef,
        merge: BlockRef,
    },
    Break {
        cond: U1,
        merge: BlockRef,
        skip: BlockRef,
    },
}

/// `Shader::IR::AbstractSyntaxNode`.
#[derive(Debug, Clone, Default)]
pub struct AbstractSyntaxNode {
    pub data: NodeData,
    pub ty: NodeType,
}

impl AbstractSyntaxNode {
    /// A node of the given shape.
    pub fn new(ty: NodeType, data: NodeData) -> Self {
        Self { data, ty }
    }

    /// A plain block node.
    pub fn block(block: BlockRef) -> Self {
        Self::new(NodeType::Block, NodeData::Block(block))
    }
}

/// `Shader::IR::AbstractSyntaxList`.
pub type AbstractSyntaxList = Vec<AbstractSyntaxNode>;

/// `Shader::IR::DumpASLNode`: the same pointer-keyed lookup `DumpBlock` uses, with the C++
/// asserting that every block and instruction has an index.
pub fn dump_asl_node(
    node: &AbstractSyntaxNode,
    block_to_index: &HashMap<usize, usize>,
    inst_to_index: &HashMap<usize, usize>,
) -> String {
    let block = |block: &BlockRef| -> usize {
        *block_to_index
            .get(&(Rc::as_ptr(block) as usize))
            .expect("the block has an index")
    };
    let inst = |cond: &U1| -> usize {
        let inst = cond.value().inst();
        *inst_to_index
            .get(&(Rc::as_ptr(&inst) as usize))
            .expect("the condition has an index")
    };
    match (&node.data, node.ty) {
        (NodeData::Block(block_ref), _) => format!("Block: ${}", block(block_ref)),
        (NodeData::If { cond, body, merge }, _) => format!(
            "If: cond = %{}, body = ${}, merge = ${}",
            inst(cond),
            block(body),
            block(merge)
        ),
        (NodeData::EndIf { merge }, _) => format!("EndIf: merge = ${}", block(merge)),
        (
            NodeData::Loop {
                body,
                continue_block,
                merge,
            },
            _,
        ) => format!(
            "Loop: body = ${}, continue = ${}, merge = ${}",
            block(body),
            block(continue_block),
            block(merge)
        ),
        (
            NodeData::Repeat {
                cond,
                loop_header,
                merge,
            },
            _,
        ) => format!(
            "Repeat: cond = %{}, header = ${}, merge = ${}",
            inst(cond),
            block(loop_header),
            block(merge)
        ),
        (NodeData::Break { cond, merge, skip }, _) => format!(
            "Break: cond = %{}, merge = ${}, skip = ${}",
            inst(cond),
            block(merge),
            block(skip)
        ),
        (NodeData::None, NodeType::Return) => "Return".to_string(),
        (NodeData::None, NodeType::Unreachable) => "Unreachable".to_string(),
        _ => panic!("Unreachable"),
    }
}

#[cfg(test)]
mod tests {
    use super::super::basic_block::Block;
    use super::super::microinstruction::Inst;
    use super::super::opcodes::Opcode;
    use super::super::r#type::Type;
    use super::super::value::Value;
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn a_node_keeps_its_tag_and_its_shape() {
        let block = Block::new_ref();
        let node = AbstractSyntaxNode::block(block.clone());
        assert_eq!(node.ty, NodeType::Block);
        assert!(matches!(node.data, NodeData::Block(_)));
        assert_eq!(NodeType::default(), NodeType::Block);
        let list: AbstractSyntaxList = vec![
            AbstractSyntaxNode::new(NodeType::Return, NodeData::None),
            AbstractSyntaxNode::new(NodeType::Unreachable, NodeData::None),
        ];
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn the_dump_names_blocks_and_conditions() {
        let body = Block::new_ref();
        let merge = Block::new_ref();
        let cond = Rc::new(RefCell::new(Inst::new(Opcode::ConditionRef, 0)));
        let cond_value = U1::new(Value::from_inst(cond.clone()));
        let block_index = HashMap::from([
            (Rc::as_ptr(&body) as usize, 1usize),
            (Rc::as_ptr(&merge) as usize, 2usize),
        ]);
        let inst_index = HashMap::from([(Rc::as_ptr(&cond) as usize, 7usize)]);

        let node = AbstractSyntaxNode::new(
            NodeType::If,
            NodeData::If {
                cond: cond_value.clone(),
                body: body.clone(),
                merge: merge.clone(),
            },
        );
        assert_eq!(
            dump_asl_node(&node, &block_index, &inst_index),
            "If: cond = %7, body = $1, merge = $2"
        );
        assert_eq!(
            dump_asl_node(&AbstractSyntaxNode::block(body), &block_index, &inst_index),
            "Block: $1"
        );
        assert_eq!(
            dump_asl_node(
                &AbstractSyntaxNode::new(NodeType::EndIf, NodeData::EndIf { merge }),
                &block_index,
                &inst_index
            ),
            "EndIf: merge = $2"
        );
        assert_eq!(
            dump_asl_node(
                &AbstractSyntaxNode::new(NodeType::Return, NodeData::None),
                &block_index,
                &inst_index
            ),
            "Return"
        );
        assert_eq!(
            dump_asl_node(
                &AbstractSyntaxNode::new(NodeType::Unreachable, NodeData::None),
                &block_index,
                &inst_index
            ),
            "Unreachable"
        );
        // A condition is whatever instruction the value names.
        let inst = cond_value.value().inst();
        assert_eq!(inst.borrow().get_opcode(), Opcode::ConditionRef);
        assert_eq!(inst.borrow().ty(), Type::U1);
    }

    #[test]
    fn the_loop_and_break_shapes_dump_their_three_blocks() {
        let body = Block::new_ref();
        let other = Block::new_ref();
        let merge = Block::new_ref();
        let skip = Block::new_ref();
        let cond = Rc::new(RefCell::new(Inst::new(Opcode::ConditionRef, 0)));
        let cond_value = U1::new(Value::from_inst(cond.clone()));
        let block_index = HashMap::from([
            (Rc::as_ptr(&body) as usize, 0usize),
            (Rc::as_ptr(&other) as usize, 1usize),
            (Rc::as_ptr(&merge) as usize, 2usize),
            (Rc::as_ptr(&skip) as usize, 3usize),
        ]);
        let inst_index = HashMap::from([(Rc::as_ptr(&cond) as usize, 5usize)]);

        let node = AbstractSyntaxNode::new(
            NodeType::Loop,
            NodeData::Loop {
                body: body.clone(),
                continue_block: other.clone(),
                merge: merge.clone(),
            },
        );
        assert_eq!(
            dump_asl_node(&node, &block_index, &inst_index),
            "Loop: body = $0, continue = $1, merge = $2"
        );
        let node = AbstractSyntaxNode::new(
            NodeType::Repeat,
            NodeData::Repeat {
                cond: cond_value,
                loop_header: body.clone(),
                merge: merge.clone(),
            },
        );
        assert_eq!(
            dump_asl_node(&node, &block_index, &inst_index),
            "Repeat: cond = %5, header = $0, merge = $2"
        );
        let node = AbstractSyntaxNode::new(
            NodeType::Break,
            NodeData::Break {
                cond: U1::new(Value::from_inst(cond)),
                merge,
                skip,
            },
        );
        assert_eq!(
            dump_asl_node(&node, &block_index, &inst_index),
            "Break: cond = %5, merge = $2, skip = $3"
        );
    }
}
