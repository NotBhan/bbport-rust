//! The structured control flow builder, ported from
//! `legacy/gpu/shadps4/shader_recompiler/frontend/structured_control_flow.{h,cpp}`.
//!
//! The C++ keeps its statement tree in a Boost intrusive list, with `Statement*` parents and list
//! iterators as node handles. The port uses a shared handle per statement, a `Vec` of children,
//! and a weak parent link so a tree drops as a whole. Where the C++ leaves a removed statement
//! in its object pool, the port drops its handle; anything else still naming the statement keeps
//! it alive. `GotoPass` is the algorithm from Erosa and Hendren, "Taming Control Flow: A
//! Structured Approach to Eliminating Goto Statements"; `TranslatePass` turns the resulting tree
//! into the abstract syntax list. The C++'s `AreSiblings` and the commented-out check that used
//! it are dead code and are not ported.

use crate::gpu::shader_recompiler::frontend::control_flow_graph::{
    Block as GcnBlock, BlockRef as GcnBlockRef, EndClass, CFG,
};
use crate::gpu::shader_recompiler::info::Info;
use crate::gpu::shader_recompiler::ir::abstract_syntax_list::{
    AbstractSyntaxList, AbstractSyntaxNode, NodeData, NodeType,
};
use crate::gpu::shader_recompiler::ir::basic_block::{
    add_branch, Block as IrBlock, BlockRef as IrBlockRef,
};
use crate::gpu::shader_recompiler::ir::condition::Condition;
use crate::gpu::shader_recompiler::ir::ir_emitter::IrEmitter;
use crate::gpu::shader_recompiler::ir::microinstruction::invalidate;
use crate::gpu::shader_recompiler::ir::value::{Value, U1};
use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Range;
use std::rc::{Rc, Weak};

/// A shared statement handle, the port's stand-in for the C++'s `Statement*` and tree iterators.
type Node = Rc<RefCell<Statement>>;

/// `StatementType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
enum StatementType {
    Code = 0,
    Goto,
    Label,
    If,
    Loop,
    Break,
    Return,
    Unreachable,
    Function,
    Identity,
    Not,
    Or,
    SetVariable,
    Variable,
}

/// The C++ `Statement`'s type tag and the union member that goes with it. The port keeps the two
/// together, so a shape can only be read as what it is.
enum StatementData {
    Code(GcnBlockRef),
    Goto {
        cond: Node,
        label: Node,
    },
    Label(u32),
    If {
        cond: Node,
        children: Vec<Node>,
    },
    Loop {
        cond: Node,
        children: Vec<Node>,
    },
    Break {
        cond: Node,
    },
    Return,
    #[allow(dead_code)]
    Unreachable,
    Function {
        children: Vec<Node>,
    },
    Identity {
        cond_ref: U1,
        guest_cond: Condition,
    },
    Not {
        op: Node,
    },
    Or {
        op_a: Node,
        op_b: Node,
    },
    SetVariable {
        id: u32,
        op: Node,
    },
    Variable(u32),
}

/// `Statement`.
struct Statement {
    data: StatementData,
    up: Option<Weak<RefCell<Statement>>>,
}

impl Statement {
    /// A statement behind a shared handle, adopting the children it was given.
    fn new(data: StatementData, up: Option<&Node>) -> Node {
        let node = Rc::new(RefCell::new(Statement {
            data,
            up: up.map(Rc::downgrade),
        }));
        update_tree_up(&node);
        node
    }

    fn ty(&self) -> StatementType {
        match self.data {
            StatementData::Code(_) => StatementType::Code,
            StatementData::Goto { .. } => StatementType::Goto,
            StatementData::Label(_) => StatementType::Label,
            StatementData::If { .. } => StatementType::If,
            StatementData::Loop { .. } => StatementType::Loop,
            StatementData::Break { .. } => StatementType::Break,
            StatementData::Return => StatementType::Return,
            StatementData::Unreachable => StatementType::Unreachable,
            StatementData::Function { .. } => StatementType::Function,
            StatementData::Identity { .. } => StatementType::Identity,
            StatementData::Not { .. } => StatementType::Not,
            StatementData::Or { .. } => StatementType::Or,
            StatementData::SetVariable { .. } => StatementType::SetVariable,
            StatementData::Variable(_) => StatementType::Variable,
        }
    }

    /// `HasChildren`.
    fn has_children(&self) -> bool {
        matches!(
            self.data,
            StatementData::If { .. } | StatementData::Loop { .. } | StatementData::Function { .. }
        )
    }

    fn up(&self) -> Option<Node> {
        self.up.as_ref().and_then(Weak::upgrade)
    }
}

/// `UpdateTreeUp`: a parent adopts its direct children.
fn update_tree_up(parent: &Node) {
    let mut statement = parent.borrow_mut();
    let up = Rc::downgrade(parent);
    let children = match &mut statement.data {
        StatementData::If { children, .. }
        | StatementData::Loop { children, .. }
        | StatementData::Function { children } => children,
        _ => return,
    };
    for child in children {
        child.borrow_mut().up = Some(up.clone());
    }
}

fn with_children<R>(node: &Node, f: impl FnOnce(&[Node]) -> R) -> R {
    let statement = node.borrow();
    let children = match &statement.data {
        StatementData::If { children, .. }
        | StatementData::Loop { children, .. }
        | StatementData::Function { children } => children,
        _ => panic!("statement has no children"),
    };
    f(children)
}

fn with_children_mut<R>(node: &Node, f: impl FnOnce(&mut Vec<Node>) -> R) -> R {
    let mut statement = node.borrow_mut();
    let children = match &mut statement.data {
        StatementData::If { children, .. }
        | StatementData::Loop { children, .. }
        | StatementData::Function { children } => children,
        _ => panic!("statement has no children"),
    };
    f(children)
}

fn children_len(node: &Node) -> usize {
    with_children(node, |children| children.len())
}

fn child_at(node: &Node, index: usize) -> Node {
    with_children(node, |children| children[index].clone())
}

fn position(parent: &Node, child: &Node) -> Option<usize> {
    with_children(parent, |children| {
        children
            .iter()
            .position(|candidate| Rc::ptr_eq(candidate, child))
    })
}

fn insert_child(parent: &Node, index: usize, child: Node) -> Node {
    child.borrow_mut().up = Some(Rc::downgrade(parent));
    let inserted = child.clone();
    with_children_mut(parent, |children| children.insert(index, child));
    inserted
}

fn remove_child(parent: &Node, index: usize) -> Node {
    with_children_mut(parent, |children| children.remove(index))
}

fn drain_children(parent: &Node, range: Range<usize>) -> Vec<Node> {
    with_children_mut(parent, |children| children.drain(range).collect())
}

fn code_block(node: &Node) -> GcnBlockRef {
    match &node.borrow().data {
        StatementData::Code(block) => block.clone(),
        _ => panic!("the statement is not a code block"),
    }
}

fn label_id_of(node: &Node) -> u32 {
    match &node.borrow().data {
        StatementData::Label(id) => *id,
        _ => panic!("the statement is not a label"),
    }
}

fn goto_cond(node: &Node) -> Node {
    match &node.borrow().data {
        StatementData::Goto { cond, .. } => cond.clone(),
        _ => panic!("the statement is not a goto"),
    }
}

