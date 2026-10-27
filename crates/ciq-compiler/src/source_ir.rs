//! Source name resolution and structured statement lowering to semantic blocks.
use crate::{
    BlockId, Diagnostic, Function, LocalId, LocatedOperation, Operation, Span, ValueId, parser,
};
use std::collections::BTreeMap;

#[derive(Clone, PartialEq, Eq)]
enum ConstructionType {
    Options(BTreeMap<String, crate::api::Type>),
    Tuple(crate::TupleType),
    Array(crate::ArrayElement),
    Dictionary(crate::DictionaryType),
}

/// Extract a supported container contract for contextual literal construction.
fn construction_type(ty: &crate::api::Type) -> Option<ConstructionType> {
    if let Some(unfolded) = crate::recursive::unfold(ty) {
        return construction_type(&unfolded);
    }

    match ty {
        crate::api::Type::TupleValue(tuple) => Some(ConstructionType::Tuple(tuple.clone())),
        crate::api::Type::Union(parts) => {
            // An opaque Array alternative does not select its element type;
            // let literals infer their own contract for overload-like profiles.
            let mut candidates = parts
                .iter()
                .filter(|ty| !matches!(ty, crate::api::Type::Named(_)))
                .filter_map(construction_type);
            let first = candidates.next()?;
            candidates.all(|other| other == first).then_some(first)
        }
        crate::api::Type::Options(fields) => Some(ConstructionType::Options(fields.clone())),
        crate::api::Type::Array(element) => {
            crate::ArrayElement::from_api_type(element).map(ConstructionType::Array)
        }
        crate::api::Type::Dictionary { .. } => {
            crate::DictionaryType::from_api_type(ty).map(ConstructionType::Dictionary)
        }
        crate::api::Type::Named(name) if name == "Toybox.Lang.Array" => {
            Some(ConstructionType::Array(crate::ArrayElement::NullableObject))
        }
        _ => None,
    }
}

/// Resolve an unqualified call through owner members and global declarations.
/// Keep inherited native dispatch and unsupported lexical callable values explicit.
fn resolve_implicit_call(
    source: &str,
    kind: parser::ExpressionKind,
    locals: &BTreeMap<String, LocalId>,
    parameters: &BTreeMap<String, usize>,
    calls: &CallContext,
    span: Span,
) -> Result<parser::ExpressionKind, Diagnostic> {
    let parser::ExpressionKind::UnqualifiedCall { member, arguments } = kind else {
        return Ok(kind);
    };
    let owner = if calls.class_bases.contains_key("self") {
        "self"
    } else {
        calls.current_module.as_deref().unwrap_or("")
    };
    if !owner.is_empty() && calls.get(&(owner.into(), member.clone())).is_some() {
        return Ok(parser::ExpressionKind::OwnerCall {
            module: owner.into(),
            member,
            arguments,
        });
    }
    if owner == "self" && (member == "method" || calls.inherited_object_member(owner, &member)) {
        if locals.contains_key(&member)
            || parameters.contains_key(&member)
            || calls.field_types.contains_key(&member)
        {
            return Err(Diagnostic::at(
                source,
                span.start,
                format!(
                    "calling a local, parameter, or field named {member} is unsupported; use self.{member} explicitly"
                ),
            ));
        }
        return Ok(parser::ExpressionKind::ModuleCall {
            module: "self".into(),
            member,
            arguments,
        });
    }
    if owner == "self" && calls.field_types.contains_key(&member) {
        return Err(Diagnostic::at(
            source,
            span.start,
            "calling an instance field is unsupported; use an explicit global call to bypass it",
        ));
    }
    if calls.get(&(String::new(), member.clone())).is_some() {
        return Ok(parser::ExpressionKind::GlobalCall { member, arguments });
    }
    Err(Diagnostic::at(
        source,
        span.start,
        format!("unresolved function {member:?}; lexical callable values are unsupported"),
    ))
}

/// Emit a checked zero-argument native constructor operation. Keep constructor
/// diagnostics outside recursive expression emission to preserve its depth budget.
fn emit_native_constructor(
    source: &str,
    kind: parser::ExpressionKind,
    span: Span,
    locals: &BTreeMap<String, LocalId>,
    parameters: &BTreeMap<String, usize>,
    result: ValueId,
) -> Result<Operation, Diagnostic> {
    let parser::ExpressionKind::NativeNew { class, qualifier } = kind else {
        unreachable!("native construction");
    };
    if locals.contains_key(&qualifier) || parameters.contains_key(&qualifier) {
        return Err(Diagnostic::at(
            source,
            span.start,
            "native constructor qualifier is shadowed by a local or parameter",
        ));
    }
    Ok(match class {
        parser::NativeConstructor::Object(class) => Operation::NativeNew { result, class },
        parser::NativeConstructor::WeakReference => Operation::WeakReferenceNew { result },
    })
}

/// Emit a source expression and return its semantic value ID. Keep recursive
/// calls and arithmetic separate from atom emission to preserve the depth budget.
fn emit(
    source: &str,
    expression: parser::Expression,
    locals: &BTreeMap<String, LocalId>,
    parameters: &BTreeMap<String, usize>,
    operations: &mut Vec<LocatedOperation>,
    calls: &CallContext,
) -> Result<ValueId, Diagnostic> {
    let kind = resolve_implicit_call(
        source,
        expression.kind,
        locals,
        parameters,
        calls,
        expression.span,
    )?;
    match kind {
        kind @ parser::ExpressionKind::MemberRead { .. } => emit_member_chain(
            source,
            kind,
            expression.span,
            locals,
            parameters,
            operations,
            calls,
        ),
        kind @ (parser::ExpressionKind::ValueCall { .. }
        | parser::ExpressionKind::ModuleCall { .. }
        | parser::ExpressionKind::GlobalCall { .. }
        | parser::ExpressionKind::OwnerCall { .. }) => {
            let operation = emit_call(
                source,
                kind,
                expression.span,
                locals,
                parameters,
                operations,
                calls,
            )?;
            let result = ValueId(operations.len() as u32);
            operations.push(LocatedOperation {
                operation,
                span: Some(expression.span),
            });
            Ok(result)
        }
        kind @ parser::ExpressionKind::Binary { .. } => emit_binary_chain(
            source,
            parser::Expression {
                kind,
                span: expression.span,
            },
            locals,
            parameters,
            operations,
            calls,
        ),
        kind => emit_non_call(
            source,
            parser::Expression {
                kind,
                span: expression.span,
            },
            locals,
            parameters,
            operations,
            calls,
        ),
    }
}

/// Emit a property chain with declaration and flow metadata, evaluating its
/// base once. Keep this work outside the recursive expression dispatch frame.
fn emit_member_chain(
    source: &str,
    kind: parser::ExpressionKind,
    span: Span,
    locals: &BTreeMap<String, LocalId>,
    parameters: &BTreeMap<String, usize>,
    operations: &mut Vec<LocatedOperation>,
    calls: &CallContext,
) -> Result<ValueId, Diagnostic> {
    let mut members = Vec::new();
    let mut base = parser::Expression { kind, span };
    while let parser::ExpressionKind::MemberRead { receiver, member } = base.kind {
        members.push((member, base.span));
        base = *receiver;
    }
    let mut receiver = emit(source, base, locals, parameters, operations, calls)?;
    for (member, span) in members.into_iter().rev() {
        let result = ValueId(operations.len() as u32);
        let field_type = calls
            .source_field(operations, receiver, &member)
            .map_err(|message| Diagnostic::at(source, span.start, message))?;
        let view_type = if field_type.is_none() {
            crate::field_flow::prefix_member_read(
                operations,
                receiver,
                &member,
                calls.parameter_types.clone(),
            )
            .map_err(|message| Diagnostic::at(source, span.start, message))?
        } else {
            None
        };
        operations.push(LocatedOperation {
            operation: Operation::MemberRead {
                result,
                receiver,
                member,
                field_type,
                view_type,
            },
            span: Some(span),
        });
        receiver = result;
    }
    Ok(receiver)
}

fn emit_non_call(
    source: &str,
    expression: parser::Expression,
    locals: &std::collections::BTreeMap<String, LocalId>,
    parameters: &std::collections::BTreeMap<String, usize>,
    operations: &mut Vec<LocatedOperation>,
    calls: &CallContext,
) -> Result<ValueId, Diagnostic> {
    let operation = match expression.kind {
        parser::ExpressionKind::UnqualifiedCall { .. } => unreachable!("resolved implicit call"),
        parser::ExpressionKind::DictionaryNew => Operation::DictionaryNew {
            dictionary_type: None,
            result: ValueId(operations.len() as u32),
        },
        parser::ExpressionKind::Joined(value) => return Ok(value),
        parser::ExpressionKind::Conditional { .. } => {
            unreachable!("conditional must be materialized")
        }
        parser::ExpressionKind::MemberRead { .. } => {
            unreachable!("member chain dispatched before non-call emission")
        }
        parser::ExpressionKind::SelfField(field) => {
            let declared = calls
                .field_types
                .get(&field)
                .cloned()
                .unwrap_or(crate::ReturnType::Integer);
            let tracked = crate::field_flow::positional(&declared);
            let result_type = if tracked {
                crate::field_flow::prefix_read(
                    operations,
                    &field,
                    &declared,
                    calls.parameter_types.clone(),
                )
                .map_err(|message| Diagnostic::at(source, expression.span.start, message))?
            } else {
                declared.clone()
            };
            Operation::SelfFieldRead {
                result: ValueId(operations.len() as u32),
                field,
                result_type,
                declared_type: tracked.then_some(declared),
            }
        }
        parser::ExpressionKind::SourceNew { class, arguments } => {
            let base = calls.class_bases.get(&class).copied().ok_or_else(|| {
                Diagnostic::at(
                    source,
                    expression.span.start,
                    "source constructor requires a declared class",
                )
            })?;
            if locals.contains_key(&class) || parameters.contains_key(&class) {
                return Err(Diagnostic::at(
                    source,
                    expression.span.start,
                    "source construction class is shadowed",
                ));
            }
            let arguments = arguments
                .into_iter()
                .map(|argument| emit(source, argument, locals, parameters, operations, calls))
                .collect::<Result<Vec<_>, _>>()?;
            Operation::SourceNew {
                result: ValueId(operations.len() as u32),
                class,
                base,
                arguments,
            }
        }
        parser::ExpressionKind::NativeConstant {
            module,
            member,
            qualifier,
        } => {
            if locals.contains_key(&qualifier) || parameters.contains_key(&qualifier) {
                return Err(Diagnostic::at(
                    source,
                    expression.span.start,
                    "native constant qualifier is shadowed",
                ));
            }
            Operation::NativeConstant {
                result: ValueId(operations.len() as u32),
                module,
                member,
            }
        }
        kind @ parser::ExpressionKind::NativeNew { .. } => emit_native_constructor(
            source,
            kind,
            expression.span,
            locals,
            parameters,
            ValueId(operations.len() as u32),
        )?,
        parser::ExpressionKind::DictionaryLiteral(_)
        | parser::ExpressionKind::ArrayLiteral(_)
        | parser::ExpressionKind::ByteArrayLiteral(_)
        | parser::ExpressionKind::ArrayAllocate(_)
        | parser::ExpressionKind::ByteArrayAllocate(_) => {
            return Err(Diagnostic::at(
                source,
                expression.span.start,
                "array literal was not materialized before value emission",
            ));
        }
        kind @ parser::ExpressionKind::ArrayRead { .. } => {
            emit_array_read(source, kind, locals, parameters, operations, calls)?
        }
        parser::ExpressionKind::Temporary(local) => Operation::LocalRead {
            result: ValueId(operations.len() as u32),
            local,
        },
        parser::ExpressionKind::Logical { .. } => {
            return Err(Diagnostic::at(
                source,
                expression.span.start,
                "logical expression was not materialized before value emission",
            ));
        }
        kind @ (parser::ExpressionKind::Not(_)
        | parser::ExpressionKind::Step { .. }
        | parser::ExpressionKind::Positive(_)
        | parser::ExpressionKind::Complement(_)
        | parser::ExpressionKind::Negate(_)) => {
            emit_unary(source, kind, locals, parameters, operations, calls)?
        }
        parser::ExpressionKind::SelfValue => Operation::SelfValue {
            result: ValueId(operations.len() as u32),
        },
        parser::ExpressionKind::Symbol(name) => Operation::Symbol {
            result: ValueId(operations.len() as u32),
            name,
        },
        parser::ExpressionKind::Char(value) => Operation::Char {
            result: ValueId(operations.len() as u32),
            value,
        },
        parser::ExpressionKind::Boolean(value) => Operation::Boolean {
            result: ValueId(operations.len() as u32),
            value,
        },
        parser::ExpressionKind::Null => Operation::Null {
            result: ValueId(operations.len() as u32),
        },
        parser::ExpressionKind::String(value) => Operation::String {
            result: ValueId(operations.len() as u32),
            value,
        },
        parser::ExpressionKind::Float(bits) => Operation::Float {
            result: ValueId(operations.len() as u32),
            bits,
        },
        parser::ExpressionKind::Double(bits) => Operation::Double {
            result: ValueId(operations.len() as u32),
            bits,
        },
        parser::ExpressionKind::Long(value) => Operation::Long {
            result: ValueId(operations.len() as u32),
            value,
        },
        parser::ExpressionKind::Integer(value) => Operation::Integer {
            result: ValueId(operations.len() as u32),
            value,
        },
        parser::ExpressionKind::Local(name) => {
            let result = ValueId(operations.len() as u32);
            if let Some(local) = locals.get(&name) {
                if operations.iter().any(
                    |op| matches!(op.operation,Operation::LocalVariable{local:at,..} if at==*local),
                ) {
                    let value_type = crate::local_flow::prefix_read(
                        operations,
                        *local,
                        calls.parameter_types.clone(),
                    )
                    .ok_or_else(|| {
                        Diagnostic::at(
                            source,
                            expression.span.start,
                            "cannot infer variable read before assignment",
                        )
                    })?;
                    Operation::VariableRead {
                        result,
                        local: *local,
                        value_type,
                    }
                } else {
                    Operation::LocalRead {
                        result,
                        local: *local,
                    }
                }
            } else if let Some(index) = parameters.get(&name) {
                Operation::ParameterRead {
                    result,
                    index: *index,
                }
            } else {
                return Err(Diagnostic::at(
                    source,
                    expression.span.start,
                    format!("unknown or not yet initialized local {name:?}"),
                ));
            }
        }
        parser::ExpressionKind::ValueCall { .. }
        | parser::ExpressionKind::ModuleCall { .. }
        | parser::ExpressionKind::GlobalCall { .. }
        | parser::ExpressionKind::OwnerCall { .. }
        | parser::ExpressionKind::Binary { .. } => unreachable!("compound expression dispatched"),
        parser::ExpressionKind::Cast {
            operand,
            mut target,
        } => {
            let operand = emit(source, *operand, locals, parameters, operations, calls)?;
            if let Some((name, nullable)) = crate::source_objects::target_contract(&target) {
                if locals.contains_key(name) || parameters.contains_key(name) {
                    return Err(Diagnostic::at(
                        source,
                        expression.span.start,
                        "source cast target is shadowed by a local or parameter",
                    ));
                }
                Operation::SourceCast {
                    result: ValueId(operations.len() as u32),
                    operand,
                    class: calls.source_class(name).map_err(|message| {
                        Diagnostic::at(source, expression.span.start, message)
                    })?,
                    nullable,
                }
            } else {
                crate::structural::resolve_api(&mut target, &mut |class| {
                    if locals.contains_key(&class.name) || parameters.contains_key(&class.name) {
                        return Err(Diagnostic::at(
                            source,
                            expression.span.start,
                            "source cast target is shadowed by a local or parameter",
                        ));
                    }
                    *class = calls.source_class(&class.name).map_err(|message| {
                        Diagnostic::at(source, expression.span.start, message)
                    })?;
                    Ok(())
                })?;
                Operation::Cast {
                    result: ValueId(operations.len() as u32),
                    operand,
                    target,
                }
            }
        }
        parser::ExpressionKind::InstanceOf { operand, class } => {
            let operand = emit(source, *operand, locals, parameters, operations, calls)?;
            if !class.contains('.') {
                if locals.contains_key(&class) || parameters.contains_key(&class) {
                    return Err(Diagnostic::at(
                        source,
                        expression.span.start,
                        "source class-test target is shadowed by a local or parameter",
                    ));
                }
                Operation::SourceInstanceOf {
                    result: ValueId(operations.len() as u32),
                    operand,
                    class: calls.source_class(&class).map_err(|message| {
                        Diagnostic::at(source, expression.span.start, message)
                    })?,
                }
            } else {
                Operation::InstanceOf {
                    result: ValueId(operations.len() as u32),
                    operand,
                    class,
                }
            }
        }
        parser::ExpressionKind::Has { receiver, symbol } => {
            let receiver = emit(source, *receiver, locals, parameters, operations, calls)?;
            let symbol = emit(source, *symbol, locals, parameters, operations, calls)?;
            Operation::Has {
                result: ValueId(operations.len() as u32),
                receiver,
                symbol,
            }
        }
        parser::ExpressionKind::Compare { kind, left, right } => {
            let left = emit(source, *left, locals, parameters, operations, calls)?;
            let right = emit(source, *right, locals, parameters, operations, calls)?;
            Operation::Compare {
                result: ValueId(operations.len() as u32),
                kind,
                left,
                right,
            }
        }
    };
    let result = ValueId(operations.len() as u32);
    operations.push(LocatedOperation {
        operation,
        span: Some(expression.span),
    });
    Ok(result)
}

