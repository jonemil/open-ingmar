//! Reconstruct method-local positional field views from reads and mutations.
use crate::{
    IrError, LocalId, LocatedOperation, Operation, ParameterType, ReturnType, ValueId, api::Type,
};
use std::collections::BTreeMap;

type State = BTreeMap<String, ReturnType>;

/// Check whether a declaration includes a positional field alternative.
pub(crate) fn positional(ty: &ReturnType) -> bool {
    crate::unions::alternatives(&ty.api_type())
        .iter()
        .any(|part| matches!(part, Type::TupleValue(_)))
}

fn fail(at: usize, message: impl Into<String>) -> IrError {
    IrError {
        operation: Some(at),
        message: message.into(),
    }
}

fn join(a: &ReturnType, b: &ReturnType) -> Option<ReturnType> {
    if a == b {
        return Some(a.clone());
    }
    let parts = crate::unions::normalize(vec![a.api_type(), b.api_type()]);
    let alternatives = crate::unions::alternatives(&parts);
    if alternatives.iter().any(|ty| matches!(ty, Type::Array(_)))
        && alternatives
            .iter()
            .all(|ty| matches!(ty, Type::Array(_) | Type::TupleValue(_) | Type::Null))
    {
        let nullable = alternatives.contains(&Type::Null);
        let inputs = alternatives
            .iter()
            .filter(|ty| **ty != Type::Null)
            .cloned()
            .collect::<Vec<_>>();
        if let Ok(ty) = crate::tuples::array_contract(&inputs, &[]) {
            return crate::unions::result(if nullable { ty.with_null() } else { ty });
        }
    }
    crate::joins::call_result(a, b)
}

fn barrier(op: &Operation) -> bool {
    matches!(
        op,
        Operation::SelfCall { .. }
            | Operation::SelfCallStatement { .. }
            | Operation::GlobalCall { .. }
            | Operation::GlobalCallStatement { .. }
            | Operation::ModuleCall { .. }
            | Operation::ModuleCallStatement { .. }
            | Operation::ObjectCall { .. }
            | Operation::ObjectCallStatement { .. }
            | Operation::NativeCall { .. }
            | Operation::NativeCallStatement { .. }
            | Operation::NativeModuleCall { .. }
            | Operation::NativeModuleCallStatement { .. }
            | Operation::SourceNew { .. }
            | Operation::NativeNew { .. }
            | Operation::WeakReferenceNew { .. }
            | Operation::Println { .. }
    )
}

fn restrict(ty: &ReturnType, non_null: bool) -> ReturnType {
    if let Some(mut contract) = ty.source_contract() {
        if non_null {
            contract.nullable = false;
            return contract.result();
        }
        return if contract.nullable {
            ReturnType::Null
        } else {
            ty.clone()
        };
    }
    let parts = crate::unions::alternatives(&ty.api_type())
        .iter()
        .filter(|part| (**part != Type::Null) == non_null)
        .cloned()
        .collect::<Vec<_>>();
    if parts.is_empty() {
        ty.clone()
    } else {
        crate::unions::result(crate::unions::normalize(parts)).unwrap_or_else(|| ty.clone())
    }
}

fn value_contract(types: &crate::source_types::Types, value: ValueId) -> Option<ReturnType> {
    let ty = types.join_contract(value)?;
    ty.source_contract()
        .map(crate::SourceUnion::result)
        .or_else(|| crate::unions::result(ty.api_type()))
}

