//! VM instruction selection for validated expressions, locals, and print statements.
use crate::{BinaryOp, Function, LocalId, Operation, ValueId};
use ciq_assembly::{Item, Operand, Program};
use ciq_mc::Opcode;
use std::collections::BTreeMap;

fn instruction(opcode: Opcode, operands: Vec<Operand>) -> Item {
    Item::Instruction { opcode, operands }
}

/// Lower a validated IR function into a typed VM assembly program.
///
/// Selects instructions and assigns parameter/local frame slots. Initializers
/// execute in order; expression operands and call arguments evaluate left to
/// right. Relies on [`Function`] validation to exclude shared expression values
/// and incompatible call schedules. Module/member aliases and literal DATA
/// placements remain unresolved. Print calls discard their result.
pub fn lower(function: &Function) -> Program {
    let mut definitions = BTreeMap::new();
    let mut strings = BTreeMap::new();
    let mut literals = Vec::new();
    let mut slots = BTreeMap::new();
    for located in function.operations() {
        if let Operation::String { value, .. } = &located.operation {
            if !strings.contains_key(value) {
                let alias = format!("__ciq_string_{}_{}", function.name(), strings.len());
                strings.insert(value.clone(), alias.clone());
                literals.push(Item::String(ciq_prg::data::StringDeclaration {
                    alias,
                    value: value.clone(),
                    at: None,
                }));
            }
        }
        match &located.operation {
            Operation::String { result, .. }
            | Operation::Integer { result, .. }
            | Operation::Binary { result, .. }
            | Operation::ParameterRead { result, .. }
            | Operation::LocalRead { result, .. }
            | Operation::ModuleCall { result, .. } => {
                definitions.insert(*result, &located.operation);
            }
            Operation::LocalInitialize { local, .. } => {
                slots.insert(
                    *local,
                    (function.parameter_count() + slots.len() + 1) as u64,
                );
            }
            Operation::Return { .. } | Operation::Println { .. } => {}
        }
    }
    let mut items = literals;
    items.push(Item::Label(function.name().into()));
    items.push(if slots.is_empty() {
        instruction(
            Opcode::Argc,
            vec![Operand::Bits(function.parameter_count() as u64 + 1)],
        )
    } else {
        instruction(
            Opcode::Argcincsp,
            vec![
                Operand::Bits(function.parameter_count() as u64 + 1),
                Operand::Bits(slots.len() as u64),
            ],
        )
    });
    for located in function.operations() {
        let (value, opcode, operands) = match located.operation {
            Operation::LocalInitialize { local, value } => {
                (value, Opcode::Lputv, vec![Operand::Bits(slots[&local])])
            }
            Operation::Return { value } => (value, Opcode::Return, vec![]),
            Operation::Println { value } => {
                items.push(instruction(
                    Opcode::Getmv,
                    vec![
                        Operand::Symbol("__ciq_sdk_System".into()),
                        Operand::Symbol("__ciq_member_println".into()),
                    ],
                ));
                items.push(instruction(Opcode::Frpush, vec![]));
                expression(value, &definitions, &slots, &strings, &mut items);
                items.push(instruction(Opcode::Invokem, vec![Operand::Bits(2)]));
                items.push(instruction(Opcode::Popv, vec![]));
                continue;
            }
            _ => continue,
        };
        expression(value, &definitions, &slots, &strings, &mut items);
        items.push(instruction(opcode, operands));
    }
    Program::new(items).expect("bounded validated IR lowers to valid assembly")
}

fn expression(
    root: ValueId,
    definitions: &BTreeMap<ValueId, &Operation>,
    slots: &BTreeMap<LocalId, u64>,
    strings: &BTreeMap<String, String>,
    items: &mut Vec<Item>,
) {
    enum Task {
        Value(ValueId),
        Binary(BinaryOp),
        Invoke(usize),
        Context,
    }
    let mut tasks = vec![Task::Value(root)];
    while let Some(task) = tasks.pop() {
        let (opcode, operands) = match task {
            Task::Value(value) => match definitions[&value] {
                Operation::String { value, .. } => {
                    (Opcode::News, vec![Operand::String(strings[value].clone())])
                }
                Operation::Integer { value: 0, .. } => (Opcode::Ipushz, vec![]),
                Operation::Integer { value, .. } => {
                    (Opcode::Ipush1, vec![Operand::Bits(*value as u64)])
                }
                Operation::ModuleCall {
                    module,
                    member,
                    arguments,
                    ..
                } => {
                    tasks.push(Task::Invoke(arguments.len()));
                    if !arguments.is_empty() {
                        tasks.extend(arguments.iter().rev().map(|value| Task::Value(*value)));
                        tasks.push(Task::Context);
                    }
                    (
                        Opcode::Getmv,
                        vec![
                            Operand::Symbol(format!("__ciq_module_{module}")),
                            Operand::Symbol(format!("__ciq_member_{member}")),
                        ],
                    )
                }
                Operation::ParameterRead { index, .. } => {
                    (Opcode::Lgetv, vec![Operand::Bits(*index as u64 + 1)])
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
            Task::Invoke(0) => (Opcode::Invokemz, vec![]),
            Task::Invoke(count) => (Opcode::Invokem, vec![Operand::Bits(count as u64 + 1)]),
            Task::Context => (Opcode::Frpush, vec![]),
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
