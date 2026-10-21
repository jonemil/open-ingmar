//! Infer and validate the values carried by mutable, inferred frame variables.
use crate::{BlockId, IrError, JoinType, LocalId, LocatedOperation, Operation, ValueId, api::Type};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) type Domain = Vec<JoinType>;
pub(crate) type State = BTreeMap<LocalId, Domain>;

fn union(target: &mut Domain, source: &[JoinType]) {
    for ty in source {
        if !target.contains(ty) {
            target.push(ty.clone());
        }
    }
}

fn transfer(state: &mut State, op: &Operation, values: &BTreeMap<ValueId, JoinType>) {
    match op {
        Operation::LocalVariable { local, value } => {
            state.insert(
                *local,
                value
                    .and_then(|v| values.get(&v).cloned())
                    .into_iter()
                    .collect(),
            );
        }
        Operation::LocalAssign { local, value } if state.contains_key(local) => {
            state.insert(*local, values.get(value).cloned().into_iter().collect());
        }
        _ => {}
    }
}

/// Compute possible write contracts at each block entry to a least fixed point.
/// Each write replaces its slot's domain; joins union predecessor alternatives.
/// Definite assignment and declaration dominance are checked independently.
pub(crate) fn inputs(
    structure: &crate::control_flow::Structure,
    ops: &[LocatedOperation],
    values: &BTreeMap<ValueId, JoinType>,
) -> BTreeMap<BlockId, State> {
    let vars = ops
        .iter()
        .filter_map(|op| {
            if let Operation::LocalVariable { local, .. } = op.operation {
                Some(local)
            } else {
                None
            }
        })
        .collect::<BTreeSet<_>>();
    let mut roots = State::new();
    for local in vars {
        roots.insert(local, Domain::new());
    }
    for op in ops {
        if let Operation::LocalAssign { local, value }
        | Operation::LocalVariable {
            local,
            value: Some(value),
        } = op.operation
        {
            if let (Some(domain), Some(ty)) = (roots.get_mut(&local), values.get(&value)) {
                union(domain, std::slice::from_ref(ty));
            }
        }
    }
    let entry = structure.blocks.first().copied().unwrap_or(BlockId(0));
    let mut incoming = structure
        .predecessors
        .keys()
        .map(|&b| (b, State::new()))
        .collect::<BTreeMap<_, _>>();
    let mut outgoing = incoming.clone();
    loop {
        let mut changed = false;
        for (&block, preds) in &structure.predecessors {
            let mut state = if block != entry && structure.roots.contains(&block) {
                roots.clone()
            } else {
                State::new()
            };
            for pred in preds {
                for (&local, domain) in &outgoing[pred] {
                    union(state.entry(local).or_default(), domain);
                }
            }
            let mut output = state.clone();
            for (at, op) in ops.iter().enumerate() {
                if structure.blocks[at] == block {
                    transfer(&mut output, &op.operation, values);
                }
            }
            changed |= incoming[&block] != state || outgoing[&block] != output;
            incoming.insert(block, state);
            outgoing.insert(block, output);
        }
        if !changed {
            return incoming;
        }
    }
}

/// Choose a supported read view without converting or changing stored values.
/// Preserve exact source identities and existing joins; heterogeneous domains
/// widen to Object (optionally Null) until general source unions are supported.
fn view(domain: &[JoinType]) -> Option<JoinType> {
    let mut result = domain.first()?.clone();
    for ty in &domain[1..] {
        result = crate::joins::contract_result(&result, ty).unwrap_or_else(|| {
            let nullable = [result.api_type(), ty.api_type()]
                .iter()
                .any(|t| crate::native::accepts(t, &Type::Null));
            JoinType::Api(if nullable {
                Type::named("Toybox.Lang.Object").with_null()
            } else {
                Type::named("Toybox.Lang.Object")
            })
        });
    }
    Some(result)
}

/// Apply the same checked class/non-null facts used by fixed local reads.
pub(crate) fn proven(
    domain: &[JoinType],
    local: LocalId,
    facts: &BTreeSet<crate::type_flow::Fact>,
) -> Option<JoinType> {
    let place = crate::type_flow::Place::Local(local);
    if let Some(class) = crate::type_flow::source_class(place, facts) {
        return Some(JoinType::Source {
            class,
            nullable: false,
        });
    }
    let mut ty = view(domain)?;
    if let JoinType::Source { nullable, .. } = &mut ty {
        if facts.contains(&crate::type_flow::Fact::NonNull(place)) {
            *nullable = false;
        }
        return Some(ty);
    }
    let value = crate::ValueType::from_api_type(&ty.api_type())?;
    Some(JoinType::Api(
        crate::type_flow::refine(value, place, facts).api_type(),
    ))
}

fn accepts(expected: &JoinType, actual: &JoinType) -> bool {
    match (expected, actual) {
        (
            JoinType::Source {
                class: a,
                nullable: a_null,
            },
            JoinType::Source {
                class: b,
                nullable: b_null,
            },
        ) => a == b && (*a_null || !b_null),
        (JoinType::Source { nullable: true, .. }, JoinType::Api(Type::Null)) => true,
        (JoinType::Source { .. }, _) => false,
        (_, JoinType::Api(Type::Any)) => *expected == JoinType::Api(Type::Any),
        (_, _) => crate::native::accepts(&expected.api_type(), &actual.api_type()),
    }
}

