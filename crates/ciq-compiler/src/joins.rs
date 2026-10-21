//! Typed single-value control-flow edges for conditional expressions.
use crate::{BlockId, IrError, LocatedOperation, Operation, ValueId, ValueType, api::Type};
use std::collections::BTreeMap;

pub(crate) use crate::type_text::{join as parse_type, join_tag as type_tag};

/// A block parameter or declared local's contract, independent of VM storage.
/// Source contracts retain exact identity in addition to their native projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JoinType {
    /// Exact alternatives whose identities survive native projection.
    SourceUnion(crate::SourceUnion),
    /// An API value type; each consumer checks its supported type domains.
    Api(Type),
    /// An exact constructible source class, optionally admitting Null.
    Source {
        class: crate::SourceClass,
        nullable: bool,
    },
}

impl JoinType {
    /// Return the API projection used for native operand and availability checks.
    /// Exact source assignability is checked separately on each incoming edge.
    pub fn api_type(&self) -> Type {
        match self {
            Self::SourceUnion(contract) => contract.api_type(),
            Self::Api(ty) => ty.clone(),
            Self::Source { class, nullable } => {
                let ty = Type::named(class.base.api_name());
                if *nullable { ty.with_null() } else { ty }
            }
        }
    }
}

impl From<Type> for JoinType {
    fn from(ty: Type) -> Self {
        Self::Api(ty)
    }
}

/// Parse a saved block-value contract without erasing source identity.
pub(crate) fn parse_contract(tag: &str) -> Option<JoinType> {
    match crate::type_text::call_result(tag) {
        Ok(crate::ReturnType::SourceObject(class)) => Some(JoinType::Source {
            class,
            nullable: false,
        }),
        Ok(crate::ReturnType::NullableSourceObject(class)) => Some(JoinType::Source {
            class,
            nullable: true,
        }),
        Ok(crate::ReturnType::SourceUnion(contract)) => Some(contract.join()),
        _ => parse_type(tag).map(JoinType::Api),
    }
}

/// Print a supported block contract, rejecting invalid source names or bases.
pub(crate) fn contract_tag(ty: &JoinType) -> Option<std::borrow::Cow<'static, str>> {
    match ty {
        JoinType::SourceUnion(contract) => Some(std::borrow::Cow::Owned(contract.text())),
        JoinType::Api(ty) => type_tag(ty),
        JoinType::Source { class, nullable } => {
            if crate::SourceClass::parse(&class.to_string()).as_ref() != Some(class) {
                return None;
            }
            Some(crate::type_text::result_tag(&if *nullable {
                crate::ReturnType::NullableSourceObject(class.clone())
            } else {
                crate::ReturnType::SourceObject(class.clone())
            }))
        }
    }
}

/// Merge exact source/Null alternatives, or reuse the ordinary API join rules.
/// Mixing exact identities with erased native alternatives remains unsupported.
pub(crate) fn contract_result(left: &JoinType, right: &JoinType) -> Option<JoinType> {
    match (left.source_contract(), right.source_contract()) {
        (Some(a), Some(b)) => return a.merge(&b).map(crate::SourceUnion::join),
        (Some(mut a), None) if *right == JoinType::Api(Type::Null) => {
            a.nullable = true;
            return Some(a.join());
        }
        (None, Some(mut b)) if *left == JoinType::Api(Type::Null) => {
            b.nullable = true;
            return Some(b.join());
        }
        _ => {}
    }
    match (left, right) {
        (JoinType::Api(Type::Any), _) | (_, JoinType::Api(Type::Any)) => {
            Some(JoinType::Api(Type::Any))
        }
        (JoinType::Api(left), JoinType::Api(right)) => result(left, right).map(JoinType::Api),
        (
            JoinType::Source {
                class: left,
                nullable: a,
            },
            JoinType::Source {
                class: right,
                nullable: b,
            },
        ) if left == right => Some(JoinType::Source {
            class: left.clone(),
            nullable: *a || *b,
        }),
        (JoinType::Source { class, .. }, JoinType::Api(Type::Null))
        | (JoinType::Api(Type::Null), JoinType::Source { class, .. }) => Some(JoinType::Source {
            class: class.clone(),
            nullable: true,
        }),
        _ => None,
    }
}

/// Join supported alternatives without converting either runtime value.
pub(crate) fn result(left: &Type, right: &Type) -> Option<Type> {
    if *left == Type::Any || *right == Type::Any {
        return Some(Type::Any);
    }
    if left == right && type_tag(left).is_some() {
        return Some(left.clone());
    }
    for (expected, actual) in [(left, right), (right, left)] {
        if matches!(
            ValueType::from_api_type(expected),
            Some(ValueType::Interface(_) | ValueType::NullableInterface(_))
        ) && crate::native::accepts(expected, actual)
        {
            return Some(expected.clone());
        }
    }
    for (interface, other) in [(left, right), (right, left)] {
        if let Some(ValueType::Interface(interface)) = ValueType::from_api_type(interface) {
            if *other == Type::Null {
                return Some(interface.api_type(true));
            }
        }
    }
    let object = parse_type("object").unwrap();
    let nullable_object = parse_type("nullable-object").unwrap();
    if ((left == &object || left == &nullable_object) && right == &Type::Null)
        || ((right == &object || right == &nullable_object) && left == &Type::Null)
        || ((left == &object || left == &nullable_object)
            && (right == &object || right == &nullable_object))
    {
        return Some(nullable_object);
    }
    for tag in [
        "nullable-integer",
        "nullable-long",
        "nullable-double",
        "nullable-float",
        "nullable-string",
        "nullable-boolean",
        "nullable-char",
        "nullable-symbol",
        "nullable-method",
        "nullable-weak-reference",
        "nullable-byte-array",
    ] {
        let ty = parse_type(tag).unwrap();
        if crate::native::accepts(&ty, left) && crate::native::accepts(&ty, right) {
            return Some(ty);
        }
    }
    for alias in [Type::integer(), Type::decimal(), Type::numeric()] {
        let nullable = alias.clone().with_null();
        for candidate in [alias, nullable] {
            if crate::native::accepts(&candidate, left) && crate::native::accepts(&candidate, right)
            {
                return Some(candidate);
            }
        }
    }
    let ty = crate::unions::normalize(vec![left.clone(), right.clone()]);
    (ValueType::from_api_type(&ty).is_some() && type_tag(&ty).is_some()).then_some(ty)
}

