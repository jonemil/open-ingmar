//! Structural validation for forward-only semantic control flow.
use crate::{BlockId, IrError, LocatedOperation, Operation};
use std::collections::BTreeMap;

pub(crate) struct Structure {
    pub explicit: bool,
    pub blocks: Vec<BlockId>,
    pub predecessors: BTreeMap<BlockId, Vec<BlockId>>,
}

/// Validate block boundaries and forward edges, returning block membership
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
                Operation::Return { .. } => terminated = true,
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
        let target = positions
            .get(&to)
            .ok_or_else(|| fail(index, "undefined block target"))?;
        if *target <= index {
            return Err(fail(index, "only forward control-flow edges are supported"));
        }
        predecessors.get_mut(&to).unwrap().push(from);
    }
    let entry = blocks[0];
    for (block, at) in positions {
        if block != entry && predecessors[&block].is_empty() {
            return Err(fail(at, "unreachable block"));
        }
    }
    Ok(Structure {
        explicit,
        blocks,
        predecessors,
    })
}
