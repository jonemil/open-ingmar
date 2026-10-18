//! Storage refinements established by null and runtime-class branches.
use crate::{
    BlockId, ComparisonOp, LocalId, LocatedOperation, Operation, ValueId, control_flow::Structure,
};
use std::collections::{BTreeMap, BTreeSet};

/// Storage identity used by branch proofs, independent of VM frame slots.
/// Parameters have no write operation; writes invalidate only the named local.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Place {
    Local(LocalId),
    Parameter(usize),
}

/// Runtime class identities used by branch facts, without text serialization.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ClassTarget {
    Native(String),
    Source(crate::SourceClass),
}

/// A branch fact about a storage location, independent of its declared type.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Fact {
    NonNull(Place),
    Class(Place, ClassTarget),
}
impl Fact {
    /// Return the local or parameter described by this fact.
    pub(crate) fn place(&self) -> Place {
        match self {
            Self::NonNull(place) | Self::Class(place, _) => *place,
        }
    }
}

/// Return an exact source identity established for this storage by a class test.
/// The fact retains its native projection independently of its source name.
pub(crate) fn source_class(place: Place, facts: &BTreeSet<Fact>) -> Option<crate::SourceClass> {
    facts.iter().find_map(|fact| match fact {
        Fact::Class(at, ClassTarget::Source(class)) if *at == place => Some(class.clone()),
        _ => None,
    })
}

/// Project a proven source/native class read to its value type, or remove Null
/// from a supported nullable read. Exact source identity uses the same facts;
/// runtime class tests do not infer container element contracts.
pub(crate) fn refine(
    ty: crate::ValueType,
    place: Place,
    facts: &BTreeSet<Fact>,
) -> crate::ValueType {
    if let Some(class) = source_class(place, facts) {
        return crate::ValueType::from_api_type(&crate::api::Type::named(class.base.api_name()))
            .expect("source class base has a value projection");
    }
    let non_null = facts.contains(&Fact::NonNull(place));
    let ty = if non_null { ty.without_null() } else { ty };
    if ty == crate::ValueType::LangObject {
        for fact in facts {
            if let Fact::Class(at, ClassTarget::Native(class)) = fact {
                if *at == place && class != "Toybox.Lang.Object" {
                    if let Some(ty) =
                        crate::ValueType::from_api_type(&crate::api::Type::named(class))
                    {
                        return ty;
                    }
                }
            }
        }
    }
    ty
}

/// Enter a block's facts or invalidate writes while replaying candidate/unit IR.
/// Validation separately checks the complete graph and its storage operations.
pub(crate) fn update(
    facts: &mut BTreeSet<Fact>,
    inputs: &BTreeMap<BlockId, BTreeSet<Fact>>,
    operation: &Operation,
) {
    match operation {
        Operation::Block { block } => *facts = inputs.get(block).cloned().unwrap_or_default(),
        Operation::LocalInitialize { local, .. } | Operation::LocalAssign { local, .. } => {
            facts.retain(|fact| fact.place() != Place::Local(*local))
        }
        _ => (),
    }
}

/// Compute refinements for a source prefix whose final block may still be open.
/// These are candidate types only; final IR checking uses the validated CFG and
/// recomputes all facts, including loop backedges and joins.
pub(crate) fn prefix_inputs(operations: &[LocatedOperation]) -> BTreeMap<BlockId, BTreeSet<Fact>> {
    // Nullable candidates already use their non-null alternative during source
    // selection. Only instanceof needs additional class facts before validation.
    if !operations.iter().any(|op| {
        matches!(
            op.operation,
            Operation::InstanceOf { .. } | Operation::SourceInstanceOf { .. }
        )
    }) {
        return BTreeMap::new();
    }
    let mut blocks = Vec::new();
    let mut current = BlockId(0);
    let mut predecessors: BTreeMap<BlockId, Vec<BlockId>> = BTreeMap::new();
    let mut explicit = false;
    for located in operations {
        if let Operation::Block { block } = located.operation {
            current = block;
            explicit = true;
        }
        predecessors.entry(current).or_default();
        blocks.push(current);
        let targets = match located.operation {
            Operation::Branch {
                on_true, on_false, ..
            } => vec![on_true, on_false],
            Operation::Jump { target } | Operation::JumpValue { target, .. } => vec![target],
            _ => Vec::new(),
        };
        for target in targets {
            let incoming = predecessors.entry(target).or_default();
            if !incoming.contains(&current) {
                incoming.push(current);
            }
        }
    }
    inputs(
        &Structure {
            explicit,
            blocks,
            predecessors,
        },
        operations,
    )
}

