//! Definite completion of private construction builders across control-flow joins.
use crate::{
    ArrayElement, BlockId, BuilderId, DictionaryType, IrError, LocatedOperation, Operation,
    control_flow::Structure,
};
use std::collections::{BTreeMap, VecDeque};

#[derive(Clone)]
pub(crate) enum Contract {
    Array(ArrayElement),
    ByteArray,
    Source {
        class: String,
        base: crate::ClassBase,
    },
    Dictionary(DictionaryType),
    Record(BTreeMap<String, crate::api::Type>),
}
impl Contract {
    pub(crate) fn element_type(&self) -> Option<crate::api::Type> {
        match self {
            Self::Array(element) => Some(element.api_type()),
            Self::ByteArray => Some(crate::api::Type::Union(vec![
                crate::api::Type::named("Toybox.Lang.Number"),
                crate::api::Type::named("Toybox.Lang.Char"),
            ])),
            Self::Dictionary(_) | Self::Record(_) | Self::Source { .. } => None,
        }
    }
    pub(crate) fn dictionary(&self) -> Option<DictionaryType> {
        if let Self::Dictionary(ty) = self {
            Some(ty.clone())
        } else {
            None
        }
    }
}

/// Check builder lifetimes and ordered fills on every reachable CFG path.
/// Return immutable construction contracts for element checking and frame layout.
pub(crate) fn validate(
    structure: &Structure,
    operations: &[LocatedOperation],
) -> Result<BTreeMap<BuilderId, (Contract, u32)>, IrError> {
    let fail = |index, message: &str| IrError {
        operation: Some(index),
        message: message.into(),
    };
    let mut declarations = BTreeMap::new();
    for (index, op) in operations.iter().enumerate() {
        if let Operation::ArrayBegin {
            builder,
            ref element_type,
            length,
        } = op.operation
        {
            if length == 0 || length > i32::MAX as u32 {
                return Err(fail(index, "builder length requires a positive Number"));
            }
            if matches!(element_type, ArrayElement::Object(class) if *class != crate::NativeClass::View)
            {
                return Err(fail(index, "object builders support only View elements"));
            }
            if declarations
                .insert(builder, (Contract::Array(element_type.clone()), length))
                .is_some()
            {
                return Err(fail(index, "duplicate array builder declaration"));
            }
        }
    }
    for (index, op) in operations.iter().enumerate() {
        if let Operation::ByteArrayBegin { builder, length } = op.operation {
            if length == 0 || length > i32::MAX as u32 {
                return Err(fail(
                    index,
                    "ByteArray builder length requires a positive Number",
                ));
            }
            if declarations
                .insert(builder, (Contract::ByteArray, length))
                .is_some()
            {
                return Err(fail(index, "duplicate container builder declaration"));
            }
        }
    }
    for (index, op) in operations.iter().enumerate() {
        if let Operation::DictionaryBegin {
            builder,
            ref dictionary_type,
            length,
        } = op.operation
        {
            if length == 0 || length > i32::MAX as u32 {
                return Err(fail(
                    index,
                    "Dictionary builder length requires a positive Number",
                ));
            }
            if declarations
                .insert(
                    builder,
                    (Contract::Dictionary(dictionary_type.clone()), length),
                )
                .is_some()
            {
                return Err(fail(index, "duplicate container builder declaration"));
            }
        }
    }
    for (index, op) in operations.iter().enumerate() {
        if let Operation::RecordBegin {
            builder,
            length,
            ref fields,
        } = op.operation
        {
            if length > i32::MAX as u32 {
                return Err(fail(index, "record length exceeds the supported range"));
            }
            crate::records::validate(fields).map_err(|message| fail(index, message))?;
            if declarations
                .insert(builder, (Contract::Record(fields.clone()), length))
                .is_some()
            {
                return Err(fail(index, "duplicate container builder declaration"));
            }
        }
    }
    for (index, op) in operations.iter().enumerate() {
        if let Operation::SourceBegin {
            builder,
            ref class,
            base,
        } = op.operation
        {
            if declarations
                .insert(
                    builder,
                    (
                        Contract::Source {
                            class: class.clone(),
                            base,
                        },
                        0,
                    ),
                )
                .is_some()
            {
                return Err(fail(index, "duplicate construction builder declaration"));
            }
        }
    }
    if declarations.is_empty()
        && !operations.iter().any(|o| {
            matches!(
                o.operation,
                Operation::SourceNew { .. }
                    | Operation::DictionaryFill { .. }
                    | Operation::DictionaryFinish { .. }
                    | Operation::ArrayFill { .. }
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
    let mut inputs = structure
        .roots
        .iter()
        .map(|root| (*root, BTreeMap::<BuilderId, u32>::new()))
        .collect::<BTreeMap<_, _>>();
    let mut pending = VecDeque::from(structure.roots.clone());
    while let Some(block) = pending.pop_front() {
        let mut active = inputs[&block].clone();
        let mut successors = vec![];
        for &index in &blocks[&block] {
            match operations[index].operation {
                Operation::SourceBegin { builder, .. }
                | Operation::DictionaryBegin { builder, .. }
                | Operation::RecordBegin { builder, .. }
                | Operation::ArrayBegin { builder, .. }
                | Operation::ByteArrayBegin { builder, .. } => {
                    if active.insert(builder, 0).is_some() {
                        return Err(fail(index, "container builder restarted before completion"));
                    }
                }
                Operation::SourceNew {
                    builder,
                    ref class,
                    base,
                    ..
                } => {
                    if !matches!(declarations.get(&builder), Some((Contract::Source { class: owner, base: native }, _)) if owner == class && *native == base)
                    {
                        return Err(fail(
                            index,
                            "source completion requires its matching allocation builder",
                        ));
                    }
                    if active.remove(&builder).is_none() {
                        return Err(fail(
                            index,
                            "source completion requires an active allocation builder",
                        ));
                    }
                }
                Operation::ArrayFill {
                    builder,
                    index: slot,
                    ..
                } => {
                    if declarations
                        .get(&builder)
                        .is_none_or(|(contract, _)| contract.element_type().is_none())
                    {
                        return Err(fail(index, "array fill requires an Array builder"));
                    }
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
                    if declarations
                        .get(&builder)
                        .is_none_or(|(contract, _)| contract.element_type().is_none())
                    {
                        return Err(fail(index, "array finish requires an Array builder"));
                    }
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
                    if declarations
                        .get(&builder)
                        .is_none_or(|(contract, _)| contract.element_type().is_none())
                    {
                        return Err(fail(index, "array fill requires an Array builder"));
                    }
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
                Operation::DictionaryFill {
                    builder,
                    start,
                    ref entries,
                } => {
                    if declarations.get(&builder).is_none_or(|(contract, _)| {
                        contract.dictionary().is_none() && !matches!(contract, Contract::Record(_))
                    }) {
                        return Err(fail(index, "Dictionary fill requires a Dictionary builder"));
                    }
                    let count = active
                        .get_mut(&builder)
                        .ok_or_else(|| fail(index, "Dictionary fill requires an active builder"))?;
                    if entries.is_empty()
                        || start != *count
                        || entries.len() > (declarations[&builder].1 - *count) as usize
                    {
                        return Err(fail(
                            index,
                            "Dictionary fills must cover consecutive remaining entries",
                        ));
                    }
                    *count += entries.len() as u32;
                }
                Operation::DictionaryFinish { result: _, builder } => {
                    if declarations.get(&builder).is_none_or(|(contract, _)| {
                        contract.dictionary().is_none() && !matches!(contract, Contract::Record(_))
                    }) {
                        return Err(fail(
                            index,
                            "Dictionary finish requires a Dictionary builder",
                        ));
                    }
                    let count = active.remove(&builder).ok_or_else(|| {
                        fail(index, "Dictionary finish requires an active builder")
                    })?;
                    if count != declarations[&builder].1 {
                        return Err(fail(
                            index,
                            "Dictionary finish requires every source entry inserted",
                        ));
                    }
                }
                Operation::Branch {
                    on_true, on_false, ..
                } => successors.extend([on_true, on_false]),
                Operation::Jump { target } | Operation::JumpValue { target, .. } => {
                    successors.push(target)
                }
                Operation::Return { .. } | Operation::ReturnVoid if !active.is_empty() => {
                    return Err(fail(index, "return leaves an unfinished container builder"));
                }
                _ => (),
            }
        }
        for next in successors {
            if !structure.same_component(block, next) {
                if !active.is_empty() {
                    return Err(fail(
                        blocks[&block][0],
                        "component exit leaves an unfinished container builder",
                    ));
                }
                continue;
            }
            if let Some(previous) = inputs.get(&next) {
                if previous != &active {
                    return Err(fail(
                        blocks[&next][0],
                        "container builder progress must agree on every incoming path",
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
