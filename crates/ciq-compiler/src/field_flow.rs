//! Reconstruct method-local positional field views from reads and mutations.
use crate::{
    IrError, LocalId, LocatedOperation, Operation, ParameterType, ReturnType, ValueId, api::Type,
};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Field {
    SelfField(String),
    Structural(Vec<crate::StructuralDeclaration>, String),
}
type State = BTreeMap<Field, ReturnType>;

/// Resolve a field on a canonical structural declaration group. A union's
/// group is distinct from its constituents, matching the SDK's polytype views.
fn member_fields(receiver: &Type, member: &str) -> Vec<(Field, ReturnType)> {
    let mut origins = Vec::new();
    let mut declared = None;
    for part in crate::unions::alternatives(receiver) {
        let Type::Structural(shape) = part else {
            return vec![];
        };
        let Some(origin) = shape.declaration else {
            return vec![];
        };
        let Some(ty) = shape.fields.get(member).filter(|ty| positional(ty)) else {
            return vec![];
        };
        origins.push(origin);
        declared = match declared {
            None => Some(ty.clone()),
            Some(previous) => join(&previous, ty),
        };
        if declared.is_none() {
            return vec![];
        }
    }
    origins.sort();
    origins.dedup();
    declared
        .map(|ty| vec![(Field::Structural(origins, member.into()), ty)])
        .unwrap_or_default()
}

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
/// Direct captures retain field provenance. Named tuple aliases establish
/// independent views; structural owner aliases retain their declaration identity.
pub(crate) fn reads(
    ops: &[LocatedOperation],
    parameters: Vec<ParameterType>,
) -> Result<BTreeMap<usize, ReturnType>, IrError> {
    let structure = crate::control_flow::prefix(ops);
    let facts = crate::type_flow::inputs(&structure, ops);
    let mut entry = State::new();
    let mut read_fields = BTreeMap::<usize, Vec<Field>>::new();
    let mut assign_fields = BTreeMap::<usize, Vec<Field>>::new();
    let mut types = crate::source_types::Types::new(parameters.clone());
    for (at, located) in ops.iter().enumerate() {
        let op = &located.operation;
        crate::type_flow::update(&mut types.facts, &facts, op);
        let mut declarations = Vec::new();
        match op {
            Operation::SelfFieldRead {
                field,
                result_type,
                declared_type,
                ..
            } => {
                let declared = declared_type.as_ref().unwrap_or(result_type);
                if declared_type.is_some() && !positional(declared) {
                    return Err(fail(
                        at,
                        "field entry metadata requires a positional contract",
                    ));
                }
                if positional(declared) {
                    declarations.push((Field::SelfField(field.clone()), declared.clone()));
                    read_fields.insert(
                        at,
                        declarations.iter().map(|(key, _)| key.clone()).collect(),
                    );
                }
            }
            Operation::MemberRead {
                receiver,
                member,
                field_type: None,
                view_type,
                ..
            } => {
                declarations = types
                    .value_type(*receiver)
                    .map(|ty| member_fields(ty, member))
                    .unwrap_or_default();
                if !declarations.is_empty() {
                    read_fields.insert(
                        at,
                        declarations.iter().map(|(key, _)| key.clone()).collect(),
                    );
                } else if view_type.is_some() {
                    return Err(fail(
                        at,
                        "structural flow view requires tuple-field declaration provenance",
                    ));
                }
            }
            Operation::SelfFieldAssign { field, .. } => {
                assign_fields.insert(at, vec![Field::SelfField(field.clone())]);
            }
            Operation::MemberWrite {
                receiver,
                member,
                field_type: None,
                ..
            } => {
                declarations = types
                    .value_type(*receiver)
                    .map(|ty| member_fields(ty, member))
                    .unwrap_or_default();
                if !declarations.is_empty() {
                    assign_fields.insert(
                        at,
                        declarations.iter().map(|(key, _)| key.clone()).collect(),
                    );
                }
            }
            _ => {}
        }
        for (key, declared) in declarations {
            if entry.get(&key).is_some_and(|old| *old != declared) {
                return Err(fail(at, "inconsistent field entry contracts"));
            }
            entry.insert(key, declared);
        }
        types.observe(op);
    }
    if entry.is_empty() {
        return Ok(BTreeMap::new());
    }
    let mut types = crate::source_types::Types::new(parameters);
    let mut origins = BTreeMap::<ValueId, Vec<Field>>::new();
    let mut locals = BTreeMap::<LocalId, Vec<Field>>::new();
    let mut conditions = BTreeMap::<ValueId, (Vec<Field>, bool)>::new();
    let mut guard_origins = BTreeMap::<ValueId, Vec<Field>>::new();
    let mut guard_locals = BTreeMap::<LocalId, Vec<Field>>::new();
    let mut events = BTreeMap::<usize, (Vec<Field>, ReturnType)>::new();
    let mut branches = BTreeMap::new();
    for (at, located) in ops.iter().enumerate() {
        let op = &located.operation;
        crate::type_flow::update(&mut types.facts, &facts, op);
        types.observe(op);
        if barrier(op) {
            guard_origins.clear();
            guard_locals.clear();
            conditions.clear();
        }
        if let Some(fields) = read_fields.get(&at) {
            let result = match op {
                Operation::SelfFieldRead { result, .. } | Operation::MemberRead { result, .. } => {
                    *result
                }
                _ => unreachable!(),
            };
            origins.insert(result, fields.clone());
            guard_origins.insert(result, fields.clone());
        }
        if let Some(receiver) = crate::tuples::removal_receiver(op) {
            if let Some(fields) = origins.get(&receiver) {
                if let Some(ty) = types
                    .value_type(receiver)
                    .and_then(crate::tuples::removal_contract)
                {
                    events.insert(
                        at,
                        (
                            fields.clone(),
                            crate::unions::result(ty).expect("erased Array result"),
                        ),
                    );
                }
            }
        }
        if let Some(fields) = assign_fields.get(&at) {
            guard_origins.retain(|_, keys| !keys.iter().any(|key| fields.contains(key)));
            guard_locals.retain(|_, keys| !keys.iter().any(|key| fields.contains(key)));
            conditions.retain(|_, (keys, _)| !keys.iter().any(|key| fields.contains(key)));
            let value = match op {
                Operation::SelfFieldAssign { value, .. } | Operation::MemberWrite { value, .. } => {
                    *value
                }
                _ => unreachable!(),
            };
            let ty = value_contract(&types, value)
                .ok_or_else(|| fail(at, "cannot derive field assignment contract"))?;
            events.insert(
                at,
                (
                    fields
                        .iter()
                        .filter(|key| entry.contains_key(*key))
                        .cloned()
                        .collect(),
                    ty,
                ),
            );
        }
        match op {
            Operation::LocalInitialize { local, value } => {
                if let Some(fields) = origins.get(value) {
                    locals.insert(*local, fields.clone());
                }
                if let Some(fields) = guard_origins.get(value) {
                    guard_locals.insert(*local, fields.clone());
                }
            }
            Operation::LocalAssign { local, .. } | Operation::LocalVariable { local, .. } => {
                locals.remove(local);
                guard_locals.remove(local);
            }
            Operation::LocalRead { result, local } | Operation::LocalTake { result, local } => {
                if let Some(fields) = locals.get(local) {
                    origins.insert(*result, fields.clone());
                }
                if let Some(fields) = guard_locals.get(local) {
                    guard_origins.insert(*result, fields.clone());
                }
            }
            Operation::TupleWrite { result, array, .. }
            | Operation::TupleAppend { result, array, .. } => {
                if let Some(fields) = origins.get(array).cloned() {
                    origins.insert(*result, fields.clone());
                    if guard_origins.contains_key(array) {
                        guard_origins.insert(*result, fields.clone());
                    }
                    let ty = value_contract(&types, *result)
                        .ok_or_else(|| fail(at, "cannot derive tuple field mutation contract"))?;
                    events.insert(at, (fields, ty));
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
                let fields = if types.value_type(*left) == Some(&Type::Null) {
                    guard_origins.get(right)
                } else if types.value_type(*right) == Some(&Type::Null) {
                    guard_origins.get(left)
                } else {
                    None
                };
                if let Some(fields) = fields {
                    conditions.insert(
                        *result,
                        (fields.clone(), *kind == crate::ComparisonOp::NotEqual),
                    );
                }
            }
            Operation::Branch {
                condition,
                on_true,
                on_false,
            } => {
                if let Some(test) = conditions.get(condition) {
                    branches.insert(structure.blocks[at], (*on_true, *on_false, test.clone()));
                }
            }
            Operation::Not { result, operand } => {
                if let Some((fields, non_null)) = conditions.get(operand).cloned() {
                    conditions.insert(*result, (fields, !non_null));
                }
            }
            _ => {}
        }
    }
    let transfer = |state: &mut State, at: usize| {
        if barrier(&ops[at].operation) {
            *state = entry.clone();
        }
        if let Some((fields, ty)) = events.get(&at) {
            for field in fields {
                state.insert(field.clone(), ty.clone());
            }
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
                if let Some((yes, no, (fields, positive))) = branches.get(pred) {
                    if yes != no {
                        let non_null = if block == *yes { *positive } else { !*positive };
                        for field in fields {
                            if let Some(ty) = state.get_mut(field) {
                                *ty = restrict(ty, non_null);
                            }
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
            for at in 0..ops.len() {
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
        if let Some(fields) = read_fields.get(&at) {
            let mut views = fields.iter().map(|field| state[field].clone());
            let first = views.next().expect("tracked field alternatives");
            let ty = views.try_fold(first, |a, b| {
                join(&a, &b)
                    .ok_or_else(|| fail(at, "field alternatives have no supported view join"))
            })?;
            result.insert(at, ty);
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

/// Infer a new structural field read from its declared owner provenance and
/// already-emitted effects. Untracked declarations retain ordinary field reads.
pub(crate) fn prefix_member_read(
    ops: &[LocatedOperation],
    receiver: ValueId,
    member: &str,
    parameters: Vec<ParameterType>,
) -> Result<Option<ReturnType>, String> {
    let mut candidate = ops.to_vec();
    candidate.push(LocatedOperation {
        operation: Operation::MemberRead {
            result: ValueId(ops.len() as u32),
            receiver,
            member: member.into(),
            field_type: None,
            view_type: None,
        },
        span: None,
    });
    Ok(reads(&candidate, parameters)
        .map_err(|e| e.message)?
        .remove(&ops.len()))
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
            || matches!(
                &op.operation,
                Operation::MemberRead {
                    field_type: None,
                    ..
                }
            )
    }) {
        return crate::local_flow::normalize(ops, parameters);
    }
    for _ in 0..=ops.len() {
        let inferred = reads(ops, parameters.clone())?;
        let mut changed = false;
        for (at, ty) in inferred {
            match &mut ops[at].operation {
                Operation::SelfFieldRead { result_type, .. } => {
                    if *result_type != ty {
                        *result_type = ty;
                        changed = true;
                    }
                }
                Operation::MemberRead { view_type, .. } => {
                    if view_type.as_ref() != Some(&ty) {
                        *view_type = Some(ty);
                        changed = true;
                    }
                }
                _ => unreachable!(),
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
    let mut types = crate::source_types::Types::new(parameters.clone());
    let inferred = reads(ops, parameters)?;
    if inferred.is_empty() {
        return Ok(());
    }
    let structure = crate::control_flow::prefix(ops);
    let facts = crate::type_flow::inputs(&structure, ops);
    for (at, op) in ops.iter().enumerate() {
        crate::type_flow::update(&mut types.facts, &facts, &op.operation);
        if let Some(expected) = inferred.get(&at) {
            let claimed = match &op.operation {
                Operation::SelfFieldRead { result_type, .. } => result_type.clone(),
                Operation::MemberRead {
                    receiver,
                    member,
                    view_type,
                    ..
                } => view_type
                    .clone()
                    .or_else(|| {
                        types
                            .value_type(*receiver)
                            .and_then(|ty| crate::structural::field_contract(ty, member).ok())
                    })
                    .ok_or_else(|| fail(at, "cannot resolve structural field declaration"))?,
                _ => unreachable!(),
            };
            if claimed != *expected {
                return Err(fail(
                    at,
                    "field read contract is not established on every incoming path",
                ));
            }
        }
        types.observe(&op.operation);
    }
    Ok(())
}

/// Check direct field provenance through immutable staging captures and append
/// results. Named inferred tuple aliases deliberately keep independent views.
pub(crate) fn tracked_receiver(ops: &[LocatedOperation], mut value: ValueId) -> bool {
    for _ in 0..=ops.len() {
        let Some(operation) = ops.iter().find_map(|op| match &op.operation {
            Operation::SelfFieldRead { result, .. }
            | Operation::MemberRead { result, .. }
            | Operation::LocalRead { result, .. }
            | Operation::LocalTake { result, .. }
            | Operation::TupleAppend { result, .. }
            | Operation::TupleWrite { result, .. }
                if *result == value =>
            {
                Some(&op.operation)
            }
            _ => None,
        }) else {
            return false;
        };
        match operation {
            Operation::SelfFieldRead {
                declared_type: Some(_),
                ..
            }
            | Operation::MemberRead {
                view_type: Some(_), ..
            } => return true,
            Operation::LocalRead { local, .. } | Operation::LocalTake { local, .. } => {
                let Some(original) = capture(ops, *local) else {
                    return false;
                };
                value = original;
            }
            Operation::TupleAppend { array, .. } | Operation::TupleWrite { array, .. } => {
                value = *array
            }
            _ => return false,
        }
    }
    false
}

fn capture(ops: &[LocatedOperation], local: LocalId) -> Option<ValueId> {
    ops.iter().find_map(|op| match op.operation {
        Operation::LocalInitialize { local: at, value } if at == local => Some(value),
        _ => None,
    })
}

/// Check whether one immutable staging local captures a tracked field receiver.
pub(crate) fn captured_receiver(ops: &[LocatedOperation], local: LocalId) -> bool {
    capture(ops, local).is_some_and(|value| tracked_receiver(ops, value))
}
