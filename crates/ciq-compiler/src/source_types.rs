//! Candidate value types and constructor classes for source call/index lowering.
//! These do not prove null safety or validity; whole-function IR validation does.
use std::collections::BTreeMap;

use crate::{
    ArrayElement, LocalId, Operation, ParameterType, ValueId, api::Type, native, operators,
};

pub(crate) struct Types {
    parameters: Vec<ParameterType>,
    values: BTreeMap<ValueId, Type>,
    locals: BTreeMap<LocalId, Type>,
    variables: std::collections::BTreeSet<LocalId>,
    builders: BTreeMap<crate::BuilderId, ArrayElement>,
    byte_arrays: std::collections::BTreeSet<crate::BuilderId>,
    dictionaries: BTreeMap<crate::BuilderId, crate::DictionaryType>,
    options: BTreeMap<crate::BuilderId, BTreeMap<String, Type>>,
    origins: crate::source_objects::Origins,
    records: crate::record_flow::State,
    symbols: BTreeMap<ValueId, String>,
    removal_captures: crate::local_flow::RemovalCaptures,
    pub(crate) facts: std::collections::BTreeSet<crate::type_flow::Fact>,
}

impl Types {
    /// Apply current candidate facts to a read's type, preserving the storage
    /// declaration and types outside the supported refinement profile.
    fn refined(&self, ty: Type, place: crate::type_flow::Place) -> Type {
        crate::ValueType::from_api_type(&ty)
            .map(|value| crate::type_flow::refine(value, place, &self.facts).api_type())
            .unwrap_or(ty)
    }

    pub(crate) fn new(parameters: Vec<ParameterType>) -> Self {
        Self {
            parameters: parameters.clone(),
            records: crate::record_flow::State::new(&parameters),
            symbols: Default::default(),
            values: BTreeMap::new(),
            locals: BTreeMap::new(),
            variables: Default::default(),
            builders: BTreeMap::new(),
            byte_arrays: Default::default(),
            dictionaries: BTreeMap::new(),
            options: Default::default(),
            origins: crate::source_objects::Origins::new(parameters),
            removal_captures: Default::default(),
            facts: Default::default(),
        }
    }

    /// Resolve an append's candidate result with exact source slot provenance.
    /// Surface unsupported shapes and metadata-budget errors before variable
    /// inference would lose the failed operation's result type.
    pub(crate) fn append_type(
        &self,
        array: ValueId,
        value: ValueId,
        spread: bool,
        positional: bool,
    ) -> Result<Type, String> {
        let receiver = self
            .values
            .get(&array)
            .ok_or("cannot resolve tuple receiver")?;
        let value_type = self
            .values
            .get(&value)
            .ok_or("cannot resolve tuple append argument")?;
        let slot = crate::tuples::value_contract(&self.origins, value, value_type)?;
        crate::tuples::append_contract(receiver, &slot, spread, positional, None)
    }

