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

type Contract = crate::SourceUnion;

#[derive(Default)]
pub(crate) struct Origins {
    symbols: BTreeMap<ValueId, String>,
    integers: BTreeMap<ValueId, i32>,
    values: BTreeMap<ValueId, Contract>,
    native_values: BTreeMap<ValueId, crate::api::Type>,
    refined_values: BTreeMap<ValueId, crate::api::Type>,
    nulls: std::collections::BTreeSet<ValueId>,
    null_locals: std::collections::BTreeSet<LocalId>,
    locals: BTreeMap<LocalId, Option<Contract>>,
    variables: std::collections::BTreeSet<LocalId>,
    dynamic: std::collections::BTreeSet<ValueId>,
    parameters: Vec<crate::ParameterType>,
}

impl Origins {
    pub(crate) fn symbol(&self, value: ValueId) -> Option<&str> {
        self.symbols.get(&value).map(String::as_str)
    }
    pub(crate) fn integer(&self, value: ValueId) -> Option<i32> {
        self.integers.get(&value).copied()
    }
    pub(crate) fn new(parameters: Vec<crate::ParameterType>) -> Self {
        Self {
            parameters,
            ..Default::default()
        }
    }
    /// Record a value independently checked as Any. It may cross a typed
    /// boundary dynamically without establishing a source-class proof.
    pub(crate) fn note_dynamic(&mut self, value: ValueId) {
        self.dynamic.insert(value);
    }
    /// Record a checked native type without turning its projection into an
    /// exact class witness. Used for native alternatives at mixed boundaries.
    pub(crate) fn note_type(&mut self, value: ValueId, ty: crate::api::Type) {
        self.native_values.insert(value, ty);
    }
    /// Return a storage read's refined projection, including native-only results
    /// after the last exact class alternative has been excluded.
    pub(crate) fn refined_type(&self, value: ValueId) -> Option<crate::api::Type> {
        self.values
            .get(&value)
            .map(Contract::api_type)
            .or_else(|| self.refined_values.get(&value).cloned())
    }
    /// Track exact classes through signatures, assertions, calls, and locals.
    /// Apply the caller's current CFG facts to reads, rejecting writes that
    /// invalidate an established source-class storage contract.
    pub(crate) fn observe(
        &mut self,
        operation: &Operation,
        facts: &std::collections::BTreeSet<crate::type_flow::Fact>,
    ) -> Result<(), &'static str> {
        if let Operation::Symbol { result, name } = operation {
            self.symbols.insert(*result, name.clone());
        }
        let record_read = match operation {
            Operation::DictionaryRead {
                result,
                dictionary,
                key,
            } => Some((*result, *dictionary, *key)),
            Operation::NativeCall {
                result,
                receiver,
                member,
                arguments,
            } if member == "get" && arguments.len() == 1 => {
                Some((*result, *receiver, arguments[0]))
            }
            _ => None,
        };
        if let Some((result, receiver, key)) = record_read {
            if let Some(crate::api::Type::Options(fields)) = self.native_values.get(&receiver) {
                if let Some(contract) =
                    crate::unions::result(crate::records::read(fields, self.symbol(key)))
                        .and_then(|r| r.source_contract())
                {
                    self.values.insert(result, contract);
                }
            }
        }
        if let Operation::Integer { result, value } = operation {
            self.integers.insert(*result, *value);
        }
        if let Operation::ArrayRead {
            result,
            array,
            index,
        } = operation
        {
            if let Some(result_type) = self.native_values.get(array).and_then(|ty| {
                if let crate::api::Type::Options(fields) = ty {
                    return crate::unions::result(crate::records::read(
                        fields,
                        self.symbol(*index),
                    ))
                    .map(Ok);
                }
                crate::tuples::read_contract(ty, self.integer(*index)).or_else(|| {
                    crate::unions::index_read(ty, &crate::api::Type::named("Toybox.Lang.Number"))
                        .ok()
                        .and_then(crate::unions::result)
                        .map(Ok)
                })
            }) {
                if let Some(contract) = result_type.ok().and_then(|ty| ty.source_contract()) {
                    self.values.insert(*result, contract);
                }
            }
        }
        if let Operation::ObjectCall {
            receiver, classes, ..
        }
        | Operation::ObjectCallStatement {
            receiver, classes, ..
        } = operation
        {
            let actual = self.values.get(receiver);
            if !actual.is_some_and(|c| {
                !c.nullable
                    && c.native.is_empty()
                    && c.classes
                        .iter()
                        .map(|c| c.name.as_str())
                        .eq(classes.iter().map(String::as_str))
            }) {
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
                if let Some(contract) = value_type.source_contract() {
                    self.values.insert(*result, contract);
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
                self.values
                    .insert(*result, Contract::single(class.clone(), *nullable));
            }
            Operation::ParameterRead { result, index } => {
                if let Some(contract) = self
                    .parameters
                    .get(*index)
                    .and_then(crate::ParameterType::source_contract)
                {
                    self.values.insert(*result, contract);
                }
            }
            Operation::MemberRead {
                result,
                receiver,
                member,
                field_type,
                view_type,
            } => {
                if let Some(contract) = field_type
                    .clone()
                    .or_else(|| view_type.clone())
                    .or_else(|| {
                        self.native_values
                            .get(receiver)
                            .and_then(|ty| crate::structural::field_contract(ty, member).ok())
                    })
                    .and_then(|ty| ty.source_contract())
                {
                    self.values.insert(*result, contract);
                }
            }
            Operation::NativeCall {
                result,
                receiver,
                member,
                ..
            } => {
                if let Some(contract) = self
                    .native_values
                    .get(receiver)
                    .and_then(|ty| crate::structural::method_result(ty, member))
                    .and_then(|result| result.source_contract())
                {
                    self.values.insert(*result, contract);
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
                if let Some(contract) = result_type.source_contract() {
                    self.values.insert(*result, contract);
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
            Operation::BlockParameter { result, value_type } => {
                if let Some(contract) = value_type.source_contract() {
                    self.values.insert(*result, contract);
                }
            }
            Operation::SourceNew {
                result,
                class,
                base,
                ..
            } => {
                self.values.insert(
                    *result,
                    Contract::single(
                        crate::SourceClass {
                            name: class.clone(),
                            base: *base,
                        },
                        false,
                    ),
                );
            }
            Operation::LocalDeclare { local, value_type } => {
                let contract = value_type.source_contract();
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
                    if !self.accepts_contract(contract, *value) {
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
            let result = match operation {
                Operation::ParameterRead { result, .. }
                | Operation::VariableRead { result, .. }
                | Operation::LocalRead { result, .. }
                | Operation::LocalTake { result, .. } => *result,
                _ => unreachable!(),
            };
            if let Some(contract) = self.values.get(&result).cloned() {
                let refined = contract.refine(place, facts);
                self.values.remove(&result);
                self.refined_values.insert(result, refined.api_type());
                if let Some(contract) = refined.source_contract() {
                    self.values.insert(result, contract);
                } else if refined == crate::JoinType::Api(crate::api::Type::Null) {
                    self.nulls.insert(result);
                }
            } else if let Some(class) = crate::type_flow::source_class(place, facts) {
                self.values.insert(result, Contract::single(class, false));
            }
        }
        Ok(())
    }
    /// Report a source witness that requires a unit-level interface proof.
    pub(crate) fn deferred_interface(&self, expected: &crate::api::Type, value: ValueId) -> bool {
        if let Some(actual) = self
            .native_values
            .get(&value)
            .filter(|ty| crate::tuples::contains(ty))
        {
            return !crate::native::accepts(expected, actual)
                && crate::source_union::matches_type_with(
                    expected,
                    actual,
                    &|_, target| {
                        crate::structural::contains(target)
                            || matches!(
                                crate::ValueType::from_api_type(target),
                                Some(crate::ValueType::Interface(_))
                            )
                    },
                    &crate::native::accepts,
                );
        }
        self.values.contains_key(&value)
            && (crate::structural::contains(expected)
                || matches!(
                    crate::ValueType::from_api_type(expected),
                    Some(
                        crate::ValueType::Interface(_)
                            | crate::ValueType::NullableInterface(_)
                            | crate::ValueType::Structural(_)
                    )
                ))
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
                    // Retain Null while deferring the exact classes' structural
                    // signatures to unit validation.
                    match crate::ValueType::from_api_type(expected) {
                        Some(
                            crate::ValueType::Interface(interface)
                            | crate::ValueType::NullableInterface(interface),
                        ) => interface.api_type(self.values[&value].nullable()),
                        _ if crate::structural::contains(expected) => {
                            if self.values[&value].nullable() {
                                expected.clone().with_null()
                            } else {
                                expected.clone()
                            }
                        }
                        _ => expected.clone(),
                    }
                } else {
                    parameters
                        .get(index)
                        .map(|expected| self.element_value(expected, value, ty.clone()))
                        .unwrap_or(ty)
                }
            })
            .collect()
    }

    /// Retain an operand's source identity when checking a source element
    /// boundary. An erased native base cannot supply a missing identity.
    pub(crate) fn element_value(
        &self,
        expected: &crate::api::Type,
        value: ValueId,
        ty: crate::api::Type,
    ) -> crate::api::Type {
        if matches!(expected, crate::api::Type::SourceValue(_)) {
            if let Some(contract) = self.contract(value) {
                return crate::api::Type::SourceValue(Box::new(contract.clone()));
            }
        }
        ty
    }

    /// Return the exact source contract retained by a value, if any.
    pub(crate) fn join_contract(&self, value: ValueId) -> Option<crate::JoinType> {
        self.values.get(&value).cloned().map(Contract::join)
    }

    /// Return retained class alternatives and nullability for unit witnesses.
    pub(crate) fn contract(&self, value: ValueId) -> Option<&crate::SourceUnion> {
        self.values.get(&value)
    }
    pub(crate) fn accepts_contract(&self, expected: &crate::SourceUnion, value: ValueId) -> bool {
        self.dynamic.contains(&value)
            || self.deferred_interface(
                &crate::api::Type::SourceValue(Box::new(expected.clone())),
                value,
            )
            || (expected.nullable && self.nulls.contains(&value))
            || self
                .values
                .get(&value)
                .is_some_and(|actual| expected.accepts(actual))
            || (!self.values.contains_key(&value)
                && self
                    .native_values
                    .get(&value)
                    .is_some_and(|ty| expected.accepts_api(ty)))
    }
    /// Return the invariant source class established by local initialization.
    pub(crate) fn local_class(&self, local: LocalId) -> Option<&str> {
        self.locals
            .get(&local)
            .and_then(|class| class.as_ref())
            .filter(|contract| contract.classes.len() == 1 && contract.native.is_empty())
            .map(|contract| contract.classes[0].as_str())
    }
    pub(crate) fn local_contract(&self, local: LocalId) -> Option<&Contract> {
        self.locals.get(&local).and_then(Option::as_ref)
    }
    pub(crate) fn set_variable_contract(&mut self, local: LocalId, ty: &crate::JoinType) {
        self.locals.insert(local, ty.source_contract());
        self.null_locals.remove(&local);
        if *ty == crate::JoinType::Api(crate::api::Type::Null) {
            self.null_locals.insert(local);
        }
    }
}

/// Infer a receiver's source class from typed parameters and emitted operations.
/// Candidate source lowering leaves any invalid assignment to final validation.
pub(crate) fn contract(
    operations: &[crate::LocatedOperation],
    value: ValueId,
    parameters: Vec<crate::ParameterType>,
) -> Option<crate::SourceUnion> {
    let mut types = crate::source_types::Types::new(parameters);
    let inputs = crate::type_flow::prefix_inputs(operations);
    for located in operations {
        crate::type_flow::update(&mut types.facts, &inputs, &located.operation);
        types.observe(&located.operation);
    }
    types
        .join_contract(value)
        .and_then(|ty| ty.source_contract())
}

/// Select one candidate class, leaving non-null proof to final validation.
pub(crate) fn class(
    operations: &[crate::LocatedOperation],
    value: ValueId,
    parameters: Vec<crate::ParameterType>,
) -> Option<String> {
    contract(operations, value, parameters)
        .filter(|c| c.classes.len() == 1 && c.native.is_empty())
        .map(|c| c.classes[0].name.clone())
}
