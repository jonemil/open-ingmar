//! VM instruction selection for validated integer expressions and initialized locals.
use crate::{BinaryOp, Function, LocalId, Operation, ValueId};
use ciq_assembly::{Item, Operand, Program};
use ciq_mc::Opcode;
use std::collections::BTreeMap;

fn instruction(opcode: Opcode, operands: Vec<Operand>) -> Item {
    Item::Instruction { opcode, operands }
}

/// Schedule each initialization/return in order, following its value dependencies.
pub fn lower(function: &Function) -> Program {
    let mut definitions = BTreeMap::new();
    let mut slots = BTreeMap::new();
    for located in function.operations() {
        match &located.operation {
            Operation::Integer { result, .. }
            | Operation::Binary { result, .. }
            | Operation::LocalRead { result, .. }
            | Operation::ModuleCall { result, .. } => {
                definitions.insert(*result, &located.operation);
            }
            Operation::LocalInitialize { local, .. } => {
                slots.insert(*local, slots.len() as u64 + 1);
            }
            Operation::Return { .. } => {}
        }
    }
    let mut items = vec![Item::Label(function.name().into())];
    items.push(if slots.is_empty() {
        instruction(Opcode::Argc, vec![Operand::Bits(1)])
    } else {
        instruction(
            Opcode::Argcincsp,
            vec![Operand::Bits(1), Operand::Bits(slots.len() as u64)],
        )
    });
    for located in function.operations() {
        let (value, opcode, operands) = match located.operation {
            Operation::LocalInitialize { local, value } => {
                (value, Opcode::Lputv, vec![Operand::Bits(slots[&local])])
            }
            Operation::Return { value } => (value, Opcode::Return, vec![]),
            _ => continue,
        };
        expression(value, &definitions, &slots, &mut items);
        items.push(instruction(opcode, operands));
    }
    Program::new(items).expect("bounded validated IR lowers to valid assembly")
}

fn expression(
    root: ValueId,
    definitions: &BTreeMap<ValueId, &Operation>,
    slots: &BTreeMap<LocalId, u64>,
    items: &mut Vec<Item>,
) {
    enum Task {
        Value(ValueId),
        Binary(BinaryOp),
        Invoke,
    }
    let mut tasks = vec![Task::Value(root)];
    while let Some(task) = tasks.pop() {
        let (opcode, operands) = match task {
            Task::Value(value) => match definitions[&value] {
                Operation::Integer { value: 0, .. } => (Opcode::Ipushz, vec![]),
                Operation::Integer { value, .. } => {
                    (Opcode::Ipush1, vec![Operand::Bits(*value as u64)])
                }
                Operation::ModuleCall { module, member, .. } => {
                    tasks.push(Task::Invoke);
                    (
                        Opcode::Getmv,
                        vec![
                            Operand::Symbol(format!("__ciq_module_{module}")),
                            Operand::Symbol(format!("__ciq_member_{member}")),
                        ],
                    )
                }
                Operation::LocalRead { local, .. } => {
                    (Opcode::Lgetv, vec![Operand::Bits(slots[local])])
                }
                Operation::Binary {
                    kind, left, right, ..
                } => {
                    tasks.extend([Task::Binary(*kind), Task::Value(*right), Task::Value(*left)]);
                    continue;
                }
                _ => unreachable!("statement is not a value definition"),
            },
            Task::Invoke => (Opcode::Invokemz, vec![]),
            Task::Binary(kind) => (
                match kind {
                    BinaryOp::Add => Opcode::Addv,
                    BinaryOp::Subtract => Opcode::Subv,
                },
                vec![],
            ),
        };
        items.push(instruction(opcode, operands));
    }
}
