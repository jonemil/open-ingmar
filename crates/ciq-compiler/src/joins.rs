//! Typed single-value control-flow edges for conditional expressions.
use crate::{BlockId, IrError, LocatedOperation, Operation, ValueId, ValueType, api::Type};
use std::collections::BTreeMap;

/// Parse an exact text tag for a supported incoming stack-value contract.
pub(crate) fn parse_type(tag: &str) -> Option<Type> {
    Some(match tag {
        "integer" => Type::named("Toybox.Lang.Number"),
        "char" => Type::named("Toybox.Lang.Char"),
        "boolean" => Type::named("Toybox.Lang.Boolean"),
        "string" => Type::named("Toybox.Lang.String"),
        "null" => Type::Null,
        "nullable-integer" => Type::Union(vec![Type::named("Toybox.Lang.Number"), Type::Null]),
        "nullable-string" => Type::Union(vec![Type::named("Toybox.Lang.String"), Type::Null]),
        "array-number" => Type::Array(Box::new(Type::named("Toybox.Lang.Number"))),
        "array-char" => Type::Array(Box::new(Type::named("Toybox.Lang.Char"))),
        _ => return None,
    })
}

/// Return the canonical text tag, rejecting types outside the join profile.
pub(crate) fn type_tag(ty: &Type) -> Option<&'static str> {
    [
        "integer",
        "char",
        "boolean",
        "string",
        "null",
        "nullable-integer",
        "nullable-string",
        "array-number",
        "array-char",
    ]
    .into_iter()
    .find(|tag| parse_type(tag).as_ref() == Some(ty))
}

/// Join supported alternatives without converting either runtime value.
pub(crate) fn result(left: &Type, right: &Type) -> Option<Type> {
    if left == right && type_tag(left).is_some() {
        return Some(left.clone());
    }
    for tag in ["nullable-integer", "nullable-string"] {
        let ty = parse_type(tag).unwrap();
        if crate::native::accepts(&ty, left) && crate::native::accepts(&ty, right) {
            return Some(ty);
        }
    }
    None
}

/// Collect source join operands without changing their predecessor identities.
pub(crate) fn incoming_values(operations: &[LocatedOperation]) -> BTreeMap<BlockId, Vec<ValueId>> {
    let mut incoming: BTreeMap<_, Vec<_>> = BTreeMap::new();
    for op in operations {
        if let Operation::JumpValue { target, value } = op.operation {
            incoming.entry(target).or_default().push(value);
        }
    }
    incoming
}

/// Infer a source join from previously checked edge operands. Their types
/// already reflect full-CFG null proofs and writes; no candidate type is trusted.
pub(crate) fn infer_type(
    incoming: &[ValueId],
    types: &BTreeMap<ValueId, ValueType>,
) -> Result<Type, &'static str> {
    let mut joined = None;
    for value in incoming {
        let ty = types
            .get(value)
            .ok_or("source join requires earlier incoming values")?
            .api_type();
        joined = Some(match joined {
            None if type_tag(&ty).is_some() => ty,
            Some(previous) => result(&previous, &ty).ok_or("conditional arms require matching core types or Number/String nullable alternatives")?,
            _ => return Err("unsupported conditional arm type"),
        });
    }
    joined.ok_or("source join requires incoming values")
}

/// Check parameter placement and every incoming edge's stack arity.
/// Ordinary branches/jumps carry no values; a value jump supplies exactly one.
pub(crate) fn validate(
    operations: &[LocatedOperation],
) -> Result<BTreeMap<BlockId, Type>, IrError> {
    let fail = |index, message: &str| IrError {
        operation: Some(index),
        message: message.into(),
    };
    let mut parameters = BTreeMap::new();
    for (index, op) in operations.iter().enumerate() {
        if let Operation::BlockParameter { ref value_type, .. } = op.operation {
            let Some(Operation::Block { block }) =
                index.checked_sub(1).map(|i| &operations[i].operation)
            else {
                return Err(fail(
                    index,
                    "block parameter must immediately follow its block marker",
                ));
            };
            if index == 1 || type_tag(value_type).is_none() {
                return Err(fail(index, "entry or unsupported block parameter type"));
            }
            parameters.insert(*block, value_type.clone());
        }
    }
    for (index, op) in operations.iter().enumerate() {
        match op.operation {
            Operation::JumpValue { target, .. } if !parameters.contains_key(&target) => {
                return Err(fail(index, "value edge requires a target block parameter"));
            }
            Operation::Jump { target } if parameters.contains_key(&target) => {
                return Err(fail(index, "target block requires an incoming value"));
            }
            Operation::Branch {
                on_true, on_false, ..
            } if parameters.contains_key(&on_true) || parameters.contains_key(&on_false) => {
                return Err(fail(index, "branch targets cannot require incoming values"));
            }
            _ => (),
        }
    }
    Ok(parameters)
}

/// Reject uses that would insert call/field/builder setup above an incoming
/// stack value before consuming it. Such uses must first stage the value in a
/// local. Ordinary expression scheduling separately enforces operand order.
pub(crate) fn validate_stack_uses(
    operations: &[LocatedOperation],
    consumers: &BTreeMap<crate::ValueId, usize>,
) -> Result<(), IrError> {
    for op in operations {
        let Operation::BlockParameter { result, .. } = op.operation else {
            continue;
        };
        let mut value = result;
        loop {
            let at = consumers[&value];
            match operations[at].operation {
                Operation::Binary { result, left, .. }
                | Operation::Compare { result, left, .. } if left == value => {
                    value = result;
                }
                Operation::Not { result, .. }
                | Operation::Positive { result, .. }
                | Operation::Negate { result, .. }
                | Operation::Complement { result, .. }
                | Operation::Step { result, .. } => value = result,
                Operation::Return { .. }
                | Operation::Discard { .. }
                | Operation::LocalInitialize { .. }
                | Operation::LocalAssign { .. }
                | Operation::JumpValue { .. }
                | Operation::Branch { .. } => break,
                _ => return Err(IrError {
                    operation: Some(at),
                    message: "incoming stack value must be consumed before call, field, or array setup; stage it in a local".into(),
                }),
            }
        }
    }
    Ok(())
}