    /// Record a candidate result using shared operator and native contracts.
    /// Failed resolution leaves validation to the final IR checker.
    pub(crate) fn observe(&mut self, operation: &Operation) {
        if let Operation::Symbol { result, name } = operation {
            self.symbols.insert(*result, name.clone());
        }
        self.records
            .observe(operation, &self.values, &self.symbols, false);
        if self.records.active() {
            for (value, ty) in &mut self.values {
                *ty = self.records.value_type(*value, ty);
            }
            for (local, ty) in &mut self.locals {
                if let Some(actual) = self.records.local_type(*local) {
                    *ty = if crate::unions::alternatives(ty).contains(&Type::Null) {
                        actual.with_null()
                    } else {
                        actual
                    };
                }
            }
        }
        if let Some(local) = self.removal_captures.observe(operation) {
            if let Some(ty) = crate::tuples::removal_receiver(operation)
                .and_then(|receiver| self.values.get(&receiver))
                .and_then(crate::tuples::removal_contract)
            {
                if self.variables.contains(&local) {
                    self.locals.insert(local, ty);
                }
            }
        }
        let _ = self.origins.observe(operation, &self.facts);
        let (result, ty) = match operation {
            Operation::LocalVariable { local, value } => {
                self.variables.insert(*local);
                self.locals.remove(local);
                if let Some(ty) = value.and_then(|v| self.values.get(&v)) {
                    self.locals.insert(*local, ty.clone());
                }
                return;
            }
            Operation::VariableRead {
                result, value_type, ..
            } => (*result, Some(value_type.api_type())),
            Operation::RecordBegin {
                builder, fields, ..
            } => {
                self.options.insert(*builder, fields.clone());
                return;
            }
            Operation::DictionaryFinish { result, builder }
                if self.options.contains_key(builder) =>
            {
                (*result, Some(Type::Options(self.options[builder].clone())))
            }
            Operation::ByteArrayBegin { builder, .. } => {
                self.byte_arrays.insert(*builder);
                return;
            }
            Operation::ByteArrayConstant { result, .. }
            | Operation::ByteArrayAllocate { result, .. } => {
                (*result, Some(Type::named("Toybox.Lang.ByteArray")))
            }
            Operation::Symbol { result, .. } => (*result, Some(Type::named("Toybox.Lang.Symbol"))),
            Operation::DictionaryBegin {
                builder,
                dictionary_type,
                ..
            } => {
                self.dictionaries.insert(*builder, dictionary_type.clone());
                return;
            }
            Operation::DictionaryFinish { result, builder } => (
                *result,
                self.dictionaries.get(builder).map(|ty| ty.api_type()),
            ),
            Operation::DictionaryConstant {
                result,
                dictionary_type,
                ..
            } => (*result, Some(dictionary_type.api_type())),
            Operation::DictionaryNew {
                result,
                dictionary_type,
            } => (
                *result,
                Some(
                    dictionary_type
                        .clone()
                        .map(|ty| ty.api_type())
                        .unwrap_or_else(|| Type::named("Toybox.Lang.Dictionary")),
                ),
            ),
            Operation::BlockParameter { result, value_type } => {
                (*result, Some(value_type.api_type()))
            }
            Operation::ArrayBegin {
                builder,
                element_type,
                ..
            } => {
                self.builders.insert(*builder, element_type.clone());
                return;
            }
            Operation::ArrayFinish { result, builder } if self.byte_arrays.contains(builder) => {
                (*result, Some(Type::named("Toybox.Lang.ByteArray")))
            }
            Operation::ArrayFinish { result, builder } => (
                *result,
                self.builders
                    .get(builder)
                    .map(|e| Type::Array(Box::new(e.api_type()))),
            ),
            Operation::NativeConstant {
                result,
                module,
                member,
            } => (
                *result,
                native::constant(module, member, None)
                    .ok()
                    .and_then(|(ty, _)| crate::ValueType::from_api_type(&ty))
                    .map(|ty| ty.api_type()),
            ),
            Operation::SourceNew { result, base, .. } => (*result, base.constructed_type()),
            Operation::TupleAppend {
                result,
                array,
                value,
                spread,
                positional,
            } => (
                *result,
                self.append_type(*array, *value, *spread, *positional).ok(),
            ),
            Operation::TupleWrite {
                result,
                array,
                index,
                value,
            } => (
                *result,
                self.values.get(array).and_then(|ty| {
                    let slot = crate::tuples::value_contract(
                        &self.origins,
                        *value,
                        self.values.get(value)?,
                    )
                    .ok()?;
                    crate::tuples::write_contract(ty, self.origins.integer(*index), &slot).ok()
                }),
            ),
            Operation::TupleNew { result, elements } => {
                let slots = elements
                    .iter()
                    .map(|value| {
                        crate::tuples::value_contract(
                            &self.origins,
                            *value,
                            self.values.get(value)?,
                        )
                        .ok()
                    })
                    .collect::<Option<Vec<_>>>();
                (
                    *result,
                    slots
                        .and_then(|slots| crate::TupleType::new(slots).ok())
                        .map(|tuple| tuple.api_type()),
                )
            }
            Operation::NativeNew { result, class } => (*result, Some(Type::named(class.name()))),
            Operation::WeakReferenceNew { result } => {
                (*result, Some(Type::named("Toybox.Lang.WeakReference")))
            }
            Operation::Long { result, .. } => (*result, Some(Type::named("Toybox.Lang.Long"))),
            Operation::Double { result, .. } => (*result, Some(Type::named("Toybox.Lang.Double"))),
            Operation::Float { result, .. } => (*result, Some(Type::named("Toybox.Lang.Float"))),
            Operation::Integer { result, .. } => (*result, Some(Type::named("Toybox.Lang.Number"))),
            Operation::Char { result, .. } => (*result, Some(Type::named("Toybox.Lang.Char"))),
            Operation::Boolean { result, .. } => {
                (*result, Some(Type::named("Toybox.Lang.Boolean")))
            }
            Operation::String { result, .. } => (*result, Some(Type::named("Toybox.Lang.String"))),
            Operation::SelfValue { result, class } => (
                *result,
                Some(Type::named(
                    class
                        .as_ref()
                        .map(|class| class.base.api_name())
                        .unwrap_or("Toybox.Lang.Object"),
                )),
            ),
            Operation::Null { result } => (*result, Some(Type::Null)),
            Operation::ArrayConstant {
                result,
                element_type,
                ..
            }
            | Operation::ArrayNew {
                result,
                element_type,
            }
            | Operation::ArrayAllocate {
                result,
                element_type,
                ..
            } => (
                *result,
                Some(Type::Array(Box::new(element_type.api_type()))),
            ),
            Operation::ParameterRead { result, index } => (
                *result,
                self.parameters.get(*index).map(|ty| {
                    self.refined(ty.api_type(), crate::type_flow::Place::Parameter(*index))
                }),
            ),
            Operation::LocalRead { result, local } | Operation::LocalTake { result, local } => (
                *result,
                self.locals
                    .get(local)
                    .map(|ty| self.refined(ty.clone(), crate::type_flow::Place::Local(*local))),
            ),
            Operation::LocalDeclare { local, value_type } => {
                self.locals.insert(*local, value_type.api_type());
                self.facts
                    .retain(|fact| fact.place() != crate::type_flow::Place::Local(*local));
                return;
            }
            Operation::LocalInitialize { local, value } => {
                self.facts
                    .retain(|fact| fact.place() != crate::type_flow::Place::Local(*local));
                if let Some(ty) = self.values.get(value) {
                    self.locals.insert(*local, ty.clone());
                }
                return;
            }
            Operation::LocalAssign { local, value } => {
                if self.variables.contains(local) {
                    if let Some(ty) = self.values.get(value) {
                        self.locals.insert(*local, ty.clone());
                    }
                }
                self.facts
                    .retain(|fact| fact.place() != crate::type_flow::Place::Local(*local));
                return;
            }
            Operation::ParameterAssign { index, .. } => {
                self.facts
                    .retain(|fact| fact.place() != crate::type_flow::Place::Parameter(*index));
                return;
            }
            Operation::MemberRead {
                result,
                receiver,
                member,
                field_type,
                view_type,
            } => (
                *result,
                field_type
                    .as_ref()
                    .or(view_type.as_ref())
                    .map(crate::ReturnType::api_type)
                    .or_else(|| {
                        self.values
                            .get(receiver)
                            .and_then(|ty| crate::structural::field_type(ty, member).ok())
                    }),
            ),
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
            } => (*result, Some(result_type.api_type())),
            Operation::DictionaryRead {
                result,
                dictionary,
                key,
            } => (
                *result,
                self.values
                    .get(dictionary)
                    .zip(self.values.get(key))
                    .and_then(|(receiver, key_type)| {
                        if let Type::Options(fields) = receiver {
                            return Some(crate::records::read(fields, self.origins.symbol(*key)));
                        }
                        native::value_call(
                            receiver,
                            "get",
                            &self.origins.native_arguments(
                                receiver,
                                "get",
                                [(*key, candidate(key_type).into_owned())],
                            ),
                            None,
                        )
                        .ok()
                    }),
            ),
            Operation::ArrayRead {
                result,
                array,
                index,
            } => (
                *result,
                self.values
                    .get(array)
                    .and_then(|ty| match candidate(ty).as_ref() {
                        Type::Options(fields) => {
                            Some(crate::records::read(fields, self.origins.symbol(*index)))
                        }
                        Type::Union(_) => crate::tuples::read_contract(
                            candidate(ty).as_ref(),
                            self.origins.integer(*index),
                        )
                        .and_then(Result::ok)
                        .map(|ty| ty.api_type())
                        .or_else(|| {
                            self.values.get(index).and_then(|key| {
                                crate::unions::index_read(candidate(ty).as_ref(), key).ok()
                            })
                        }),
                        Type::Any => Some(Type::Any),
                        Type::TupleValue(tuple) => tuple
                            .read(self.origins.integer(*index))
                            .ok()
                            .map(|ty| ty.api_type()),
                        Type::Array(element) => Some((**element).clone()),
                        Type::Named(name) if name == "Toybox.Lang.ByteArray" => {
                            Some(Type::named("Toybox.Lang.Number"))
                        }
                        _ => None,
                    }),
            ),
            Operation::NativeModuleCall {
                result,
                module,
                member,
                arguments,
            } => {
                let ty = arguments
                    .iter()
                    .map(|id| self.values.get(id).cloned())
                    .collect::<Option<Vec<_>>>()
                    .and_then(|types| native::module_value(module, member, &types, None).ok());
                (*result, ty)
            }
            Operation::BindMethod {
                result, signature, ..
            } => (*result, Some(Type::Callback(signature.clone()))),
            Operation::NativeCall {
                result,
                receiver,
                member,
                arguments,
            } => {
                let ty = self.values.get(receiver).and_then(|receiver| {
                    let argument_types: Option<Vec<_>> = arguments
                        .iter()
                        .map(|id| {
                            self.values
                                .get(id)
                                .map(|ty| (*id, candidate(ty).into_owned()))
                        })
                        .collect();
                    let argument_types = self.origins.native_arguments(
                        &candidate(receiver),
                        member,
                        argument_types?,
                    );
                    let result =
                        native::value_call(&candidate(receiver), member, &argument_types, None)
                            .ok()?;
                    let result = crate::records::refine_result(
                        &candidate(receiver),
                        member,
                        arguments.first().and_then(|v| self.origins.symbol(*v)),
                        result,
                    );
                    crate::tuples::refine_slice(
                        &candidate(receiver),
                        member,
                        &argument_types,
                        &arguments
                            .iter()
                            .map(|v| self.origins.integer(*v))
                            .collect::<Vec<_>>(),
                    )
                    .ok()
                    .map(|view| view.unwrap_or(result))
                });
                (*result, ty)
            }
            Operation::Binary {
                result,
                kind,
                left,
                right,
            } => (
                *result,
                self.values
                    .get(left)
                    .zip(self.values.get(right))
                    .and_then(|(left, right)| {
                        operators::binary_result(*kind, &candidate(left), &candidate(right))
                    }),
            ),
            Operation::SourceCast {
                result,
                class,
                nullable,
                ..
            } => (
                *result,
                class
                    .base
                    .constructed_type()
                    .map(|ty| if *nullable { ty.with_null() } else { ty }),
            ),
            Operation::Cast { result, target, .. } => (
                *result,
                crate::casts::value_type(target).map(|ty| ty.api_type()),
            ),
            Operation::SourceInstanceOf { result, .. }
            | Operation::InstanceOf { result, .. }
            | Operation::Has { result, .. } => (*result, Some(Type::named("Toybox.Lang.Boolean"))),
            Operation::Compare {
                result,
                kind,
                left,
                right,
            } => (
                *result,
                self.values
                    .get(left)
                    .zip(self.values.get(right))
                    .and_then(|(left, right)| {
                        operators::comparison_result(*kind, &candidate(left), &candidate(right))
                    }),
            ),
            Operation::Not { result, operand } => (
                *result,
                self.values
                    .get(operand)
                    .and_then(|ty| operators::not_result(&candidate(ty))),
            ),
            Operation::Step {
                result,
                operand,
                increment,
            } => (
                *result,
                self.values
                    .get(operand)
                    .and_then(|ty| operators::step_result(&candidate(ty), *increment)),
            ),
            Operation::Positive { result, operand } => (
                *result,
                self.values
                    .get(operand)
                    .and_then(|ty| operators::positive_result(&candidate(ty))),
            ),
            Operation::Negate { result, operand } => (
                *result,
                self.values
                    .get(operand)
                    .and_then(|ty| operators::sign_result(&candidate(ty))),
            ),
            Operation::Complement { result, operand } => (
                *result,
                self.values
                    .get(operand)
                    .and_then(|ty| operators::complement_result(&candidate(ty))),
            ),
            _ => return,
        };
        if let Some(ty) = ty {
            let ty = if crate::type_flow::read_place(operation).is_some()
                || matches!(operation, Operation::LocalTake { .. })
            {
                self.origins.refined_type(result).unwrap_or(ty)
            } else {
                ty
            };
            self.origins.note_type(result, ty.clone());
            if ty == Type::Any {
                self.origins.note_dynamic(result);
            }
            self.records.note_result(result, &ty);
            let ty = self.records.value_type(result, &ty);
            self.values.insert(result, ty);
        }
    }

    /// Read a stored local's candidate type; final IR checking remains authoritative.
    pub(crate) fn local_type(&self, local: LocalId) -> Option<&Type> {
        self.locals.get(&local)
    }
    /// Return a local read's candidate type under current null/class facts,
    /// without changing the storage declaration used by later control flow.
    pub(crate) fn local_read_type(&self, local: LocalId) -> Option<std::borrow::Cow<'_, Type>> {
        self.locals.get(&local).map(|ty| {
            if self
                .facts
                .iter()
                .any(|fact| fact.place() == crate::type_flow::Place::Local(local))
            {
                std::borrow::Cow::Owned(
                    self.refined(ty.clone(), crate::type_flow::Place::Local(local)),
                )
            } else {
                std::borrow::Cow::Borrowed(ty)
            }
        })
    }
    pub(crate) fn local_class(&self, local: LocalId) -> Option<&str> {
        self.origins.local_class(local)
    }
    pub(crate) fn local_contract(&self, local: LocalId) -> Option<&crate::SourceUnion> {
        self.origins.local_contract(local)
    }
    /// Return candidate source identity or the ordinary API value contract.
    pub(crate) fn join_contract(&self, value: ValueId) -> Option<crate::JoinType> {
        self.origins
            .join_contract(value)
            .or_else(|| self.values.get(&value).cloned().map(crate::JoinType::Api))
    }
    pub(crate) fn value_type(&self, value: ValueId) -> Option<&Type> {
        self.values.get(&value)
    }
    pub(crate) fn api_types(&self) -> BTreeMap<ValueId, Type> {
        self.values.clone()
    }
    pub(crate) fn contracts(&self) -> BTreeMap<ValueId, crate::JoinType> {
        self.values
            .keys()
            .filter_map(|&v| self.join_contract(v).map(|ty| (v, ty)))
            .collect()
    }
    pub(crate) fn refresh_variables(
        &mut self,
        operations: &[crate::LocatedOperation],
        block: crate::BlockId,
    ) {
        if let Some(state) =
            crate::record_flow::inputs(operations, &self.parameters, &self.values).remove(&block)
        {
            self.records = state;
        }
        if self.variables.is_empty() {
            return;
        }
        let structure = crate::control_flow::prefix(operations);
        let inputs = crate::local_flow::inputs(&structure, operations, &self.contracts());
        for local in &self.variables {
            if let Some(domain) = inputs.get(&block).and_then(|s| s.get(local)) {
                if let Some(ty) = crate::local_flow::proven(domain, *local, &self.facts) {
                    self.locals.insert(*local, ty.api_type());
                    self.origins.set_variable_contract(*local, &ty);
                }
            }
        }
    }

    pub(crate) fn set_record(&mut self, builder: crate::BuilderId, fields: BTreeMap<String, Type>) {
        self.records.set_builder(builder, fields.clone());
        self.options.insert(builder, fields);
    }
    /// Select the private builder's final element contract after the first element.
    pub(crate) fn set_dictionary(&mut self, builder: crate::BuilderId, ty: crate::DictionaryType) {
        self.dictionaries.insert(builder, ty);
    }
}

/// Select the possible non-null class without asserting a control-flow proof.
fn candidate(ty: &Type) -> std::borrow::Cow<'_, Type> {
    if let Some(value) = crate::ValueType::from_api_type(ty) {
        if value.without_null() != value {
            return std::borrow::Cow::Owned(value.without_null().api_type());
        }
    }
    std::borrow::Cow::Borrowed(ty)
}

/// Replay candidate types for a primitive emitter that has no builder cache.
/// Whole-function validation still checks null proofs, argument types, and API levels.
pub(crate) fn value_type(
    operations: &[crate::LocatedOperation],
    value: ValueId,
    parameters: Vec<ParameterType>,
) -> Option<Type> {
    let mut types = Types::new(parameters);
    let inputs = crate::type_flow::prefix_inputs(operations);
    for op in operations {
        if let Operation::Block { block } = op.operation {
            types.facts = inputs.get(&block).cloned().unwrap_or_default();
        }
        types.observe(&op.operation);
    }
    types.value_type(value).cloned()
}
