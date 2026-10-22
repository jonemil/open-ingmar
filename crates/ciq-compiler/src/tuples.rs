//! Positional Array contracts shared by source construction and saved IR.
use crate::{ReturnType, ValueId, api::Type};

/// An ordered tuple of value contracts, independent of VM storage.
/// Slots retain exact source identities, including within nested tuples.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TupleType {
    pub(crate) slots: Vec<ReturnType>,
}

impl TupleType {
    /// Normalize slot contracts and construct a positional tuple. Reject
    /// unsupported slots (including Void and graphics contexts), and
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
        if self.slots.len() > i32::MAX as usize {
            return Err("tuple requires a representable slot count");
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
        if body.is_empty() {
            vec![]
        } else {
            crate::structural::split(body, ';')?
        }
        .into_iter()
        .map(crate::type_text::call_result)
        .collect::<Result<_, _>>()?,
    )
}

/// Derive the receiver's positional contract after Array.add or Array.addAll.
/// Spreading requires a tuple contract with a known slot count; it never claims
/// a fixed length for an ordinary Array. Check the native method's API level.
pub(crate) fn append_contract(
    receiver: &Type,
    value: &ReturnType,
    spread: bool,
    context: Option<crate::api::ApiContext>,
) -> Result<Type, String> {
    if let Some(context) = context {
        context.check_type(crate::api::core(), receiver)?;
        context.method(
            crate::api::core(),
            "Toybox.Lang.Array",
            if spread { "addAll" } else { "add" },
        )?;
    }
    let tail = if spread {
        match value {
            ReturnType::Tuple(tuple) => vec![tuple.slots.clone()],
            ReturnType::Union(Type::Union(parts)) => parts
                .iter()
                .map(|part| {
                    if let Type::TupleValue(tuple) = part {
                        Ok(tuple.slots.clone())
                    } else {
                        Err("tuple addAll requires positional argument alternatives")
                    }
                })
                .collect::<Result<Vec<_>, _>>()?,
            _ => {
                return Err("tuple addAll requires an argument with known positional shape".into());
            }
        }
    } else {
        vec![vec![value.clone()]]
    };
    let mut results = Vec::new();
    // Repeated self-appends can double a flat or nested signature with each
    // operation. Bound derived metadata before cloning the concatenated slots;
    // this is a compiler work budget, independent of VM Array capacity.
    let mut signature_budget = 64 * 1024usize;
    for part in crate::unions::alternatives(receiver) {
        let Type::TupleValue(tuple) = part else {
            return Err("tuple append requires non-null positional receivers".into());
        };
        for tail in &tail {
            let separator = usize::from(!tuple.slots.is_empty() && !tail.is_empty());
            let bytes = tuple.tag().len()
                + tail
                    .iter()
                    .map(|slot| crate::type_text::result_tag(slot).len())
                    .sum::<usize>()
                + tail.len().saturating_sub(1)
                + separator;
            signature_budget = signature_budget
                .checked_sub(bytes)
                .ok_or("tuple append exceeds the 64 KiB derived-signature budget")?;
            let mut slots = tuple.slots.clone();
            slots.extend_from_slice(tail);
            results.push(TupleType::new(slots)?.api_type());
        }
    }
    Ok(crate::unions::normalize(results))
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

/// Replace a known slot in each tuple alternative and return the new contract.
/// Dynamic indices cannot establish a positional shape; reject them until
/// generalized Array element unions can represent the resulting storage.
pub(crate) fn write_contract(
    receiver: &Type,
    index: Option<i32>,
    value: &ReturnType,
) -> Result<Type, &'static str> {
    match receiver {
        Type::TupleValue(tuple) => {
            let index = index.ok_or("dynamic tuple writes require generalized Array contracts")?;
            tuple.read(Some(index))?;
            let mut slots = tuple.slots.clone();
            slots[index as usize] = value.clone();
            Ok(TupleType::new(slots)?.api_type())
        }
        Type::Union(parts) => Ok(crate::unions::normalize(
            parts
                .iter()
                .map(|part| write_contract(part, index, value))
                .collect::<Result<_, _>>()?,
        )),
        _ => Err("tuple writes require non-null positional receivers"),
    }
}
