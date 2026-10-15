//! Definite completion of private array builders across control-flow joins.
use crate::{
    ArrayElement, BlockId, BuilderId, IrError, LocatedOperation, Operation, control_flow::Structure,
};
use std::collections::{BTreeMap, VecDeque};

/// Check builder lifetimes and ordered fills on every reachable CFG path.
/// Return immutable construction contracts for element checking and frame layout.
pub(crate) fn validate(
    structure: &Structure,
    operations: &[LocatedOperation],
) -> Result<BTreeMap<BuilderId, (ArrayElement, u32)>, IrError> {
    let fail = |index, message: &str| IrError {
        operation: Some(index),
        message: message.into(),
    };
    let mut declarations = BTreeMap::new();
    for (index, op) in operations.iter().enumerate() {
        if let Operation::ArrayBegin {
            builder,
            element_type,
            length,
        } = op.operation
        {
            if length == 0 || length > i32::MAX as u32 {
                return Err(fail(index, "builder length requires a positive Number"));
            }
            if matches!(element_type, ArrayElement::Object(class) if class != crate::NativeClass::View)
            {
                return Err(fail(index, "object builders support only View elements"));
            }
            if declarations
                .insert(builder, (element_type, length))
                .is_some()
            {
                return Err(fail(index, "duplicate array builder declaration"));
            }
        }
    }
    if declarations.is_empty()
        && !operations.iter().any(|o| {
            matches!(
                o.operation,
                Operation::ArrayFill { .. }
                    | Operation::ArrayFillMany { .. }
                    | Operation::ArrayFinish { .. }
            )
        })
    {
        return Ok(declarations);
    }
    let mut blocks = BTreeMap::<BlockId, Vec<usize>>::new();
    for (index, block) in structure.blocks.iter().enumerate() {
        blocks.entry(*block).or_default().push(index);
    }
    let entry = structure.blocks[0];
    let mut inputs = BTreeMap::from([(entry, BTreeMap::<BuilderId, u32>::new())]);
    let mut pending = VecDeque::from([entry]);
    while let Some(block) = pending.pop_front() {
        let mut active = inputs[&block].clone();
        let mut successors = vec![];
        for &index in &blocks[&block] {
            match operations[index].operation {
                Operation::ArrayBegin { builder, .. } => {
                    if active.insert(builder, 0).is_some() {
                        return Err(fail(index, "array builder restarted before completion"));
                    }
                }
                Operation::ArrayFill {
                    builder,
                    index: slot,
                    ..
                } => {
                    let count = active
                        .get_mut(&builder)
                        .ok_or_else(|| fail(index, "array fill requires an active builder"))?;
                    if slot != *count || slot >= declarations[&builder].1 {
                        return Err(fail(
                            index,
                            "array fills must initialize each index exactly once in order",
                        ));
                    }
                    *count += 1;
                }
                Operation::ArrayFinish { builder, .. } => {
                    let count = active
                        .remove(&builder)
                        .ok_or_else(|| fail(index, "array finish requires an active builder"))?;
                    if count != declarations[&builder].1 {
                        return Err(fail(
                            index,
                            "array finish requires all elements initialized",
                        ));
                    }
                }
                Operation::ArrayFillMany {
                    builder,
                    start,
                    ref values,
                } => {
                    let count = active
                        .get_mut(&builder)
                        .ok_or_else(|| fail(index, "array fill requires an active builder"))?;
                    if values.is_empty()
                        || start != *count
                        || values.len() > (declarations[&builder].1 - *count) as usize
                    {
                        return Err(fail(
                            index,
                            "array fill group must initialize consecutive remaining slots",
                        ));
                    }
                    *count += values.len() as u32;
                }
                Operation::Branch {
                    on_true, on_false, ..
                } => successors.extend([on_true, on_false]),
                Operation::Jump { target } => successors.push(target),
                Operation::Return { .. } | Operation::ReturnVoid if !active.is_empty() => {
                    return Err(fail(index, "return leaves an unfinished array builder"));
                }
                _ => (),
            }
        }
        for next in successors {
            if let Some(previous) = inputs.get(&next) {
                if previous != &active {
                    return Err(fail(
                        blocks[&next][0],
                        "array builder progress must agree on every incoming path",
                    ));
                }
            } else {
                inputs.insert(next, active.clone());
                pending.push_back(next);
            }
        }
    }
    Ok(declarations)
}