/// Compute storage refinements at block entries by intersecting incoming facts.
/// Null equality/inequality and positive instanceof edges establish facts; Not
/// reverses the tested edge. Writes invalidate that local's facts. Stored Boolean
/// conditions carry no proof, and entry has no external guarantees.
pub(crate) fn inputs(
    structure: &Structure,
    operations: &[LocatedOperation],
) -> BTreeMap<BlockId, BTreeSet<Fact>> {
    if !structure.explicit {
        return BTreeMap::new();
    }
    let mut definitions = BTreeMap::new();
    let mut kills: BTreeMap<BlockId, BTreeSet<Place>> = structure
        .predecessors
        .keys()
        .map(|block| (*block, BTreeSet::new()))
        .collect();
    let mut edges = BTreeMap::new();
    let mut universe = BTreeSet::new();
    for (at, located) in operations.iter().enumerate() {
        let block = structure.blocks[at];
        match located.operation {
            Operation::Null { result }
            | Operation::LocalRead { result, .. }
            | Operation::LocalTake { result, .. }
            | Operation::ParameterRead { result, .. }
            | Operation::Not { result, .. }
            | Operation::SourceInstanceOf { result, .. }
            | Operation::InstanceOf { result, .. }
            | Operation::Compare { result, .. } => {
                definitions.insert(result, at);
                if let Some(place) = read_place(&located.operation) {
                    universe.insert(Fact::NonNull(place));
                }
            }
            Operation::LocalInitialize { local, .. } | Operation::LocalAssign { local, .. } => {
                kills.get_mut(&block).unwrap().insert(Place::Local(local));
                universe.insert(Fact::NonNull(Place::Local(local)));
            }
            Operation::Branch {
                condition,
                on_true,
                on_false,
            } => {
                let guard = guard_place(condition, at, structure, operations, &definitions);
                if let Some((place, _, Some(class))) = &guard {
                    universe.insert(Fact::Class(*place, class.clone()));
                }
                // Both outcomes reaching one block provide no branch proof.
                for (target, outcome) in [(on_true, true), (on_false, false)] {
                    edges.insert(
                        (block, target),
                        guard
                            .as_ref()
                            .filter(|(_, edge, _)| on_true != on_false && outcome == *edge)
                            .map(|(place, _, class)| {
                                let mut facts = BTreeSet::from([Fact::NonNull(*place)]);
                                if let Some(class) = class {
                                    facts.insert(Fact::Class(*place, class.clone()));
                                }
                                facts
                            })
                            .unwrap_or_default(),
                    );
                }
            }
            Operation::Jump { target } | Operation::JumpValue { target, .. } => {
                edges.insert((block, target), BTreeSet::new());
            }
            _ => (),
        }
    }
    let entry = structure.blocks[0];
    let mut inputs: BTreeMap<_, _> = structure
        .predecessors
        .keys()
        .map(|block| (*block, universe.clone()))
        .collect();
    let mut outputs: BTreeMap<_, _> = edges.keys().map(|edge| (*edge, universe.clone())).collect();
    loop {
        let mut changed = false;
        for (block, predecessors) in &structure.predecessors {
            let mut incoming = if *block == entry || predecessors.is_empty() {
                BTreeSet::new()
            } else {
                outputs[&(predecessors[0], *block)].clone()
            };
            for predecessor in predecessors.iter().skip(1) {
                incoming.retain(|place| outputs[&(*predecessor, *block)].contains(place));
            }
            changed |= inputs[block] != incoming;
            inputs.insert(*block, incoming.clone());
            incoming.retain(|fact| !kills[block].contains(&fact.place()));
            for (&edge, guard) in edges.range((*block, BlockId(0))..=(*block, BlockId(u32::MAX))) {
                let mut outgoing = incoming.clone();
                outgoing.extend(guard.iter().cloned());
                changed |= outputs[&edge] != outgoing;
                outputs.insert(edge, outgoing);
            }
        }
        if !changed {
            return inputs;
        }
    }
}

pub(crate) fn read_place(operation: &Operation) -> Option<Place> {
    match *operation {
        Operation::LocalRead { local, .. } => Some(Place::Local(local)),
        Operation::ParameterRead { index, .. } => Some(Place::Parameter(index)),
        _ => None,
    }
}

// A local write after a checked read makes the comparison refer to an older
// value. Parameter reads refer to immutable argument slots.
fn guard_place(
    condition: ValueId,
    branch: usize,
    structure: &Structure,
    operations: &[LocatedOperation],
    definitions: &BTreeMap<ValueId, usize>,
) -> Option<(Place, bool, Option<ClassTarget>)> {
    let mut comparison = *definitions.get(&condition)?;
    let mut non_null = false;
    let mut consumer = branch;
    while let Operation::Not { operand, .. } = operations[comparison].operation {
        if comparison >= consumer || structure.blocks[comparison] != structure.blocks[branch] {
            return None;
        }
        consumer = comparison;
        comparison = *definitions.get(&operand)?;
        non_null = !non_null;
    }
    let (read, class) = match &operations[comparison].operation {
        Operation::SourceInstanceOf { operand, class, .. } => {
            non_null = !non_null;
            (
                *definitions.get(operand)?,
                Some(ClassTarget::Source(class.clone())),
            )
        }
        Operation::InstanceOf { operand, class, .. } => {
            non_null = !non_null;
            (
                *definitions.get(operand)?,
                Some(ClassTarget::Native(class.clone())),
            )
        }
        Operation::Compare {
            kind, left, right, ..
        } if kind.is_equality() => {
            if *kind == ComparisonOp::NotEqual {
                non_null = !non_null;
            }
            let (&left, &right) = (definitions.get(left)?, definitions.get(right)?);
            let read = match (&operations[left].operation, &operations[right].operation) {
                (read, Operation::Null { .. }) if read_place(read).is_some() => left,
                (Operation::Null { .. }, read) if read_place(read).is_some() => right,
                _ => return None,
            };
            (read, None)
        }
        _ => return None,
    };
    let place = read_place(&operations[read].operation)?;
    if read >= comparison || comparison >= consumer
        || structure.blocks[read] != structure.blocks[branch]
        || structure.blocks[comparison] != structure.blocks[branch]
        || operations[read + 1..branch].iter().any(|op| matches!(op.operation,
            Operation::LocalAssign { local: changed, .. } | Operation::LocalInitialize { local: changed, .. } if place == Place::Local(changed))) {
        return None;
    }
    Some((place, non_null, class))
}
