//! Candidate value types and constructor classes for source call/Array lowering.
//! These do not prove null safety or validity; whole-function IR validation does.
use std::collections::BTreeMap;

use crate::{
    ArrayElement, LocalId, Operation, ParameterType, ValueId, api::Type, native, operators,
};

pub(crate) struct Types {
    parameters: Vec<ParameterType>,
    values: BTreeMap<ValueId, Type>,
    locals: BTreeMap<LocalId, Type>,
    origins: crate::source_objects::Origins,
}

impl Types {
    pub(crate) fn new(parameters: Vec<ParameterType>) -> Self {
        Self {
            parameters: parameters.clone(),
            values: BTreeMap::new(),
            locals: BTreeMap::new(),
            origins: crate::source_objects::Origins::new(parameters),
        }
    }

    /// Record a candidate result using shared operator and native contracts.
    /// Failed resolution leaves validation to the final IR checker.
    pub(crate) fn observe(&mut self, operation: &Operation) {
        let _ = self.origins.observe(operation);
        let (result, ty) = match operation {
            Operation::NativeConstant {
                result,
                module,
                member,
            } => (
                *result,
                native::constant(module, member, None)
                    .ok()
                    .map(|(ty, _)| ty),
            ),
            Operation::SourceNew { result, .. } => {
                (*result, Some(Type::named("Toybox.WatchUi.View")))
            }
            Operation::TupleNew { result, .. } => {
                (*result, Some(crate::TupleShape::OneView.api_type()))
            }
            Operation::NativeNew { result, class } => (*result, Some(Type::named(class.name()))),
            Operation::Integer { result, .. } => (*result, Some(Type::named("Toybox.Lang.Number"))),
            Operation::Char { result, .. } => (*result, Some(Type::named("Toybox.Lang.Char"))),
            Operation::Boolean { result, .. } => {
                (*result, Some(Type::named("Toybox.Lang.Boolean")))
            }
            Operation::String { result, .. } => (*result, Some(Type::named("Toybox.Lang.String"))),
            Operation::Null { result } => (*result, Some(Type::Null)),
            Operation::ArrayNew {
                result,
                element_type,
            } => (
                *result,
                Some(Type::Array(Box::new(element_type.api_type()))),
            ),
            Operation::ParameterRead { result, index } => {
                (*result, self.parameters.get(*index).map(|ty| ty.api_type()))
            }
            Operation::LocalRead { result, local } => (*result, self.locals.get(local).cloned()),
            Operation::LocalInitialize { local, value } => {
                if let Some(ty) = self.values.get(value) {
                    self.locals.insert(*local, ty.clone());
                }
                return;
            }
            Operation::SelfFieldRead {
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
            Operation::ArrayRead { result, array, .. } => (
                *result,
                self.values.get(array).and_then(|ty| match ty {
                    Type::Array(element) => Some((**element).clone()),
                    _ => None,
                }),
            ),
            Operation::NativeCall {
                result,
                receiver,
                member,
                arguments,
            } => {
                let ty = self.values.get(receiver).and_then(|receiver| {
                    let arguments: Option<Vec<_>> = arguments
                        .iter()
                        .map(|id| self.values.get(id).map(candidate))
                        .collect();
                    native::value_call(
                        candidate(receiver),
                        member,
                        &arguments?.into_iter().cloned().collect::<Vec<_>>(),
                        None,
                    )
                    .ok()
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
                        operators::binary_result(*kind, candidate(left), candidate(right))
                    }),
            ),
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
                        operators::comparison_result(*kind, candidate(left), candidate(right))
                    }),
            ),
            Operation::Not { result, operand } => (
                *result,
                self.values
                    .get(operand)
                    .and_then(|ty| operators::not_result(candidate(ty))),
            ),
            Operation::Negate { result, operand } => (
                *result,
                self.values
                    .get(operand)
                    .and_then(|ty| operators::negate_result(candidate(ty))),
            ),
            _ => return,
        };
        if let Some(ty) = ty {
            self.values.insert(result, ty);
        }
    }

    pub(crate) fn element(&self, value: ValueId) -> Option<ArrayElement> {
        self.values
            .get(&value)
            .and_then(|ty| ArrayElement::from_api_type(candidate(ty)))
    }

    /// Read a stored local's candidate type; final IR checking remains authoritative.
    pub(crate) fn local_type(&self, local: LocalId) -> Option<&Type> {
        self.locals.get(&local)
    }
    pub(crate) fn local_class(&self, local: LocalId) -> Option<&str> {
        self.origins.local_class(local)
    }
    pub(crate) fn value_type(&self, value: ValueId) -> Option<&Type> {
        self.values.get(&value)
    }

    /// Replace a provisional allocation contract before emitting any array reads.
    pub(crate) fn set_array(&mut self, value: ValueId, local: LocalId, element: ArrayElement) {
        let ty = Type::Array(Box::new(element.api_type()));
        self.values.insert(value, ty.clone());
        self.locals.insert(local, ty);
    }
}

/// Select the possible non-null class without asserting a control-flow proof.
fn candidate(ty: &Type) -> &Type {
    if let Type::Union(types) = ty {
        let mut non_null = types.iter().filter(|ty| **ty != Type::Null);
        if let Some(only) = non_null.next() {
            if non_null.next().is_none() {
                return only;
            }
        }
    }
    ty
}