/// Derive field-read contracts without trusting their claimed result types.
/// Fixed receiver captures preserve direct field provenance; inferred aliases
/// establish independent views. Calls and explicit field assignments reset views.
pub(crate) fn reads(
    ops: &[LocatedOperation],
    parameters: Vec<ParameterType>,
) -> Result<BTreeMap<usize, ReturnType>, IrError> {
    let mut entry = State::new();
    for (at, located) in ops.iter().enumerate() {
        if let Operation::SelfFieldRead {
            field,
            result_type,
            declared_type,
            ..
        } = &located.operation
        {
            let declared = declared_type.as_ref().unwrap_or(result_type);
            if declared_type.is_some() && !positional(declared) {
                return Err(fail(
                    at,
                    "field entry metadata requires a positional contract",
                ));
            }
            if positional(declared) {
                if let Some(old) = entry.get(field) {
                    if declared_type.is_some() && old != declared {
                        return Err(fail(at, "inconsistent field entry contracts"));
                    }
                } else {
                    entry.insert(field.clone(), declared.clone());
                }
            }
        }
    }
    if entry.is_empty() {
        return Ok(BTreeMap::new());
    }
    let structure = crate::control_flow::prefix(ops);
    let facts = crate::type_flow::inputs(&structure, ops);
    let mut types = crate::source_types::Types::new(parameters);
    let mut origins = BTreeMap::<ValueId, String>::new();
    let mut locals = BTreeMap::<LocalId, String>::new();
    let mut conditions = BTreeMap::<ValueId, (String, bool)>::new();
    let mut guard_origins = BTreeMap::<ValueId, String>::new();
    let mut guard_locals = BTreeMap::<LocalId, String>::new();
    let mut events = BTreeMap::<usize, (String, ReturnType)>::new();
    let mut assignments = BTreeMap::<usize, (String, ReturnType)>::new();
    for (at, located) in ops.iter().enumerate() {
        let op = &located.operation;
        crate::type_flow::update(&mut types.facts, &facts, op);
        types.observe(op);
        if barrier(op) {
            guard_origins.clear();
            guard_locals.clear();
        }
        match op {
            Operation::SelfFieldRead { result, field, .. } if entry.contains_key(field) => {
                origins.insert(*result, field.clone());
                guard_origins.insert(*result, field.clone());
            }
            Operation::LocalInitialize { local, value } => {
                if let Some(field) = origins.get(value) {
                    locals.insert(*local, field.clone());
                }
                if let Some(field) = guard_origins.get(value) {
                    guard_locals.insert(*local, field.clone());
                }
            }
            Operation::LocalAssign { local, .. } | Operation::LocalVariable { local, .. } => {
                locals.remove(local);
                guard_locals.remove(local);
            }
            Operation::LocalRead { result, local } | Operation::LocalTake { result, local } => {
                if let Some(field) = locals.get(local) {
                    origins.insert(*result, field.clone());
                }
                if let Some(field) = guard_locals.get(local) {
                    guard_origins.insert(*result, field.clone());
                }
            }
            Operation::SelfFieldAssign { field, value } if entry.contains_key(field) => {
                guard_origins.retain(|_, name| name != field);
                guard_locals.retain(|_, name| name != field);
                if let Some(ty) = value_contract(&types, *value) {
                    assignments.insert(at, (field.clone(), ty));
                }
            }
            Operation::TupleWrite { result, array, .. }
            | Operation::TupleAppend { result, array, .. } => {
                if let Some(field) = origins.get(array).cloned() {
                    origins.insert(*result, field.clone());
                    if guard_origins.contains_key(array) {
                        guard_origins.insert(*result, field.clone());
                    }
                    let ty = value_contract(&types, *result)
                        .ok_or_else(|| fail(at, "cannot derive tuple field mutation contract"))?;
                    events.insert(at, (field, ty));
                }
            }
            Operation::Compare {
                result,
                kind,
                left,
                right,
            } if matches!(
                kind,
                crate::ComparisonOp::Equal | crate::ComparisonOp::NotEqual
            ) =>
            {
                let field = if types.value_type(*left) == Some(&Type::Null) {
                    guard_origins.get(right)
                } else if types.value_type(*right) == Some(&Type::Null) {
                    guard_origins.get(left)
                } else {
                    None
                };
                if let Some(field) = field {
                    conditions.insert(
                        *result,
                        (field.clone(), *kind == crate::ComparisonOp::NotEqual),
                    );
                }
            }
            Operation::Not { result, operand } => {
                if let Some((field, non_null)) = conditions.get(operand).cloned() {
                    conditions.insert(*result, (field, !non_null));
                }
            }
            _ => {}
        }
    }
    let branches = ops
        .iter()
        .enumerate()
        .filter_map(|(at, located)| {
            if let Operation::Branch {
                condition,
                on_true,
                on_false,
            } = located.operation
            {
                conditions
                    .get(&condition)
                    .map(|test| (structure.blocks[at], (on_true, on_false, test.clone())))
            } else {
                None
            }
        })
        .collect::<BTreeMap<_, _>>();
    let transfer = |state: &mut State, at: usize| {
        let op = &ops[at].operation;
        if barrier(op) {
            *state = entry.clone();
        }
        if let Operation::SelfFieldAssign { field, .. } = op {
            if let Some((_, ty)) = assignments.get(&at) {
                state.insert(field.clone(), ty.clone());
            } else if let Some(ty) = entry.get(field) {
                state.insert(field.clone(), ty.clone());
            }
        }
        if let Some((field, ty)) = events.get(&at) {
            state.insert(field.clone(), ty.clone());
        }
    };
    let mut incoming = structure
        .predecessors
        .keys()
        .map(|&block| (block, None::<State>))
        .collect::<BTreeMap<_, _>>();
    let mut outgoing = incoming.clone();
    for iteration in 0..=ops.len() {
        let mut changed = false;
        for (&block, preds) in &structure.predecessors {
            let mut states = Vec::new();
            if structure.roots.contains(&block) {
                states.push(entry.clone());
            }
            for pred in preds {
                let Some(mut state) = outgoing[pred].clone() else {
                    continue;
                };
                if let Some((yes, no, (field, positive))) = branches.get(pred) {
                    if yes != no {
                        let non_null = if block == *yes { *positive } else { !*positive };
                        if let Some(ty) = state.get_mut(field) {
                            *ty = restrict(ty, non_null);
                        }
                    }
                }
                states.push(state);
            }
            let Some(mut state) = states.first().cloned() else {
                continue;
            };
            for other in &states[1..] {
                for (field, ty) in &mut state {
                    *ty = join(ty, &other[field])
                        .ok_or_else(|| fail(0, "field paths have no supported contract join"))?;
                }
            }
            let mut output = state.clone();
            for (at, _) in ops.iter().enumerate() {
                if structure.blocks[at] == block {
                    transfer(&mut output, at);
                }
            }
            changed |= incoming[&block].as_ref() != Some(&state)
                || outgoing[&block].as_ref() != Some(&output);
            incoming.insert(block, Some(state));
            outgoing.insert(block, Some(output));
        }
        if !changed {
            break;
        }
        if iteration == ops.len() {
            return Err(fail(0, "tuple field inference did not converge"));
        }
    }
    let mut result = BTreeMap::new();
    let mut state = entry.clone();
    for (at, located) in ops.iter().enumerate() {
        if let Operation::Block { block } = located.operation {
            state = incoming[&block].clone().unwrap_or_else(|| entry.clone());
        }
        if let Operation::SelfFieldRead { field, .. } = &located.operation {
            if let Some(ty) = state.get(field) {
                result.insert(at, ty.clone());
            }
        }
        transfer(&mut state, at);
    }
    Ok(result)
}