fn goto_label(node: &Node) -> Node {
    match &node.borrow().data {
        StatementData::Goto { label, .. } => label.clone(),
        _ => panic!("the statement is not a goto"),
    }
}

/// `Level`.
fn level(stmt: &Node) -> usize {
    let mut level = 0;
    let mut current = stmt.borrow().up();
    while let Some(node) = current {
        level += 1;
        current = node.borrow().up();
    }
    level
}

fn same_weak(a: &Option<Weak<RefCell<Statement>>>, b: &Option<Weak<RefCell<Statement>>>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => Weak::ptr_eq(a, b),
        (None, None) => true,
        _ => false,
    }
}

fn same_parent(a: &Node, b: &Node) -> bool {
    same_weak(&a.borrow().up, &b.borrow().up)
}

/// `IsDirectlyRelated`.
fn is_directly_related(goto_stmt: &Node, label_stmt: &Node) -> bool {
    let goto_level = level(goto_stmt);
    let label_level = level(label_stmt);
    let (min_level, mut max_level, min, mut max) = if label_level < goto_level {
        (
            label_level,
            goto_level,
            label_stmt.clone(),
            goto_stmt.clone(),
        )
    } else {
        (
            goto_level,
            label_level,
            goto_stmt.clone(),
            label_stmt.clone(),
        )
    };
    while max_level > min_level {
        max_level -= 1;
        let up = max.borrow().up().expect("the statement has a parent");
        max = up;
    }
    same_parent(&min, &max)
}

/// `IsIndirectlyRelated`.
fn is_indirectly_related(goto_stmt: &Node, label_stmt: &Node) -> bool {
    !same_parent(goto_stmt, label_stmt) && !is_directly_related(goto_stmt, label_stmt)
}

/// `AreOrdered`: `left` is anywhere before `right` in the same list.
fn are_ordered(left: &Node, right: &Node) -> bool {
    let parent = right.borrow().up().expect("the statement has a parent");
    let start = position(&parent, right).expect("the statement is in its parent");
    with_children(&parent, |children| {
        !children[start..]
            .iter()
            .any(|child| Rc::ptr_eq(child, left))
    })
}

/// `SiblingFromNephew`.
fn sibling_from_nephew(uncle: &Node, nephew: &Node) -> Node {
    let parent = uncle.borrow().up.clone();
    let mut it = nephew.clone();
    loop {
        if same_weak(&it.borrow().up, &parent) {
            return it;
        }
        let up = it
            .borrow()
            .up()
            .expect("the nephew is below the uncle's parent");
        it = up;
    }
}

/// `NeedsLift`.
fn needs_lift(goto_stmt: &Node, label_stmt: &Node) -> bool {
    let sibling = sibling_from_nephew(goto_stmt, label_stmt);
    are_ordered(&sibling, goto_stmt)
}

/// `TryFindForwardBlock`: the first code statement, stopping at one that has children.
fn try_find_forward_block(children: &[Node], start: usize) -> Option<Node> {
    for node in &children[start..] {
        let statement = node.borrow();
        if statement.has_children() {
            break;
        }
        if statement.ty() == StatementType::Code {
            return Some(node.clone());
        }
    }
    None
}

/// `SanitizeNoBreaks`: only the direct children are inspected.
fn sanitize_no_breaks(children: &[Node]) {
    assert!(
        !children
            .iter()
            .any(|child| child.borrow().ty() == StatementType::Break),
        "Capturing statement with break nodes"
    );
}

/// `DumpExpr`.
#[allow(dead_code)]
fn dump_expr(stmt: &Node) -> String {
    match &stmt.borrow().data {
        StatementData::Identity { guest_cond, .. } => format!("{guest_cond}"),
        StatementData::Not { op } => format!("!{}", dump_expr(op)),
        StatementData::Or { op_a, op_b } => {
            format!("{} || {}", dump_expr(op_a), dump_expr(op_b))
        }
        StatementData::Variable(id) => format!("goto_L{id}"),
        _ => "<invalid type>".to_string(),
    }
}

/// `DumpTree`.
#[allow(dead_code)]
fn dump_tree(tree: &[Node], indentation: usize) -> String {
    let indent = " ".repeat(indentation);
    let mut ret = String::new();
    for stmt in tree {
        let statement = stmt.borrow();
        match &statement.data {
            StatementData::Code(gcn_block) => {
                let address = Rc::as_ptr(gcn_block) as usize;
                let block = gcn_block.borrow();
                ret += &format!(
                    "{indent}    Block {:04x} -> {:04x} (0x{address:016x});\n",
                    block.begin, block.end
                );
            }
            StatementData::Goto { cond, label } => {
                ret += &format!(
                    "{indent}    if ({}) goto L{};\n",
                    dump_expr(cond),
                    label_id_of(label)
                );
            }
            StatementData::Label(id) => {
                ret += &format!("{indent}L{id}:\n");
            }
            StatementData::If { cond, children } => {
                ret += &format!("{indent}    if ({}) {{\n", dump_expr(cond));
                ret += &dump_tree(children, indentation + 4);
                ret += &format!("{indent}    }}\n");
            }
            StatementData::Loop { cond, children } => {
                ret += &format!("{indent}    do {{\n");
                ret += &dump_tree(children, indentation + 4);
                ret += &format!("{indent}    }} while ({});\n", dump_expr(cond));
            }
            StatementData::Break { cond } => {
                ret += &format!("{indent}    if ({}) break;\n", dump_expr(cond));
            }
            StatementData::Return => {
                ret += &format!("{indent}    return;\n");
            }
            StatementData::Unreachable => {
                ret += &format!("{indent}    unreachable;\n");
            }
            StatementData::SetVariable { id, op } => {
                ret += &format!("{indent}    goto_L{id} = {};\n", dump_expr(op));
            }
            StatementData::Function { .. }
            | StatementData::Identity { .. }
            | StatementData::Not { .. }
            | StatementData::Or { .. }
            | StatementData::Variable(_) => {
                panic!("Statement can't be printed");
            }
        }
    }
    ret
}

/// `GotoPass`.
struct GotoPass {
    root: Node,
}

impl GotoPass {
    fn new(cfg: &CFG) -> Self {
        let root = Statement::new(
            StatementData::Function {
                children: Vec::new(),
            },
            None,
        );
        let mut pass = GotoPass { root };
        let gotos = pass.build_tree(cfg);
        for goto_stmt in gotos.iter().rev() {
            pass.remove_goto(goto_stmt.clone());
        }
        pass
    }

