//! Positional Array contracts shared by source construction and saved IR.
use crate::{ReturnType, ValueId, api::Type};

/// An ordered, nonempty tuple of value contracts, independent of VM storage.
/// Slots retain exact source identities, including within nested tuples.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TupleType {
    pub(crate) slots: Vec<ReturnType>,
}

impl TupleType {
    /// Normalize slot contracts and construct a positional tuple. Reject empty
    /// shapes, unsupported slots (including Void and graphics contexts), and
    /// nesting beyond the shared 64-level signature limit.
    pub fn new(slots: Vec<ReturnType>) -> Result<Self, &'static str> {
        let mut tuple = Self { slots };
        tuple.validate()?;
        for slot in &mut tuple.slots {
            *slot = slot
                .source_contract()
                .map(crate::SourceUnion::result)
                .or_else(|| crate::unions::result(crate::unions::normalize(vec![slot.api_type()])))
                .ok_or("unsupported tuple slot contract")?;
        }
        Ok(tuple)
    }

    /// Return slot contracts in positional order.
    pub fn slots(&self) -> &[ReturnType] {
        &self.slots
    }

    pub(crate) fn validate(&self) -> Result<(), &'static str> {
        if self.slots.is_empty() || self.slots.len() > i32::MAX as usize {
            return Err("tuple requires a nonempty representable slot count");
        }
        if self
            .slots
            .iter()
            .any(|ty| !crate::structural::result_depth(ty, 1))
        {
            return Err("tuple contract exceeds 64 levels");
        }
        for slot in &self.slots {
            if *slot == ReturnType::Void {
                return Err("tuple slots cannot be Void");
            }
            if crate::unions::alternatives(&slot.api_type())
                .iter()
                .any(|ty| *ty == Type::named("Toybox.Graphics.Dc"))
            {
                return Err("graphics contexts cannot be stored in tuple slots");
            }
            if let ReturnType::Tuple(tuple) = slot {
                tuple.validate()?;
            } else if slot.source_contract().is_none()
                && crate::unions::result(crate::unions::normalize(vec![slot.api_type()])).is_none()
            {
                return Err("unsupported tuple slot contract");
            }
        }
        Ok(())
    }

    /// Return the executable API profile without erasing source slot identity.
    pub fn api_type(&self) -> Type {
        Type::TupleValue(self.clone())
    }

    /// Return the native one-View callback result contract.
    pub fn view() -> Self {
        Self {
            slots: vec![ReturnType::Object(crate::NativeClass::View)],
        }
    }

    /// Return the executable View/BehaviorDelegate callback contract.
    pub fn view_delegate() -> Self {
        Self {
            slots: vec![
                ReturnType::Object(crate::NativeClass::View),
                ReturnType::Object(crate::NativeClass::BehaviorDelegate),
            ],
        }
    }

    /// Return one known slot or join the possible slots for a dynamic index.
    /// Reject out-of-bounds constants and results with no supported union.
    pub(crate) fn read(&self, index: Option<i32>) -> Result<ReturnType, &'static str> {
        if let Some(index) = index {
            return usize::try_from(index)
                .ok()
                .and_then(|i| self.slots.get(i))
                .cloned()
                .ok_or("tuple index is out of bounds");
        }
        let (first, rest) = self
            .slots
            .split_first()
            .ok_or("tuple requires at least one slot")?;
        rest.iter().try_fold(first.clone(), |left, right| {
            crate::joins::call_result(&left, right)
                .ok_or("tuple index has no supported result union")
        })
    }

    pub(crate) fn tag(&self) -> String {
        format!(
            "tuple({})",
            self.slots
                .iter()
                .map(|ty| crate::type_text::result_tag(ty).into_owned())
                .collect::<Vec<_>>()
                .join(";")
        )
    }
}

/// Parse a saved `tuple(tag;...)` contract with balanced, bounded nesting.
pub(crate) fn parse(tag: &str) -> Result<TupleType, &'static str> {
    let body = tag
        .strip_prefix("tuple(")
        .and_then(|body| body.strip_suffix(')'))
        .ok_or("invalid tuple contract")?;
    TupleType::new(
        crate::structural::split(body, ';')?
            .into_iter()
            .map(crate::type_text::call_result)
            .collect::<Result<_, _>>()?,
    )
}

/// Recover an evaluated slot contract, retaining source provenance separately
/// from its native projection. Erased values cannot invent a source identity.
pub(crate) fn value_contract(
    origins: &crate::source_objects::Origins,
    value: ValueId,
    ty: &Type,
) -> Result<ReturnType, &'static str> {
    origins
        .contract(value)
        .cloned()
        .map(crate::SourceUnion::result)
        .or_else(|| crate::unions::result(ty.clone()))
        .ok_or("unsupported tuple slot contract")
}

/// Check slot assignability without substituting a native base for an exact
/// source identity. Any retains its ordinary gradual boundary behavior.
pub(crate) fn accepts(expected: &ReturnType, actual: &ReturnType) -> bool {
    if *actual == ReturnType::Any {
        return true;
    }
    if let Some(expected) = expected.source_contract() {
        return actual.source_contract().map_or_else(
            || expected.accepts_api(&actual.api_type()),
            |actual| expected.accepts(&actual),
        );
    }
    crate::native::accepts(&expected.api_type(), &actual.api_type())
}

/// Resolve positional reads across tuple alternatives before native projection.
/// Non-tuple alternatives are handled by the ordinary indexable-value checker.
pub(crate) fn read_contract(
    receiver: &Type,
    index: Option<i32>,
) -> Option<Result<ReturnType, &'static str>> {
    match receiver {
        Type::TupleValue(tuple) => Some(tuple.read(index)),
        Type::Union(parts) => {
            let mut results = parts
                .iter()
                .map(|ty| read_contract(ty, index))
                .collect::<Option<Vec<_>>>()?
                .into_iter();
            let first = results.next()?;
            Some(results.fold(first, |left, right| {
                crate::joins::call_result(&left?, &right?)
                    .ok_or("tuple alternatives have no supported result union")
            }))
        }
        _ => None,
    }
}