/// Check typed reads against storage flow reconstructed from the validated IR.
/// Read metadata is a claim, not permission to narrow an incoming value.
pub(crate) fn validate(
    structure: &crate::control_flow::Structure,
    ops: &[LocatedOperation],
    types: &BTreeMap<ValueId, crate::ValueType>,
    origins: &crate::source_objects::Origins,
) -> Result<(), IrError> {
    if !ops
        .iter()
        .any(|op| matches!(op.operation, Operation::LocalVariable { .. }))
    {
        return Ok(());
    }
    let values = types
        .iter()
        .map(|(&id, ty)| {
            (
                id,
                origins
                    .join_contract(id)
                    .unwrap_or_else(|| JoinType::Api(ty.api_type())),
            )
        })
        .collect();
    let incoming = inputs(structure, ops, &values);
    let facts = crate::type_flow::inputs(structure, ops);
    let mut current = State::new();
    let mut refinements = Default::default();
    for (at, op) in ops.iter().enumerate() {
        if let Operation::Block { block } = op.operation {
            current = incoming[&block].clone();
        }
        crate::type_flow::update(&mut refinements, &facts, &op.operation);
        if let Operation::VariableRead {
            local, value_type, ..
        } = &op.operation
        {
            let actual = current
                .get(local)
                .and_then(|domain| proven(domain, *local, &refinements));
            if !actual
                .as_ref()
                .is_some_and(|actual| accepts(value_type, actual))
            {
                return Err(IrError {
                    operation: Some(at),
                    message: "variable read contract is not established on every incoming path"
                        .into(),
                });
            }
        }
        transfer(&mut current, &op.operation, &values);
    }
    Ok(())
}

/// Infer a source-prefix read from its current slot, including current facts.
pub(crate) fn prefix_read(
    ops: &[LocatedOperation],
    local: LocalId,
    parameters: Vec<crate::ParameterType>,
) -> Option<JoinType> {
    let mut types = crate::source_types::Types::new(parameters);
    let facts = crate::type_flow::prefix_inputs(ops);
    for op in ops {
        if let Operation::Block { block } = op.operation {
            types.facts = facts.get(&block).cloned().unwrap_or_default();
        }
        types.observe(&op.operation);
    }
    let structure = crate::control_flow::prefix(ops);
    let values = types.contracts();
    let incoming = inputs(&structure, ops, &values);
    let block = structure.blocks.last().copied().unwrap_or(BlockId(0));
    let mut state = incoming.get(&block).cloned().unwrap_or_default();
    for (at, op) in ops.iter().enumerate() {
        if structure.blocks[at] == block {
            transfer(&mut state, &op.operation, &values);
        }
    }
    proven(state.get(&local)?, local, &types.facts)
}

/// Recompute source read annotations after backedges and all writes are known.
/// Validation independently rechecks the resulting contracts after construction.
pub(crate) fn normalize(
    ops: &mut [LocatedOperation],
    parameters: Vec<crate::ParameterType>,
) -> Result<(), IrError> {
    if !ops
        .iter()
        .any(|op| matches!(op.operation, Operation::LocalVariable { .. }))
    {
        return Ok(());
    }
    for _ in 0..=ops.len() {
        let mut types = crate::source_types::Types::new(parameters.clone());
        let structure = crate::control_flow::prefix(ops);
        let facts = crate::type_flow::inputs(&structure, ops);
        let edges = crate::joins::incoming_values(ops);
        let mut changed = false;
        for (at, op) in ops.iter_mut().enumerate() {
            crate::type_flow::update(&mut types.facts, &facts, &op.operation);
            if let Operation::BlockParameter { value_type, .. } = &mut op.operation {
                if let Some(values) = edges.get(&structure.blocks[at]) {
                    let mut contracts = values.iter().filter_map(|&v| types.join_contract(v));
                    if let Some(mut ty) = contracts.next() {
                        for next in contracts {
                            ty = crate::joins::contract_result(&ty, &next).ok_or_else(|| {
                                IrError {
                                    operation: Some(at),
                                    message:
                                        "conditional arms no longer have compatible inferred types"
                                            .into(),
                                }
                            })?;
                        }
                        if *value_type != ty {
                            *value_type = ty;
                            changed = true;
                        }
                    }
                }
            }
            types.observe(&op.operation);
        }
        let values = types.contracts();
        let structure = crate::control_flow::prefix(ops);
        let incoming = inputs(&structure, ops, &values);
        let facts = crate::type_flow::inputs(&structure, ops);
        let mut current = State::new();
        let mut refinements = Default::default();
        for (at, op) in ops.iter_mut().enumerate() {
            if let Operation::Block { block } = op.operation {
                current = incoming[&block].clone();
            }
            crate::type_flow::update(&mut refinements, &facts, &op.operation);
            if let Operation::VariableRead {
                local, value_type, ..
            } = &mut op.operation
            {
                let ty = current
                    .get(local)
                    .and_then(|d| proven(d, *local, &refinements))
                    .ok_or_else(|| IrError {
                        operation: Some(at),
                        message: "cannot infer variable read before assignment".into(),
                    })?;
                if *value_type != ty {
                    *value_type = ty;
                    changed = true;
                }
            }
            transfer(&mut current, &op.operation, &values);
        }
        if !changed {
            return Ok(());
        }
    }
    Err(IrError {
        operation: None,
        message: "variable type inference did not converge".into(),
    })
}