    fn remove_goto(&mut self, mut goto_stmt: Node) {
        let label_stmt = goto_label(&goto_stmt);
        if is_indirectly_related(&goto_stmt, &label_stmt) {
            while !is_directly_related(&goto_stmt, &label_stmt) {
                goto_stmt = self.move_outward(&goto_stmt);
            }
        }
        if is_directly_related(&goto_stmt, &label_stmt) {
            let label_level = level(&label_stmt);
            let mut goto_level = level(&goto_stmt);
            if goto_level > label_level {
                while goto_level > label_level {
                    goto_stmt = self.move_outward(&goto_stmt);
                    goto_level -= 1;
                }
            } else {
                if needs_lift(&goto_stmt, &label_stmt) {
                    goto_stmt = self.lift(&goto_stmt);
                    // The C++ does not refresh goto_level after the lift; keep the stale count.
                }
                while goto_level < label_level {
                    goto_stmt = self.move_inward(&goto_stmt);
                    goto_level += 1;
                }
            }
        }
        let parent = goto_stmt.borrow().up().expect("the goto has a parent");
        let index = position(&parent, &goto_stmt).expect("the goto is in its parent");
        let next_is_label = with_children(&parent, |children| {
            children
                .get(index + 1)
                .is_some_and(|next| Rc::ptr_eq(next, &label_stmt))
        });
        if next_is_label {
            remove_child(&parent, index);
        } else if are_ordered(&goto_stmt, &label_stmt) {
            self.eliminate_as_conditional(&goto_stmt, &label_stmt);
        } else {
            self.eliminate_as_loop(&goto_stmt, &label_stmt);
        }
    }

    fn build_tree(&mut self, cfg: &CFG) -> Vec<Node> {
        let root = self.root.clone();
        let false_stmt = Statement::new(
            StatementData::Identity {
                cond_ref: U1::new(Value::from_u1(false)),
                guest_cond: Condition::False,
            },
            Some(&root),
        );
        let mut gotos = Vec::new();
        let mut local_labels = HashMap::with_capacity(cfg.blocks.len());

        for (label_id, block) in cfg.blocks.iter().enumerate() {
            let label = Statement::new(StatementData::Label(label_id as u32), Some(&root));
            with_children_mut(&root, |children| children.push(label.clone()));
            local_labels.insert(Rc::as_ptr(block) as usize, label);
        }

        for block in &cfg.blocks {
            let label = local_labels
                .get(&(Rc::as_ptr(block) as usize))
                .expect("every block has a label")
                .clone();
            let id = label_id_of(&label);

            let front_reset = Statement::new(
                StatementData::SetVariable {
                    id,
                    op: false_stmt.clone(),
                },
                Some(&root),
            );
            with_children_mut(&root, |children| children.insert(0, front_reset));

            let mut insert_point = position(&root, &label).expect("the label is in the tree") + 1;
            let reset = Statement::new(
                StatementData::SetVariable {
                    id,
                    op: false_stmt.clone(),
                },
                Some(&root),
            );
            insert_child(&root, insert_point, reset);
            insert_point += 1;

            let code = Statement::new(StatementData::Code(block.clone()), Some(&root));
            insert_child(&root, insert_point, code);
            insert_point += 1;

            let end_class = block.borrow().end_class;
            let cond = block.borrow().cond;
            match end_class {
                EndClass::Branch => {
                    let always_cond = Statement::new(
                        StatementData::Identity {
                            cond_ref: U1::new(Value::from_u1(true)),
                            guest_cond: Condition::True,
                        },
                        Some(&root),
                    );
                    if cond == Condition::True {
                        let target = block.borrow().branch_true.clone().expect("a true branch");
                        let label = local_labels[&(Rc::as_ptr(&target) as usize)].clone();
                        let goto = Statement::new(
                            StatementData::Goto {
                                cond: always_cond,
                                label,
                            },
                            Some(&root),
                        );
                        insert_child(&root, insert_point, goto.clone());
                        gotos.push(goto);
                    } else if cond == Condition::False {
                        let target = block.borrow().branch_false.clone().expect("a false branch");
                        let label = local_labels[&(Rc::as_ptr(&target) as usize)].clone();
                        let goto = Statement::new(
                            StatementData::Goto {
                                cond: always_cond,
                                label,
                            },
                            Some(&root),
                        );
                        insert_child(&root, insert_point, goto.clone());
                        gotos.push(goto);
                    } else {
                        let ir_block = block
                            .borrow()
                            .ir_block
                            .clone()
                            .expect("the block was translated");
                        let value = ir_block.borrow().branch_cond.clone().into_value();
                        assert!(!value.is_empty());
                        let cond_ref = value.inst();
                        let argument = cond_ref.borrow().arg(0);
                        let true_target =
                            block.borrow().branch_true.clone().expect("a true branch");
                        let true_label = local_labels[&(Rc::as_ptr(&true_target) as usize)].clone();
                        let false_target =
                            block.borrow().branch_false.clone().expect("a false branch");
                        let false_label =
                            local_labels[&(Rc::as_ptr(&false_target) as usize)].clone();
                        let true_cond = Statement::new(
                            StatementData::Identity {
                                cond_ref: U1::new(argument),
                                guest_cond: cond,
                            },
                            Some(&root),
                        );
                        invalidate(&cond_ref);
                        let goto_true = Statement::new(
                            StatementData::Goto {
                                cond: true_cond,
                                label: true_label,
                            },
                            Some(&root),
                        );
                        insert_child(&root, insert_point, goto_true.clone());
                        gotos.push(goto_true);
                        let goto_false = Statement::new(
                            StatementData::Goto {
                                cond: always_cond,
                                label: false_label,
                            },
                            Some(&root),
                        );
                        insert_child(&root, insert_point + 1, goto_false.clone());
                        gotos.push(goto_false);
                    }
                }
                EndClass::Exit => {
                    let ret = Statement::new(StatementData::Return, Some(&root));
                    insert_child(&root, insert_point, ret);
                }
            }
        }
        gotos
    }

    fn eliminate_as_conditional(&mut self, goto_stmt: &Node, label_stmt: &Node) {
        let parent = goto_stmt.borrow().up().expect("the goto has a parent");
        let (goto_index, label_index) = positions(&parent, goto_stmt, label_stmt);
        let if_body = drain_children(&parent, goto_index + 1..label_index);
        let cond = Statement::new(
            StatementData::Not {
                op: goto_cond(goto_stmt),
            },
            Some(&self.root),
        );
        let if_stmt = Statement::new(
            StatementData::If {
                cond,
                children: if_body,
            },
            Some(&parent),
        );
        insert_child(&parent, goto_index, if_stmt);
        remove_child(&parent, goto_index + 1);
    }

    fn eliminate_as_loop(&mut self, goto_stmt: &Node, label_stmt: &Node) {
        let parent = goto_stmt.borrow().up().expect("the goto has a parent");
        let (goto_index, label_index) = positions(&parent, goto_stmt, label_stmt);
        let loop_body = drain_children(&parent, label_index..goto_index);
        let cond = goto_cond(goto_stmt);
        let loop_stmt = Statement::new(
            StatementData::Loop {
                cond,
                children: loop_body,
            },
            Some(&parent),
        );
        insert_child(&parent, label_index, loop_stmt);
        remove_child(&parent, label_index + 1);
    }

    fn move_outward(&mut self, goto_stmt: &Node) -> Node {
        let parent = goto_stmt.borrow().up().expect("the goto has a parent");
        let ty = parent.borrow().ty();
        match ty {
            StatementType::If => self.move_outward_if(goto_stmt, &parent),
            StatementType::Loop => self.move_outward_loop(goto_stmt, &parent),
            _ => panic!("Invalid outward movement"),
        }
    }

