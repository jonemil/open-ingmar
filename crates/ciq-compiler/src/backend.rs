//! VM instruction selection for validated expressions, locals, and control flow.
use crate::{BinaryOp, ComparisonOp, Function, LocalId, Operation, ValueId};
use ciq_assembly::{Item, Operand, Program};
use ciq_mc::Opcode;
use std::collections::BTreeMap;

fn instruction(opcode: Opcode, operands: Vec<Operand>) -> Item {
    Item::Instruction { opcode, operands }
}

/// Lower a validated IR function into a typed VM assembly program.
///
/// Selects instructions and assigns parameter/local frame slots. Assignments
/// evaluate their right-hand expression before replacing the existing slot.
/// Initializers
/// execute in order; expression operands and call arguments evaluate left to
/// right. Relies on [`Function`] validation to exclude shared expression values
/// and incompatible effect schedules. Module/member aliases and literal DATA
/// placements remain unresolved. Print calls discard their result. Explicit
/// blocks lower to labels and symbolic branches, with an empty evaluation stack
/// on each edge; local slots are allocated across the whole function.
pub fn lower(function: &Function) -> Program {
    lower_named(function, function.name())
}

/// Lower a function using a unit-unique entry label and internal alias prefix.
pub(crate) fn lower_named(function: &Function, label: &str) -> Program {
    let mut definitions = BTreeMap::new();
    let mut strings = BTreeMap::new();
    let mut literals = Vec::new();
    let mut slots = BTreeMap::new();
    for located in function.operations() {
        if let Operation::String { value, .. } = &located.operation {
            if !strings.contains_key(value) {
                let alias = format!("__ciq_string_{}_{}", label, strings.len());
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
            | Operation::Compare { result, .. }
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
            _ => {}
        }
    }
    let mut items = literals;
    items.push(Item::Label(label.into()));
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
            Operation::Block { block } => {
                items.push(Item::Label(format!("__ciq_block_{}_{}", label, block.0)));
                continue;
            }
            Operation::Jump { target } => {
                items.push(instruction(
                    Opcode::Goto,
                    vec![Operand::Code(format!("__ciq_block_{}_{}", label, target.0))],
                ));
                continue;
            }
            Operation::Branch {
                condition,
                on_true,
                on_false,
            } => {
                expression(condition, &definitions, &slots, &strings, &mut items);
                items.push(instruction(
                    Opcode::Bf,
                    vec![Operand::Code(format!(
                        "__ciq_block_{}_{}",
                        label, on_false.0
                    ))],
                ));
                items.push(instruction(
                    Opcode::Goto,
                    vec![Operand::Code(format!(
                        "__ciq_block_{}_{}",
                        label, on_true.0
                    ))],
                ));
                continue;
            }
            Operation::LocalInitialize { local, value }
            | Operation::LocalAssign { local, value } => {
                (value, Opcode::Lputv, vec![Operand::Bits(slots[&local])])
            }
            Operation::Return { value } => (value, Opcode::Return, vec![]),
            Operation::ReturnVoid => {
                items.push(instruction(Opcode::Return, vec![]));
                continue;
            }
            Operation::ModuleCallStatement {
                ref module,
                ref member,
                ref arguments,
            } => {
                items.push(instruction(
                    Opcode::Getmv,
                    vec![
                        Operand::Symbol(format!("__ciq_module_{module}")),
                        Operand::Symbol(format!("__ciq_member_{member}")),
                    ],
                ));
                if !arguments.is_empty() {
                    items.push(instruction(Opcode::Frpush, vec![]));
                    for value in arguments {
                        expression(*value, &definitions, &slots, &strings, &mut items);
                    }
                }
                items.push(if arguments.is_empty() {
                    instruction(Opcode::Invokemz, vec![])
                } else {
                    instruction(
                        Opcode::Invokem,
                        vec![Operand::Bits(arguments.len() as u64 + 1)],
                    )
                });
                items.push(instruction(Opcode::Popv, vec![]));
                continue;
            }
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
        Compare(ComparisonOp),
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
                Operation::Compare {
                    kind, left, right, ..
                } => {
                    tasks.extend([
                        Task::Compare(*kind),
                        Task::Value(*right),
                        Task::Value(*left),
                    ]);
                    continue;
                }
                _ => unreachable!("statement is not a value definition"),
            },
            Task::Invoke(0) => (Opcode::Invokemz, vec![]),
            Task::Invoke(count) => (Opcode::Invokem, vec![Operand::Bits(count as u64 + 1)]),
            Task::Context => (Opcode::Frpush, vec![]),
            Task::Compare(kind) => (
                match kind {
                    ComparisonOp::Equal => Opcode::Eq,
                    ComparisonOp::Greater => Opcode::Gt,
                },
                vec![],
            ),
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