/// Emit one unary operand and its operation without enlarging every recursive
/// expression frame. Source/parser depth limits remain unchanged.
fn emit_unary(
    source: &str,
    kind: parser::ExpressionKind,
    locals: &BTreeMap<String, LocalId>,
    parameters: &BTreeMap<String, usize>,
    operations: &mut Vec<LocatedOperation>,
    calls: &CallContext,
) -> Result<Operation, Diagnostic> {
    enum Unary {
        Not,
        Positive,
        Complement,
        Negate,
        Step(bool),
    }
    use parser::ExpressionKind as E;
    let (operand, kind) = match kind {
        E::Not(operand) => (operand, Unary::Not),
        E::Positive(operand) => (operand, Unary::Positive),
        E::Complement(operand) => (operand, Unary::Complement),
        E::Negate(operand) => (operand, Unary::Negate),
        E::Step { operand, increment } => (operand, Unary::Step(increment)),
        _ => unreachable!("unary expression"),
    };
    let operand = emit(source, *operand, locals, parameters, operations, calls)?;
    let result = ValueId(operations.len() as u32);
    Ok(match kind {
        Unary::Not => Operation::Not { result, operand },
        Unary::Positive => Operation::Positive { result, operand },
        Unary::Complement => Operation::Complement { result, operand },
        Unary::Negate => Operation::Negate { result, operand },
        Unary::Step(increment) => Operation::Step {
            result,
            operand,
            increment,
        },
    })
}

/// Emit a left-associated arithmetic chain without one Rust frame per term.
/// Preserve operand order and source spans; grouped operands use bounded depth.
fn emit_binary_chain(
    source: &str,
    mut expression: parser::Expression,
    locals: &BTreeMap<String, LocalId>,
    parameters: &BTreeMap<String, usize>,
    operations: &mut Vec<LocatedOperation>,
    calls: &CallContext,
) -> Result<ValueId, Diagnostic> {
    let mut chain = Vec::new();
    while let parser::ExpressionKind::Binary { kind, left, right } = expression.kind {
        chain.push((kind, *right, expression.span));
        expression = *left;
    }
    let mut left = emit(source, expression, locals, parameters, operations, calls)?;
    for (kind, right, span) in chain.into_iter().rev() {
        let right = emit(source, right, locals, parameters, operations, calls)?;
        let result = ValueId(operations.len() as u32);
        operations.push(LocatedOperation {
            operation: Operation::Binary {
                result,
                kind,
                left,
                right,
            },
            span: Some(span),
        });
        left = result;
    }
    Ok(left)
}

/// Emit receiver/key operands in order and select Array or Dictionary read IR. Keep this
/// frame separate so long arithmetic trees retain their existing stack budget.
fn emit_array_read(
    source: &str,
    kind: parser::ExpressionKind,
    locals: &BTreeMap<String, LocalId>,
    parameters: &BTreeMap<String, usize>,
    operations: &mut Vec<LocatedOperation>,
    calls: &CallContext,
) -> Result<Operation, Diagnostic> {
    let parser::ExpressionKind::ArrayRead { array, index } = kind else {
        unreachable!()
    };
    let array = emit(source, *array, locals, parameters, operations, calls)?;
    let index = emit(source, *index, locals, parameters, operations, calls)?;
    let result = ValueId(operations.len() as u32);
    if matches!(
        crate::source_types::value_type(operations, array, calls.parameter_types.clone()),
        Some(crate::api::Type::Dictionary { .. } | crate::api::Type::Options(_))
    ) {
        Ok(Operation::DictionaryRead {
            result,
            dictionary: array,
            key: index,
        })
    } else {
        Ok(Operation::ArrayRead {
            result,
            array,
            index,
        })
    }
}

/// Resolve call ownership and emit receiver/arguments in source order. Keep call
/// construction separate so long arithmetic trees retain small recursive frames.
fn emit_call(
    source: &str,
    kind: parser::ExpressionKind,
    span: Span,
    locals: &BTreeMap<String, LocalId>,
    parameters: &BTreeMap<String, usize>,
    operations: &mut Vec<LocatedOperation>,
    calls: &CallContext,
) -> Result<Operation, Diagnostic> {
    let (receiver, module, member, arguments) = match kind {
        parser::ExpressionKind::GlobalCall { member, arguments } => (None, None, member, arguments),
        parser::ExpressionKind::OwnerCall {
            module,
            member,
            arguments,
        } => (None, Some(module), member, arguments),
        parser::ExpressionKind::ValueCall {
            receiver,
            member,
            arguments,
        } => (Some(*receiver), None, member, arguments),
        parser::ExpressionKind::ModuleCall {
            module,
            member,
            arguments,
        } => {
            if module == "Application.AppBase" {
                return Err(Diagnostic::at(
                    source,
                    span.start,
                    "native base initialization has no expression result",
                ));
            }
            if locals.contains_key(&module) || parameters.contains_key(&module) {
                (
                    Some(parser::Expression {
                        kind: parser::ExpressionKind::Local(module),
                        span,
                    }),
                    None,
                    member,
                    arguments,
                )
            } else if module == "self"
                && (member == "method" || calls.inherited_object_member("self", &member))
                && calls.get(&(module.clone(), member.clone())).is_none()
            {
                (
                    Some(parser::Expression {
                        kind: parser::ExpressionKind::SelfValue,
                        span,
                    }),
                    None,
                    member,
                    arguments,
                )
            } else {
                (None, Some(module), member, arguments)
            }
        }
        _ => unreachable!("only calls enter call construction"),
    };
    let receiver = receiver
        .map(|value| emit(source, value, locals, parameters, operations, calls))
        .transpose()?;
    let classes = receiver
        .and_then(|value| {
            crate::source_objects::contract(operations, value, calls.parameter_types.clone()).map(
                |c| {
                    c.classes()
                        .iter()
                        .map(|c| c.name.clone())
                        .collect::<Vec<_>>()
                },
            )
        })
        .filter(|classes| {
            classes.iter().any(|class| {
                (member != "method" && !calls.inherited_object_member(class, &member))
                    || calls.get(&(class.clone(), member.clone())).is_some()
            })
        });
    let binding_signature = if member == "method" && classes.is_none() {
        if let [
            parser::Expression {
                kind: parser::ExpressionKind::Symbol(selector),
                ..
            },
        ] = arguments.as_slice()
        {
            receiver.and_then(|receiver| {
                let owners = if operations.iter().any(
                    |op| matches!(op.operation,Operation::SelfValue{result} if result==receiver),
                ) {
                    vec!["self".to_owned()]
                } else {
                    crate::source_objects::contract(
                        operations,
                        receiver,
                        calls.parameter_types.clone(),
                    )
                    .filter(|c| !c.nullable() && c.native().is_empty())
                    .map(|c| c.classes().iter().map(|c| c.name.clone()).collect())
                    .unwrap_or_default()
                };
                if owners.is_empty() {
                    let mut types = crate::source_types::Types::new(calls.parameter_types.clone());
                    for operation in operations.iter() {
                        types.observe(&operation.operation);
                    }
                    return crate::callbacks::native_signature(
                        types.value_type(receiver)?,
                        selector,
                        None,
                    )
                    .ok();
                }
                let signatures =
                    owners
                        .iter()
                        .map(|owner| {
                            if calls.class_bases.get(owner).is_some_and(|base| {
                                crate::native::callback(*base, selector).is_some()
                            }) {
                                return None;
                            }
                            calls
                                .get(&(owner.clone(), selector.clone()))
                                .map(|signature| crate::StructuralMethod {
                                    parameters: signature.parameters.clone(),
                                    result: signature.result.clone(),
                                })
                                .or_else(|| {
                                    crate::api::core()
                                        .method_for_type(
                                            &crate::api::Type::named(
                                                calls.class_bases.get(owner)?.api_name(),
                                            ),
                                            selector,
                                        )
                                        .and_then(|method| {
                                            if method.variadic.is_some() {
                                                return None;
                                            }
                                            crate::StructuralMethod::from_native(
                                                method.parameters,
                                                method.result,
                                            )
                                        })
                                })
                        })
                        .collect::<Option<Vec<_>>>()?;
                let first = signatures.first()?;
                signatures.iter().all(|s| s == first).then(|| first.clone())
            })
        } else {
            None
        }
    } else {
        None
    };
    let arguments = arguments
        .into_iter()
        .map(|argument| emit(source, argument, locals, parameters, operations, calls))
        .collect::<Result<Vec<_>, _>>()?;
    let result = ValueId(operations.len() as u32);
    Ok(if let Some(receiver) = receiver {
        if let Some(classes) = classes {
            let contract = crate::source_objects::contract(
                operations,
                receiver,
                calls.parameter_types.clone(),
            )
            .expect("source call retains its receiver contract");
            let targets = calls.member_targets(&contract, &member);
            if calls.unchecked && targets.is_empty() && calls.known_selector(&member) {
                return Ok(Operation::NativeCall {
                    result,
                    receiver,
                    member,
                    arguments,
                });
            }
            // Candidate result selection uses native declarations. Actual arguments,
            // exact source witnesses, and API availability are checked by the unit.
            let result_type = targets
                .result(
                    &member,
                    |ty| crate::native::value_parameters(ty, &member).unwrap_or_default(),
                    None,
                )
                .map_err(|message| Diagnostic::at(source, span.start, message))?;
            Operation::ObjectCall {
                result,
                result_type,
                receiver,
                classes,
                member,
                arguments,
            }
        } else if let Some(signature) = binding_signature {
            Operation::BindMethod {
                result,
                receiver,
                selector: arguments[0],
                signature: Box::new(signature),
            }
        } else {
            if calls.unchecked {
                if let Some(ty) = crate::source_types::value_type(
                    operations,
                    receiver,
                    calls.parameter_types.clone(),
                ) {
                    if ty != crate::api::Type::Any
                        && crate::native::value_parameters(&ty, &member).is_none()
                        && !calls.known_selector(&member)
                    {
                        return Err(Diagnostic::at(
                            source,
                            span.start,
                            format!("undefined member selector {member}"),
                        ));
                    }
                }
            }
            Operation::NativeCall {
                result,
                receiver,
                member,
                arguments,
            }
        }
    } else if module.as_deref().is_some_and(|module| {
        matches!(
            module,
            "Lang" | "System" | "WatchUi" | "Graphics" | "Application"
        )
    }) {
        Operation::NativeModuleCall {
            result,
            module: format!("Toybox.{}", module.unwrap()),
            member,
            arguments,
        }
    } else if module.as_deref() == Some("self") {
        Operation::SelfCall {
            result,
            result_type: calls
                .get(&("self".into(), member.clone()))
                .map(|signature| signature.result.clone())
                .unwrap_or(crate::ReturnType::Integer),
            member,
            arguments,
        }
    } else if let Some(module) = module {
        Operation::ModuleCall {
            result,
            result_type: calls
                .get(&(module.clone(), member.clone()))
                .map(|signature| signature.result.clone())
                .unwrap_or(crate::ReturnType::Integer),
            module,
            member,
            arguments,
        }
    } else {
        Operation::GlobalCall {
            result,
            result_type: calls
                .get(&(String::new(), member.clone()))
                .map(|signature| signature.result.clone())
                .unwrap_or(crate::ReturnType::Integer),
            member,
            arguments,
        }
    })
}

/// Resolved break destination and optional loop latch for a structured construct.
/// Switches intercept break while continue searches for the enclosing loop.
#[derive(Clone)]
struct ControlTargets {
    break_to: BlockId,
    continue_to: Option<BlockId>,
}

struct Builder<'a> {
    positional_callback: bool,
    source: &'a str,
    calls: &'a CallContext,
    result_type: crate::ReturnType,
    // Bare returns supply Null only in Any bodies with no value returns.
    // Mixed value/bare-return bodies must still fail the return-contract check.
    null_return: bool,
    parameters: BTreeMap<String, usize>,
    operations: Vec<LocatedOperation>,
    types: crate::source_types::Types,
    next_local: u32,
    named_locals: std::collections::BTreeSet<LocalId>,
    // Named tuple-derived storage keeps inferred element contracts after its
    // positional shape is lost. Ordinary typed Arrays use invariant native calls.
    shape_arrays: std::collections::BTreeSet<LocalId>,
    pending_locals: BTreeMap<LocalId, usize>,
    next_block: u32,
    controls: Vec<ControlTargets>,
}

/// Discover named mutation receivers; type checking selects positional or
/// ordinary Array behavior after resolving the receiver.
fn tuple_mutation_name(expression: &parser::Expression) -> Option<(&str, &str)> {
    use parser::ExpressionKind as E;
    let (name, member) = match &expression.kind {
        E::ModuleCall { module, member, .. } => (module.as_str(), member.as_str()),
        E::ValueCall {
            receiver, member, ..
        } => {
            let E::Local(name) = &receiver.kind else {
                return None;
            };
            (name.as_str(), member.as_str())
        }
        _ => return None,
    };
    matches!(member, "add" | "addAll" | "remove" | "removeAll").then_some((name, member))
}

fn tuple_append_name(expression: &parser::Expression) -> Option<(&str, bool)> {
    let (name, member) = tuple_mutation_name(expression)?;
    match member {
        "add" => Some((name, false)),
        "addAll" => Some((name, true)),
        _ => None,
    }
}

