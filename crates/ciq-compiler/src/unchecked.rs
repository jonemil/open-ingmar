//! Explicit no-op assertions for supported unchecked source operations.
use crate::{LocatedOperation, Operation, ParameterType, ValueId, api::Type};
use std::collections::{BTreeMap, BTreeSet};

/// Insert lexical assertions at unchecked value uses, preserving declarations,
/// receiver identities, call signatures, and evaluation order. Source emission
/// allocates result IDs from operation indices; fresh IDs start past that range.
/// The resulting ordinary IR must pass all function and unit validators.
pub(crate) fn lower(
    operations: &mut Vec<LocatedOperation>,
    parameters: Vec<ParameterType>,
    preserve_return: bool,
) {
    let mut types = crate::source_types::Types::new(parameters);
    let inputs = crate::type_flow::prefix_inputs(operations);
    let mut fixed = BTreeSet::new();
    let mut next = operations.len() as u32;
    let mut output = Vec::new();
    let mut assertions = BTreeMap::<usize, Vec<LocatedOperation>>::new();
    for mut located in std::mem::take(operations) {
        crate::type_flow::update(&mut types.facts, &inputs, &located.operation);
        types.observe(&located.operation);
        let span = located.span;
        let mut assert = |value: &mut ValueId, target: Type| {
            let result = ValueId(next);
            next += 1;
            assertions
                .entry(value.0 as usize)
                .or_default()
                .push(LocatedOperation {
                    operation: Operation::Cast {
                        result,
                        operand: *value,
                        target,
                    },
                    span,
                });
            *value = result;
        };
        // Keep resolved result types so later native calls retain API checks.
        // Erase operator operands only when the checked profile cannot resolve them.
        match &mut located.operation {
            Operation::Binary {
                result,
                left,
                right,
                ..
            }
            | Operation::Compare {
                result,
                left,
                right,
                ..
            } => {
                if types.value_type(*result).is_none() {
                    assert(left, Type::Any);
                    assert(right, Type::Any);
                }
            }
            Operation::Not { result, operand }
            | Operation::Complement { result, operand }
            | Operation::Negate { result, operand }
            | Operation::Positive { result, operand }
            | Operation::Step {
                result, operand, ..
            } => {
                if types.value_type(*result).is_none() {
                    assert(operand, Type::Any);
                }
            }
            Operation::Return { value } if !preserve_return => assert(value, Type::Any),
            Operation::JumpValue { value, .. }
            | Operation::ParameterAssign { value, .. }
            | Operation::SelfFieldAssign { value, .. }
            | Operation::MemberWrite { value, .. }
            | Operation::LocalInitialize { value, .. }
            | Operation::Println { value } => assert(value, Type::Any),
            Operation::LocalDeclare { local, .. } => {
                fixed.insert(*local);
            }
            Operation::LocalAssign { local, value } if fixed.contains(local) => {
                assert(value, Type::Any)
            }
            Operation::Branch { condition, .. } => {
                let boolean = Type::named("Toybox.Lang.Boolean");
                if types.value_type(*condition) != Some(&boolean) {
                    assert(condition, boolean);
                }
            }
            Operation::ArrayRead { index, .. } => {
                if !types.value_type(*index).is_some_and(|ty| {
                    crate::native::accepts(&Type::named("Toybox.Lang.Number"), ty)
                }) {
                    assert(index, Type::Any);
                }
            }
            Operation::ArrayWrite { index, value, .. } => {
                if !types.value_type(*index).is_some_and(|ty| {
                    crate::native::accepts(&Type::named("Toybox.Lang.Number"), ty)
                }) {
                    assert(index, Type::Any);
                }
                assert(value, Type::Any);
            }
            Operation::DictionaryRead {
                dictionary, key, ..
            }
            | Operation::DictionaryWrite {
                dictionary, key, ..
            } => {
                let expected = types
                    .value_type(*dictionary)
                    .and_then(|ty| crate::native::value_parameters(ty, "get"))
                    .and_then(|parameters| parameters.into_iter().next());
                if !expected
                    .as_ref()
                    .zip(types.value_type(*key))
                    .is_some_and(|(expected, actual)| crate::native::accepts(expected, actual))
                {
                    assert(key, Type::Any);
                }
            }
            Operation::GlobalCall { arguments, .. }
            | Operation::GlobalCallStatement { arguments, .. }
            | Operation::ModuleCall { arguments, .. }
            | Operation::ModuleCallStatement { arguments, .. }
            | Operation::SelfCall { arguments, .. }
            | Operation::SelfCallStatement { arguments, .. }
            | Operation::ObjectCall { arguments, .. }
            | Operation::ObjectCallStatement { arguments, .. }
            | Operation::SourceNew { arguments, .. } => {
                for value in arguments {
                    assert(value, Type::Any);
                }
            }
            Operation::NativeCall {
                receiver,
                member,
                arguments,
                ..
            }
            | Operation::NativeCallStatement {
                receiver,
                member,
                arguments,
            } => {
                let parameters = types
                    .value_type(*receiver)
                    .and_then(|ty| crate::native::value_parameters(ty, member))
                    .unwrap_or_default();
                for (value, expected) in arguments.iter_mut().zip(parameters) {
                    if !types
                        .value_type(*value)
                        .is_some_and(|actual| crate::native::accepts(&expected, actual))
                    {
                        assert(value, Type::Any);
                    }
                }
            }
            Operation::NativeModuleCall {
                module,
                member,
                arguments,
                ..
            }
            | Operation::NativeModuleCallStatement {
                module,
                member,
                arguments,
            } => {
                let parameters =
                    crate::native::module_parameters(module, member).unwrap_or_default();
                for (value, expected) in arguments.iter_mut().zip(parameters) {
                    if !types
                        .value_type(*value)
                        .is_some_and(|actual| crate::native::accepts(&expected, actual))
                    {
                        assert(value, Type::Any);
                    }
                }
            }
            _ => {}
        }
        if let Operation::DictionaryWrite { value, .. } = &mut located.operation {
            assert(value, Type::Any);
        }
        output.push(located);
    }
    // Assertions are pure but still participate in IR expression order. Place
    // each immediately after its operand rather than after later call arguments.
    for (at, operation) in output.into_iter().enumerate() {
        operations.push(operation);
        operations.extend(assertions.remove(&at).unwrap_or_default());
    }
    debug_assert!(
        assertions.is_empty(),
        "source result IDs use operation indices"
    );
}
