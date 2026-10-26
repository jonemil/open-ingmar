//! Infer and validate the values carried by mutable, inferred frame variables.
use crate::{BlockId, IrError, JoinType, LocalId, LocatedOperation, Operation, ValueId, api::Type};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) type Domain = Vec<JoinType>;
pub(crate) type State = BTreeMap<LocalId, Domain>;

/// Track direct variable references through immutable evaluation captures.
/// Named variable aliases start independent roots. Rebinding breaks an earlier
/// capture's connection; assignments of the same reference keep it live.
#[derive(Default)]
pub(crate) struct RemovalCaptures {
    versions: BTreeMap<LocalId, usize>,
    values: BTreeMap<ValueId, (LocalId, usize)>,
    captures: BTreeMap<LocalId, (LocalId, usize)>,
}

impl RemovalCaptures {
    /// Observe one operation and return the still-active inferred local addressed
    /// by a removal. The caller independently checks its receiver/result types.
    pub(crate) fn observe(&mut self, op: &Operation) -> Option<LocalId> {
        match op {
            Operation::LocalVariable { local, .. } | Operation::LocalDeclare { local, .. } => {
                *self.versions.entry(*local).or_default() += 1;
                self.captures.remove(local);
            }
            Operation::VariableRead { result, local, .. } => {
                self.values
                    .insert(*result, (*local, *self.versions.get(local).unwrap_or(&0)));
            }
            Operation::LocalInitialize { local, value } => {
                if let Some(root) = self.values.get(value) {
                    self.captures.insert(*local, *root);
                }
            }
            Operation::LocalRead { result, local } | Operation::LocalTake { result, local } => {
                if let Some(root) = self.captures.get(local) {
                    self.values.insert(*result, *root);
                }
            }
            Operation::LocalAssign { local, value } => {
                let version = self.versions.entry(*local).or_default();
                if self.values.get(value) != Some(&(*local, *version)) {
                    *version += 1;
                }
                self.captures.remove(local);
            }
            Operation::TupleAppend { result, array, .. }
            | Operation::TupleWrite { result, array, .. } => {
                if let Some(root) = self.values.get(array) {
                    self.values.insert(*result, *root);
                }
            }
            _ => {}
        }
        let receiver = crate::tuples::removal_receiver(op)?;
        let &(local, version) = self.values.get(&receiver)?;
        (self.versions.get(&local).copied().unwrap_or(0) == version).then_some(local)
    }
}

type RemovalEffects = BTreeMap<usize, (LocalId, JoinType)>;

fn removal_effects(
    ops: &[LocatedOperation],
    values: &BTreeMap<ValueId, JoinType>,
) -> RemovalEffects {
    let mut captures = RemovalCaptures::default();
    ops.iter()
        .enumerate()
        .filter_map(|(at, op)| {
            let local = captures.observe(&op.operation)?;
            let receiver = crate::tuples::removal_receiver(&op.operation)?;
            let ty = crate::tuples::removal_contract(&values.get(&receiver)?.api_type())?;
            Some((at, (local, JoinType::Api(ty))))
        })
        .collect()
}

fn union(target: &mut Domain, source: &[JoinType]) {
    for ty in source {
        if !target.contains(ty) {
            target.push(ty.clone());
        }
    }
}

