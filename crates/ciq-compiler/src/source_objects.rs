//! Constructor provenance for bounded source-object calls.
//! Reconstruct this from IR rather than trusting a source-only class annotation.
use crate::{LocalId, Operation, ValueId};
use std::collections::BTreeMap;

#[derive(Default)]
pub(crate) struct Origins {
    values: BTreeMap<ValueId, String>,
    locals: BTreeMap<LocalId, Option<String>>,
}

impl Origins {
    /// Track exact constructor classes through locals, rejecting writes that
    /// would invalidate a local's established source-class contract.
    pub(crate) fn observe(&mut self, operation: &Operation) -> Result<(), &'static str> {
        match operation {
            Operation::SourceNew { result, class } => {
                self.values.insert(*result, class.clone());
            }
            Operation::LocalInitialize { local, value } => {
                self.locals.insert(*local, self.values.get(value).cloned());
            }
            Operation::LocalAssign { local, value } => {
                if let Some(Some(class)) = self.locals.get(local) {
                    if self.values.get(value) != Some(class) {
                        return Err(
                            "source-object local assignment must preserve its constructor class",
                        );
                    }
                }
            }
            Operation::LocalRead { result, local } => {
                if let Some(Some(class)) = self.locals.get(local) {
                    self.values.insert(*result, class.clone());
                }
            }
            Operation::ObjectCall {
                receiver, class, ..
            }
            | Operation::ObjectCallStatement {
                receiver, class, ..
            } if self.values.get(receiver) != Some(class) => {
                return Err("object call class does not match proven receiver construction");
            }
            _ => {}
        }
        Ok(())
    }
    /// Return an exact source constructor identity, if the value retains one.
    pub(crate) fn class(&self, value: ValueId) -> Option<&str> {
        self.values.get(&value).map(String::as_str)
    }
    /// Return the invariant constructor class established by local initialization.
    pub(crate) fn local_class(&self, local: LocalId) -> Option<&str> {
        self.locals.get(&local).and_then(|class| class.as_deref())
    }
}

/// Infer a receiver's constructor class from the already emitted operations.
/// Candidate source lowering leaves any invalid assignment to final validation.
pub(crate) fn class(operations: &[crate::LocatedOperation], value: ValueId) -> Option<String> {
    let mut origins = Origins::default();
    for located in operations {
        let _ = origins.observe(&located.operation);
    }
    origins.class(value).map(str::to_owned)
}