    fn move_inward(&mut self, goto_stmt: &Node) -> Node {
        let parent = goto_stmt.borrow().up().expect("the goto has a parent");
        let label = goto_label(goto_stmt);
        let nested = sibling_from_nephew(goto_stmt, &label);
        let id = label_id_of(&label);

        let goto_index = position(&parent, goto_stmt).expect("the goto is in its parent");
        let set_var = Statement::new(
            StatementData::SetVariable {
                id,
                op: goto_cond(goto_stmt),
            },
            Some(&parent),
        );
        insert_child(&parent, goto_index, set_var);

        let nested_index = position(&parent, &nested).expect("the sibling is in its parent");
        let if_body = drain_children(&parent, goto_index + 2..nested_index);
        let variable = Statement::new(StatementData::Variable(id), Some(&self.root));
        let mut goto_index = goto_index + 1;
        if !if_body.is_empty() {
            let neg_var = Statement::new(
                StatementData::Not {
                    op: variable.clone(),
                },
                Some(&self.root),
            );
            let if_stmt = Statement::new(
                StatementData::If {
                    cond: neg_var,
                    children: if_body,
                },
                Some(&parent),
            );
            insert_child(&parent, goto_index, if_stmt);
            goto_index += 1;
        }
        remove_child(&parent, goto_index);

        match &mut nested.borrow_mut().data {
            StatementData::If { cond, .. } => {
                let old_cond = cond.clone();
                let new_cond = Statement::new(
                    StatementData::Or {
                        op_a: variable.clone(),
                        op_b: old_cond,
                    },
                    Some(&self.root),
                );
                *cond = new_cond;
            }
            StatementData::Loop { .. } => {}
            _ => panic!("Invalid inward movement"),
        }

        let new_goto = Statement::new(
            StatementData::Goto {
                cond: variable,
                label,
            },
            Some(&nested),
        );
        insert_child(&nested, 0, new_goto.clone());
        new_goto
    }

    fn lift(&mut self, goto_stmt: &Node) -> Node {
        let parent = goto_stmt.borrow().up().expect("the goto has a parent");
        let label = goto_label(goto_stmt);
        let id = label_id_of(&label);
        let nested = sibling_from_nephew(goto_stmt, &label);

        let (nested_index, goto_index) = positions(&parent, &nested, goto_stmt);
        let loop_body = drain_children(&parent, nested_index..goto_index);
        sanitize_no_breaks(&loop_body);

        let variable = Statement::new(StatementData::Variable(id), Some(&self.root));
        let loop_stmt = Statement::new(
            StatementData::Loop {
                cond: variable.clone(),
                children: loop_body,
            },
            Some(&parent),
        );
        insert_child(&parent, nested_index, loop_stmt.clone());
        remove_child(&parent, nested_index + 1);

        let new_goto = Statement::new(
            StatementData::Goto {
                cond: variable,
                label,
            },
            Some(&loop_stmt),
        );
        insert_child(&loop_stmt, 0, new_goto.clone());
        let set_var = Statement::new(
            StatementData::SetVariable {
                id,
                op: goto_cond(goto_stmt),
            },
            Some(&loop_stmt),
        );
        let last = children_len(&loop_stmt);
        insert_child(&loop_stmt, last, set_var);
        new_goto
    }

    fn move_outward_if(&mut self, goto_stmt: &Node, parent: &Node) -> Node {
        let (cond, label) = (goto_cond(goto_stmt), goto_label(goto_stmt));
        let id = label_id_of(&label);
        let goto_index = position(parent, goto_stmt).expect("the goto is in its parent");
        let set_var = Statement::new(StatementData::SetVariable { id, op: cond }, Some(parent));
        insert_child(parent, goto_index, set_var);

        let end = children_len(parent);
        let mut if_body = drain_children(parent, goto_index + 2..end);
        if_body.remove(0);

        let variable = Statement::new(StatementData::Variable(id), Some(&self.root));
        let neg_cond = Statement::new(StatementData::Not { op: variable }, Some(&self.root));
        let if_stmt = Statement::new(
            StatementData::If {
                cond: neg_cond,
                children: if_body,
            },
            Some(parent),
        );
        insert_child(parent, goto_index + 1, if_stmt);
        remove_child(parent, goto_index + 2);

        let parent_up = parent.borrow().up().expect("the if has a parent");
        let new_cond = Statement::new(StatementData::Variable(id), Some(&self.root));
        let new_goto = Statement::new(
            StatementData::Goto {
                cond: new_cond,
                label,
            },
            Some(&parent_up),
        );
        let parent_index = position(&parent_up, parent).expect("the if is in its parent");
        insert_child(&parent_up, parent_index + 1, new_goto.clone());
        new_goto
    }

    fn move_outward_loop(&mut self, goto_stmt: &Node, parent: &Node) -> Node {
        let (cond, label) = (goto_cond(goto_stmt), goto_label(goto_stmt));
        let id = label_id_of(&label);
        let goto_index = position(parent, goto_stmt).expect("the goto is in its parent");
        let set_var = Statement::new(StatementData::SetVariable { id, op: cond }, Some(parent));
        insert_child(parent, goto_index, set_var);
        let break_cond = Statement::new(StatementData::Variable(id), Some(&self.root));
        let break_stmt = Statement::new(StatementData::Break { cond: break_cond }, Some(parent));
        insert_child(parent, goto_index + 1, break_stmt);
        remove_child(parent, goto_index + 2);

        let parent_up = parent.borrow().up().expect("the loop has a parent");
        let new_cond = Statement::new(StatementData::Variable(id), Some(&self.root));
        let new_goto = Statement::new(
            StatementData::Goto {
                cond: new_cond,
                label,
            },
            Some(&parent_up),
        );
        let parent_index = position(&parent_up, parent).expect("the loop is in its parent");
        insert_child(&parent_up, parent_index + 1, new_goto.clone());
        new_goto
    }
}

fn positions(parent: &Node, first: &Node, second: &Node) -> (usize, usize) {
    with_children(parent, |children| {
        let first_index = children
            .iter()
            .position(|child| Rc::ptr_eq(child, first))
            .expect("the statement is in its parent");
        let second_index = children
            .iter()
            .position(|child| Rc::ptr_eq(child, second))
            .expect("the statement is in its parent");
        (first_index, second_index)
    })
}

/// `TranslatePass`.
struct TranslatePass {
    syntax_list: AbstractSyntaxList,
}

impl TranslatePass {
    fn new(root_stmt: &Node) -> Self {
        let mut pass = TranslatePass {
            syntax_list: Vec::new(),
        };
        pass.visit(root_stmt, None, None);
        pass
    }

