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
    /// Track exact classes through signatures, assertions, calls, and locals.
    /// Apply the caller's current CFG facts to reads, rejecting writes that
    /// invalidate an established source-class storage contract.
    pub(crate) fn observe(
        &mut self,
        operation: &Operation,
        facts: &std::collections::BTreeSet<crate::type_flow::Fact>,
    ) -> Result<(), &'static str> {
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
            Operation::SourceCast { result, class, .. } => {
                self.values.insert(*result, class.name.clone());
            }
            Operation::ParameterRead { result, index } => {
                if let Some(crate::ParameterType::SourceObject(class)) = self.parameters.get(*index)
                {
                    self.values.insert(*result, class.name.clone());
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
                self.values.insert(*result, class.name.clone());
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
        let place = if let Operation::LocalTake { local, .. } = operation {
            Some(crate::type_flow::Place::Local(*local))
        } else {
            crate::type_flow::read_place(operation)
        };
        if let Some(place) = place {
            if let Some(class) = crate::type_flow::source_class(place, facts) {
                let result = match operation {
                    Operation::ParameterRead { result, .. }
                    | Operation::LocalRead { result, .. }
                    | Operation::LocalTake { result, .. } => *result,
                    _ => unreachable!(),
                };
                self.values.insert(result, class.name);
            }
        }
        Ok(())
    }
    /// Report a source witness that requires a unit-level interface proof.
    pub(crate) fn deferred_interface(&self, expected: &crate::api::Type, value: ValueId) -> bool {
        self.class(value).is_some()
            && matches!(
                crate::ValueType::from_api_type(expected),
                Some(crate::ValueType::Interface(_) | crate::ValueType::NullableInterface(_))
            )
    }

    /// Project exact source arguments to non-null interface types for call
    /// checking. Unit validation must still resolve their classes and required
    /// members; erased native references cannot claim that obligation.
    pub(crate) fn native_arguments(
        &self,
        receiver: &crate::api::Type,
        member: &str,
        arguments: impl IntoIterator<Item = (ValueId, crate::api::Type)>,
    ) -> Vec<crate::api::Type> {
        let arguments = arguments.into_iter().collect::<Vec<_>>();
        let parameters = crate::native::value_parameters(receiver, member).unwrap_or_default();
        arguments
            .into_iter()
            .enumerate()
            .map(|(index, (value, ty))| {
                if let Some(expected) = parameters
                    .get(index)
                    .filter(|expected| self.deferred_interface(expected, value))
                {
                    // An exact source instance cannot be Null, even when the
                    // consumer also permits Null to select default behavior.
                    match crate::ValueType::from_api_type(expected) {
                        Some(
                            crate::ValueType::Interface(interface)
                            | crate::ValueType::NullableInterface(interface),
                        ) => interface.api_type(false),
                        _ => unreachable!("deferred interface argument"),
                    }
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
    let inputs = crate::type_flow::prefix_inputs(operations);
    let mut facts = Default::default();
    for located in operations {
        crate::type_flow::update(&mut facts, &inputs, &located.operation);
        let _ = origins.observe(&located.operation, &facts);
    }
    origins.class(value).map(str::to_owned)
}
