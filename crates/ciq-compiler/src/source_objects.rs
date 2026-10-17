//! Exact source-class identity through construction, signatures, and locals.
//! Reconstruct this from IR; unit checking resolves signature/callee claims.
use crate::{LocalId, Operation, ValueId};
use std::collections::BTreeMap;

#[derive(Default)]
pub(crate) struct Origins {
    values: BTreeMap<ValueId, String>,
    locals: BTreeMap<LocalId, Option<String>>,
    parameters: Vec<crate::ParameterType>,
}

impl Origins {
    pub(crate) fn new(parameters: Vec<crate::ParameterType>) -> Self {
        Self {
            parameters,
            ..Default::default()
        }
    }
    /// Track exact source classes through typed parameters, calls, and locals,
    /// rejecting writes that invalidate an established source-class contract.
    pub(crate) fn observe(&mut self, operation: &Operation) -> Result<(), &'static str> {
        if let Operation::ObjectCall {
            receiver, class, ..
        }
        | Operation::ObjectCallStatement {
            receiver, class, ..
        } = operation
        {
            if self.values.get(receiver) != Some(class) {
                return Err("object call class does not match the receiver's source contract");
            }
        }
        match operation {
            Operation::ParameterRead { result, index } => {
                if let Some(crate::ParameterType::SourceObject(class)) = self.parameters.get(*index)
                {
                    self.values.insert(*result, class.clone());
                }
            }
            Operation::SelfFieldRead {
                result,
                result_type: crate::ReturnType::SourceObject(class),
                ..
            }
            | Operation::ModuleCall {
                result,
                result_type: crate::ReturnType::SourceObject(class),
                ..
            }
            | Operation::SelfCall {
                result,
                result_type: crate::ReturnType::SourceObject(class),
                ..
            }
            | Operation::ObjectCall {
                result,
                result_type: crate::ReturnType::SourceObject(class),
                ..
            } => {
                self.values.insert(*result, class.clone());
            }
            Operation::SourceNew { result, class, .. } => {
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
            Operation::LocalRead { result, local } | Operation::LocalTake { result, local } => {
                if let Some(Some(class)) = self.locals.get(local) {
                    self.values.insert(*result, class.clone());
                }
            }
            _ => {}
        }
        Ok(())
    }
    /// Project a source comparator argument to its interface type for call
    /// checking. Unit validation must still resolve its class and required members;
    /// erased native references cannot claim that obligation.
    pub(crate) fn native_arguments(
        &self,
        receiver: &crate::api::Type,
        member: &str,
        arguments: impl IntoIterator<Item = (ValueId, crate::api::Type)>,
    ) -> Vec<crate::api::Type> {
        let arguments = arguments.into_iter().collect::<Vec<_>>();
        let comparator = matches!(receiver, crate::api::Type::Array(_))
            && member == "sort"
            && arguments.len() == 1;
        arguments
            .into_iter()
            .map(|(value, ty)| {
                if comparator && self.class(value).is_some() {
                    crate::api::Type::named("Toybox.Lang.Comparator")
                } else {
                    ty
                }
            })
            .collect()
    }

    /// Return an exact source-class identity, if the value retains one.
    pub(crate) fn class(&self, value: ValueId) -> Option<&str> {
        self.values.get(&value).map(String::as_str)
    }
    /// Return the invariant source class established by local initialization.
    pub(crate) fn local_class(&self, local: LocalId) -> Option<&str> {
        self.locals.get(&local).and_then(|class| class.as_deref())
    }
}

/// Infer a receiver's source class from typed parameters and emitted operations.
/// Candidate source lowering leaves any invalid assignment to final validation.
pub(crate) fn class(
    operations: &[crate::LocatedOperation],
    value: ValueId,
    parameters: Vec<crate::ParameterType>,
) -> Option<String> {
    let mut origins = Origins::new(parameters);
    for located in operations {
        let _ = origins.observe(&located.operation);
    }
    origins.class(value).map(str::to_owned)
}
