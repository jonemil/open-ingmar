//! Resolve source-instance fields without projecting away their class identity.
use crate::{ReturnType, SourceClass, SourceUnion};

/// Check whether a captured value is an annotated source-instance field read.
pub(crate) fn read(ops: &[crate::LocatedOperation], value: crate::ValueId) -> bool {
    let mut value = value;
    for _ in 0..=ops.len() {
        let Some(op) = ops.iter().find_map(|op| match &op.operation {
            crate::Operation::MemberRead { result, .. }
            | crate::Operation::LocalRead { result, .. }
            | crate::Operation::LocalTake { result, .. }
                if *result == value =>
            {
                Some(&op.operation)
            }
            _ => None,
        }) else {
            return false;
        };
        match op {
            crate::Operation::MemberRead { field_type, .. } => return field_type.is_some(),
            crate::Operation::LocalRead { local, .. }
            | crate::Operation::LocalTake { local, .. } => {
                let Some(original) = ops.iter().find_map(|op| match op.operation {
                    crate::Operation::LocalInitialize { local: at, value } if at == *local => {
                        Some(value)
                    }
                    _ => None,
                }) else {
                    return false;
                };
                value = original;
            }
            _ => return false,
        }
    }
    false
}

/// Require live exact source identities before accepting saved field metadata.
pub(crate) fn receiver(receiver: Option<&SourceUnion>) -> Result<&SourceUnion, String> {
    receiver
        .filter(|c| !c.nullable && c.native.is_empty())
        .ok_or_else(|| "source field access requires non-null exact source receivers".into())
}

/// Resolve one invariant field contract on every possible source receiver.
/// Nullable or mixed native receivers require narrowing before field access.
/// Different declarations must agree, so one saved contract checks both reads
/// and writes without widening a mutable field.
pub(crate) fn contract(
    receiver: &SourceUnion,
    member: &str,
    mut lookup: impl FnMut(&SourceClass, &str) -> Option<ReturnType>,
) -> Result<ReturnType, String> {
    let receiver = self::receiver(Some(receiver))?;
    let mut result = None;
    for class in &receiver.classes {
        let ty = lookup(class, member)
            .ok_or_else(|| format!("{} has no declared field {member}", class.name))?;
        if result.as_ref().is_some_and(|old| *old != ty) {
            return Err("source receiver alternatives have different field contracts".into());
        }
        result = Some(ty);
    }
    result.ok_or_else(|| "source field access requires an exact source receiver".into())
}
