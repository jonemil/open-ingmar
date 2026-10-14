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
            Operation::Null { result }
            | Operation::Boolean { result, .. }
            | Operation::Char { result, .. }
            | Operation::String { result, .. }
            | Operation::Integer { result, .. }
            | Operation::SelfFieldRead { result, .. }
            | Operation::Binary { result, .. }
            | Operation::Negate { result, .. }
            | Operation::Not { result, .. }
            | Operation::ArrayRead { result, .. }
            | Operation::ArrayNew { result, .. }
            | Operation::NativeNew { result, .. }
            | Operation::SourceNew { result, .. }
            | Operation::NativeConstant { result, .. }
            | Operation::TupleNew { result, .. }
            | Operation::Compare { result, .. }
            | Operation::ParameterRead { result, .. }
            | Operation::LocalRead { result, .. }
            | Operation::ModuleCall { result, .. }
            | Operation::SelfCall { result, .. }
            | Operation::ObjectCall { result, .. }
            | Operation::NativeCall { result, .. } => {
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
            Operation::Discard { value } => (value, Opcode::Popv, vec![]),
            Operation::NativeBaseInitialize { base } => {
                items.extend([
                    instruction(
                        Opcode::Getselfv,
                        vec![Operand::Symbol(
                            match base {
                                crate::ClassBase::ApplicationAppBase => "__ciq_native_AppBase",
                                crate::ClassBase::WatchUiView => "__ciq_native_View",
                                crate::ClassBase::WatchUiBehaviorDelegate => {
                                    "__ciq_native_BehaviorDelegate"
                                }
                            }
                            .into(),
                        )],
                    ),
                    instruction(
                        Opcode::Getsv,
                        vec![Operand::Symbol("__ciq_native_initialize".into())],
                    ),
                    instruction(Opcode::Invokemz, vec![]),
                    instruction(Opcode::Popv, vec![]),
                ]);
                continue;
            }
            Operation::SelfFieldAssign { ref field, value } => {
                items.push(instruction(Opcode::Getself, vec![]));
                items.push(instruction(
                    Opcode::Spush,
                    vec![Operand::Symbol(format!("__ciq_field_{field}"))],
                ));
                expression(value, &definitions, &slots, &strings, &mut items);
                items.push(instruction(Opcode::Putv, vec![]));
                continue;
            }
            Operation::ObjectCallStatement {
                receiver,
                ref member,
                ref arguments,
                ..
            }
            | Operation::NativeCallStatement {
                receiver,
                ref member,
                ref arguments,
            } => {
                expression(receiver, &definitions, &slots, &strings, &mut items);
                items.push(instruction(
                    Opcode::Getsv,
                    vec![Operand::Symbol(format!(
                        "{}{member}",
                        if matches!(located.operation, Operation::ObjectCallStatement { .. }) {
                            "__ciq_member_"
                        } else {
                            "__ciq_native_"
                        }
                    ))],
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
            Operation::ReturnVoid => {
                items.push(instruction(Opcode::Return, vec![]));
                continue;
            }
            Operation::ModuleCallStatement {
                ref module,
                ref member,
                ref arguments,
            } => {
                statement_call(
                    Some(module),
                    member,
                    arguments,
                    &definitions,
                    &slots,
                    &strings,
                    &mut items,
                );
                continue;
            }
            Operation::SelfCallStatement {
                ref member,
                ref arguments,
            } => {
                statement_call(
                    None,
                    member,
                    arguments,
                    &definitions,
                    &slots,
                    &strings,
                    &mut items,
                );
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
            Operation::NativeModuleCallStatement {
                ref module,
                ref member,
                ..
            } => {
                // Validation admits only the supported zero-argument native functions.
                items.push(instruction(
                    Opcode::Getmv,
                    vec![
                        Operand::Symbol(format!(
                            "__ciq_sdk_{}",
                            module.rsplit('.').next().unwrap()
                        )),
                        Operand::Symbol(format!("__ciq_native_{member}")),
                    ],
                ));
                items.push(instruction(Opcode::Invokemz, vec![]));
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
        Negate,
        False,
        Not,
        ArrayRead,
        Compare(ComparisonOp),
        Invoke(usize),
        Context,
        NativeMember(String),
        SourceMember(String),
        TupleStore,
        TupleElement(ValueId, usize),
        FinishConstructor(usize),
    }
    let mut tasks = vec![Task::Value(root)];
    while let Some(task) = tasks.pop() {
        let (opcode, operands) = match task {
            Task::Value(value) => match definitions[&value] {
                Operation::NativeConstant { module, member, .. } => {
                    let (_, value) =
                        crate::native::constant(module, member, None).expect("validated constant");
                    (Opcode::Ipush1, vec![Operand::Bits(value as u64)])
                }
                Operation::SourceNew {
                    class, arguments, ..
                } => {
                    items.extend([
                        instruction(
                            Opcode::Getmv,
                            vec![
                                Operand::Symbol("__ciq_source_globals".into()),
                                Operand::Symbol(format!("__ciq_class_{class}")),
                            ],
                        ),
                        instruction(Opcode::Newc, vec![]),
                        instruction(Opcode::Dup, vec![Operand::Bits(0)]),
                        instruction(
                            Opcode::Getsv,
                            vec![Operand::Symbol("__ciq_native_initialize".into())],
                        ),
                        instruction(Opcode::Dup, vec![Operand::Bits(1)]),
                    ]);
                    tasks.push(Task::FinishConstructor(arguments.len()));
                    tasks.extend(arguments.iter().rev().map(|value| Task::Value(*value)));
                    continue;
                }
                Operation::NativeNew {
                    class: crate::NativeClass::Dc,
                    ..
                } => unreachable!("Dc construction is not admitted"),
                Operation::TupleNew { elements, .. } => {
                    items.extend([
                        instruction(Opcode::Ipush1, vec![Operand::Bits(elements.len() as u64)]),
                        instruction(Opcode::Newa, vec![]),
                    ]);
                    tasks.extend(
                        elements
                            .iter()
                            .enumerate()
                            .rev()
                            .map(|(index, value)| Task::TupleElement(*value, index)),
                    );
                    continue;
                }
                Operation::NativeNew {
                    class: class @ (crate::NativeClass::View | crate::NativeClass::BehaviorDelegate),
                    ..
                } => {
                    items.extend([
                        instruction(
                            Opcode::Getmv,
                            vec![
                                Operand::Symbol("__ciq_sdk_WatchUi".into()),
                                Operand::Symbol(format!(
                                    "__ciq_native_{}",
                                    class.name().rsplit('.').next().unwrap()
                                )),
                            ],
                        ),
                        instruction(Opcode::Newc, vec![]),
                        instruction(Opcode::Dup, vec![Operand::Bits(0)]),
                        instruction(
                            Opcode::Getsv,
                            vec![Operand::Symbol("__ciq_native_initialize".into())],
                        ),
                        instruction(Opcode::Dup, vec![Operand::Bits(1)]),
                        instruction(Opcode::Invokem, vec![Operand::Bits(1)]),
                    ]);
                    (Opcode::Popv, vec![])
                }
                Operation::ArrayNew { .. } => {
                    items.push(instruction(Opcode::Ipushz, vec![]));
                    (Opcode::Newa, vec![])
                }
                Operation::Null { .. } => (Opcode::Npush, vec![]),
                Operation::Char { value, .. } => {
                    (Opcode::Cpush, vec![Operand::Bits(u64::from(*value))])
                }
                Operation::Boolean { value: true, .. } => (Opcode::Btpush, vec![]),
                Operation::Boolean { value: false, .. } => (Opcode::Bfpush, vec![]),
                Operation::String { value, .. } => {
                    (Opcode::News, vec![Operand::String(strings[value].clone())])
                }
                Operation::SelfFieldRead { field, .. } => (
                    Opcode::Getselfv,
                    vec![Operand::Symbol(format!("__ciq_field_{field}"))],
                ),
                Operation::Integer { value: 0, .. } => (Opcode::Ipushz, vec![]),
                Operation::Integer { value, .. } => {
                    let (opcode, mask) = if i8::try_from(*value).is_ok() {
                        (Opcode::Ipush1, 0xff)
                    } else if i16::try_from(*value).is_ok() {
                        (Opcode::Ipush2, 0xffff)
                    } else if (-8_388_608..=8_388_607).contains(value) {
                        (Opcode::Ipush3, 0xffffff)
                    } else {
                        (Opcode::Ipush4, 0xffffffff)
                    };
                    (opcode, vec![Operand::Bits(u64::from(*value as u32) & mask)])
                }
                Operation::ObjectCall {
                    receiver,
                    member,
                    arguments,
                    ..
                }
                | Operation::NativeCall {
                    receiver,
                    member,
                    arguments,
                    ..
                } => {
                    tasks.push(Task::Invoke(arguments.len()));
                    if !arguments.is_empty() {
                        tasks.extend(arguments.iter().rev().map(|v| Task::Value(*v)));
                        tasks.push(Task::Context);
                    }
                    tasks.extend([
                        if matches!(definitions[&value], Operation::ObjectCall { .. }) {
                            Task::SourceMember(member.clone())
                        } else {
                            Task::NativeMember(member.clone())
                        },
                        Task::Value(*receiver),
                    ]);
                    continue;
                }
                Operation::ModuleCall {
                    member, arguments, ..
                }
                | Operation::SelfCall {
                    member, arguments, ..
                } => {
                    tasks.push(Task::Invoke(arguments.len()));
                    if !arguments.is_empty() {
                        tasks.extend(arguments.iter().rev().map(|value| Task::Value(*value)));
                        tasks.push(Task::Context);
                    }
                    let module = if let Operation::ModuleCall { module, .. } = definitions[&value] {
                        Some(module.as_str())
                    } else {
                        None
                    };
                    let Item::Instruction { opcode, operands } = call_target(module, member) else {
                        unreachable!()
                    };
                    (opcode, operands)
                }
                Operation::ParameterRead { index, .. } => {
                    (Opcode::Lgetv, vec![Operand::Bits(*index as u64 + 1)])
                }
                Operation::LocalRead { local, .. } => {
                    (Opcode::Lgetv, vec![Operand::Bits(slots[local])])
                }
                Operation::ArrayRead { array, index, .. } => {
                    tasks.extend([Task::ArrayRead, Task::Value(*index), Task::Value(*array)]);
                    continue;
                }
                Operation::Not { operand, .. } => {
                    tasks.extend([Task::Not, Task::False, Task::Value(*operand)]);
                    continue;
                }
                Operation::Negate { operand, .. } => {
                    tasks.extend([Task::Negate, Task::Value(*operand)]);
                    (Opcode::Ipushz, vec![])
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
            Task::Negate => (Opcode::Subv, vec![]),
            Task::False => (Opcode::Bfpush, vec![]),
            Task::Not => (Opcode::Eq, vec![]),
            Task::ArrayRead => (Opcode::Agetv, vec![]),
            Task::Invoke(0) => (Opcode::Invokemz, vec![]),
            Task::Invoke(count) => (Opcode::Invokem, vec![Operand::Bits(count as u64 + 1)]),
            Task::Context => (Opcode::Frpush, vec![]),
            Task::SourceMember(member) => (
                Opcode::Getsv,
                vec![Operand::Symbol(format!("__ciq_member_{member}"))],
            ),
            Task::NativeMember(member) => (
                Opcode::Getsv,
                vec![Operand::Symbol(format!("__ciq_native_{member}"))],
            ),
            Task::Compare(kind) => (
                match kind {
                    ComparisonOp::Equal => Opcode::Eq,
                    ComparisonOp::Greater => Opcode::Gt,
                    ComparisonOp::Less => Opcode::Lt,
                    ComparisonOp::LessEqual => Opcode::Lte,
                    ComparisonOp::GreaterEqual => Opcode::Gte,
                    ComparisonOp::NotEqual => Opcode::Ne,
                },
                vec![],
            ),
            Task::FinishConstructor(count) => {
                items.push(instruction(
                    Opcode::Invokem,
                    vec![Operand::Bits(count as u64 + 1)],
                ));
                (Opcode::Popv, vec![])
            }
            Task::TupleElement(value, index) => {
                items.push(instruction(Opcode::Dup, vec![Operand::Bits(0)]));
                items.push(instruction(
                    Opcode::Ipush1,
                    vec![Operand::Bits(index as u64)],
                ));
                tasks.push(Task::TupleStore);
                tasks.push(Task::Value(value));
                continue;
            }
            Task::TupleStore => (Opcode::Aputv, vec![]),
            Task::Binary(kind) => (
                match kind {
                    BinaryOp::Add => Opcode::Addv,
                    BinaryOp::Subtract => Opcode::Subv,
                    BinaryOp::Multiply => Opcode::Mulv,
                    BinaryOp::Divide => Opcode::Divv,
                    BinaryOp::Remainder => Opcode::Modv,
                },
                vec![],
            ),
        };
        items.push(instruction(opcode, operands));
    }
}

fn call_target(module: Option<&str>, member: &str) -> Item {
    let member = Operand::Symbol(format!("__ciq_member_{member}"));
    if let Some(module) = module {
        instruction(
            Opcode::Getmv,
            vec![Operand::Symbol(format!("__ciq_module_{module}")), member],
        )
    } else {
        instruction(Opcode::Getselfv, vec![member])
    }
}
fn statement_call(
    module: Option<&str>,
    member: &str,
    arguments: &[ValueId],
    definitions: &BTreeMap<ValueId, &Operation>,
    slots: &BTreeMap<LocalId, u64>,
    strings: &BTreeMap<String, String>,
    items: &mut Vec<Item>,
) {
    items.push(call_target(module, member));
    if !arguments.is_empty() {
        items.push(instruction(Opcode::Frpush, vec![]));
        for value in arguments {
            expression(*value, definitions, slots, strings, items);
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
}
