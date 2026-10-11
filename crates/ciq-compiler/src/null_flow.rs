//! Non-null local facts established by explicit null-check branches.
use crate::{
    BlockId, ComparisonOp, LocalId, LocatedOperation, Operation, ValueId, control_flow::Structure,
};
use std::collections::{BTreeMap, BTreeSet};

/// Compute non-null locals at block entries. Facts are intersected across all
/// incoming edges, invalidated by every write, and established only on the false
/// edge of a direct local-read == null test. Entry has no external guarantees.
pub(crate) fn local_inputs(
    structure: &Structure,
    operations: &[LocatedOperation],
) -> BTreeMap<BlockId, BTreeSet<LocalId>> {
    if !structure.explicit {
        return BTreeMap::new();
    }
    let mut definitions = BTreeMap::new();
    let mut kills: BTreeMap<BlockId, BTreeSet<LocalId>> = structure
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
            | Operation::Compare { result, .. } => {
                definitions.insert(result, at);
            }
            Operation::LocalInitialize { local, .. } | Operation::LocalAssign { local, .. } => {
                kills.get_mut(&block).unwrap().insert(local);
                universe.insert(local);
            }
            Operation::Branch {
                condition,
                on_true,
                on_false,
            } => {
                let guard = guard_local(condition, at, structure, operations, &definitions);
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
                incoming.retain(|local| outputs[&(*predecessor, *block)].contains(local));
            }
            changed |= inputs[block] != incoming;
            inputs.insert(*block, incoming.clone());
            incoming.retain(|local| !kills[block].contains(local));
            for (&edge, &guard) in edges.range((*block, BlockId(0))..=(*block, BlockId(u32::MAX))) {
                let mut outgoing = incoming.clone();
                if let Some(local) = guard {
                    outgoing.insert(local);
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

// Only direct comparisons prove a local fact. A write after the checked read
// would make the branch describe an older value and cannot narrow the local.
fn guard_local(
    condition: ValueId,
    branch: usize,
    structure: &Structure,
    operations: &[LocatedOperation],
    definitions: &BTreeMap<ValueId, usize>,
) -> Option<LocalId> {
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
        (Operation::LocalRead { .. }, Operation::Null { .. }) => left,
        (Operation::Null { .. }, Operation::LocalRead { .. }) => right,
        _ => return None,
    };
    let Operation::LocalRead { local, .. } = operations[read].operation else {
        unreachable!()
    };
    if read >= comparison || comparison >= branch
        || structure.blocks[read] != structure.blocks[branch]
        || structure.blocks[comparison] != structure.blocks[branch]
        || operations[read + 1..branch].iter().any(|op| matches!(op.operation,
            Operation::LocalAssign { local: changed, .. } | Operation::LocalInitialize { local: changed, .. } if changed == local)) {
        return None;
    }
    Some(local)
}
