//! Non-null storage facts established by explicit null-check branches.
use crate::{
    BlockId, ComparisonOp, LocalId, LocatedOperation, Operation, ValueId, control_flow::Structure,
};
use std::collections::{BTreeMap, BTreeSet};

/// Storage identity used by null-check proofs, independent of VM frame slots.
/// Parameters have no write operation; writes invalidate only the named local.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Place {
    Local(LocalId),
    Parameter(usize),
}

/// Compute non-null locals and parameters at block entries. Facts intersect
/// across incoming edges; local writes invalidate the corresponding fact.
/// Direct read == null tests establish a fact on the false edge only.
/// Entry has no external guarantees.
pub(crate) fn inputs(
    structure: &Structure,
    operations: &[LocatedOperation],
) -> BTreeMap<BlockId, BTreeSet<Place>> {
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
            | Operation::ParameterRead { result, .. }
            | Operation::Compare { result, .. } => {
                definitions.insert(result, at);
                if let Some(place) = read_place(&located.operation) {
                    universe.insert(place);
                }
            }
            Operation::LocalInitialize { local, .. } | Operation::LocalAssign { local, .. } => {
                kills.get_mut(&block).unwrap().insert(Place::Local(local));
                universe.insert(Place::Local(local));
            }
            Operation::Branch {
                condition,
                on_true,
                on_false,
            } => {
                let guard = guard_place(condition, at, structure, operations, &definitions);
                edges.insert((block, on_true), None);
                // Both outcomes reaching one block provide no non-null proof.
                edges.insert(
                    (block, on_false),
                    if on_true != on_false { guard } else { None },
                );
            }
            Operation::Jump { target } => {
                edges.insert((block, target), None);
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
            let mut incoming = if *block == entry {
                BTreeSet::new()
            } else {
                outputs[&(predecessors[0], *block)].clone()
            };
            for predecessor in predecessors.iter().skip(1) {
                incoming.retain(|place| outputs[&(*predecessor, *block)].contains(place));
            }
            changed |= inputs[block] != incoming;
            inputs.insert(*block, incoming.clone());
            incoming.retain(|place| !kills[block].contains(place));
            for (&edge, &guard) in edges.range((*block, BlockId(0))..=(*block, BlockId(u32::MAX))) {
                let mut outgoing = incoming.clone();
                if let Some(place) = guard {
                    outgoing.insert(place);
                }
                changed |= outputs[&edge] != outgoing;
                outputs.insert(edge, outgoing);
            }
        }
        if !changed {
            return inputs;
        }
    }
}

fn read_place(operation: &Operation) -> Option<Place> {
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
) -> Option<Place> {
    let &comparison = definitions.get(&condition)?;
    let Operation::Compare {
        kind: ComparisonOp::Equal,
        left,
        right,
        ..
    } = operations[comparison].operation
    else {
        return None;
    };
    let (&left, &right) = (definitions.get(&left)?, definitions.get(&right)?);
    let read = match (&operations[left].operation, &operations[right].operation) {
        (read, Operation::Null { .. }) if read_place(read).is_some() => left,
        (Operation::Null { .. }, read) if read_place(read).is_some() => right,
        _ => return None,
    };
    let place = read_place(&operations[read].operation)?;
    if read >= comparison || comparison >= branch
        || structure.blocks[read] != structure.blocks[branch]
        || structure.blocks[comparison] != structure.blocks[branch]
        || operations[read + 1..branch].iter().any(|op| matches!(op.operation,
            Operation::LocalAssign { local: changed, .. } | Operation::LocalInitialize { local: changed, .. } if place == Place::Local(changed))) {
        return None;
    }
    Some(place)
}
