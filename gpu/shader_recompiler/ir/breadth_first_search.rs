//! Breadth-first search over the SSA graph, ported from
//! `legacy/gpu/shadps4/shader_recompiler/ir/breadth_first_search.h`. The C++ template returns
//! whatever the predicate returns, wrapped in an optional; the port takes a predicate returning
//! `Option<R>` and hands that straight back. Arguments are visited right to left, as the C++
//! visits them.

use super::microinstruction::InstRef;
use super::value::Value;
use std::collections::VecDeque;
use std::rc::Rc;

/// `BreadthFirstSearch(Instruction*, Pred)`: tries the instruction itself, then searches the
/// instructions behind its arguments.
pub fn breadth_first_search<F, R>(inst: &InstRef, pred: F) -> Option<R>
where
    F: Fn(&InstRef) -> Option<R>,
{
    // Most often the instruction is the desired one already.
    if let Some(result) = pred(inst) {
        return Some(result);
    }

    let mut visited: Vec<usize> = vec![Rc::as_ptr(inst) as usize];
    let mut queue: VecDeque<InstRef> = VecDeque::new();
    queue.push_back(inst.clone());

    while let Some(current) = queue.pop_front() {
        if let Some(result) = pred(&current) {
            return Some(result);
        }
        // Visit the right-most arguments first.
        let arg_count = current.borrow().num_args();
        for arg_index in (0..arg_count).rev() {
            let arg_value = current.borrow().arg(arg_index);
            if arg_value.is_immediate() {
                continue;
            }
            let arg_inst = arg_value.inst();
            let key = Rc::as_ptr(&arg_inst) as usize;
            if visited.contains(&key) {
                continue;
            }
            visited.push(key);
            queue.push_back(arg_inst);
        }
    }
    // The SSA tree has been traversed and the result has not been found.
    None
}

/// `BreadthFirstSearch(const Value&, Pred)`: an immediate has nothing behind it.
pub fn breadth_first_search_value<F, R>(value: &Value, pred: F) -> Option<R>
where
    F: Fn(&InstRef) -> Option<R>,
{
    if value.is_immediate() {
        return None;
    }
    breadth_first_search(&value.inst(), pred)
}

#[cfg(test)]
mod tests {
    use super::super::microinstruction::{set_arg, Inst};
    use super::super::opcodes::Opcode;
    use super::*;
    use std::cell::RefCell;

    fn new_inst(op: Opcode) -> InstRef {
        Rc::new(RefCell::new(Inst::new(op, 0)))
    }

    /// add(load(const), mul(const, const)): the multiply is one level deeper than the load.
    fn tree() -> (InstRef, InstRef, InstRef) {
        let load = new_inst(Opcode::ReadConst);
        let multiply = new_inst(Opcode::IMul32);
        set_arg(&multiply, 0, Value::from_u32(2));
        set_arg(&multiply, 1, Value::from_u32(3));
        let add = new_inst(Opcode::IAdd32);
        set_arg(&add, 0, Value::from_inst(load.clone()));
        set_arg(&add, 1, Value::from_inst(multiply.clone()));
        (add, load, multiply)
    }

    #[test]
    fn the_search_finds_the_root_first_then_descends() {
        let (add, load, multiply) = tree();
        // The root matches immediately.
        let found = breadth_first_search(&add, |inst| {
            (inst.borrow().get_opcode() == Opcode::IAdd32).then_some("add")
        });
        assert_eq!(found, Some("add"));
        // A deeper match is reached through the arguments.
        let found = breadth_first_search(&add, |inst| {
            (inst.borrow().get_opcode() == Opcode::IMul32).then_some(Rc::as_ptr(inst) as usize)
        });
        assert_eq!(found, Some(Rc::as_ptr(&multiply) as usize));
        let found = breadth_first_search(&add, |inst| {
            (inst.borrow().get_opcode() == Opcode::ReadConst).then_some(Rc::as_ptr(inst) as usize)
        });
        assert_eq!(found, Some(Rc::as_ptr(&load) as usize));
        // Nothing matches a name the tree does not hold.
        assert_eq!(
            breadth_first_search(&add, |inst| (inst.borrow().get_opcode() == Opcode::Barrier)
                .then_some(0)),
            None
        );
    }

    #[test]
    fn immediates_end_the_search_and_a_value_of_one_finds_nothing() {
        let (add, _load, _multiply) = tree();
        assert_eq!(
            breadth_first_search_value(&Value::from_u32(7), |_| Some(0)),
            None
        );
        let found = breadth_first_search_value(&Value::from_inst(add.clone()), |inst| {
            (inst.borrow().get_opcode() == Opcode::IAdd32).then_some(1)
        });
        assert_eq!(found, Some(1));
        // An instruction whose arguments are all immediates terminates straight away.
        let alone = new_inst(Opcode::ReadConst);
        assert_eq!(
            breadth_first_search(&alone, |inst| (inst.borrow().get_opcode()
                == Opcode::Barrier)
                .then_some(0)),
            None
        );
    }

    #[test]
    fn a_shared_operand_is_visited_once_and_a_cycle_terminates() {
        // Both operands read the same instruction.
        let shared = new_inst(Opcode::ReadConst);
        let root = new_inst(Opcode::IAdd32);
        set_arg(&root, 0, Value::from_inst(shared.clone()));
        set_arg(&root, 1, Value::from_inst(shared.clone()));
        let visits = std::cell::Cell::new(0);
        let found = breadth_first_search(&root, |inst| {
            if inst.borrow().get_opcode() == Opcode::ReadConst {
                visits.set(visits.get() + 1);
                return Some(visits.get());
            }
            None
        });
        assert_eq!(found, Some(1), "the first visit answers");
        assert_eq!(shared.borrow().use_count(), 2);

        // A cycle in the argument graph is bounded by the visited list.
        let first = new_inst(Opcode::IAdd32);
        let second = new_inst(Opcode::IAdd32);
        set_arg(&first, 0, Value::from_inst(second.clone()));
        set_arg(&second, 0, Value::from_inst(first.clone()));
        assert_eq!(
            breadth_first_search(&first, |inst| (inst.borrow().get_opcode()
                == Opcode::Barrier)
                .then_some(0)),
            None
        );
    }
}
