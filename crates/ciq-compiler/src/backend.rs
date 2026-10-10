//! VM instruction selection for the validated pure integer expression subset.
use crate::{BinaryOp, Function, Operation, ValueId};
use ciq_assembly::{Item, Operand, Program};
use ciq_mc::Opcode;
use std::collections::BTreeMap;

/// Walk value dependencies rather than assuming IR definition order is stack order.
pub fn lower(function: &Function) -> Program {
    let mut definitions = BTreeMap::new();
    let mut root = None;
    for located in function.operations() {
        match &located.operation {
            Operation::Integer { result, .. } | Operation::Binary { result, .. } => {
                definitions.insert(*result, &located.operation);
            }
            Operation::Return { value } => root = Some(*value),
        }
    }
    enum Task {
        Value(ValueId),
        Binary(BinaryOp),
    }
    let mut tasks = vec![Task::Value(root.expect("validated IR has a return"))];
    let mut items = vec![
        Item::Label(function.name().into()),
        Item::Instruction {
            opcode: Opcode::Argc,
            operands: vec![Operand::Bits(1)],
        },
    ];
    while let Some(task) = tasks.pop() {
        let (opcode, operands) = match task {
            Task::Value(value) => match definitions[&value] {
                Operation::Integer { value: 0, .. } => (Opcode::Ipushz, vec![]),
                Operation::Integer { value, .. } => {
                    (Opcode::Ipush1, vec![Operand::Bits(*value as u64)])
                }
                Operation::Binary {
                    kind, left, right, ..
                } => {
                    tasks.extend([Task::Binary(*kind), Task::Value(*right), Task::Value(*left)]);
                    continue;
                }
                Operation::Return { .. } => unreachable!("return is not a value definition"),
            },
            Task::Binary(kind) => (
                match kind {
                    BinaryOp::Add => Opcode::Addv,
                    BinaryOp::Subtract => Opcode::Subv,
                },
                vec![],
            ),
        };
        items.push(Item::Instruction { opcode, operands });
    }
    items.push(Item::Instruction {
        opcode: Opcode::Return,
        operands: vec![],
    });
    Program::new(items).expect("bounded validated expression IR lowers to valid assembly")
}