impl Builder<'_> {
    fn tuple_storage(&self, local: LocalId) -> bool {
        self.types.local_read_type(local).is_some_and(|ty| {
            crate::tuples::read_contract(&ty, None).is_some()
                || (self.shape_arrays.contains(&local)
                    && self
                        .controls
                        .iter()
                        .any(|control| control.continue_to.is_some())
                    && crate::unions::alternatives(&ty).iter().all(|part| {
                        matches!(
                            part,
                            crate::api::Type::Array(_) | crate::api::Type::TupleValue(_)
                        )
                    }))
        })
    }
    /// Include tuple append effects when deciding which expression trees need
    /// staged operands. Ordinary Arrays retain their existing staging rules.
    fn needs_staging(
        &self,
        expression: &parser::Expression,
        locals: &BTreeMap<String, LocalId>,
    ) -> bool {
        if expression.needs_materialization() {
            return true;
        }
        let mut needed = false;
        expression.visit(&mut |expression| {
            if matches!(&expression.kind, parser::ExpressionKind::ValueCall { receiver, member, .. }
                if matches!(member.as_str(), "add" | "addAll")
                    && !matches!(receiver.kind, parser::ExpressionKind::Local(_)))
            {
                needed = true;
            }
            if let Some((name, _)) = tuple_append_name(expression) {
                needed |= locals
                    .get(name)
                    .is_some_and(|local| self.tuple_storage(*local));
            }
        });
        needed
    }

    /// Evaluate an append argument once, retaining positional literal contents.
    /// Empty literals have a known zero-slot shape instead of an unknown Array.
    fn append_argument(
        &mut self,
        arguments: Vec<parser::Expression>,
        span: Span,
        locals: &BTreeMap<String, LocalId>,
    ) -> Result<parser::Expression, Diagnostic> {
        use parser::{Expression, ExpressionKind as E};
        let [argument]: [Expression; 1] = arguments.try_into().map_err(|_| {
            Diagnostic::at(
                self.source,
                span.start,
                "tuple add/addAll require exactly one argument",
            )
        })?;
        let argument = if let E::ArrayLiteral(elements) = argument.kind {
            let value = self.tuple_value(elements, None, argument.span, locals)?;
            let temporary = self.local_id(argument.span)?;
            self.push(
                Operation::LocalInitialize {
                    local: temporary,
                    value,
                },
                Some(argument.span),
            );
            Expression {
                kind: E::Temporary(temporary),
                span: argument.span,
            }
        } else {
            self.snapshot(argument, locals)?
        };
        Ok(argument)
    }

    /// Append through an evaluated receiver without rebinding its parent place.
    /// The result gets its own slot view; aliases retain their previous views.
    fn computed_tuple_append(
        &mut self,
        receiver: parser::Expression,
        arguments: Vec<parser::Expression>,
        spread: bool,
        positional: bool,
        span: Span,
        locals: &BTreeMap<String, LocalId>,
    ) -> Result<parser::Expression, Diagnostic> {
        let argument = self.append_argument(arguments, span, locals)?;
        let array = self.value(receiver, locals)?;
        let value = self.value(argument, locals)?;
        self.types
            .append_type(array, value, spread, positional)
            .map_err(|message| Diagnostic::at(self.source, span.start, message))?;
        let result = ValueId(self.operations.len() as u32);
        self.push(
            Operation::TupleAppend {
                result,
                array,
                value,
                spread,
                positional,
            },
            Some(span),
        );
        let local = self.local_id(span)?;
        self.push(
            Operation::LocalInitialize {
                local,
                value: result,
            },
            Some(span),
        );
        Ok(parser::Expression {
            kind: parser::ExpressionKind::Temporary(local),
            span,
        })
    }

    /// Check an evaluated receiver's candidate positional profile. The final IR
    /// validator independently checks its null safety and slot contracts.
    fn positional_snapshot(&self, receiver: &parser::Expression) -> bool {
        match receiver.kind {
            parser::ExpressionKind::Temporary(local) => self
                .types
                .local_type(local)
                .is_some_and(|ty| crate::tuples::read_contract(ty, None).is_some()),
            _ => false,
        }
    }

    /// Append to named storage, retaining nested positional refinements. Capture
    /// its original tuple view when an argument can erase that binding's view.
    fn tuple_append(
        &mut self,
        expression: parser::Expression,
        locals: &BTreeMap<String, LocalId>,
    ) -> Result<parser::Expression, Diagnostic> {
        use parser::{Expression, ExpressionKind as E};
        let span = expression.span;
        let (name, spread) = tuple_append_name(&expression).expect("known named append");
        let positional = !self
            .controls
            .iter()
            .any(|control| control.continue_to.is_some());
        let name = name.to_owned();
        let local = locals[&name];
        let arguments = match expression.kind {
            E::ModuleCall { arguments, .. } | E::ValueCall { arguments, .. } => arguments,
            _ => unreachable!(),
        };
        self.promote_variable(local);
        let mut can_erase = false;
        for argument in &arguments {
            argument.visit(&mut |expression| {
                can_erase |= matches!(tuple_mutation_name(expression),
                    Some((owner, "remove" | "removeAll")) if owner == name);
            });
        }
        let receiver = if can_erase {
            Some(self.snapshot(
                Expression {
                    kind: E::Local(name.clone()),
                    span,
                },
                locals,
            )?)
        } else {
            None
        };
        let argument = self.append_argument(arguments, span, locals)?;
        let erased = self.types.local_read_type(local).is_some_and(|ty|
            matches!(*ty, crate::api::Type::Array(ref element) if **element == crate::api::Type::Any));
        let receiver = match (erased, receiver) {
            (true, Some(receiver)) => receiver,
            _ => Expression {
                kind: E::Local(name.clone()),
                span,
            },
        };
        let array = self.value(receiver, locals)?;
        let value = self.value(argument, locals)?;
        self.types
            .append_type(array, value, spread, positional)
            .map_err(|message| Diagnostic::at(self.source, span.start, message))?;
        let result = ValueId(self.operations.len() as u32);
        self.push(
            Operation::TupleAppend {
                result,
                array,
                value,
                spread,
                positional,
            },
            Some(span),
        );
        self.push(
            Operation::LocalAssign {
                local,
                value: result,
            },
            Some(span),
        );
        self.shape_arrays.insert(local);
        Ok(Expression {
            kind: E::Local(name),
            span,
        })
    }
    /// Promote a named slot when assignment changes its inferred contract.
    /// Preserve frame identity and earlier value snapshots; hidden temporaries
    /// keep fixed contracts and do not participate in variable inference.
    fn promote_variable(&mut self, local: LocalId) {
        if self
            .operations
            .iter()
            .any(|op| matches!(op.operation,Operation::LocalVariable{local:at,..}if at==local))
        {
            return;
        }
        for op in &mut self.operations {
            match op.operation {
                Operation::LocalInitialize { local: at, value } if at == local => {
                    op.operation = Operation::LocalVariable {
                        local,
                        value: Some(value),
                    };
                    self.types.observe(&op.operation);
                }
                Operation::LocalDeclare { local: at, .. } if at == local => {
                    op.operation = Operation::LocalVariable { local, value: None };
                    self.types.observe(&op.operation);
                }
                Operation::LocalRead { local: at, result } if at == local => {
                    let value_type = self
                        .types
                        .join_contract(result)
                        .expect("earlier source local read has a contract");
                    op.operation = Operation::VariableRead {
                        result,
                        local,
                        value_type,
                    };
                }
                _ => {}
            }
        }
    }
    /// Allocate source and temporary locals under the shared frame-slot limit.
    fn local_id(&mut self, span: Span) -> Result<LocalId, Diagnostic> {
        if self.next_local as usize >= crate::MAX_LOCALS {
            return Err(Diagnostic::at(
                self.source,
                span.start,
                "function exceeds 255-local limit",
            ));
        }
        if self.parameters.len() + self.next_local as usize >= crate::MAX_LOCALS {
            return Err(Diagnostic::at(
                self.source,
                span.start,
                "parameters plus locals exceed 255-slot limit",
            ));
        }
        let id = LocalId(self.next_local);
        self.next_local += 1;
        Ok(id)
    }

    /// Evaluate an operand now and save its result before later operands branch.
    /// The returned builder-only expression emits a fresh read at the use site.
    fn snapshot(
        &mut self,
        expression: parser::Expression,
        locals: &BTreeMap<String, LocalId>,
    ) -> Result<parser::Expression, Diagnostic> {
        self.snapshot_with_context(expression, locals, None)
    }

    /// Capture an index before later effects, retaining an immediate Number's
    /// positional proof without assigning a constant contract to mutable storage.
    fn snapshot_index(
        &mut self,
        expression: parser::Expression,
        locals: &BTreeMap<String, LocalId>,
        positional: bool,
    ) -> Result<parser::Expression, Diagnostic> {
        if positional && matches!(expression.kind, parser::ExpressionKind::Integer(_)) {
            Ok(expression)
        } else {
            self.snapshot(expression, locals)
        }
    }

    /// Write an indexed value, updating a named tuple's storage view when known.
    /// Computed receivers retain independent parent/alias views. Self fields
    /// use field-flow tracking; source-instance fields retain declared views.
    fn indexed_write(
        &mut self,
        target: Option<LocalId>,
        array: ValueId,
        index: ValueId,
        value: ValueId,
        span: Span,
        field_receiver: bool,
    ) -> Result<(), Diagnostic> {
        if (self
            .controls
            .iter()
            .any(|control| control.continue_to.is_some())
            && (target.is_some_and(|local| self.shape_arrays.contains(&local))
                || crate::field_flow::tracked_receiver(&self.operations, array)))
            || self
                .types
                .value_type(array)
                .is_some_and(|ty| crate::tuples::read_contract(ty, None).is_some())
        {
            if field_receiver
                && !crate::source_fields::read(&self.operations, array)
                && !crate::field_flow::tracked_receiver(&self.operations, array)
            {
                return Err(Diagnostic::at(
                    self.source,
                    span.start,
                    "tuple field mutation requires field contract flow tracking",
                ));
            }
            if let Some(local) = target {
                self.promote_variable(local);
            }
            let result = ValueId(self.operations.len() as u32);
            self.push(
                Operation::TupleWrite {
                    result,
                    array,
                    index,
                    value,
                },
                Some(span),
            );
            if let Some(local) = target {
                self.push(
                    Operation::LocalAssign {
                        local,
                        value: result,
                    },
                    Some(span),
                );
                self.shape_arrays.insert(local);
            } else {
                self.push(Operation::Discard { value: result }, Some(span));
            }
        } else {
            let operation = if matches!(
                self.types.value_type(array),
                Some(crate::api::Type::Dictionary { .. } | crate::api::Type::Options(_))
            ) {
                Operation::DictionaryWrite {
                    dictionary: array,
                    key: index,
                    value,
                }
            } else {
                Operation::ArrayWrite {
                    array,
                    index,
                    value,
                }
            };
            self.push(operation, Some(span));
        }
        Ok(())
    }

    /// Save an operand, passing container context into empty literals/conditional arms.
    fn snapshot_with_context(
        &mut self,
        expression: parser::Expression,
        locals: &BTreeMap<String, LocalId>,
        element: Option<ConstructionType>,
    ) -> Result<parser::Expression, Diagnostic> {
        let span = expression.span;
        let value = self.value_with_context(expression, locals, element)?;
        let local = self.local_id(span)?;
        self.push(Operation::LocalInitialize { local, value }, Some(span));
        Ok(parser::Expression {
            kind: parser::ExpressionKind::Temporary(local),
            span,
        })
    }

    /// Materialize a logical result with a Boolean local and empty-stack edges.
    /// Both outcomes reach a join; this stored result carries no null proof.
    fn logical_value(
        &mut self,
        expression: parser::Expression,
        locals: &BTreeMap<String, LocalId>,
    ) -> Result<parser::Expression, Diagnostic> {
        let span = expression.span;
        if !matches!(
            self.operations.first().map(|op| &op.operation),
            Some(Operation::Block { .. })
        ) {
            self.operations.insert(
                0,
                LocatedOperation {
                    operation: Operation::Block { block: BlockId(0) },
                    span: None,
                },
            );
        }
        let local = self.local_id(span)?;
        let value = ValueId(self.operations.len() as u32);
        self.push(
            Operation::Boolean {
                result: value,
                value: false,
            },
            Some(span),
        );
        self.push(Operation::LocalInitialize { local, value }, Some(span));
        let on_true = self.block_id();
        let on_false = self.block_id();
        let join = self.block_id();
        self.condition(expression, locals, on_true, on_false)?;
        self.push(Operation::Block { block: on_true }, None);
        let value = ValueId(self.operations.len() as u32);
        self.push(
            Operation::Boolean {
                result: value,
                value: true,
            },
            Some(span),
        );
        self.push(Operation::LocalAssign { local, value }, Some(span));
        self.push(Operation::Jump { target: join }, Some(span));
        self.push(Operation::Block { block: on_false }, None);
        self.push(Operation::Jump { target: join }, Some(span));
        self.push(Operation::Block { block: join }, None);
        Ok(parser::Expression {
            kind: parser::ExpressionKind::Temporary(local),
            span,
        })
    }

    /// Build lazy arms and a typed stack join, without a default initializer.
    /// An expected Array contract supplies context to empty literals in each arm.
    fn conditional_value(
        &mut self,
        expression: parser::Expression,
        locals: &BTreeMap<String, LocalId>,
        expected: Option<ConstructionType>,
    ) -> Result<parser::Expression, Diagnostic> {
        let parser::ExpressionKind::Conditional {
            condition,
            when_true,
            when_false,
        } = expression.kind
        else {
            unreachable!()
        };
        let span = expression.span;
        if !matches!(
            self.operations.first().map(|op| &op.operation),
            Some(Operation::Block { .. })
        ) {
            self.operations.insert(
                0,
                LocatedOperation {
                    operation: Operation::Block { block: BlockId(0) },
                    span: None,
                },
            );
        }
        let on_true = self.block_id();
        let on_false = self.block_id();
        let join = self.block_id();
        self.condition(*condition, locals, on_true, on_false)?;
        self.push(Operation::Block { block: on_true }, None);
        let left = self.value_with_context(*when_true, locals, expected.clone())?;
        self.push(
            Operation::JumpValue {
                target: join,
                value: left,
            },
            Some(span),
        );
        self.push(Operation::Block { block: on_false }, None);
        let right = self.value_with_context(*when_false, locals, expected)?;
        self.push(
            Operation::JumpValue {
                target: join,
                value: right,
            },
            Some(span),
        );
        let value_type = self
            .types
            .join_contract(left)
            .zip(self.types.join_contract(right))
            .and_then(|(left, right)| crate::joins::contract_result(&left, &right))
            .ok_or_else(|| {
                Diagnostic::at(
                    self.source,
                    span.start,
                    "conditional arms require compatible core types or the same source class, optionally with Null",
                )
            })?;
        self.push(Operation::Block { block: join }, None);
        let result = ValueId(self.operations.len() as u32);
        self.push(Operation::BlockParameter { result, value_type }, Some(span));
        Ok(parser::Expression {
            kind: parser::ExpressionKind::Joined(result),
            span,
        })
    }

    /// Build a Dictionary from validated scalar tokens or ordered runtime fills.
    /// Unchecked mismatches retain actual constants or use asserted builder fills.
    fn dictionary_literal(
        &mut self,
        entries: Vec<(parser::Expression, parser::Expression)>,
        expected: Option<ConstructionType>,
        span: Span,
        locals: &BTreeMap<String, LocalId>,
    ) -> Result<parser::Expression, Diagnostic> {
        fn scalar(expression: &parser::Expression) -> Option<crate::DictionaryScalar> {
            use crate::DictionaryScalar as S;
            use parser::ExpressionKind as E;
            Some(match &expression.kind {
                E::Integer(n) => S::Number(*n as u32),
                E::Long(n) => S::Long(*n as u64),
                E::Float(n) => S::Float(*n),
                E::Double(n) => S::Double(*n),
                E::String(s) => S::String(s.clone()),
                E::Char(c) => S::Char(*c),
                E::Boolean(b) => S::Boolean(*b),
                E::Null => S::Null,
                _ => return None,
            })
        }
        let expected = expected.and_then(|ty| {
            if let ConstructionType::Dictionary(ty) = ty {
                Some(ty.clone())
            } else {
                None
            }
        });
        if let Some(constants) = entries
            .iter()
            .map(|(key, value)| Some((scalar(key)?, scalar(value)?)))
            .collect::<Option<Vec<_>>>()
        {
            if expected.as_ref().is_some_and(|ty| {
                matches!(
                    ty.key,
                    crate::DictionaryKey::Interface(_) | crate::DictionaryKey::Structural(_)
                )
            }) {
                if !self.calls.unchecked
                    && constants.first().is_some_and(|(first, _)| {
                        constants.iter().any(|(key, _)| {
                            std::mem::discriminant(key) != std::mem::discriminant(first)
                        })
                    })
                {
                    return Err(Diagnostic::at(
                        self.source,
                        span.start,
                        "mixed scalar-token interface keys require runtime insertion",
                    ));
                }
            } else {
                let checked = crate::dictionary::constant_type(&constants, expected.clone());
                let dictionary_type = if self.calls.unchecked {
                    checked
                        .ok()
                        .or_else(|| crate::dictionary::constant_type(&constants, None).ok())
                } else {
                    Some(
                        checked
                            .map_err(|message| Diagnostic::at(self.source, span.start, message))?,
                    )
                };
                if let Some(dictionary_type) = dictionary_type {
                    let result = ValueId(self.operations.len() as u32);
                    self.push(
                        Operation::DictionaryConstant {
                            result,
                            dictionary_type,
                            entries: constants,
                        },
                        Some(span),
                    );
                    return Ok(parser::Expression {
                        kind: parser::ExpressionKind::Joined(result),
                        span,
                    });
                }
                // Incompatible flat tokens still use ordinary runtime insertion.
                // Unchecked fill assertions retain their actual key/value objects.
            }
        }
        let grouped = entries.iter().all(|(key, value)| {
            !self.needs_staging(key, locals) && !self.needs_staging(value, locals)
        });
        let builder = crate::BuilderId(self.local_id(span)?.0);
        let length = entries.len() as u32;
        let provisional = expected.clone().unwrap_or(crate::DictionaryType {
            key: crate::DictionaryKey::String,
            value: crate::DictionaryValue::Number,
        });
        self.push(
            Operation::DictionaryBegin {
                builder,
                dictionary_type: provisional,
                length,
            },
            Some(span),
        );
        let mut values = Vec::new();
        let mut candidates = Vec::new();
        for (index, (key, value)) in entries.into_iter().enumerate() {
            let (key, value) = if grouped {
                (key, value)
            } else {
                (self.snapshot(key, locals)?, self.snapshot(value, locals)?)
            };
            let key = self.value(key, locals)?;
            let value = self.value(value, locals)?;
            candidates.push(
                self.types
                    .value_type(key)
                    .cloned()
                    .zip(self.types.value_type(value).cloned()),
            );
            if grouped {
                values.push((key, value));
            } else {
                self.push(
                    Operation::DictionaryFill {
                        builder,
                        start: index as u32,
                        entries: vec![(key, value)],
                    },
                    Some(span),
                );
            }
        }
        let dictionary_type = if let Some(expected) = expected {
            expected
        } else {
            candidates.into_iter().collect::<Option<Vec<_>>>().and_then(|types| crate::dictionary::inferred_type(&types))
                .ok_or_else(|| Diagnostic::at(self.source, span.start, "computed Dictionary entries require homogeneous scalar keys and scalar/Null values; Null-only values need typed context"))?
        };
        let allocation = self.operations.iter_mut().find(|op| matches!(op.operation, Operation::DictionaryBegin { builder: id, .. } if id == builder)).expect("Dictionary allocation precedes its entries");
        allocation.operation = Operation::DictionaryBegin {
            builder,
            dictionary_type: dictionary_type.clone(),
            length,
        };
        self.types.set_dictionary(builder, dictionary_type);
        if grouped {
            self.push(
                Operation::DictionaryFill {
                    builder,
                    start: 0,
                    entries: values,
                },
                Some(span),
            );
        }
        let result = ValueId(self.operations.len() as u32);
        self.push(Operation::DictionaryFinish { result, builder }, Some(span));
        Ok(parser::Expression {
            kind: parser::ExpressionKind::Joined(result),
            span,
        })
    }

    /// Infer positional slots without context, or build the expected typed Array.
    /// Typed Arrays use constants or allocate before ordered element evaluation.
    /// A private builder prevents uninitialized slots from escaping as values;
    /// branching elements stage values with empty-stack CFG edges. A nullable
    /// expected contract permits Null and the base type without widening an
    /// existing Array reference; its staging locals retain each operand's type.
    /// Unchecked mismatches retain actual constant types or asserted builder fills.
    fn array_literal(
        &mut self,
        elements: Vec<parser::Expression>,
        expected: Option<crate::ArrayElement>,
        span: Span,
        locals: &BTreeMap<String, LocalId>,
    ) -> Result<parser::Expression, Diagnostic> {
        let Some(expected) = expected else {
            let value = self.tuple_value(elements, None, span, locals)?;
            let local = self.local_id(span)?;
            self.push(Operation::LocalInitialize { local, value }, Some(span));
            return Ok(parser::Expression {
                kind: parser::ExpressionKind::Temporary(local),
                span,
            });
        };
        if elements.is_empty() {
            let result = ValueId(self.operations.len() as u32);
            self.push(
                Operation::ArrayNew {
                    result,
                    element_type: expected,
                },
                Some(span),
            );
            return Ok(parser::Expression {
                kind: parser::ExpressionKind::Joined(result),
                span,
            });
        }
        // Select only literal tokens, without folding computed expressions.
        let constant = elements
            .iter()
            .map(|e| match e.kind {
                parser::ExpressionKind::Integer(n) => Some((
                    Some(crate::ArrayElement::Number),
                    crate::ArrayConstantValue::Number(n as u32),
                )),
                parser::ExpressionKind::Long(n) => Some((
                    Some(crate::ArrayElement::Long),
                    crate::ArrayConstantValue::Long(n as u64),
                )),
                parser::ExpressionKind::Float(n) => Some((
                    Some(crate::ArrayElement::Float),
                    crate::ArrayConstantValue::Float(n),
                )),
                parser::ExpressionKind::Double(n) => Some((
                    Some(crate::ArrayElement::Double),
                    crate::ArrayConstantValue::Double(n),
                )),
                parser::ExpressionKind::Char(c) => Some((
                    Some(crate::ArrayElement::Char),
                    crate::ArrayConstantValue::Char(c),
                )),
                parser::ExpressionKind::String(ref s) => Some((
                    Some(crate::ArrayElement::String),
                    crate::ArrayConstantValue::String(s.clone()),
                )),
                parser::ExpressionKind::Boolean(b) => Some((
                    Some(crate::ArrayElement::Boolean),
                    crate::ArrayConstantValue::Boolean(b),
                )),
                parser::ExpressionKind::Null => Some((None, crate::ArrayConstantValue::Null)),
                _ => None,
            })
            .collect::<Option<Vec<_>>>();
        if let Some(values) = constant {
            let actual_types = values
                .iter()
                .map(|v| {
                    v.0.as_ref()
                        .map(|ty| ty.api_type())
                        .unwrap_or(crate::api::Type::Null)
                })
                .collect::<Vec<_>>();
            let element_type = if actual_types
                .iter()
                .all(|actual| crate::native::accepts(&expected.api_type(), actual))
            {
                Some(expected.clone())
            } else if self.calls.unchecked {
                crate::ArrayElement::from_api_type(&crate::unions::normalize(actual_types))
            } else {
                None
            };
            if let Some(element_type) = element_type {
                let result = ValueId(self.operations.len() as u32);
                self.push(
                    Operation::ArrayConstant {
                        result,
                        element_type,
                        values: values.into_iter().map(|v| v.1).collect(),
                    },
                    Some(span),
                );
                let array = self.local_id(span)?;
                self.push(
                    Operation::LocalInitialize {
                        local: array,
                        value: result,
                    },
                    Some(span),
                );
                return Ok(parser::Expression {
                    kind: parser::ExpressionKind::Temporary(array),
                    span,
                });
            }
        }
        let builder = crate::BuilderId(self.local_id(span)?.0);
        let grouped = elements.iter().all(|e| !self.needs_staging(e, locals));
        let mut values = Vec::new();
        let length = elements.len() as u32;
        self.push(
            Operation::ArrayBegin {
                builder,
                element_type: expected.clone(),
                length,
            },
            Some(span),
        );
        for (index, expression) in elements.into_iter().enumerate() {
            let element_span = expression.span;
            let context = construction_type(&expected.api_type());
            let value = self.value_with_context(expression, locals, context)?;
            if grouped {
                values.push(value);
                continue;
            }
            let element = self.local_id(element_span)?;
            self.push(
                Operation::LocalInitialize {
                    local: element,
                    value,
                },
                Some(element_span),
            );
            let value = ValueId(self.operations.len() as u32);
            self.push(
                Operation::LocalRead {
                    result: value,
                    local: element,
                },
                Some(element_span),
            );
            self.push(
                Operation::ArrayFill {
                    builder,
                    index: index as u32,
                    value,
                },
                Some(element_span),
            );
        }
        if grouped {
            self.push(
                Operation::ArrayFillMany {
                    builder,
                    start: 0,
                    values,
                },
                Some(span),
            );
        }
        let result = ValueId(self.operations.len() as u32);
        self.push(Operation::ArrayFinish { result, builder }, Some(span));
        let array = self.local_id(span)?;
        self.push(
            Operation::LocalInitialize {
                local: array,
                value: result,
            },
            Some(span),
        );
        Ok(parser::Expression {
            kind: parser::ExpressionKind::Temporary(array),
            span,
        })
    }

    /// Build a byte literal using exact DATA bytes or a private ordered initializer.
    /// Out-of-range Number/Char tokens keep the indexed-store runtime check.
    fn byte_array_literal(
        &mut self,
        elements: Vec<parser::Expression>,
        span: Span,
        locals: &BTreeMap<String, LocalId>,
    ) -> Result<parser::Expression, Diagnostic> {
        let constant = elements
            .iter()
            .map(|e| match e.kind {
                parser::ExpressionKind::Integer(n) if (-128..=255).contains(&n) => Some(n as u8),
                parser::ExpressionKind::Char(c) if c <= 127 => Some(c as u8),
                _ => None,
            })
            .collect::<Option<Vec<_>>>();
        let result = if let Some(values) = constant {
            let result = ValueId(self.operations.len() as u32);
            self.push(Operation::ByteArrayConstant { result, values }, Some(span));
            result
        } else {
            let builder = crate::BuilderId(self.local_id(span)?.0);
            let grouped = elements.iter().all(|e| !self.needs_staging(e, locals));
            self.push(
                Operation::ByteArrayBegin {
                    builder,
                    length: elements.len() as u32,
                },
                Some(span),
            );
            let mut values = Vec::new();
            for (index, expression) in elements.into_iter().enumerate() {
                let element_span = expression.span;
                let value = if grouped {
                    self.value(expression, locals)?
                } else {
                    self.value_for_setup(expression, locals)?
                };
                if grouped {
                    values.push(value);
                } else {
                    self.push(
                        Operation::ArrayFill {
                            builder,
                            index: index as u32,
                            value,
                        },
                        Some(element_span),
                    );
                }
            }
            if grouped {
                self.push(
                    Operation::ArrayFillMany {
                        builder,
                        start: 0,
                        values,
                    },
                    Some(span),
                );
            }
            let result = ValueId(self.operations.len() as u32);
            self.push(Operation::ArrayFinish { result, builder }, Some(span));
            result
        };
        let array = self.local_id(span)?;
        self.push(
            Operation::LocalInitialize {
                local: array,
                value: result,
            },
            Some(span),
        );
        Ok(parser::Expression {
            kind: parser::ExpressionKind::Temporary(array),
            span,
        })
    }

    /// Prepare children needing flow or construction in source order, snapshotting
    /// operands before emitting their parent in the final block.
    fn prepare(
        &mut self,
        expression: parser::Expression,
        locals: &BTreeMap<String, LocalId>,
    ) -> Result<parser::Expression, Diagnostic> {
        use parser::ExpressionKind as E;
        let span = expression.span;
        if tuple_append_name(&expression).is_some_and(|(name, _)| {
            locals
                .get(name)
                .is_some_and(|local| self.tuple_storage(*local))
        }) {
            return self.tuple_append(expression, locals);
        }
        let kind = resolve_implicit_call(
            self.source,
            expression.kind,
            locals,
            &self.parameters,
            self.calls,
            span,
        )?;
        let kind = match kind {
            E::MemberRead { receiver, member } => E::MemberRead {
                receiver: Box::new(self.prepare(*receiver, locals)?),
                member,
            },
            kind @ E::ByteArrayAllocate(_) => {
                return self.snapshot(parser::Expression { kind, span }, locals);
            }
            E::ArrayAllocate(_) => {
                return Err(Diagnostic::at(
                    self.source,
                    span.start,
                    "sized Array allocation requires a concrete expected nullable Array element type",
                ));
            }
            kind @ E::Conditional { .. } => {
                return self.snapshot(parser::Expression { kind, span }, locals);
            }
            E::DictionaryLiteral(entries) => {
                return self.dictionary_literal(entries, None, span, locals);
            }
            E::ArrayLiteral(elements) => return self.array_literal(elements, None, span, locals),
            E::ByteArrayLiteral(elements) => {
                return self.byte_array_literal(elements, span, locals);
            }
            kind @ E::Logical { .. } => {
                return self.logical_value(parser::Expression { kind, span }, locals);
            }
            E::Not(operand) => E::Not(Box::new(self.prepare(*operand, locals)?)),
            E::Step { operand, increment } => E::Step {
                operand: Box::new(self.prepare(*operand, locals)?),
                increment,
            },
            E::Positive(operand) => E::Positive(Box::new(self.prepare(*operand, locals)?)),
            E::Complement(operand) => E::Complement(Box::new(self.prepare(*operand, locals)?)),
            E::Negate(operand) => E::Negate(Box::new(self.prepare(*operand, locals)?)),
            E::ArrayRead { array, index } => {
                let array = self.snapshot(*array, locals)?;
                let index =
                    self.snapshot_index(*index, locals, self.positional_snapshot(&array))?;
                E::ArrayRead {
                    array: Box::new(array),
                    index: Box::new(index),
                }
            }
            E::Binary { kind, left, right } => E::Binary {
                kind,
                left: Box::new(self.snapshot(*left, locals)?),
                right: Box::new(self.snapshot(*right, locals)?),
            },
            E::Cast { operand, target } => E::Cast {
                operand: Box::new(self.prepare(*operand, locals)?),
                target,
            },
            E::InstanceOf { operand, class } => E::InstanceOf {
                operand: Box::new(self.prepare(*operand, locals)?),
                class,
            },
            E::Has { receiver, symbol } => E::Has {
                receiver: Box::new(self.snapshot(*receiver, locals)?),
                symbol: Box::new(self.snapshot(*symbol, locals)?),
            },
            E::Compare { kind, left, right } => E::Compare {
                kind,
                left: Box::new(self.snapshot(*left, locals)?),
                right: Box::new(self.snapshot(*right, locals)?),
            },
            E::SourceNew { class, arguments } => {
                let expected = self.call_argument_context(&class, "initialize");
                E::SourceNew {
                    class,
                    arguments: self.snapshot_arguments(arguments, locals, &expected)?,
                }
            }
            E::ValueCall {
                receiver,
                member,
                arguments,
            } => {
                let field_receiver = matches!(receiver.kind, E::MemberRead { .. });
                let self_field = if let E::SelfField(field) = &receiver.kind {
                    Some(field.clone())
                } else {
                    None
                };
                let loop_self_field = self_field.as_ref().is_some_and(|field| {
                    self.calls
                        .field_types
                        .get(field)
                        .is_some_and(crate::field_flow::positional)
                }) && self
                    .controls
                    .iter()
                    .any(|control| control.continue_to.is_some());
                let computed_receiver = !matches!(receiver.kind, E::Local(_));
                let receiver = self.snapshot(*receiver, locals)?;
                let loop_field = loop_self_field
                    || (self.controls.iter().any(|c| c.continue_to.is_some())
                        && matches!(receiver.kind, E::Temporary(local) if crate::field_flow::captured_receiver(&self.operations, local)));
                if computed_receiver
                    && (self.positional_snapshot(&receiver) || loop_field)
                    && matches!(member.as_str(), "add" | "addAll")
                {
                    let source_field = matches!(receiver.kind, E::Temporary(local)
                        if self.operations.iter().any(|op| matches!(&op.operation,
                            Operation::LocalInitialize { local: at, value }
                            if *at == local && crate::source_fields::read(&self.operations, *value))));
                    let tracked_field = matches!(receiver.kind, E::Temporary(local)
                        if crate::field_flow::captured_receiver(&self.operations, local));
                    if field_receiver && !source_field && !tracked_field {
                        return Err(Diagnostic::at(
                            self.source,
                            span.start,
                            "tuple field mutation requires field contract flow tracking",
                        ));
                    }
                    return self.computed_tuple_append(
                        receiver,
                        arguments,
                        member == "addAll",
                        !loop_field,
                        span,
                        locals,
                    );
                }
                if member == "slice" && self.positional_snapshot(&receiver) {
                    let arguments = arguments
                        .into_iter()
                        .map(|argument| {
                            if matches!(argument.kind, E::Integer(_) | E::Null) {
                                Ok(argument)
                            } else {
                                self.snapshot(argument, locals)
                            }
                        })
                        .collect::<Result<Vec<_>, Diagnostic>>()?;
                    return Ok(parser::Expression {
                        kind: E::ValueCall {
                            receiver: Box::new(receiver),
                            member,
                            arguments,
                        },
                        span,
                    });
                }
                let expected = self.receiver_argument_context(&receiver, &member);
                E::ValueCall {
                    receiver: Box::new(receiver),
                    member,
                    arguments: self.snapshot_arguments(arguments, locals, &expected)?,
                }
            }
            E::ModuleCall {
                module,
                member,
                arguments,
            } => {
                let receiver =
                    if locals.contains_key(&module) || self.parameters.contains_key(&module) {
                        Some(Box::new(self.snapshot(
                            parser::Expression {
                                kind: E::Local(module.clone()),
                                span,
                            },
                            locals,
                        )?))
                    } else {
                        None
                    };
                let expected = if let Some(receiver) = &receiver {
                    self.receiver_argument_context(receiver, &member)
                } else {
                    self.call_argument_context(&module, &member)
                };
                let arguments = self.snapshot_arguments(arguments, locals, &expected)?;
                if let Some(receiver) = receiver {
                    E::ValueCall {
                        receiver,
                        member,
                        arguments,
                    }
                } else {
                    E::ModuleCall {
                        module,
                        member,
                        arguments,
                    }
                }
            }
            E::GlobalCall { member, arguments } => {
                let expected = self.call_argument_context("", &member);
                E::GlobalCall {
                    member,
                    arguments: self.snapshot_arguments(arguments, locals, &expected)?,
                }
            }
            E::OwnerCall {
                module,
                member,
                arguments,
            } => {
                let expected = self.call_argument_context(&module, &member);
                E::OwnerCall {
                    module,
                    member,
                    arguments: self.snapshot_arguments(arguments, locals, &expected)?,
                }
            }
            kind => kind,
        };
        Ok(parser::Expression { kind, span })
    }

    /// Evaluate statement arguments in source order. If any argument branches,
    /// save all results and read them only after the final argument has completed.
    fn arguments(
        &mut self,
        arguments: Vec<parser::Expression>,
        locals: &BTreeMap<String, LocalId>,
        expected: &[Option<ConstructionType>],
    ) -> Result<Vec<ValueId>, Diagnostic> {
        let arguments = if arguments
            .iter()
            .any(|expression| self.needs_staging(expression, locals))
        {
            self.snapshot_arguments(arguments, locals, expected)?
        } else {
            arguments
        };
        arguments
            .into_iter()
            .map(|arg| self.value(arg, locals))
            .collect()
    }
    /// Snapshot call operands in order, supplying declared container argument types.
    fn snapshot_arguments(
        &mut self,
        arguments: Vec<parser::Expression>,
        locals: &BTreeMap<String, LocalId>,
        expected: &[Option<ConstructionType>],
    ) -> Result<Vec<parser::Expression>, Diagnostic> {
        arguments
            .into_iter()
            .enumerate()
            .map(|(index, argument)| {
                self.snapshot_with_context(argument, locals, expected.get(index).cloned().flatten())
            })
            .collect()
    }

    /// Read literal-construction expectations from source or native call signatures.
    fn call_argument_context(&self, module: &str, member: &str) -> Vec<Option<ConstructionType>> {
        if matches!(
            module,
            "Lang" | "System" | "WatchUi" | "Graphics" | "Application"
        ) {
            return crate::native::module_parameters(&format!("Toybox.{module}"), member)
                .map(|parameters| parameters.iter().map(construction_type).collect())
                .unwrap_or_default();
        }
        self.calls
            .get(&(module.into(), member.into()))
            .map(|signature| {
                signature
                    .parameters
                    .iter()
                    .map(|ty| construction_type(&ty.api_type()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Resolve container argument expectations from a source or native receiver.
    /// This supplies construction context; final validation checks the call.
    fn receiver_argument_context(
        &self,
        receiver: &parser::Expression,
        member: &str,
    ) -> Vec<Option<ConstructionType>> {
        let parser::ExpressionKind::Temporary(local) = receiver.kind else {
            return Vec::new();
        };
        if let Some(contract) = self.types.local_contract(local) {
            let targets = self.calls.member_targets(contract, member);
            let signatures = targets.parameters(member);
            if let Some(first) = signatures.first() {
                return first
                    .iter()
                    .enumerate()
                    .map(|(i, first_type)| {
                        let ty = signatures
                            .iter()
                            .filter_map(|s| s.get(i))
                            .find(|ty| **ty != crate::ParameterType::Any)
                            .unwrap_or(first_type);
                        signatures
                            .iter()
                            .all(|s| {
                                s.get(i).is_some_and(|other| {
                                    other == ty || *other == crate::ParameterType::Any
                                })
                            })
                            .then(|| construction_type(&ty.api_type()))
                            .flatten()
                    })
                    .collect();
            }
        }
        self.types
            .local_type(local)
            .and_then(|ty| crate::native::value_parameters(ty, member))
            .map(|parameters| parameters.iter().map(construction_type).collect())
            .unwrap_or_default()
    }

    /// Emit a Boolean condition as explicit edges. Evaluate logical operands
    /// left to right and enter the right operand only when the left requires it.
    /// Grouping and negation preserve this flow without cross-block value IDs.
    fn condition(
        &mut self,
        expression: parser::Expression,
        locals: &BTreeMap<String, LocalId>,
        on_true: BlockId,
        on_false: BlockId,
    ) -> Result<(), Diagnostic> {
        let span = expression.span;
        match expression.kind {
            parser::ExpressionKind::Logical { and, left, right } => {
                let next = self.block_id();
                if and {
                    self.condition(*left, locals, next, on_false)?;
                } else {
                    self.condition(*left, locals, on_true, next)?;
                }
                self.push(Operation::Block { block: next }, None);
                self.condition(*right, locals, on_true, on_false)?;
            }
            parser::ExpressionKind::Not(operand) => {
                self.condition(*operand, locals, on_false, on_true)?;
            }
            kind => {
                let condition = self.value(parser::Expression { kind, span }, locals)?;
                self.push(
                    Operation::Branch {
                        condition,
                        on_true,
                        on_false,
                    },
                    Some(span),
                );
            }
        }
        Ok(())
    }
    fn push(&mut self, operation: Operation, span: Option<Span>) {
        if let Operation::Block { block } = operation {
            self.types.facts = crate::type_flow::prefix_inputs(&self.operations)
                .remove(&block)
                .unwrap_or_default();
            self.types.refresh_variables(&self.operations, block);
        }
        self.types.observe(&operation);
        self.operations.push(LocatedOperation { operation, span });
    }
    fn block_id(&mut self) -> BlockId {
        let id = BlockId(self.next_block);
        self.next_block += 1;
        id
    }
    /// Emit a call statement, retaining Void as a statement-only native contract.
    fn value_statement(
        &mut self,
        expression: parser::Expression,
        locals: &BTreeMap<String, LocalId>,
        span: Span,
    ) -> Result<(), Diagnostic> {
        let value = self.value(expression, locals)?;
        if self.types.value_type(value) == Some(&crate::api::Type::Void) {
            let last = self
                .operations
                .last_mut()
                .expect("call emitted an operation");
            if let Operation::SelfCall {
                member, arguments, ..
            } = &last.operation
            {
                last.operation = Operation::SelfCallStatement {
                    member: member.clone(),
                    arguments: arguments.clone(),
                };
                return Ok(());
            }
            if let Operation::GlobalCall {
                member, arguments, ..
            } = &last.operation
            {
                last.operation = Operation::GlobalCallStatement {
                    member: member.clone(),
                    arguments: arguments.clone(),
                };
                return Ok(());
            }
            if let Operation::ModuleCall {
                module,
                member,
                arguments,
                ..
            } = &last.operation
            {
                last.operation = Operation::ModuleCallStatement {
                    module: module.clone(),
                    member: member.clone(),
                    arguments: arguments.clone(),
                };
                return Ok(());
            }
            if let Operation::ObjectCall {
                receiver,
                classes,
                member,
                arguments,
                ..
            } = &last.operation
            {
                last.operation = Operation::ObjectCallStatement {
                    receiver: *receiver,
                    classes: classes.clone(),
                    member: member.clone(),
                    arguments: arguments.clone(),
                };
                return Ok(());
            }
            if let Operation::NativeCall {
                receiver,
                member,
                arguments,
                ..
            } = &last.operation
            {
                last.operation = Operation::NativeCallStatement {
                    receiver: *receiver,
                    member: member.clone(),
                    arguments: arguments.clone(),
                };
                return Ok(());
            }
        }
        self.push(Operation::Discard { value }, Some(span));
        Ok(())
    }

    /// Build a return value under a callback's positional or ordinary contract.
    fn returned_value(
        &mut self,
        expression: parser::Expression,
        locals: &BTreeMap<String, LocalId>,
    ) -> Result<ValueId, Diagnostic> {
        if !self.positional_callback {
            return self.value_with_context(
                expression,
                locals,
                construction_type(&self.result_type.api_type()),
            );
        }
        let span = expression.span;
        let parser::ExpressionKind::ArrayLiteral(elements) = expression.kind else {
            return Err(Diagnostic::at(
                self.source,
                span.start,
                "initial View result requires a direct View or View/delegate literal",
            ));
        };
        if !(1..=2).contains(&elements.len()) {
            return Err(Diagnostic::at(
                self.source,
                span.start,
                "initial View result requires one or two slots",
            ));
        }
        self.tuple_value(elements, None, span, locals)
    }

    /// Evaluate tuple slots in order, propagating any declared slot contexts,
    /// then pack their stored reads under the shared positional IR checks.
    fn tuple_value(
        &mut self,
        elements: Vec<parser::Expression>,
        expected: Option<&crate::TupleType>,
        span: Span,
        locals: &BTreeMap<String, LocalId>,
    ) -> Result<ValueId, Diagnostic> {
        if expected.is_some_and(|tuple| tuple.slots().len() != elements.len()) {
            return Err(Diagnostic::at(
                self.source,
                span.start,
                "tuple literal length does not match expected slots",
            ));
        }
        let snapshots = elements
            .into_iter()
            .enumerate()
            .map(|(index, element)| {
                let context =
                    expected.and_then(|tuple| construction_type(&tuple.slots()[index].api_type()));
                self.snapshot_with_context(element, locals, context)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let elements = snapshots
            .into_iter()
            .map(|snapshot| self.value(snapshot, locals))
            .collect::<Result<Vec<_>, _>>()?;
        let result = ValueId(self.operations.len() as u32);
        self.push(Operation::TupleNew { result, elements }, Some(span));
        Ok(result)
    }

    fn value(
        &mut self,
        expression: parser::Expression,
        locals: &BTreeMap<String, LocalId>,
    ) -> Result<ValueId, Diagnostic> {
        self.value_with_context(expression, locals, None)
    }

    /// Build a value, passing container contracts into direct literals and
    /// conditional arms. Existing Array references retain their invariant type;
    /// nullable Array context may type a fresh literal's scalar/Null elements.
    fn value_with_context(
        &mut self,
        expression: parser::Expression,
        locals: &BTreeMap<String, LocalId>,
        expected: Option<ConstructionType>,
    ) -> Result<ValueId, Diagnostic> {
        if matches!(expression.kind, parser::ExpressionKind::ArrayLiteral(_))
            && matches!(expected, Some(ConstructionType::Tuple(_)))
        {
            let parser::ExpressionKind::ArrayLiteral(elements) = expression.kind else {
                unreachable!()
            };
            let Some(ConstructionType::Tuple(tuple)) = expected else {
                unreachable!()
            };
            return self.tuple_value(elements, Some(&tuple), expression.span, locals);
        }

        let inferred_options = expected.is_none()
            && matches!(&expression.kind, parser::ExpressionKind::DictionaryLiteral(entries)
                if !entries.is_empty() && entries.iter().all(|(key,_)| matches!(key.kind,parser::ExpressionKind::Symbol(_))));
        if (matches!(expected, Some(ConstructionType::Options(_))) || inferred_options)
            && matches!(
                &expression.kind,
                parser::ExpressionKind::DictionaryNew
                    | parser::ExpressionKind::DictionaryLiteral(_)
            )
        {
            let entries = match expression.kind {
                parser::ExpressionKind::DictionaryNew => Vec::new(),
                parser::ExpressionKind::DictionaryLiteral(entries) => entries,
                _ => unreachable!(),
            };
            let span = expression.span;
            let builder = crate::BuilderId(self.local_id(span)?.0);
            self.push(
                Operation::RecordBegin {
                    builder,
                    fields: if let Some(ConstructionType::Options(fields)) = &expected {
                        fields.clone()
                    } else {
                        BTreeMap::new()
                    },
                    length: entries.len() as u32,
                },
                Some(span),
            );
            let mut inferred_fields = BTreeMap::new();
            for (index, (key, value)) in entries.into_iter().enumerate() {
                let parser::ExpressionKind::Symbol(name) = key.kind else {
                    return Err(Diagnostic::at(
                        self.source,
                        key.span.start,
                        "option keys require literal Symbols",
                    ));
                };
                let context = if let Some(ConstructionType::Options(fields)) = &expected {
                    fields.get(&name).and_then(construction_type)
                } else {
                    None
                };
                let value = self.snapshot_with_context(value, locals, context)?;
                let key = ValueId(self.operations.len() as u32);
                self.push(
                    Operation::Symbol {
                        result: key,
                        name: name.clone(),
                    },
                    Some(span),
                );
                let value = self.value(value, locals)?;
                let ty = self
                    .types
                    .join_contract(value)
                    .map(|ty| {
                        ty.source_contract()
                            .map(|c| crate::api::Type::SourceValue(Box::new(c)))
                            .unwrap_or_else(|| ty.api_type())
                    })
                    .ok_or_else(|| {
                        Diagnostic::at(self.source, span.start, "cannot infer record value")
                    })?;
                inferred_fields.insert(name.clone(), ty);
                self.push(
                    Operation::DictionaryFill {
                        builder,
                        start: index as u32,
                        entries: vec![(key, value)],
                    },
                    Some(span),
                );
            }
            if expected.is_none() {
                crate::records::validate(&inferred_fields)
                    .map_err(|message| Diagnostic::at(self.source, span.start, message))?;
                for op in &mut self.operations {
                    if let Operation::RecordBegin {
                        builder: owner,
                        fields,
                        ..
                    } = &mut op.operation
                    {
                        if *owner == builder {
                            *fields = inferred_fields.clone();
                        }
                    }
                }
                self.types.set_record(builder, inferred_fields);
            }
            let result = ValueId(self.operations.len() as u32);
            self.push(Operation::DictionaryFinish { result, builder }, Some(span));
            return Ok(result);
        }
        if let parser::ExpressionKind::ByteArrayAllocate(length) = expression.kind {
            let length = self.value(*length, locals)?;
            let result = ValueId(self.operations.len() as u32);
            self.push(
                Operation::ByteArrayAllocate { result, length },
                Some(expression.span),
            );
            return Ok(result);
        }
        if let parser::ExpressionKind::ArrayAllocate(length) = expression.kind {
            let Some(ConstructionType::Array(element_type)) = expected else {
                return Err(Diagnostic::at(
                    self.source,
                    expression.span.start,
                    "sized Array allocation requires a concrete expected nullable Array element type",
                ));
            };
            if element_type != crate::ArrayElement::Null && element_type.nullable_base().is_none() {
                return Err(Diagnostic::at(
                    self.source,
                    expression.span.start,
                    "sized Array allocation requires nullable elements",
                ));
            }
            let length = self.value(*length, locals)?;
            let result = ValueId(self.operations.len() as u32);
            self.push(
                Operation::ArrayAllocate {
                    result,
                    element_type,
                    length,
                },
                Some(expression.span),
            );
            return Ok(result);
        }
        if let parser::ExpressionKind::DictionaryLiteral(entries) = expression.kind {
            let expression = self.dictionary_literal(entries, expected, expression.span, locals)?;
            return emit(
                self.source,
                expression,
                locals,
                &self.parameters,
                &mut self.operations,
                self.calls,
            );
        }
        if matches!(expression.kind, parser::ExpressionKind::DictionaryNew) {
            if let Some(ConstructionType::Dictionary(dictionary_type)) = expected {
                let result = ValueId(self.operations.len() as u32);
                self.push(
                    Operation::DictionaryNew {
                        result,
                        dictionary_type: Some(dictionary_type),
                    },
                    Some(expression.span),
                );
                return Ok(result);
            }
        }
        if matches!(expression.kind, parser::ExpressionKind::Conditional { .. }) {
            let expression = self.conditional_value(expression, locals, expected)?;
            return emit(
                self.source,
                expression,
                locals,
                &self.parameters,
                &mut self.operations,
                self.calls,
            );
        }
        let expression = match (expression, expected) {
            (
                parser::Expression {
                    kind: parser::ExpressionKind::ArrayLiteral(elements),
                    span,
                },
                Some(ConstructionType::Array(element)),
            ) => self.array_literal(elements, Some(element), span, locals)?,
            (expression, _) if self.needs_staging(&expression, locals) => {
                self.prepare(expression, locals)?
            }
            (expression, _) => expression,
        };
        let first = self.operations.len();
        let result = emit(
            self.source,
            expression,
            locals,
            &self.parameters,
            &mut self.operations,
            self.calls,
        )?;
        for operation in &self.operations[first..] {
            self.types.observe(&operation.operation);
        }
        Ok(result)
    }

    /// Stage a direct incoming value before a consumer pushes its stack setup.
    /// Other expressions have already materialized branching children as locals.
    fn value_for_setup(
        &mut self,
        expression: parser::Expression,
        locals: &BTreeMap<String, LocalId>,
    ) -> Result<ValueId, Diagnostic> {
        self.value_for_setup_with_context(expression, locals, None)
    }

    /// Stage a setup value under a field's literal-construction contract.
    /// A join result must leave the operand stack before receiver setup.
    fn value_for_setup_with_context(
        &mut self,
        expression: parser::Expression,
        locals: &BTreeMap<String, LocalId>,
        expected: Option<ConstructionType>,
    ) -> Result<ValueId, Diagnostic> {
        let span = expression.span;
        let value = self.value_with_context(expression, locals, expected)?;
        if !self.operations.last().is_some_and(|op| matches!(op.operation, Operation::BlockParameter { result, .. } if result == value)) {
            return Ok(value);
        }
        let local = self.local_id(span)?;
        self.push(Operation::LocalInitialize { local, value }, Some(span));
        let result = ValueId(self.operations.len() as u32);
        self.push(Operation::LocalRead { result, local }, Some(span));
        Ok(result)
    }

    /// Evaluate an indexed or member compound target once, then update its value.
    /// Keep location operands in typed locals; stage a branching RHS before
    /// defining the final write operands so their IDs remain block-local.
    fn place_compound(
        &mut self,
        target: parser::Expression,
        kind: crate::BinaryOp,
        value: parser::Expression,
        span: Span,
        locals: &BTreeMap<String, LocalId>,
    ) -> Result<(), Diagnostic> {
        self.place_change(target, span, locals, |old| {
            let value_span = Span {
                start: old.span.start,
                end: value.span.end,
            };
            parser::Expression {
                kind: parser::ExpressionKind::Binary {
                    kind,
                    left: Box::new(old),
                    right: Box::new(value),
                },
                span: value_span,
            }
        })
    }

    /// Capture an indexed or member location and emit a checked read/change/write.
    /// The change may stage control flow without reevaluating location operands.
    fn place_change(
        &mut self,
        target: parser::Expression,
        span: Span,
        locals: &BTreeMap<String, LocalId>,
        change: impl FnOnce(parser::Expression) -> parser::Expression,
    ) -> Result<(), Diagnostic> {
        use parser::{Expression, ExpressionKind as E};
        let target_span = target.span;
        if let E::MemberRead { receiver, member } = target.kind {
            let receiver = self.snapshot(*receiver, locals)?;
            let E::Temporary(local) = receiver.kind else {
                unreachable!()
            };
            let old = Expression {
                kind: E::MemberRead {
                    receiver: Box::new(Expression {
                        kind: E::Temporary(local),
                        span: receiver.span,
                    }),
                    member: member.clone(),
                },
                span: target_span,
            };
            let mut value = change(old);
            if self.needs_staging(&value, locals) {
                value = self.snapshot(value, locals)?;
            }
            let receiver = self.value(receiver, locals)?;
            let value = self.value(value, locals)?;
            let field_type = self
                .calls
                .source_field(&self.operations, receiver, &member)
                .map_err(|message| Diagnostic::at(self.source, span.start, message))?;
            self.push(
                Operation::MemberWrite {
                    receiver,
                    member,
                    value,
                    field_type,
                },
                Some(span),
            );
            return Ok(());
        }
        let E::ArrayRead { array, index } = target.kind else {
            unreachable!("parsed indexed compound target")
        };
        let field_receiver = matches!(array.kind, E::MemberRead { .. });
        let target = if let E::Local(name) = &array.kind {
            locals.get(name).copied()
        } else {
            None
        };
        let array = self.snapshot(*array, locals)?;
        let positional = match array.kind {
            E::Temporary(local) => self
                .types
                .local_type(local)
                .is_some_and(|ty| crate::tuples::read_contract(ty, None).is_some()),
            _ => false,
        };
        let index = self.snapshot_index(*index, locals, positional)?;
        let temporary = |expression: &Expression| {
            if let E::Integer(value) = expression.kind {
                return Expression {
                    kind: E::Integer(value),
                    span: expression.span,
                };
            }
            let E::Temporary(local) = expression.kind else {
                unreachable!("captured target operand")
            };
            Expression {
                kind: E::Temporary(local),
                span: expression.span,
            }
        };
        let old = Expression {
            kind: E::ArrayRead {
                array: Box::new(temporary(&array)),
                index: Box::new(temporary(&index)),
            },
            span: target_span,
        };
        let mut value = change(old);
        if self.needs_staging(&value, locals) {
            value = self.snapshot(value, locals)?;
        }
        let array = self.value(array, locals)?;
        let index = self.value(index, locals)?;
        let value = self.value(value, locals)?;
        self.indexed_write(target, array, index, value, span, field_receiver)
    }

    /// Resolve a named assignment target to declared local or parameter storage.
    fn named_place(
        &self,
        name: &str,
        locals: &BTreeMap<String, LocalId>,
        span: Span,
    ) -> Result<crate::type_flow::Place, Diagnostic> {
        locals
            .get(name)
            .copied()
            .map(crate::type_flow::Place::Local)
            .or_else(|| {
                self.parameters
                    .get(name)
                    .copied()
                    .map(crate::type_flow::Place::Parameter)
            })
            .ok_or_else(|| {
                Diagnostic::at(
                    self.source,
                    span.start,
                    format!("assignment requires an in-scope declared local or parameter {name:?}"),
                )
            })
    }

    /// Emit a statement update with one target read and one checked write.
    /// Indexed targets retain captured array/index values across the operation.
    fn update(
        &mut self,
        target: parser::Expression,
        increment: bool,
        span: Span,
        locals: &BTreeMap<String, LocalId>,
    ) -> Result<(), Diagnostic> {
        use parser::{Expression, ExpressionKind as E};
        let step = |operand: Expression| Expression {
            kind: E::Step {
                operand: Box::new(operand),
                increment,
            },
            span,
        };
        if matches!(target.kind, E::ArrayRead { .. } | E::MemberRead { .. }) {
            return self.place_change(target, span, locals, step);
        }
        let operation = match &target.kind {
            E::Local(name) => {
                let place = self.named_place(name, locals, span)?;
                let value = self.value(step(target), locals)?;
                match place {
                    crate::type_flow::Place::Local(local) => {
                        Operation::LocalAssign { local, value }
                    }
                    crate::type_flow::Place::Parameter(index) => {
                        Operation::ParameterAssign { index, value }
                    }
                }
            }
            E::SelfField(name) => {
                let field = name.clone();
                let value = self.value(step(target), locals)?;
                Operation::SelfFieldAssign { field, value }
            }
            _ => unreachable!("parsed update target"),
        };
        self.push(operation, Some(span));
        Ok(())
    }

    /// Lower a loop through explicit test/body/update/exit blocks. Retain dead
    /// header clauses for semantic checking; continue selects the actual latch.
    fn loop_statement(
        &mut self,
        condition: Option<parser::Expression>,
        body: Vec<parser::Statement>,
        update: Vec<parser::Statement>,
        post_test: bool,
        locals: &BTreeMap<String, LocalId>,
        span: Span,
    ) -> Result<bool, Diagnostic> {
        let test = self.block_id();
        let body_block = self.block_id();
        let exit = self.block_id();
        let latch = if !update.is_empty() {
            self.block_id()
        } else {
            test
        };
        self.push(
            Operation::Jump {
                target: if post_test { body_block } else { test },
            },
            Some(span),
        );
        if !post_test {
            self.push(Operation::Block { block: test }, None);
        }
        let mut condition = condition;
        if !post_test {
            if let Some(condition) = condition.take() {
                self.condition(condition, locals, body_block, exit)?;
            } else {
                self.push(Operation::Jump { target: body_block }, Some(span));
            }
        }
        self.push(Operation::Block { block: body_block }, None);
        self.controls.push(ControlTargets {
            break_to: exit,
            continue_to: Some(latch),
        });
        let body_terminates = self.sequence(body, &mut locals.clone())?;
        self.controls.pop();
        if !body_terminates {
            self.push(Operation::Jump { target: latch }, Some(span));
        }
        if !update.is_empty() {
            self.push(Operation::Block { block: latch }, None);
            self.sequence(update, &mut locals.clone())?;
            self.push(Operation::Jump { target: test }, Some(span));
        }
        if post_test {
            self.push(Operation::Block { block: test }, None);
            self.condition(
                condition.expect("post-test loops have a condition"),
                locals,
                body_block,
                exit,
            )?;
        }
        self.push(Operation::Block { block: exit }, None);
        let terminated = !crate::control_flow::prefix(&self.operations).reachable(exit);
        if terminated {
            // A retained dead test can still target this epilogue. It produces
            // no runtime return and must not change the function result type.
            self.push(Operation::ReturnVoid, Some(span));
        }
        Ok(terminated)
    }

    /// Lower ordered case tests and fall-through bodies. Local/parameter selectors
    /// stay stable during dispatch; other expressions are evaluated into one
    /// temporary before testing. Case bodies share one lexical switch scope.
    fn switch_statement(
        &mut self,
        selector: parser::Expression,
        cases: Vec<parser::SwitchCase>,
        locals: &BTreeMap<String, LocalId>,
        span: Span,
    ) -> Result<bool, Diagnostic> {
        let exit = self.block_id();
        let mut fallback = exit;
        let mut tests = Vec::new();
        let mut bodies = Vec::new();
        for case in cases {
            let block = self.block_id();
            match case.pattern {
                parser::CasePattern::Default => fallback = block,
                pattern => tests.push((pattern, block, case.span)),
            }
            bodies.push((block, case.body, case.span));
        }
        if tests.is_empty() {
            let value = self.value(selector, locals)?;
            self.push(Operation::Discard { value }, Some(span));
            self.push(Operation::Jump { target: fallback }, Some(span));
        } else {
            // Case expressions cannot rebind a caller's lexical local/parameter.
            // Reuse that storage so instanceof facts apply to source reads.
            let selector = if matches!(selector.kind, parser::ExpressionKind::Local(_)) {
                selector
            } else {
                self.snapshot(selector, locals)?
            };
            let count = tests.len();
            for (index, (pattern, target, case_span)) in tests.into_iter().enumerate() {
                let operand = parser::Expression {
                    kind: match &selector.kind {
                        parser::ExpressionKind::Local(name) => {
                            parser::ExpressionKind::Local(name.clone())
                        }
                        parser::ExpressionKind::Temporary(local) => {
                            parser::ExpressionKind::Temporary(*local)
                        }
                        _ => unreachable!("switch selector is stable storage"),
                    },
                    span: selector.span,
                };
                let kind = match pattern {
                    parser::CasePattern::Value(value) => parser::ExpressionKind::ValueCall {
                        receiver: Box::new(operand),
                        member: "equals".into(),
                        arguments: vec![value],
                    },
                    parser::CasePattern::Class(class) => parser::ExpressionKind::InstanceOf {
                        operand: Box::new(operand),
                        class,
                    },
                    parser::CasePattern::Default => unreachable!("default has no test"),
                };
                let next = if index + 1 == count {
                    fallback
                } else {
                    self.block_id()
                };
                self.condition(
                    parser::Expression {
                        kind,
                        span: case_span,
                    },
                    locals,
                    target,
                    next,
                )?;
                if next != fallback {
                    self.push(Operation::Block { block: next }, None);
                }
            }
        }
        self.controls.push(ControlTargets {
            break_to: exit,
            continue_to: None,
        });
        let mut scoped = locals.clone();
        let mut bodies = bodies.into_iter().peekable();
        while let Some((block, body, case_span)) = bodies.next() {
            self.push(Operation::Block { block }, None);
            if !self.sequence(body, &mut scoped)? {
                let target = bodies.peek().map_or(exit, |next| next.0);
                self.push(Operation::Jump { target }, Some(case_span));
            }
        }
        self.controls.pop();
        self.push(Operation::Block { block: exit }, None);
        let terminated = !crate::control_flow::prefix(&self.operations).reachable(exit);
        if terminated {
            self.push(Operation::ReturnVoid, Some(span));
        }
        Ok(terminated)
    }

    /// Emit a statement sequence and report whether all paths terminate the
    /// current block with return, break, or continue. Declarations stay scoped
    /// to the supplied name map; controls select a loop latch or structured exit.
    fn sequence(
        &mut self,
        statements: Vec<parser::Statement>,
        locals: &mut BTreeMap<String, LocalId>,
    ) -> Result<bool, Diagnostic> {
        let mut terminated = false;
        for statement in statements {
            let span = statement.span();
            if terminated {
                return Err(Diagnostic::at(
                    self.source,
                    span.start,
                    "unreachable statement after all paths terminate",
                ));
            }
            match statement {
                parser::Statement::Block { body, .. } => {
                    terminated = self.sequence(body, &mut locals.clone())?;
                }
                parser::Statement::Update {
                    target, increment, ..
                } => {
                    self.update(target, increment, span, locals)?;
                }
                parser::Statement::PlaceCompound {
                    target,
                    kind,
                    value,
                    ..
                } => {
                    self.place_compound(target, kind, value, span, locals)?;
                }
                parser::Statement::ArrayAssign {
                    array,
                    index,
                    value,
                    ..
                } => {
                    let field_receiver =
                        matches!(array.kind, parser::ExpressionKind::MemberRead { .. });
                    let target = if let parser::ExpressionKind::Local(name) = &array.kind {
                        locals.get(name).copied()
                    } else {
                        None
                    };
                    // Keep earlier operands stable across control-flow staging
                    // in any later expression; plain operands need no temporaries.
                    let (array, index, value) = if self.needs_staging(&array, locals)
                        || self.needs_staging(&index, locals)
                        || self.needs_staging(&value, locals)
                    {
                        let array = self.snapshot(array, locals)?;
                        let positional = self.positional_snapshot(&array);
                        (
                            array,
                            self.snapshot_index(index, locals, positional)?,
                            self.snapshot(value, locals)?,
                        )
                    } else {
                        (array, index, value)
                    };
                    let array = self.value(array, locals)?;
                    let index = self.value(index, locals)?;
                    let value = self.value(value, locals)?;
                    self.indexed_write(target, array, index, value, span, field_receiver)?;
                }
                parser::Statement::MemberAssign {
                    receiver,
                    member,
                    value,
                    ..
                } => {
                    let (receiver, value, field_type) = if self.needs_staging(&receiver, locals)
                        || self.needs_staging(&value, locals)
                    {
                        // Keep the owner in a frame local across RHS control flow.
                        let receiver = self.snapshot(receiver, locals)?;
                        let parser::ExpressionKind::Temporary(local) = receiver.kind else {
                            unreachable!()
                        };
                        let field_type = self
                            .types
                            .local_contract(local)
                            .map(|contract| self.calls.source_field_contract(contract, &member))
                            .transpose()
                            .map_err(|message| Diagnostic::at(self.source, span.start, message))?
                            .flatten();
                        let ty = if let Some(ty) = &field_type {
                            ty.api_type()
                        } else {
                            self.calls
                                .field_value_type(self.types.local_type(local), &member)
                                .map_err(|message| {
                                    Diagnostic::at(self.source, span.start, message)
                                })?
                        };
                        let value =
                            self.snapshot_with_context(value, locals, construction_type(&ty))?;
                        (
                            self.value(receiver, locals)?,
                            self.value(value, locals)?,
                            field_type,
                        )
                    } else {
                        let receiver = self.value(receiver, locals)?;
                        let field_type = self
                            .calls
                            .source_field(&self.operations, receiver, &member)
                            .map_err(|message| Diagnostic::at(self.source, span.start, message))?;
                        let ty = if let Some(ty) = &field_type {
                            ty.api_type()
                        } else {
                            self.calls
                                .field_value_type(self.types.value_type(receiver), &member)
                                .map_err(|message| {
                                    Diagnostic::at(self.source, span.start, message)
                                })?
                        };
                        (
                            receiver,
                            self.value_with_context(value, locals, construction_type(&ty))?,
                            field_type,
                        )
                    };
                    self.push(
                        Operation::MemberWrite {
                            receiver,
                            member,
                            value,
                            field_type,
                        },
                        Some(span),
                    );
                }
                parser::Statement::FieldAssign { name, value, .. } => {
                    let expected = self
                        .calls
                        .field_types
                        .get(&name)
                        .and_then(|ty| construction_type(&ty.api_type()));
                    let value = self.value_for_setup_with_context(value, locals, expected)?;
                    self.push(
                        Operation::SelfFieldAssign { field: name, value },
                        Some(span),
                    );
                }
                parser::Statement::Local(local) => {
                    if self.parameters.contains_key(&local.name) {
                        return Err(Diagnostic::at(
                            self.source,
                            span.start,
                            "local declaration shadows a parameter",
                        ));
                    }
                    if locals.contains_key(&local.name) {
                        return Err(Diagnostic::at(
                            self.source,
                            span.start,
                            "duplicate local declaration or shadowing an outer local",
                        ));
                    }
                    let id;
                    if let Some(initializer) = local.initializer {
                        let value = self.value(initializer, locals)?;
                        id = self.local_id(span)?;
                        // Positional locals need storage flow from declaration,
                        // so an early guard can constrain a later mutation/loop.
                        let operation = if self.types.value_type(value).is_some_and(|ty| {
                            crate::unions::alternatives(ty)
                                .iter()
                                .any(|part| matches!(part, crate::api::Type::TupleValue(_)))
                        }) {
                            Operation::LocalVariable {
                                local: id,
                                value: Some(value),
                            }
                        } else {
                            Operation::LocalInitialize { local: id, value }
                        };
                        self.push(operation, Some(span));
                    } else {
                        id = self.local_id(span)?;
                        self.pending_locals.insert(id, self.operations.len());
                        self.push(
                            Operation::LocalDeclare {
                                local: id,
                                value_type: crate::JoinType::Api(crate::api::Type::Null),
                            },
                            Some(span),
                        );
                    }
                    self.named_locals.insert(id);
                    locals.insert(local.name, id);
                }
                parser::Statement::Println { argument, .. } => {
                    if locals.contains_key("System") || self.parameters.contains_key("System") {
                        return Err(Diagnostic::at(
                            self.source,
                            span.start,
                            "System receiver is shadowed",
                        ));
                    }
                    let value = self.value_for_setup(argument, locals)?;
                    self.push(Operation::Println { value }, Some(span));
                }
                parser::Statement::Assign { name, value, .. } => {
                    let place = self.named_place(&name, locals, span)?;
                    if let crate::type_flow::Place::Local(local) = place {
                        self.shape_arrays.remove(&local);
                    }
                    let expected = match place {
                        crate::type_flow::Place::Local(local) => {
                            self.types.local_type(local)
                                .filter(|ty| !matches!(ty, crate::api::Type::Array(element) if **element == crate::api::Type::Any))
                                .and_then(construction_type)
                                // A positional local has an inferred flow view,
                                // not a declaration fixing its next literal's slots.
                                .filter(|ty| !matches!(ty, ConstructionType::Tuple(_) | ConstructionType::Options(_)))
                        }
                        crate::type_flow::Place::Parameter(index) => {
                            construction_type(&self.calls.parameter_types[index].api_type())
                        }
                    };
                    let value = self.value_with_context(value, locals, expected)?;
                    if let crate::type_flow::Place::Local(local) = place {
                        if let Some(at) = self.pending_locals.remove(&local) {
                            let value_type = self.types.join_contract(value).ok_or_else(|| {
                                Diagnostic::at(
                                    self.source,
                                    span.start,
                                    "cannot infer local assignment type",
                                )
                            })?;
                            let declaration = Operation::LocalDeclare { local, value_type };
                            self.types.observe(&declaration);
                            self.operations[at].operation = declaration;
                        }
                        let incoming = self.types.join_contract(value);
                        let incoming_class = incoming.as_ref().and_then(|ty| {
                            if let crate::JoinType::Source { class, .. } = ty {
                                Some(class.name.as_str())
                            } else {
                                None
                            }
                        });
                        if incoming.as_ref().is_some_and(|ty| {
                            crate::unions::alternatives(&ty.api_type())
                                .iter()
                                .any(|part| matches!(part, crate::api::Type::TupleValue(_)))
                        }) || incoming
                            .as_ref()
                            .is_some_and(|ty| self.types.local_type(local) != Some(&ty.api_type()))
                            || self.types.local_class(local) != incoming_class
                        {
                            self.promote_variable(local);
                        }
                    }
                    let operation = match place {
                        crate::type_flow::Place::Local(local) => {
                            Operation::LocalAssign { local, value }
                        }
                        crate::type_flow::Place::Parameter(index) => {
                            Operation::ParameterAssign { index, value }
                        }
                    };
                    self.push(operation, Some(span));
                }
                parser::Statement::BaseInitialize { span, base } => {
                    self.push(Operation::NativeBaseInitialize { base }, Some(span));
                }
                parser::Statement::NativeModuleCall {
                    module,
                    member,
                    arguments,
                    ..
                } => {
                    if locals.contains_key("System") || self.parameters.contains_key("System") {
                        return Err(Diagnostic::at(
                            self.source,
                            span.start,
                            "a local/parameter cannot be used as the System module receiver",
                        ));
                    }
                    let arguments = self.arguments(arguments, locals, &[])?;
                    self.push(
                        Operation::NativeModuleCallStatement {
                            module,
                            member,
                            arguments,
                        },
                        Some(span),
                    );
                }
                parser::Statement::ValueCall { expression, .. } => {
                    self.value_statement(expression, locals, span)?;
                }
                parser::Statement::Call {
                    module,
                    member,
                    arguments,
                    ..
                } => {
                    if module == "Lang"
                        || locals.contains_key(&module)
                        || self.parameters.contains_key(&module)
                        || (module == "self"
                            && (member == "method"
                                || self.calls.inherited_object_member("self", &member))
                            && self.calls.get(&(module.clone(), member.clone())).is_none())
                    {
                        self.value_statement(
                            parser::Expression {
                                kind: parser::ExpressionKind::ModuleCall {
                                    module,
                                    member,
                                    arguments,
                                },
                                span,
                            },
                            locals,
                            span,
                        )?;
                    } else {
                        let expected = self.call_argument_context(&module, &member);
                        let arguments = self.arguments(arguments, locals, &expected)?;
                        let operation = if module == "self" {
                            Operation::SelfCallStatement { member, arguments }
                        } else {
                            Operation::ModuleCallStatement {
                                module,
                                member,
                                arguments,
                            }
                        };
                        self.push(operation, Some(span));
                    }
                }
                parser::Statement::Return { value, .. } => {
                    let operation = if let Some(value) = value {
                        Operation::Return {
                            value: self.returned_value(value, locals)?,
                        }
                    } else if self.null_return {
                        let value = ValueId(self.operations.len() as u32);
                        self.push(Operation::Null { result: value }, Some(span));
                        Operation::Return { value }
                    } else {
                        Operation::ReturnVoid
                    };
                    self.push(operation, Some(span));
                    terminated = true;
                }
                parser::Statement::Break { .. } | parser::Statement::Continue { .. } => {
                    let target = if matches!(statement, parser::Statement::Break { .. }) {
                        self.controls.last().map(|targets| targets.break_to)
                    } else {
                        self.controls
                            .iter()
                            .rev()
                            .find_map(|targets| targets.continue_to)
                    }
                    .ok_or_else(|| {
                        Diagnostic::at(
                            self.source,
                            span.start,
                            "break requires a loop or switch; continue requires a loop",
                        )
                    })?;
                    self.push(Operation::Jump { target }, Some(span));
                    terminated = true;
                }
                parser::Statement::Switch {
                    selector, cases, ..
                } => {
                    terminated = self.switch_statement(selector, cases, locals, span)?;
                }
                parser::Statement::While {
                    condition, body, ..
                } => {
                    terminated = self.loop_statement(
                        Some(condition),
                        body,
                        Vec::new(),
                        false,
                        locals,
                        span,
                    )?;
                }
                parser::Statement::For {
                    initialize,
                    condition,
                    update,
                    body,
                    ..
                } => {
                    let mut scoped = locals.clone();
                    self.sequence(initialize, &mut scoped)?;
                    terminated =
                        self.loop_statement(condition, body, update, false, &scoped, span)?;
                }
                parser::Statement::DoWhile {
                    body, condition, ..
                } => {
                    terminated =
                        self.loop_statement(Some(condition), body, Vec::new(), true, locals, span)?;
                }
                parser::Statement::If {
                    condition,
                    on_true,
                    on_false,
                    ..
                } => {
                    let true_block = self.block_id();
                    let false_block = self.block_id();
                    let join = self.block_id();
                    self.condition(condition, locals, true_block, false_block)?;
                    self.push(Operation::Block { block: true_block }, None);
                    let true_terminates = self.sequence(on_true, &mut locals.clone())?;
                    if !true_terminates {
                        self.push(Operation::Jump { target: join }, Some(span));
                    }
                    self.push(Operation::Block { block: false_block }, None);
                    let false_terminates = self.sequence(on_false, &mut locals.clone())?;
                    if !false_terminates {
                        self.push(Operation::Jump { target: join }, Some(span));
                    }
                    terminated = true_terminates && false_terminates;
                    if !terminated {
                        self.push(Operation::Block { block: join }, None);
                    }
                }
            }
            if self.operations.len() > crate::MAX_OPERATIONS {
                return Err(Diagnostic::at(
                    self.source,
                    span.start,
                    "function exceeds 1024-operation limit",
                ));
            }
        }
        Ok(terminated)
    }
}

/// Resolve source names and emit function IR under declared type contracts.
/// Value functions require a result on every path. Void fallthrough gains a
/// return terminator; mixed returns and unreachable statements are rejected.
pub(crate) fn build(
    source: &str,
    ast: parser::ParsedFunction,
    context: Option<crate::api::ApiContext>,
    unchecked: bool,
) -> Result<Function, Diagnostic> {
    if ast.parameter_types.iter().flatten().any(|ty| {
        matches!(
            ty,
            crate::ParameterType::SourceObject(_)
                | crate::ParameterType::NullableSourceObject(_)
                | crate::ParameterType::SourceUnion(_)
        )
    }) || matches!(
        &ast.result_type,
        Some(
            crate::ReturnType::SourceObject(_)
                | crate::ReturnType::NullableSourceObject(_)
                | crate::ReturnType::SourceUnion(_)
        )
    ) {
        return Err(Diagnostic::at(
            source,
            ast.body_span.start,
            "source-class annotations require compilation-unit declarations",
        ));
    }
    let mut calls = CallContext::new();
    calls.unchecked = unchecked;
    build_body(source, ast, None, context, &calls)
}

#[derive(Clone)]
pub(crate) struct CallSignature {
    pub result: crate::ReturnType,
    pub parameters: Vec<crate::ParameterType>,
}

/// Collected callee contracts and the current body's parameter contracts.
#[derive(Clone, Default)]
pub(crate) struct CallContext {
    pub(crate) unchecked: bool,
    signatures: BTreeMap<(String, String), CallSignature>,
    pub(crate) class_bases: BTreeMap<String, crate::ClassBase>,
    parameter_types: Vec<crate::ParameterType>,
    pub(crate) field_types: BTreeMap<String, crate::ReturnType>,
    pub(crate) source_fields: BTreeMap<String, BTreeMap<String, crate::ReturnType>>,
    pub(crate) current_module: Option<String>,
}
impl CallContext {
    pub(crate) fn new() -> Self {
        Self::default()
    }
    /// Resolve source field metadata from exact receiver identities. Structural
    /// receivers continue to resolve their fields from their expanded shape.
    fn known_selector(&self, member: &str) -> bool {
        self.signatures.keys().any(|(_, name)| name == member)
            || self
                .source_fields
                .values()
                .any(|fields| fields.contains_key(member))
            || crate::api::core().has_selector(member)
    }

    fn source_field_contract(
        &self,
        receiver: &crate::SourceUnion,
        member: &str,
    ) -> Result<Option<crate::ReturnType>, String> {
        let result = self.declared_source_field(receiver, member);
        if self.unchecked
            && (result.is_err() || receiver.nullable() || !receiver.native().is_empty())
            && self.known_selector(member)
        {
            Ok(None)
        } else {
            result.map(Some)
        }
    }

    fn field_value_type(
        &self,
        receiver: Option<&crate::api::Type>,
        member: &str,
    ) -> Result<crate::api::Type, String> {
        let result = receiver
            .ok_or_else(|| "unknown field receiver type".to_owned())
            .and_then(|ty| crate::structural::field_type(ty, member));
        if self.unchecked && result.is_err() && self.known_selector(member) {
            Ok(crate::api::Type::Any)
        } else {
            result
        }
    }

    fn source_field(
        &self,
        ops: &[LocatedOperation],
        receiver: ValueId,
        member: &str,
    ) -> Result<Option<crate::ReturnType>, String> {
        if let Some(contract) =
            crate::source_objects::contract(ops, receiver, self.parameter_types.clone())
        {
            return self.source_field_contract(&contract, member);
        }
        if self.unchecked {
            let ty = crate::source_types::value_type(ops, receiver, self.parameter_types.clone());
            self.field_value_type(ty.as_ref(), member)?;
        }
        Ok(None)
    }

    /// Select a declaration for candidate emission. Prefix CFGs may not yet
    /// carry complete null proofs; final validation requires a non-null receiver.
    fn declared_source_field(
        &self,
        receiver: &crate::SourceUnion,
        member: &str,
    ) -> Result<crate::ReturnType, String> {
        let mut receiver = receiver.clone();
        receiver.nullable = false;
        crate::source_fields::contract(&receiver, member, |class, field| {
            self.source_fields.get(&class.name)?.get(field).cloned()
        })
    }

    /// Resolve an expression's source type target before emitting semantic IR.
    fn source_class(&self, name: &str) -> Result<crate::SourceClass, String> {
        let base = self
            .class_bases
            .get(name)
            .copied()
            .filter(|base| base.constructed_type().is_some())
            .ok_or_else(|| format!("unresolved or unsupported source type {name}"))?;
        Ok(crate::SourceClass {
            name: name.into(),
            base,
        })
    }
    pub(crate) fn insert(&mut self, key: (String, String), signature: CallSignature) {
        self.signatures.insert(key, signature);
    }
    fn member_targets(
        &self,
        receiver: &crate::SourceUnion,
        member: &str,
    ) -> crate::source_calls::Targets {
        crate::source_calls::targets(receiver, member, |class| {
            self.get(&(class.name.clone(), member.into()))
                .map(|signature| crate::StructuralMethod {
                    parameters: signature.parameters.clone(),
                    result: signature.result.clone(),
                })
        })
    }
    /// Resolve inherited Object members only when the source class has no override.
    fn inherited_object_member(&self, class: &str, member: &str) -> bool {
        self.class_bases.get(class) == Some(&crate::ClassBase::LangObject)
            && crate::api::core()
                .method("Toybox.Lang.Object", member)
                .is_some()
            && self.get(&(class.into(), member.into())).is_none()
    }
    fn get(&self, key: &(String, String)) -> Option<&CallSignature> {
        self.signatures.get(key)
    }
}

pub(crate) fn build_with_calls(
    source: &str,
    ast: parser::ParsedFunction,
    context: Option<crate::api::ApiContext>,
    method: Option<crate::ClassBase>,
    calls: &CallContext,
) -> Result<Function, Diagnostic> {
    build_body(source, ast, method, context, calls)
}

fn build_body(
    source: &str,
    ast: parser::ParsedFunction,
    method: Option<crate::ClassBase>,
    context: Option<crate::api::ApiContext>,
    calls: &CallContext,
) -> Result<Function, Diagnostic> {
    let callback = method.and_then(|base| crate::native::callback(base, &ast.name));
    let effective_result = callback
        .as_ref()
        .map(|signature| signature.result.clone())
        .unwrap_or_else(|| declared_result(&ast));
    if callback.as_ref().is_some_and(|signature| {
        ast.result_type
            .as_ref()
            .is_some_and(|ty| *ty != signature.result)
    }) {
        return Err(Diagnostic::at(
            source,
            ast.body_span.start,
            "result annotation conflicts with native callback contract",
        ));
    }
    let value_result = effective_result != crate::ReturnType::Void;
    let has_result_return = has_value_return(&ast.statements);
    let result_type = ast.result_type;
    let annotations = ast.parameter_types;
    let candidate_parameters = callback
        .as_ref()
        .map(|signature| signature.parameters.clone())
        .unwrap_or_else(|| {
            annotations
                .iter()
                .map(|ty| ty.clone().unwrap_or(crate::ParameterType::Any))
                .collect()
        });
    let mut body_calls = calls.clone();
    body_calls.parameter_types = candidate_parameters.clone();
    if let Some(base) = method {
        body_calls.class_bases.insert("self".into(), base);
    }
    let mut builder = Builder {
        positional_callback: callback
            .as_ref()
            .is_some_and(|signature| matches!(signature.result, crate::ReturnType::Tuple(_))),
        source,
        calls: &body_calls,
        result_type: effective_result.clone(),
        null_return: effective_result == crate::ReturnType::Any && !has_result_return,
        parameters: ast
            .parameters
            .into_iter()
            .enumerate()
            .map(|(i, n)| (n, i))
            .collect(),
        operations: Vec::new(),
        types: crate::source_types::Types::new(candidate_parameters.clone()),
        next_local: 0,
        named_locals: std::collections::BTreeSet::new(),
        shape_arrays: Default::default(),
        pending_locals: BTreeMap::new(),
        next_block: 1,
        controls: Vec::new(),
    };
    // Logical value expressions add an entry block when they first need flow.
    if has_control_flow(&ast.statements) {
        builder.push(Operation::Block { block: BlockId(0) }, None);
    }
    // Positional parameter annotations describe the entry value. A mutable
    // local view tracks subsequent slot replacement without changing the ABI.
    let mut locals = BTreeMap::new();
    let tuple_writes = tuple_write_receivers(&ast.statements);
    for (name, index) in builder.parameters.clone() {
        if tuple_writes.contains(name.as_str())
            && candidate_parameters.get(index).is_some_and(|ty| {
                crate::unions::alternatives(&ty.api_type())
                    .iter()
                    .any(|part| matches!(part, crate::api::Type::TupleValue(_)))
            })
        {
            let local = builder.local_id(ast.body_span)?;
            let value = ValueId(builder.operations.len() as u32);
            builder.push(
                Operation::ParameterRead {
                    result: value,
                    index,
                },
                None,
            );
            builder.push(
                Operation::LocalVariable {
                    local,
                    value: Some(value),
                },
                None,
            );
            builder.named_locals.insert(local);
            locals.insert(name, local);
        }
    }
    if !builder.sequence(ast.statements, &mut locals)? {
        if value_result && (effective_result != crate::ReturnType::Any || has_result_return) {
            return Err(Diagnostic::at(
                source,
                ast.body_span.end - 1,
                "function requires a value return on every path",
            ));
        }
        let operation = if effective_result == crate::ReturnType::Any {
            let value = ValueId(builder.operations.len() as u32);
            builder.push(Operation::Null { result: value }, None);
            Operation::Return { value }
        } else {
            Operation::ReturnVoid
        };
        builder.push(
            operation,
            Some(Span {
                start: ast.body_span.end - 1,
                end: ast.body_span.end,
            }),
        );
    }
    crate::field_flow::normalize(&mut builder.operations, body_calls.parameter_types.clone())
        .map_err(|error| {
            Diagnostic::at(
                source,
                error
                    .operation
                    .and_then(|at| builder.operations.get(at))
                    .and_then(|op| op.span)
                    .map_or(ast.body_span.start, |s| s.start),
                error.message,
            )
        })?;
    // Hidden expression owners transfer on their final read. Named locals keep
    // source lifetimes, even if their last syntactic use occurs earlier.
    let mut final_reads = BTreeMap::new();
    for (at, op) in builder.operations.iter().enumerate() {
        if let Operation::LocalRead { local, result } = op.operation {
            fn immutable(ty: &crate::api::Type) -> bool {
                match ty {
                    crate::api::Type::Null => true,
                    crate::api::Type::Named(name) => matches!(
                        name.as_str(),
                        "Toybox.Lang.Number"
                            | "Toybox.Lang.Long"
                            | "Toybox.Lang.Float"
                            | "Toybox.Lang.Double"
                            | "Toybox.Lang.String"
                            | "Toybox.Lang.Boolean"
                            | "Toybox.Lang.Symbol"
                            | "Toybox.Lang.Char"
                    ),
                    crate::api::Type::Union(types) => types.iter().all(immutable),
                    _ => false,
                }
            }
            if !builder.named_locals.contains(&local)
                && !builder.types.value_type(result).is_some_and(immutable)
            {
                final_reads.insert(local, at);
            }
        }
    }
    for at in final_reads.into_values() {
        if let Operation::LocalRead { result, local } = builder.operations[at].operation {
            builder.operations[at].operation = Operation::LocalTake { result, local };
        }
    }
    if calls.unchecked {
        crate::unchecked::lower(
            &mut builder.operations,
            candidate_parameters.clone(),
            callback.is_some(),
        );
    }
    let spans: Vec<_> = builder.operations.iter().map(|op| op.span).collect();
    let parameter_types = if let Some(signature) = &callback {
        if builder.parameters.len() != signature.parameters.len() {
            return Err(Diagnostic::at(
                source,
                ast.body_span.start,
                format!(
                    "{} parameter count does not match its native callback contract",
                    ast.name
                ),
            ));
        }
        if annotations
            .iter()
            .zip(&signature.parameters)
            .any(|(annotation, expected)| annotation.as_ref().is_some_and(|ty| ty != expected))
        {
            return Err(Diagnostic::at(
                source,
                ast.body_span.start,
                "parameter annotation conflicts with native callback contract",
            ));
        }
        signature.parameters.clone()
    } else {
        annotations
            .into_iter()
            .map(|ty| ty.clone().unwrap_or(crate::ParameterType::Any))
            .collect()
    };
    let result = Function::construct_with_join_inference(
        ast.name,
        parameter_types,
        builder.operations,
        method.is_some(),
        context,
        callback
            .as_ref()
            .filter(|signature| {
                !matches!(signature.result, crate::ReturnType::Tuple(_)) || !has_result_return
            })
            .map(|signature| signature.result.clone())
            .or(result_type)
            .or_else(|| callback.is_none().then_some(effective_result.clone())),
        true,
    );
    let function = result.map_err(|error| {
        let offset = error
            .operation
            .and_then(|index| spans.get(index).copied().flatten())
            .map_or(0, |span| span.start);
        Diagnostic::at(source, offset, error.message)
    })?;
    let expected = effective_result;
    if !callback.as_ref().map_or_else(
        || function.return_type() == expected,
        |signature| signature.accepts_result(&function.return_type()),
    ) {
        return Err(Diagnostic::at(
            source,
            function
                .operations()
                .iter()
                .find_map(|op| {
                    matches!(
                        op.operation,
                        Operation::Return { .. } | Operation::ReturnVoid
                    )
                    .then_some(op.span)
                    .flatten()
                    .map(|span| span.start)
                })
                .unwrap_or(ast.body_span.start),
            "return type does not match function contract",
        ));
    }
    Ok(function)
}

pub(crate) fn declared_result(ast: &parser::ParsedFunction) -> crate::ReturnType {
    ast.result_type.clone().unwrap_or(crate::ReturnType::Any)
}

/// Read user-declared call contracts before constructing any function body.
pub(crate) fn declared_signature(ast: &parser::ParsedFunction) -> CallSignature {
    CallSignature {
        result: declared_result(ast),
        parameters: ast
            .parameter_types
            .iter()
            .map(|ty| ty.clone().unwrap_or(crate::ParameterType::Any))
            .collect(),
    }
}

/// Detect graph-producing statements through lexical scopes, which emit no blocks.
fn has_control_flow(statements: &[parser::Statement]) -> bool {
    statements.iter().any(|s| match s {
        parser::Statement::If { .. }
        | parser::Statement::Switch { .. }
        | parser::Statement::While { .. }
        | parser::Statement::For { .. }
        | parser::Statement::DoWhile { .. } => true,
        parser::Statement::Block { body, .. } => has_control_flow(body),
        _ => false,
    })
}

/// Find direct named slot-write receivers before building branch-local views.
/// Only mutated parameters need an extra frame slot; read-only tuple parameters
/// retain the ordinary parameter path and its existing frame capacity.
fn tuple_write_receivers(statements: &[parser::Statement]) -> std::collections::BTreeSet<&str> {
    use parser::{ExpressionKind as E, Statement as S};
    let mut names = std::collections::BTreeSet::new();
    for statement in statements {
        if let S::Call { module, member, .. } = statement {
            if matches!(member.as_str(), "add" | "addAll" | "remove" | "removeAll") {
                names.insert(module.as_str());
            }
        }
        for expression in statement.expressions() {
            expression.visit(&mut |expression| {
                if let Some((name, _)) = tuple_mutation_name(expression) {
                    names.insert(name);
                }
            });
        }
        let receiver = match statement {
            S::ArrayAssign { array, .. } => Some(array),
            S::Update { target, .. } | S::PlaceCompound { target, .. } => {
                if let E::ArrayRead { array, .. } = &target.kind {
                    Some(array.as_ref())
                } else {
                    None
                }
            }
            _ => None,
        };
        if let Some(parser::Expression {
            kind: E::Local(name),
            ..
        }) = receiver
        {
            names.insert(name.as_str());
        }
        match statement {
            S::If {
                on_true, on_false, ..
            } => {
                names.extend(tuple_write_receivers(on_true));
                names.extend(tuple_write_receivers(on_false));
            }
            S::Switch { cases, .. } => {
                for case in cases {
                    names.extend(tuple_write_receivers(&case.body));
                }
            }
            S::For {
                initialize,
                body,
                update,
                ..
            } => {
                for statements in [initialize, body, update] {
                    names.extend(tuple_write_receivers(statements));
                }
            }
            S::Block { body, .. } | S::While { body, .. } | S::DoWhile { body, .. } => {
                names.extend(tuple_write_receivers(body));
            }
            _ => {}
        }
    }
    names
}

fn has_value_return(statements: &[parser::Statement]) -> bool {
    statements.iter().any(|s| match s {
        parser::Statement::Return { value: Some(_), .. } => true,
        parser::Statement::If {
            on_true, on_false, ..
        } => has_value_return(on_true) || has_value_return(on_false),
        parser::Statement::Switch { cases, .. } => {
            cases.iter().any(|case| has_value_return(&case.body))
        }
        parser::Statement::Block { body, .. }
        | parser::Statement::While { body, .. }
        | parser::Statement::For { body, .. }
        | parser::Statement::DoWhile { body, .. } => has_value_return(body),
        _ => false,
    })
}