    /// `TranslatePass::Visit`.
    fn visit(
        &mut self,
        parent: &Node,
        break_block: Option<&IrBlockRef>,
        fallthrough_block: Option<&IrBlockRef>,
    ) {
        let first = with_children(parent, |children| try_find_forward_block(children, 0));
        let (mut current_block, mut insert_point) = match first {
            Some(code_stmt) => {
                let block = code_block(&code_stmt);
                let ir_block = block
                    .borrow()
                    .ir_block
                    .clone()
                    .expect("the block was translated");
                (Some(ir_block), Some(0))
            }
            None => {
                let block = IrBlock::new_ref();
                self.syntax_list
                    .push(AbstractSyntaxNode::block(block.clone()));
                (Some(block), None)
            }
        };

        let mut index = 0;
        while index < children_len(parent) {
            let stmt = child_at(parent, index);
            index += 1;
            let statement_type = stmt.borrow().ty();
            match statement_type {
                StatementType::Label => {}
                StatementType::Code => {
                    let block = code_block(&stmt);
                    let ir_block = block
                        .borrow()
                        .ir_block
                        .clone()
                        .expect("the block was translated");
                    for node in &self.syntax_list {
                        if let NodeData::Block(existing) = &node.data {
                            assert!(!Rc::ptr_eq(existing, &ir_block));
                        }
                    }
                    current_block = Some(ir_block.clone());
                    insert_point = None;
                    self.syntax_list.push(AbstractSyntaxNode::block(ir_block));
                }
                StatementType::SetVariable => {
                    let (id, op) = match &stmt.borrow().data {
                        StatementData::SetVariable { id, op } => (*id, op.clone()),
                        _ => unreachable!(),
                    };
                    let block = current_block.as_ref().expect("a current block");
                    emit(block, &mut insert_point, |ir| {
                        let value = visit_expr(ir, &op);
                        ir.set_goto_variable(id, &value);
                    });
                }
                StatementType::If => {
                    let cond_node = match &stmt.borrow().data {
                        StatementData::If { cond, .. } => cond.clone(),
                        _ => unreachable!(),
                    };
                    let merge_block = self.merge_block(parent, &stmt);
                    let cond = {
                        let block = current_block.as_ref().expect("a current block");
                        emit(block, &mut insert_point, |ir| {
                            let value = visit_expr(ir, &cond_node);
                            ir.condition_ref(&value)
                        })
                    };
                    let if_node_index = self.syntax_list.len();
                    self.syntax_list.push(AbstractSyntaxNode::default());

                    let then_block_index = self.syntax_list.len();
                    self.visit(&stmt, break_block, Some(&merge_block));

                    let then_block = asl_block(&self.syntax_list[then_block_index]);
                    let block = current_block.as_ref().expect("a current block");
                    add_branch(block, &then_block);
                    add_branch(block, &merge_block);
                    current_block = Some(merge_block.clone());
                    insert_point = Some(0);

                    self.syntax_list[if_node_index] = AbstractSyntaxNode::new(
                        NodeType::If,
                        NodeData::If {
                            cond,
                            body: then_block,
                            merge: merge_block.clone(),
                        },
                    );
                    self.syntax_list.push(AbstractSyntaxNode::new(
                        NodeType::EndIf,
                        NodeData::EndIf { merge: merge_block },
                    ));
                }
                StatementType::Loop => {
                    let cond_node = match &stmt.borrow().data {
                        StatementData::Loop { cond, .. } => cond.clone(),
                        _ => unreachable!(),
                    };
                    let loop_header_block = IrBlock::new_ref();
                    if let Some(block) = &current_block {
                        add_branch(block, &loop_header_block);
                    }
                    self.syntax_list
                        .push(AbstractSyntaxNode::block(loop_header_block.clone()));

                    let continue_block = IrBlock::new_ref();
                    let merge_block = self.merge_block(parent, &stmt);
                    let loop_node_index = self.syntax_list.len();
                    self.syntax_list.push(AbstractSyntaxNode::default());

                    let body_block_index = self.syntax_list.len();
                    self.visit(&stmt, Some(&merge_block), Some(&continue_block));

                    let cond = {
                        let mut insert_point = None;
                        emit(&continue_block, &mut insert_point, |ir| {
                            let value = visit_expr(ir, &cond_node);
                            ir.condition_ref(&value)
                        })
                    };

                    let body_block = asl_block(&self.syntax_list[body_block_index]);
                    add_branch(&loop_header_block, &body_block);
                    add_branch(&continue_block, &loop_header_block);
                    add_branch(&continue_block, &merge_block);
                    current_block = Some(merge_block.clone());
                    insert_point = Some(0);

                    self.syntax_list[loop_node_index] = AbstractSyntaxNode::new(
                        NodeType::Loop,
                        NodeData::Loop {
                            body: body_block,
                            continue_block: continue_block.clone(),
                            merge: merge_block.clone(),
                        },
                    );
                    self.syntax_list
                        .push(AbstractSyntaxNode::block(continue_block));
                    self.syntax_list.push(AbstractSyntaxNode::new(
                        NodeType::Repeat,
                        NodeData::Repeat {
                            cond,
                            loop_header: loop_header_block,
                            merge: merge_block,
                        },
                    ));
                }
                StatementType::Break => {
                    let cond_node = match &stmt.borrow().data {
                        StatementData::Break { cond } => cond.clone(),
                        _ => unreachable!(),
                    };
                    let skip_block = self.merge_block(parent, &stmt);
                    let break_block = break_block.expect("a break outside a loop").clone();
                    let cond = {
                        let block = current_block.as_ref().expect("a current block");
                        emit(block, &mut insert_point, |ir| {
                            let value = visit_expr(ir, &cond_node);
                            ir.condition_ref(&value)
                        })
                    };
                    let block = current_block.as_ref().expect("a current block");
                    add_branch(block, &break_block);
                    add_branch(block, &skip_block);
                    current_block = Some(skip_block.clone());
                    insert_point = None;

                    self.syntax_list.push(AbstractSyntaxNode::new(
                        NodeType::Break,
                        NodeData::Break {
                            cond,
                            merge: break_block,
                            skip: skip_block,
                        },
                    ));
                }
                StatementType::Return => {
                    let return_block = IrBlock::new_ref();
                    let mut point = None;
                    emit(&return_block, &mut point, |ir| ir.epilogue());
                    let block = current_block.as_ref().expect("a current block");
                    add_branch(block, &return_block);
                    self.syntax_list
                        .push(AbstractSyntaxNode::block(return_block));
                    current_block = None;
                    insert_point = None;
                    self.syntax_list
                        .push(AbstractSyntaxNode::new(NodeType::Return, NodeData::None));
                }
                StatementType::Unreachable => {
                    current_block = None;
                    insert_point = None;
                    self.syntax_list.push(AbstractSyntaxNode::new(
                        NodeType::Unreachable,
                        NodeData::None,
                    ));
                }
                other => panic!("Statement type {}", other as u32),
            }
        }

        if let Some(block) = current_block {
            if let Some(fallthrough) = fallthrough_block {
                add_branch(&block, fallthrough);
            } else {
                self.syntax_list.push(AbstractSyntaxNode::new(
                    NodeType::Unreachable,
                    NodeData::None,
                ));
            }
        }
    }

    /// `MergeBlock`.
    fn merge_block(&mut self, parent: &Node, stmt: &Node) -> IrBlockRef {
        let index = position(parent, stmt).expect("the statement is in its parent");
        let forward = with_children(parent, |children| {
            try_find_forward_block(children, index + 1)
        });
        if let Some(code_stmt) = forward {
            let block = code_block(&code_stmt);
            return block
                .borrow()
                .ir_block
                .clone()
                .expect("the block was translated");
        }
        let dummy = Rc::new(RefCell::new(GcnBlock {
            is_dummy: true,
            ..Default::default()
        }));
        let ir_block = IrBlock::new_ref();
        dummy.borrow_mut().ir_block = Some(ir_block.clone());
        let merge_stmt = Statement::new(StatementData::Code(dummy), Some(parent));
        insert_child(parent, index + 1, merge_stmt);
        ir_block
    }
}

