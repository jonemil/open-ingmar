//! Block structure, reachability, and definite local initialization for semantic IR.
use crate::{BlockId, IrError, LocalId, LocatedOperation, Operation};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) struct Structure {
    pub explicit: bool,
    pub blocks: Vec<BlockId>,
    pub predecessors: BTreeMap<BlockId, Vec<BlockId>>,
}

/// Validate terminated blocks and reachable targets, returning block membership
/// and predecessor lists for value and definite-initialization checks.
pub(crate) fn validate(operations: &[LocatedOperation]) -> Result<Structure, IrError> {
    let explicit = operations.iter().any(|o| {
        matches!(
            o.operation,
            Operation::Block { .. } | Operation::Branch { .. } | Operation::Jump { .. }
        )
    });
    if !explicit {
        return Ok(Structure {
            explicit: false,
            blocks: vec![BlockId(0); operations.len()],
            predecessors: BTreeMap::new(),
        });
    }
    let fail = |index, message: &str| IrError {
        operation: Some(index),
        message: message.into(),
    };
    let mut positions = BTreeMap::new();
    let mut blocks = Vec::new();
    let mut current = None;
    let mut terminated = true;
    let mut edges = Vec::new();
    for (index, located) in operations.iter().enumerate() {
        if let Operation::Block { block } = located.operation {
            if !terminated {
                return Err(fail(index, "block requires an explicit terminator"));
            }
            if positions.insert(block, index).is_some() {
                return Err(fail(index, "duplicate block identity"));
            }
            current = Some(block);
            terminated = false;
        } else {
            let block = current
                .ok_or_else(|| fail(index, "control-flow function must start with a block"))?;
            if terminated {
                return Err(fail(index, "operation after block terminator"));
            }
            match located.operation {
                Operation::Branch {
                    on_true, on_false, ..
                } => {
                    edges.extend([(index, block, on_true), (index, block, on_false)]);
                    terminated = true;
                }
                Operation::Jump { target } => {
                    edges.push((index, block, target));
                    terminated = true;
                }
                Operation::Return { .. } | Operation::ReturnVoid => terminated = true,
                _ => (),
            }
        }
        blocks.push(current.unwrap());
    }
    if !terminated {
        return Err(fail(
            operations.len() - 1,
            "block requires an explicit terminator",
        ));
    }
    let mut predecessors: BTreeMap<_, Vec<_>> =
        positions.keys().map(|b| (*b, Vec::new())).collect();
    for (index, from, to) in edges {
        if !positions.contains_key(&to) {
            return Err(fail(index, "undefined block target"));
        }
        predecessors.get_mut(&to).unwrap().push(from);
    }
    let entry = blocks[0];
    let mut reachable = BTreeSet::from([entry]);
    let mut pending = vec![entry];
    let mut successors: BTreeMap<BlockId, Vec<BlockId>> = BTreeMap::new();
    for (to, froms) in &predecessors {
        for from in froms {
            successors.entry(*from).or_default().push(*to);
        }
    }
    while let Some(block) = pending.pop() {
        for target in successors.get(&block).into_iter().flatten() {
            if reachable.insert(*target) {
                pending.push(*target);
            }
        }
    }
    for (block, at) in positions {
        if !reachable.contains(&block) {
            return Err(fail(at, "unreachable block"));
        }
    }
    Ok(Structure {
        explicit,
        blocks,
        predecessors,
    })
}

/// Compute the locals definitely initialized at each block entry. The greatest
/// fixed point intersects predecessor outputs; entry also has an empty external
/// predecessor, so a backedge cannot establish first-entry initialization.
pub(crate) fn local_inputs(
    structure: &Structure,
    operations: &[LocatedOperation],
) -> BTreeMap<BlockId, BTreeSet<LocalId>> {
    if !structure.explicit {
        return BTreeMap::new();
    }
    let mut generated: BTreeMap<_, BTreeSet<_>> = structure
        .predecessors
        .keys()
        .map(|b| (*b, BTreeSet::new()))
        .collect();
    let mut universe = BTreeSet::new();
    for (index, located) in operations.iter().enumerate() {
        if let Operation::LocalInitialize { local, .. } = located.operation {
            universe.insert(local);
            generated
                .get_mut(&structure.blocks[index])
                .unwrap()
                .insert(local);
        }
    }
    let entry = structure.blocks[0];
    let mut inputs: BTreeMap<_, _> = generated.keys().map(|b| (*b, universe.clone())).collect();
    let mut outputs = inputs.clone();
    loop {
        let mut changed = false;
        for (block, predecessors) in &structure.predecessors {
            let mut available = if *block == entry {
                BTreeSet::new()
            } else {
                outputs[&predecessors[0]].clone()
            };
            for predecessor in predecessors.iter().skip(1) {
                available.retain(|local| outputs[predecessor].contains(local));
            }
            let mut outgoing = available.clone();
            outgoing.extend(&generated[block]);
            changed |= inputs[block] != available || outputs[block] != outgoing;
            inputs.insert(*block, available);
            outputs.insert(*block, outgoing);
        }
        if !changed {
            return inputs;
        }
    }
}