/// Collect source join operands without changing their predecessor identities.
pub(crate) fn incoming_values(operations: &[LocatedOperation]) -> BTreeMap<BlockId, Vec<ValueId>> {
    let mut incoming: BTreeMap<_, Vec<_>> = BTreeMap::new();
    for op in operations {
        if let Operation::JumpValue { target, value } = op.operation {
            incoming.entry(target).or_default().push(value);
        }
    }
    incoming
}

/// Infer a source join from previously checked edge operands. Their types
/// already reflect full-CFG null proofs and writes; no candidate type is trusted.
pub(crate) fn infer_type(
    incoming: &[ValueId],
    types: &BTreeMap<ValueId, ValueType>,
    origins: &crate::source_objects::Origins,
) -> Result<JoinType, &'static str> {
    let mut joined = None;
    for value in incoming {
        let api_type = types
            .get(value)
            .ok_or("source join requires earlier incoming values")?
            .api_type();
        let ty = origins
            .join_contract(*value)
            .unwrap_or(JoinType::Api(api_type));
        joined = Some(match joined {
            None if contract_tag(&ty).is_some() => ty,
            Some(previous) => contract_result(&previous, &ty).ok_or(
                "conditional arms require supported native or exact source alternatives, optionally with Null",
            )?,
            _ => return Err("unsupported conditional arm type"),
        });
    }
    joined.ok_or("source join requires incoming values")
}

/// Check parameter placement and every incoming edge's stack arity.
/// Ordinary branches/jumps carry no values; a value jump supplies exactly one.
pub(crate) fn validate(
    operations: &[LocatedOperation],
) -> Result<BTreeMap<BlockId, JoinType>, IrError> {
    let fail = |index, message: &str| IrError {
        operation: Some(index),
        message: message.into(),
    };
    let mut parameters = BTreeMap::new();
    for (index, op) in operations.iter().enumerate() {
        if let Operation::BlockParameter { ref value_type, .. } = op.operation {
            let Some(Operation::Block { block }) =
                index.checked_sub(1).map(|i| &operations[i].operation)
            else {
                return Err(fail(
                    index,
                    "block parameter must immediately follow its block marker",
                ));
            };
            if index == 1 || contract_tag(value_type).is_none() {
                return Err(fail(index, "entry or unsupported block parameter type"));
            }
            parameters.insert(*block, value_type.clone());
        }
    }
    for (index, op) in operations.iter().enumerate() {
        match op.operation {
            Operation::JumpValue { target, .. } if !parameters.contains_key(&target) => {
                return Err(fail(index, "value edge requires a target block parameter"));
            }
            Operation::Jump { target } if parameters.contains_key(&target) => {
                return Err(fail(index, "target block requires an incoming value"));
            }
            Operation::Branch {
                on_true, on_false, ..
            } if parameters.contains_key(&on_true) || parameters.contains_key(&on_false) => {
                return Err(fail(index, "branch targets cannot require incoming values"));
            }
            _ => (),
        }
    }
    Ok(parameters)
}

/// Reject uses that would insert call/field/builder setup above an incoming
/// stack value before consuming it. Such uses must first stage the value in a
/// local. Ordinary expression scheduling separately enforces operand order.
pub(crate) fn validate_stack_uses(
    operations: &[LocatedOperation],
    consumers: &BTreeMap<crate::ValueId, usize>,
) -> Result<(), IrError> {
    for op in operations {
        let Operation::BlockParameter { result, .. } = op.operation else {
            continue;
        };
        let mut value = result;
        loop {
            let at = consumers[&value];
            match operations[at].operation {
                Operation::Binary { result, left, .. }
                | Operation::Compare { result, left, .. } if left == value => {
                    value = result;
                }
                Operation::SourceCast { result, .. }
                | Operation::ArrayAllocate { result, .. }
                | Operation::ByteArrayAllocate { result, .. }
                | Operation::Cast { result, .. }
                | Operation::Not { result, .. }
                | Operation::Positive { result, .. }
                | Operation::Negate { result, .. }
                | Operation::Complement { result, .. }
                | Operation::Step { result, .. } => value = result,
                Operation::Return { .. }
                | Operation::Discard { .. }
                | Operation::LocalInitialize { .. }
                | Operation::LocalVariable { value: Some(_), .. }
                | Operation::LocalAssign { .. }
                | Operation::ParameterAssign { .. }
                | Operation::JumpValue { .. }
                | Operation::Branch { .. } => break,
                _ => return Err(IrError {
                    operation: Some(at),
                    message: "incoming stack value must be consumed before call, field, or array setup; stage it in a local".into(),
                }),
            }
        }
    }
    Ok(())
}