fn transfer(
    state: &mut State,
    op: &Operation,
    values: &BTreeMap<ValueId, JoinType>,
    removal: Option<&(LocalId, JoinType)>,
) {
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
    if let Some((local, ty)) = removal {
        if let Some(domain) = state.get_mut(local) {
            *domain = vec![ty.clone()];
        }
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
    let removals = removal_effects(ops, values);
    let facts = crate::type_flow::inputs(structure, ops);
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
    for (local, ty) in removals.values() {
        if let Some(domain) = roots.get_mut(local) {
            union(domain, std::slice::from_ref(ty));
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
            // Persist proven edge restrictions in the storage domain. A guard
            // before a loop must still exclude its Null input after a non-null
            // mutation kills the original guard fact on the backedge.
            if let Some(facts) = facts.get(&block).filter(|facts| !facts.is_empty()) {
                for (&local, domain) in &mut state {
                    if facts.contains(&crate::type_flow::Fact::NonNull(
                        crate::type_flow::Place::Local(local),
                    )) && domain.iter().all(|ty| {
                        matches!(ty, JoinType::Api(ty)
                            if crate::unions::alternatives(ty).iter().all(|part|
                                matches!(part, Type::TupleValue(_) | Type::Array(_) | Type::Null)))
                    }) {
                        if let Some(ty) = proven(domain, local, facts) {
                            *domain = vec![ty];
                        }
                    }
                }
            }
            let mut output = state.clone();
            for (at, op) in ops.iter().enumerate() {
                if structure.blocks[at] == block {
                    transfer(&mut output, &op.operation, values, removals.get(&at));
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
/// Preserve supported native and exact source alternatives. Unsupported private
/// domains retain the conservative Object/Null view.
fn view(domain: &[JoinType]) -> Option<JoinType> {
    let parts = domain
        .iter()
        .filter_map(|ty| {
            if let JoinType::Api(ty) = ty {
                Some(ty)
            } else {
                None
            }
        })
        .flat_map(crate::unions::alternatives)
        .collect::<Vec<_>>();
    if parts.iter().any(|ty| matches!(ty, Type::TupleValue(_)))
        && parts.iter().any(|ty| matches!(ty, Type::Array(_)))
        && domain.iter().all(|ty| matches!(ty, JoinType::Api(_)))
        && parts
            .iter()
            .all(|ty| matches!(ty, Type::TupleValue(_) | Type::Array(_) | Type::Null))
    {
        let nullable = parts.contains(&&Type::Null);
        let types = parts
            .into_iter()
            .filter(|ty| **ty != Type::Null)
            .cloned()
            .collect::<Vec<_>>();
        if let Ok(ty) = crate::tuples::array_contract(&types, &[]) {
            return Some(JoinType::Api(if nullable { ty.with_null() } else { ty }));
        }
    }
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
    let ty = view(domain)?;
    if let Some(contract) = ty.source_contract() {
        return Some(contract.refine(place, facts));
    }
    let value = crate::ValueType::from_api_type(&ty.api_type())?;
    Some(JoinType::Api(
        crate::type_flow::refine(value, place, facts).api_type(),
    ))
}

fn accepts(expected: &JoinType, actual: &JoinType) -> bool {
    if let Some(expected) = expected.source_contract() {
        return actual
            .source_contract()
            .is_some_and(|actual| expected.accepts_boundary_with(&actual, |_, _| false))
            || (actual.source_contract().is_none() && expected.accepts_api(&actual.api_type()));
    }
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
        (_, _) => crate::native::accepts_boundary(&expected.api_type(), &actual.api_type()),
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
    let removals = removal_effects(ops, &values);
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
            if crate::record_flow::has_view(&value_type.api_type()) {
                continue;
            }
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
        transfer(&mut current, &op.operation, &values, removals.get(&at));
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
    let removals = removal_effects(ops, &values);
    let incoming = inputs(&structure, ops, &values);
    let block = structure.blocks.last().copied().unwrap_or(BlockId(0));
    let mut state = incoming.get(&block).cloned().unwrap_or_default();
    for (at, op) in ops.iter().enumerate() {
        if structure.blocks[at] == block {
            transfer(&mut state, &op.operation, &values, removals.get(&at));
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
        let removals = removal_effects(ops, &values);
        let record_inputs = crate::record_flow::inputs(ops, &parameters, &types.api_types());
        let mut record_state = crate::record_flow::State::new(&parameters);
        let records_enabled = crate::record_flow::needed(ops, &parameters);
        let record_cyclic = if records_enabled {
            crate::record_flow::cyclic(ops)
        } else {
            Default::default()
        };
        let record_symbols = ops
            .iter()
            .filter_map(|op| {
                if let Operation::Symbol { result, name } = &op.operation {
                    Some((*result, name.clone()))
                } else {
                    None
                }
            })
            .collect();
        let incoming = inputs(&structure, ops, &values);
        let facts = crate::type_flow::inputs(&structure, ops);
        let mut current = State::new();
        let mut refinements = Default::default();
        for (at, op) in ops.iter_mut().enumerate() {
            if let Operation::Block { block } = op.operation {
                current = incoming[&block].clone();
                record_state = record_inputs[&block].clone();
            }
            crate::type_flow::update(&mut refinements, &facts, &op.operation);
            let record_ty = if let Operation::VariableRead { local, .. } = op.operation {
                record_state.local_type(local)
            } else {
                None
            };
            if records_enabled {
                record_state.observe(
                    &op.operation,
                    &types.api_types(),
                    &record_symbols,
                    record_cyclic.contains(&structure.blocks[at]),
                );
            }
            if let Operation::VariableRead {
                local, value_type, ..
            } = &mut op.operation
            {
                let ty = record_ty
                    .and_then(|ty| crate::ValueType::from_api_type(&ty))
                    .map(|ty| {
                        JoinType::Api(
                            crate::type_flow::refine(
                                ty,
                                crate::type_flow::Place::Local(*local),
                                &refinements,
                            )
                            .api_type(),
                        )
                    })
                    .or_else(|| {
                        current
                            .get(local)
                            .and_then(|d| proven(d, *local, &refinements))
                    })
                    .ok_or_else(|| IrError {
                        operation: Some(at),
                        message: "cannot infer variable read before assignment".into(),
                    })?;
                if *value_type != ty {
                    *value_type = ty;
                    changed = true;
                }
            }
            transfer(&mut current, &op.operation, &values, removals.get(&at));
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