/// Infer a new source read under the field's declared entry contract and the
/// effects already emitted. Complete-function validation rechecks this claim.
pub(crate) fn prefix_read(
    ops: &[LocatedOperation],
    field: &str,
    declared: &ReturnType,
    parameters: Vec<ParameterType>,
) -> Result<ReturnType, String> {
    let mut candidate = ops.to_vec();
    candidate.push(LocatedOperation {
        operation: Operation::SelfFieldRead {
            result: ValueId(ops.len() as u32),
            field: field.into(),
            result_type: declared.clone(),
            declared_type: Some(declared.clone()),
        },
        span: None,
    });
    reads(&candidate, parameters)
        .map_err(|e| e.message)?
        .remove(&ops.len())
        .ok_or_else(|| "cannot derive tuple field read".into())
}

/// Solve field and frame-local read annotations together after all backedges
/// are known. Reject unsupported joins or inference that does not converge.
pub(crate) fn normalize(
    ops: &mut [LocatedOperation],
    parameters: Vec<ParameterType>,
) -> Result<(), IrError> {
    if !ops.iter().any(|op| {
        matches!(&op.operation, Operation::SelfFieldRead { result_type, declared_type, .. }
        if declared_type.is_some() || positional(result_type))
    }) {
        return crate::local_flow::normalize(ops, parameters);
    }
    for _ in 0..=ops.len() {
        let inferred = reads(ops, parameters.clone())?;
        let mut changed = false;
        for (at, ty) in inferred {
            if let Operation::SelfFieldRead { result_type, .. } = &mut ops[at].operation {
                if *result_type != ty {
                    *result_type = ty;
                    changed = true;
                }
            }
        }
        let before = ops.to_vec();
        crate::local_flow::normalize(ops, parameters.clone())?;
        changed |= ops != before;
        if !changed {
            return Ok(());
        }
    }
    Err(fail(0, "tuple field/local inference did not converge"))
}

/// Check saved field views against reconstructed flow. Unit validation must
/// additionally resolve every declared entry contract against its class field.
pub(crate) fn validate(
    ops: &[LocatedOperation],
    parameters: Vec<ParameterType>,
) -> Result<(), IrError> {
    for (at, ty) in reads(ops, parameters)? {
        if let Operation::SelfFieldRead { result_type, .. } = &ops[at].operation {
            if *result_type != ty {
                return Err(fail(
                    at,
                    "field read contract is not established on every incoming path",
                ));
            }
        }
    }
    Ok(())
}

/// Recover a direct field receiver through immutable staging captures. Named
/// inferred aliases deliberately do not update the field's flow view.
pub(crate) fn direct_field(ops: &[LocatedOperation], mut value: ValueId) -> Option<String> {
    for _ in 0..=ops.len() {
        let operation = ops.iter().find_map(|op| match &op.operation {
            Operation::SelfFieldRead { result, .. }
            | Operation::LocalRead { result, .. }
            | Operation::LocalTake { result, .. }
            | Operation::TupleAppend { result, .. }
            | Operation::TupleWrite { result, .. }
                if *result == value =>
            {
                Some(&op.operation)
            }
            _ => None,
        })?;
        match operation {
            Operation::SelfFieldRead {
                field,
                declared_type: Some(_),
                ..
            } => return Some(field.clone()),
            Operation::LocalRead { local, .. } | Operation::LocalTake { local, .. } => {
                value = ops.iter().find_map(|op| match op.operation {
                    Operation::LocalInitialize { local: at, value } if at == *local => Some(value),
                    _ => None,
                })?;
            }
            Operation::TupleAppend { array, .. } | Operation::TupleWrite { array, .. } => {
                value = *array
            }
            _ => return None,
        }
    }
    None
}
