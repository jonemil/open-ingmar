//! Exact source-class identity through construction, signatures, and locals.
//! Reconstruct this from IR; unit checking resolves signature/callee claims.
use crate::{LocalId, Operation, ValueId};
use std::collections::BTreeMap;

/// Read an unresolved bare source-class cast target and its optional Null arm.
/// Native paths and other unions remain outside source declaration resolution.
pub(crate) fn target_contract(ty: &crate::api::Type) -> Option<(&str, bool)> {
    let (ty, nullable) = match ty {
        crate::api::Type::Union(parts) => match parts.as_slice() {
            [ty, crate::api::Type::Null] => (ty, true),
            _ => return None,
        },
        ty => (ty, false),
    };
    match ty {
        crate::api::Type::Named(name) if !name.contains('.') => Some((name, nullable)),
        _ => None,
    }
}

/// Exact class requirement carried by a value or invariant local storage.
#[derive(Clone)]
struct Contract {
    class: crate::SourceClass,
    nullable: bool,
}

#[derive(Default)]
pub(crate) struct Origins {
    values: BTreeMap<ValueId, Contract>,
    nulls: std::collections::BTreeSet<ValueId>,
    null_locals: std::collections::BTreeSet<LocalId>,
    locals: BTreeMap<LocalId, Option<Contract>>,
    variables: std::collections::BTreeSet<LocalId>,
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
            if self.class(*receiver) != Some(class.as_str()) {
                return Err("object call class does not match the receiver's source contract");
            }
        }
        match operation {
            Operation::LocalVariable { local, value } => {
                self.variables.insert(*local);
                self.locals
                    .insert(*local, value.and_then(|v| self.values.get(&v).cloned()));
                self.null_locals.remove(local);
                if value.is_some_and(|v| self.nulls.contains(&v)) {
                    self.null_locals.insert(*local);
                }
            }
            Operation::VariableRead {
                result, value_type, ..
            } => {
                if let crate::JoinType::Source { class, nullable } = value_type {
                    self.values.insert(
                        *result,
                        Contract {
                            class: class.clone(),
                            nullable: *nullable,
                        },
                    );
                } else if *value_type == crate::JoinType::Api(crate::api::Type::Null) {
                    self.nulls.insert(*result);
                }
            }
            Operation::SourceCast {
                result,
                class,
                nullable,
                ..
            } => {
                self.values.insert(
                    *result,
                    Contract {
                        class: class.clone(),
                        nullable: *nullable,
                    },
                );
            }
            Operation::ParameterRead { result, index } => {
                if let Some(
                    crate::ParameterType::SourceObject(class)
                    | crate::ParameterType::NullableSourceObject(class),
                ) = self.parameters.get(*index)
                {
                    self.values.insert(
                        *result,
                        Contract {
                            class: class.clone(),
                            nullable: matches!(
                                self.parameters[*index],
                                crate::ParameterType::NullableSourceObject(_)
                            ),
                        },
                    );
                }
            }
            Operation::SelfFieldRead {
                result,
                result_type,
                ..
            }
            | Operation::GlobalCall {
                result,
                result_type,
                ..
            }
            | Operation::ModuleCall {
                result,
                result_type,
                ..
            }
            | Operation::SelfCall {
                result,
                result_type,
                ..
            }
            | Operation::ObjectCall {
                result,
                result_type,
                ..
            } => {
                if let crate::ReturnType::SourceObject(class)
                | crate::ReturnType::NullableSourceObject(class) = result_type
                {
                    self.values.insert(
                        *result,
                        Contract {
                            class: class.clone(),
                            nullable: matches!(
                                result_type,
                                crate::ReturnType::NullableSourceObject(_)
                            ),
                        },
                    );
                }
            }
            Operation::Null { result } => {
                self.nulls.insert(*result);
            }
            Operation::BlockParameter { result, value_type }
                if *value_type == crate::JoinType::Api(crate::api::Type::Null) =>
            {
                self.nulls.insert(*result);
            }
            Operation::BlockParameter {
                result,
                value_type: crate::JoinType::Source { class, nullable },
            } => {
                self.values.insert(
                    *result,
                    Contract {
                        class: class.clone(),
                        nullable: *nullable,
                    },
                );
            }
            Operation::SourceNew {
                result,
                class,
                base,
                ..
            } => {
                self.values.insert(
                    *result,
                    Contract {
                        class: crate::SourceClass {
                            name: class.clone(),
                            base: *base,
                        },
                        nullable: false,
                    },
                );
            }
            Operation::LocalDeclare { local, value_type } => {
                let contract = match value_type {
                    crate::JoinType::Source { class, nullable } => Some(Contract {
                        class: class.clone(),
                        nullable: *nullable,
                    }),
                    _ => None,
                };
                self.locals.insert(*local, contract);
                self.null_locals.remove(local);
                if *value_type == crate::JoinType::Api(crate::api::Type::Null) {
                    self.null_locals.insert(*local);
                }
            }
            Operation::LocalInitialize { local, value } => {
                self.locals.insert(*local, self.values.get(value).cloned());
                if self.nulls.contains(value) {
                    self.null_locals.insert(*local);
                }
            }
            Operation::LocalAssign { local, value } => {
                if self.variables.contains(local) {
                    self.locals.insert(*local, self.values.get(value).cloned());
                    self.null_locals.remove(local);
                    if self.nulls.contains(value) {
                        self.null_locals.insert(*local);
                    }
                } else if let Some(Some(contract)) = self.locals.get(local) {
                    if !self.accepts(&contract.class, contract.nullable, *value) {
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
                if self.null_locals.contains(local) {
                    self.nulls.insert(*result);
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
                    | Operation::VariableRead { result, .. }
                    | Operation::LocalRead { result, .. }
                    | Operation::LocalTake { result, .. } => *result,
                    _ => unreachable!(),
                };
                self.values.insert(
                    result,
                    Contract {
                        class,
                        nullable: false,
                    },
                );
            } else if facts.contains(&crate::type_flow::Fact::NonNull(place)) {
                let result = match operation {
                    Operation::ParameterRead { result, .. }
                    | Operation::VariableRead { result, .. }
                    | Operation::LocalRead { result, .. }
                    | Operation::LocalTake { result, .. } => *result,
                    _ => unreachable!(),
                };
                if let Some(contract) = self.values.get_mut(&result) {
                    contract.nullable = false;
                }
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

    /// Return the exact source contract retained by a value, if any.
    pub(crate) fn join_contract(&self, value: ValueId) -> Option<crate::JoinType> {
        self.values
            .get(&value)
            .map(|contract| crate::JoinType::Source {
                class: contract.class.clone(),
                nullable: contract.nullable,
            })
    }

    /// Return an exact source-class identity, if the value retains one.
    pub(crate) fn class(&self, value: ValueId) -> Option<&str> {
        self.values
            .get(&value)
            .filter(|contract| !contract.nullable)
            .map(|contract| contract.class.as_str())
    }
    /// Check exact source assignability, admitting Null only for nullable storage.
    pub(crate) fn accepts(
        &self,
        class: &crate::SourceClass,
        nullable: bool,
        value: ValueId,
    ) -> bool {
        (nullable && self.nulls.contains(&value))
            || self
                .values
                .get(&value)
                .is_some_and(|actual| &actual.class == class && (nullable || !actual.nullable))
    }

    /// Return the invariant source class established by local initialization.
    pub(crate) fn local_class(&self, local: LocalId) -> Option<&str> {
        self.locals
            .get(&local)
            .and_then(|class| class.as_ref())
            .map(|contract| contract.class.as_str())
    }
    pub(crate) fn set_variable_contract(&mut self, local: LocalId, ty: &crate::JoinType) {
        self.locals.insert(
            local,
            match ty {
                crate::JoinType::Source { class, nullable } => Some(Contract {
                    class: class.clone(),
                    nullable: *nullable,
                }),
                _ => None,
            },
        );
        self.null_locals.remove(&local);
        if *ty == crate::JoinType::Api(crate::api::Type::Null) {
            self.null_locals.insert(local);
        }
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
    // Source selection uses the possible class; final validation requires a
    // non-null receiver and recomputes CFG facts from the complete body.
    origins
        .values
        .get(&value)
        .map(|contract| contract.class.name.clone())
}
