//! Block structure, reachability, and definite local initialization for semantic IR.
use crate::{BlockId, IrError, LocalId, LocatedOperation, Operation};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) struct Structure {
    pub explicit: bool,
    pub blocks: Vec<BlockId>,
    pub predecessors: BTreeMap<BlockId, Vec<BlockId>>,
    pub roots: Vec<BlockId>,
    components: BTreeMap<BlockId, usize>,
}

impl Structure {
    /// Partition entry-reachable code and disconnected validation components.
    /// Edges into an earlier component cannot contribute execution proofs there.
    fn new(
        explicit: bool,
        blocks: Vec<BlockId>,
        mut predecessors: BTreeMap<BlockId, Vec<BlockId>>,
    ) -> Self {
        let mut successors = BTreeMap::<BlockId, Vec<BlockId>>::new();
        for (to, froms) in &predecessors {
            for from in froms {
                successors.entry(*from).or_default().push(*to);
            }
        }
        let mut roots = Vec::new();
        let mut components = BTreeMap::new();
        for &root in blocks.iter().chain(predecessors.keys()) {
            if components.contains_key(&root) {
                continue;
            }
            let component = roots.len();
            roots.push(root);
            let mut pending = vec![root];
            while let Some(block) = pending.pop() {
                if components.contains_key(&block) {
                    continue;
                }
                components.insert(block, component);
                pending.extend(successors.get(&block).into_iter().flatten().copied());
            }
        }
        for (to, froms) in &mut predecessors {
            froms.retain(|from| components[from] == components[to]);
        }
        Self {
            explicit,
            blocks,
            predecessors,
            roots,
            components,
        }
    }

    pub(crate) fn reachable(&self, block: BlockId) -> bool {
        self.components.get(&block) == Some(&0)
    }

    pub(crate) fn same_component(&self, left: BlockId, right: BlockId) -> bool {
        self.components.get(&left) == self.components.get(&right)
    }
}

/// Describe a source prefix whose last block or branch targets may still be open.
/// This supplies candidate proofs only; final validation checks the full graph.
pub(crate) fn prefix(operations: &[LocatedOperation]) -> Structure {
    let mut blocks = Vec::new();
    let mut current = BlockId(0);
    let mut predecessors = BTreeMap::<BlockId, Vec<BlockId>>::new();
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
    Structure::new(explicit, blocks, predecessors)
}

/// Validate terminated blocks and declared targets, returning checking components
/// and predecessor lists for value and definite-initialization checks.
pub(crate) fn validate(operations: &[LocatedOperation]) -> Result<Structure, IrError> {
    let explicit = operations.iter().any(|o| {
        matches!(
            o.operation,
            Operation::Block { .. }
                | Operation::Branch { .. }
                | Operation::Jump { .. }
                | Operation::JumpValue { .. }
        )
    });
    if !explicit {
        return Ok(Structure::new(
            false,
            vec![BlockId(0); operations.len()],
            BTreeMap::new(),
        ));
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
                Operation::Jump { target } | Operation::JumpValue { target, .. } => {
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
    Ok(Structure::new(explicit, blocks, predecessors))
}

/// Compute the locals definitely initialized at each block entry. The greatest
/// fixed point intersects predecessor outputs; entry also has an empty external
/// predecessor, so a backedge cannot establish first-entry initialization.
/// Disconnected roots may read earlier declarations without establishing facts
/// for executable code; all operand and type checks still apply.
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
            } else if structure.roots.contains(block) {
                // Unexecuted header clauses can refer to already declared locals;
                // ordinary operand/name/type validation still checks their uses.
                universe.clone()
            } else {
                outputs[&predecessors[0]].clone()
            };
            for predecessor in predecessors.iter().skip(1) {
                available.retain(|local| outputs[predecessor].contains(local));
            }
            let mut outgoing = available.clone();
            for (at, op) in operations.iter().enumerate() {
                if structure.blocks[at] != *block {
                    continue;
                }
                match op.operation {
                    Operation::LocalInitialize { local, .. } => {
                        outgoing.insert(local);
                    }
                    Operation::LocalTake { local, .. } => {
                        outgoing.remove(&local);
                    }
                    _ => {}
                }
            }
            changed |= inputs[block] != available || outputs[block] != outgoing;
            inputs.insert(*block, available);
            outputs.insert(*block, outgoing);
        }
        if !changed {
            return inputs;
        }
    }
}