fn asl_block(node: &AbstractSyntaxNode) -> IrBlockRef {
    match &node.data {
        NodeData::Block(block) => block.clone(),
        _ => panic!("the node is not a block"),
    }
}

/// Emits through one emitter positioned at `insert_point`, then moves the point past everything
/// it wrote. The C++ iterator keeps pointing at the instruction it named, which the inserted
/// instructions push forward; the port's index has to be advanced the same way so that a second
/// emitter at the same point lands after the first one's instructions.
fn emit<R>(
    block: &IrBlockRef,
    insert_point: &mut Option<usize>,
    f: impl FnOnce(&mut IrEmitter) -> R,
) -> R {
    let before = block.borrow().instructions.len();
    let mut emitter = match *insert_point {
        Some(at) => IrEmitter::at(block, at),
        None => IrEmitter::new(block),
    };
    let result = f(&mut emitter);
    if let Some(at) = *insert_point {
        *insert_point = Some(at + block.borrow().instructions.len() - before);
    }
    result
}

/// `VisitExpr`.
fn visit_expr(ir: &mut IrEmitter, stmt: &Node) -> U1 {
    match &stmt.borrow().data {
        StatementData::Identity { cond_ref, .. } => cond_ref.clone(),
        StatementData::Not { op } => {
            let op = op.clone();
            let value = visit_expr(ir, &op);
            ir.logical_not(&value)
        }
        StatementData::Or { op_a, op_b } => {
            let op_a = op_a.clone();
            let op_b = op_b.clone();
            let a = visit_expr(ir, &op_a);
            let b = visit_expr(ir, &op_b);
            ir.logical_or(&a, &b)
        }
        StatementData::Variable(id) => {
            let id = *id;
            ir.get_goto_variable(id)
        }
        _ => {
            let ty = stmt.borrow().ty();
            panic!("Statement type {}", ty as u32)
        }
    }
}

/// `BuildASL`.
pub fn build_asl(cfg: &CFG, _info: &mut Info) -> AbstractSyntaxList {
    let goto_pass = GotoPass::new(cfg);
    TranslatePass::new(&goto_pass.root).syntax_list
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::common::object_pool::ObjectPool;
    use crate::gpu::shader_recompiler::ir::basic_block::{append_new_inst, Block, BlockList};
    use crate::gpu::shader_recompiler::ir::microinstruction::InstRef;
    use crate::gpu::shader_recompiler::ir::opcodes::Opcode;
    use crate::gpu::shader_recompiler::params::ShaderParams;
    use crate::gpu::shader_recompiler::runtime_info::{HwStage, SwStage};
    use std::panic::AssertUnwindSafe;

    fn block(begin: u32, end: u32, end_class: EndClass, cond: Condition) -> GcnBlockRef {
        let block = Rc::new(RefCell::new(GcnBlock {
            begin,
            end,
            end_class,
            cond,
            ..Default::default()
        }));
        block.borrow_mut().ir_block = Some(Block::new_ref());
        block
    }

    fn exit_block(begin: u32, end: u32) -> GcnBlockRef {
        block(begin, end, EndClass::Exit, Condition::True)
    }

    fn ir_block(block: &GcnBlockRef) -> IrBlockRef {
        block.borrow().ir_block.clone().expect("a translated block")
    }

    fn cfg_of(blocks: Vec<GcnBlockRef>) -> CFG {
        CFG {
            block_pool: ObjectPool::new(4),
            inst_list: Vec::new(),
            index_to_pc: vec![0],
            labels: Vec::new(),
            blocks,
        }
    }

    fn run(cfg: &CFG) -> AbstractSyntaxList {
        let mut info = Info::new(
            HwStage::Vertex,
            SwStage::Vertex,
            ShaderParams::new([0; 16], &[], 0),
        );
        build_asl(cfg, &mut info)
    }

    fn node_types(asl: &AbstractSyntaxList) -> Vec<NodeType> {
        asl.iter().map(|node| node.ty).collect()
    }

    fn append(block: &IrBlockRef, op: Opcode, args: &[Value]) -> InstRef {
        let index = append_new_inst(block, op, args);
        block.borrow().instructions[index].clone()
    }

    fn opcodes(block: &IrBlockRef) -> Vec<Opcode> {
        block
            .borrow()
            .instructions()
            .iter()
            .map(|inst| inst.borrow().get_opcode())
            .collect()
    }

    fn goto_variable_ids(block: &IrBlockRef) -> Vec<u32> {
        block
            .borrow()
            .instructions()
            .iter()
            .filter(|inst| inst.borrow().get_opcode() == Opcode::SetGotoVariable)
            .map(|inst| inst.borrow().arg(0).u32())
            .collect()
    }

    fn successors(block: &IrBlockRef) -> BlockList {
        block.borrow().imm_successors().to_vec()
    }

    fn if_data(node: &AbstractSyntaxNode) -> (&U1, &IrBlockRef, &IrBlockRef) {
        match &node.data {
            NodeData::If { cond, body, merge } => (cond, body, merge),
            _ => panic!("the node is not an if"),
        }
    }

    #[test]
    fn a_straight_line_block_becomes_a_block_and_a_return() {
        let gcn = exit_block(0, 0x10);
        let ir = ir_block(&gcn);
        append_new_inst(&ir, Opcode::Barrier, &[]);

        let asl = run(&cfg_of(vec![gcn]));
        assert_eq!(
            node_types(&asl),
            vec![NodeType::Block, NodeType::Block, NodeType::Return]
        );
        assert!(Rc::ptr_eq(&asl_block(&asl[0]), &ir));

        // The two resets — the one pushed to the front of the tree and the one following the
        // label — both land at the start of the block, in that order, ahead of its own code.
        assert_eq!(
            opcodes(&ir),
            vec![
                Opcode::SetGotoVariable,
                Opcode::SetGotoVariable,
                Opcode::Barrier
            ]
        );
        assert_eq!(goto_variable_ids(&ir), vec![0, 0]);
        let first = ir.borrow().instructions()[0].clone();
        assert_eq!(first.borrow().arg(0).u32(), 0);
        assert!(!first.borrow().arg(1).u1(), "the reset stores false");

        let return_block = asl_block(&asl[1]);
        assert_eq!(
            return_block.borrow().instructions()[0]
                .borrow()
                .get_opcode(),
            Opcode::Epilogue
        );
        assert!(Rc::ptr_eq(&successors(&ir)[0], &return_block));
        assert!(Rc::ptr_eq(
            &return_block.borrow().imm_predecessors()[0],
            &ir
        ));
    }

    #[test]
    fn a_goto_next_to_its_label_is_erased() {
        // 0: goto 4; 4: exit.
        let first = block(0, 4, EndClass::Branch, Condition::True);
        let second = exit_block(4, 8);
        first.borrow_mut().branch_true = Some(second.clone());

        let asl = run(&cfg_of(vec![first.clone(), second.clone()]));
        assert_eq!(
            node_types(&asl),
            vec![
                NodeType::Block,
                NodeType::Block,
                NodeType::Block,
                NodeType::Return
            ]
        );
        assert!(Rc::ptr_eq(&asl_block(&asl[0]), &ir_block(&first)));
        assert!(Rc::ptr_eq(&asl_block(&asl[1]), &ir_block(&second)));

        // Front resets are pushed in reverse block order. The reset that follows the second
        // label is still emitted into the first block, because a label does not change the
        // current block; only a code statement does.
        assert_eq!(goto_variable_ids(&ir_block(&first)), vec![1, 0, 0, 1]);
        assert!(
            successors(&ir_block(&first)).is_empty(),
            "the translate pass does not link consecutive code statements"
        );
    }

    #[test]
    fn a_conditional_goto_becomes_an_if() {
        // 0: if (scc) goto 8 else 4; 4: exit; 8: exit.
        let first = block(0, 4, EndClass::Branch, Condition::Scc1);
        let then_block = exit_block(4, 8);
        let else_block = exit_block(8, 12);
        first.borrow_mut().branch_true = Some(else_block.clone());
        first.borrow_mut().branch_false = Some(then_block.clone());

        let first_ir = ir_block(&first);
        let scc = append(&first_ir, Opcode::GetScc, &[]);
        let condition_ref = append(
            &first_ir,
            Opcode::ConditionRef,
            &[Value::from_inst(scc.clone())],
        );
        first_ir.borrow_mut().branch_cond = U1::new(Value::from_inst(condition_ref.clone()));

        let asl = run(&cfg_of(vec![
            first.clone(),
            then_block.clone(),
            else_block.clone(),
        ]));
        assert_eq!(
            node_types(&asl),
            vec![
                NodeType::Block,
                NodeType::If,
                NodeType::Block,
                NodeType::Block,
                NodeType::Return,
                NodeType::EndIf,
                NodeType::Block,
                NodeType::Block,
                NodeType::Return
            ]
        );

        let (cond, body, merge) = if_data(&asl[1]);
        assert!(Rc::ptr_eq(body, &ir_block(&then_block)));
        assert!(Rc::ptr_eq(merge, &ir_block(&else_block)));

        // The if condition is the negation of the branch condition, taken from the condition
        // reference's operand before the reference is invalidated.
        assert_eq!(
            cond.value().inst().borrow().get_opcode(),
            Opcode::ConditionRef
        );
        let not = cond.value().inst().borrow().arg(0).inst();
        assert_eq!(not.borrow().get_opcode(), Opcode::LogicalNot);
        assert!(Rc::ptr_eq(&not.borrow().arg(0).inst(), &scc));
        assert_eq!(
            condition_ref.borrow().get_opcode(),
            Opcode::Void,
            "the branch's condition reference is invalidated"
        );

        let first_successors = successors(&first_ir);
        assert_eq!(first_successors.len(), 2);
        assert!(Rc::ptr_eq(&first_successors[0], &ir_block(&then_block)));
        assert!(Rc::ptr_eq(&first_successors[1], &ir_block(&else_block)));

        // Each arm ends in its own return block.
        assert!(Rc::ptr_eq(
            &successors(&ir_block(&then_block))[0],
            &asl_block(&asl[3])
        ));
        assert!(Rc::ptr_eq(
            &successors(&ir_block(&else_block))[0],
            &asl_block(&asl[7])
        ));

        // The four goto-variable resets precede the branch code.
        assert_eq!(goto_variable_ids(&first_ir), vec![2, 1, 0, 0]);
        let ops = opcodes(&first_ir);
        assert_eq!(
            &ops[ops.len() - 2..],
            &[Opcode::LogicalNot, Opcode::ConditionRef]
        );
    }

    #[test]
    fn a_backward_goto_becomes_a_loop() {
        // 0: goto 4; 4: goto 0. A dummy merge block is made for the loop.
        let header_source = block(0, 4, EndClass::Branch, Condition::True);
        let latch = block(4, 8, EndClass::Branch, Condition::True);
        header_source.borrow_mut().branch_true = Some(latch.clone());
        latch.borrow_mut().branch_true = Some(header_source.clone());

        let asl = run(&cfg_of(vec![header_source.clone(), latch.clone()]));
        assert_eq!(
            node_types(&asl),
            vec![
                NodeType::Block,
                NodeType::Block,
                NodeType::Loop,
                NodeType::Block,
                NodeType::Block,
                NodeType::Block,
                NodeType::Repeat,
                NodeType::Block,
                NodeType::Unreachable
            ]
        );

        let (body, continue_block, merge) = match &asl[2].data {
            NodeData::Loop {
                body,
                continue_block,
                merge,
            } => (body, continue_block, merge),
            _ => panic!("the node is not a loop"),
        };
        assert!(Rc::ptr_eq(body, &ir_block(&header_source)));
        assert!(Rc::ptr_eq(continue_block, &asl_block(&asl[5])));
        assert!(Rc::ptr_eq(merge, &asl_block(&asl[7])));

        let header = asl_block(&asl[1]);
        let preheader = asl_block(&asl[0]);
        assert!(Rc::ptr_eq(&successors(&preheader)[0], &header));
        assert!(Rc::ptr_eq(&successors(&header)[0], body));
        assert!(successors(&ir_block(&header_source)).is_empty());
        assert!(Rc::ptr_eq(
            &successors(&ir_block(&latch))[0],
            continue_block
        ));
        assert!(Rc::ptr_eq(&successors(continue_block)[0], &header));
        assert!(Rc::ptr_eq(&successors(continue_block)[1], merge));

        // The repeat tests the loop's condition in the continue block.
        let (cond, loop_header, repeat_merge) = match &asl[6].data {
            NodeData::Repeat {
                cond,
                loop_header,
                merge,
            } => (cond, loop_header, merge),
            _ => panic!("the node is not a repeat"),
        };
        assert!(Rc::ptr_eq(loop_header, &header));
        assert!(Rc::ptr_eq(repeat_merge, merge));
        // A backward goto at the label's level is lifted into a loop whose condition reads the
        // goto variable, so the repeat tests the variable in the continue block.
        assert_eq!(
            cond.value().inst().borrow().get_opcode(),
            Opcode::ConditionRef
        );
        let variable = cond.value().inst().borrow().arg(0).inst();
        assert_eq!(variable.borrow().get_opcode(), Opcode::GetGotoVariable);
        assert_eq!(variable.borrow().arg(0).u32(), 0);
        assert_eq!(
            opcodes(continue_block),
            vec![Opcode::GetGotoVariable, Opcode::ConditionRef]
        );

        // The preheader gets both front resets; the body gets the two label resets, and the
        // lifted loop's tail reset lands in the second block.
        assert_eq!(goto_variable_ids(&preheader), vec![1, 0]);
        assert_eq!(goto_variable_ids(&ir_block(&header_source)), vec![0, 1]);
        assert_eq!(goto_variable_ids(&ir_block(&latch)), vec![0]);
        assert!(merge.borrow().instructions().is_empty());
    }

    #[test]
    fn moving_a_goto_inward_updates_the_enclosing_condition() {
        // 0: if (scc) goto 4 else 8. The false goto wraps 4..8 in an if first, then the true
        // goto has to be moved into that if and its condition joined with or.
        let first = block(0, 4, EndClass::Branch, Condition::Scc1);
        let true_block = exit_block(4, 8);
        let false_block = exit_block(8, 12);
        first.borrow_mut().branch_true = Some(true_block.clone());
        first.borrow_mut().branch_false = Some(false_block.clone());

        let first_ir = ir_block(&first);
        let scc = append(&first_ir, Opcode::GetScc, &[]);
        append(
            &first_ir,
            Opcode::ConditionRef,
            &[Value::from_inst(scc.clone())],
        );
        let condition_ref = first_ir.borrow().instructions()[1].clone();
        first_ir.borrow_mut().branch_cond = U1::new(Value::from_inst(condition_ref.clone()));

        let asl = run(&cfg_of(vec![
            first.clone(),
            true_block.clone(),
            false_block.clone(),
        ]));
        assert_eq!(
            node_types(&asl),
            vec![
                NodeType::Block,
                NodeType::If,
                NodeType::Block,
                NodeType::Block,
                NodeType::Return,
                NodeType::EndIf,
                NodeType::Block,
                NodeType::Block,
                NodeType::Return
            ]
        );

        // If (goto_L1 || !true) { ... }: the moved goto's variable is or-ed into the condition.
        let (cond, body, merge) = if_data(&asl[1]);
        let condition_ref = cond.value().inst();
        assert_eq!(condition_ref.borrow().get_opcode(), Opcode::ConditionRef);
        let or = condition_ref.borrow().arg(0).inst();
        assert_eq!(or.borrow().get_opcode(), Opcode::LogicalOr);
        let variable = or.borrow().arg(0).inst();
        assert_eq!(variable.borrow().get_opcode(), Opcode::GetGotoVariable);
        assert_eq!(variable.borrow().arg(0).u32(), 1);
        let not = or.borrow().arg(1).inst();
        assert_eq!(not.borrow().get_opcode(), Opcode::LogicalNot);
        assert!(not.borrow().arg(0).u1());

        assert!(Rc::ptr_eq(body, &ir_block(&true_block)));
        assert!(Rc::ptr_eq(merge, &ir_block(&false_block)));
        assert_eq!(goto_variable_ids(&first_ir), vec![2, 1, 0, 0, 1]);
        assert_eq!(
            condition_ref_ops(&first_ir),
            vec![Opcode::LogicalNot, Opcode::LogicalOr, Opcode::ConditionRef]
        );
    }

    fn condition_ref_ops(block: &IrBlockRef) -> Vec<Opcode> {
        opcodes(block)
            .into_iter()
            .filter(|op| {
                matches!(
                    op,
                    Opcode::LogicalNot | Opcode::LogicalOr | Opcode::ConditionRef
                )
            })
            .collect()
    }

    #[test]
    fn an_empty_tree_gets_a_block_and_an_unreachable() {
        let asl = run(&cfg_of(Vec::new()));
        assert_eq!(
            node_types(&asl),
            vec![NodeType::Block, NodeType::Unreachable]
        );
        assert!(asl_block(&asl[0]).borrow().instructions().is_empty());
    }

    #[test]
    fn the_statement_dump_prints_the_tree() {
        let root = Statement::new(
            StatementData::Function {
                children: Vec::new(),
            },
            None,
        );
        let label = Statement::new(StatementData::Label(3), Some(&root));
        let identity = Statement::new(
            StatementData::Identity {
                cond_ref: U1::new(Value::from_u1(true)),
                guest_cond: Condition::Scc1,
            },
            Some(&root),
        );
        let variable = Statement::new(StatementData::Variable(4), Some(&root));
        assert_eq!(dump_expr(&identity), "Scc1");
        assert_eq!(dump_expr(&variable), "goto_L4");
        let not = Statement::new(
            StatementData::Not {
                op: variable.clone(),
            },
            Some(&root),
        );
        assert_eq!(dump_expr(&not), "!goto_L4");
        let or = Statement::new(
            StatementData::Or {
                op_a: identity.clone(),
                op_b: variable.clone(),
            },
            Some(&root),
        );
        assert_eq!(dump_expr(&or), "Scc1 || goto_L4");
        assert_eq!(dump_expr(&label), "<invalid type>");

        let gcn = Rc::new(RefCell::new(GcnBlock {
            begin: 0x10,
            end: 0x20,
            ..Default::default()
        }));
        let code = Statement::new(StatementData::Code(gcn.clone()), Some(&root));
        let goto = Statement::new(
            StatementData::Goto {
                cond: identity,
                label: label.clone(),
            },
            Some(&root),
        );
        let ret = Statement::new(StatementData::Return, Some(&root));
        let break_stmt = Statement::new(StatementData::Break { cond: or }, Some(&root));
        let if_stmt = Statement::new(
            StatementData::If {
                cond: not,
                children: vec![ret],
            },
            Some(&root),
        );
        let loop_stmt = Statement::new(
            StatementData::Loop {
                cond: variable,
                children: vec![break_stmt],
            },
            Some(&root),
        );

        let text = dump_tree(&[label, code, goto, if_stmt, loop_stmt], 2);
        let expected = format!(
            "  L3:\n      Block 0010 -> 0020 (0x{:016x});\n      if (Scc1) goto L3;\n      if (!goto_L4) {{\n          return;\n      }}\n      do {{\n          if (Scc1 || goto_L4) break;\n      }} while (goto_L4);\n",
            Rc::as_ptr(&gcn) as usize
        );
        assert_eq!(text, expected);

        assert!(
            std::panic::catch_unwind(AssertUnwindSafe(|| dump_tree(
                &[Statement::new(
                    StatementData::Identity {
                        cond_ref: U1::new(Value::from_u1(false)),
                        guest_cond: Condition::True,
                    },
                    Some(&root),
                )],
                0
            )))
            .is_err(),
            "an identity statement cannot be printed as a tree"
        );
    }

    #[test]
    fn a_loop_body_with_a_break_is_refused() {
        let root = Statement::new(
            StatementData::Function {
                children: Vec::new(),
            },
            None,
        );
        let break_stmt = Statement::new(
            StatementData::Break {
                cond: Statement::new(
                    StatementData::Identity {
                        cond_ref: U1::new(Value::from_u1(true)),
                        guest_cond: Condition::True,
                    },
                    Some(&root),
                ),
            },
            Some(&root),
        );
        let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
            sanitize_no_breaks(&[break_stmt]);
        }));
        assert!(result.is_err(), "a lifted loop cannot contain a break");
    }

    #[test]
    fn the_goto_variable_operand_is_the_label_id() {
        // A statement-level check that SetVariable keeps the label id on the left.
        let root = Statement::new(
            StatementData::Function {
                children: Vec::new(),
            },
            None,
        );
        let cond = Statement::new(
            StatementData::Identity {
                cond_ref: U1::new(Value::from_u1(true)),
                guest_cond: Condition::True,
            },
            Some(&root),
        );
        let set_var = Statement::new(
            StatementData::SetVariable {
                id: 7,
                op: cond.clone(),
            },
            Some(&root),
        );
        assert_eq!(dump_expr(&set_var), "<invalid type>");
        assert_eq!(dump_tree(&[set_var], 0), "    goto_L7 = True;\n");
    }
}
