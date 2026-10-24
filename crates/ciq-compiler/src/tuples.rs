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
    positional: bool,
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
    if !positional
        || read_contract(receiver, None).is_none()
        || (spread && read_contract(&value.api_type(), None).is_none())
    {
        return if spread {
            array_contract(&[receiver.clone(), value.api_type()], &[]).map_err(String::from)
        } else {
            array_contract(std::slice::from_ref(receiver), std::slice::from_ref(value))
                .map_err(String::from)
        };
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

/// Retain exact source identities when adapting a slot to the shared type model.
pub(crate) fn exact_type(contract: &ReturnType) -> Type {
    contract
        .source_contract()
        .map(|c| Type::SourceValue(Box::new(c)))
        .unwrap_or_else(|| contract.api_type())
}

/// Report positional contracts that may carry source witnesses at a boundary.
pub(crate) fn contains(ty: &Type) -> bool {
    match ty {
        Type::TupleValue(_) | Type::Options(_) => true,
        Type::Array(element) => contains(element),
        Type::Union(parts) => parts.iter().any(contains),
        Type::SourceValue(contract) => contract.native().iter().any(contains),
        _ => false,
    }
}

/// Match positional values, delegating scalar/source proof to the caller.
/// A function may defer a class witness; the unit must verify its declaration.
/// Ordinary Array element domains stay invariant except for nested tuple views.
pub(crate) fn matches_with(
    expected: &Type,
    actual: &Type,
    leaf: &impl Fn(&Type, &Type) -> bool,
) -> bool {
    if *actual == Type::Any || *expected == Type::Any {
        return crate::native::accepts(expected, actual);
    }
    if let Type::Union(parts) = actual {
        return parts.iter().all(|part| matches_with(expected, part, leaf));
    }
    if let Type::Union(parts) = expected {
        // A source contract may contain several classes plus Null/native arms.
        // Its leaf proof distributes those arms across the expected union.
        if !matches!(actual, Type::SourceValue(_)) {
            return parts.iter().any(|part| matches_with(part, actual, leaf));
        }
    }
    match (expected, actual) {
        (Type::Dictionary { key, value }, Type::Options(fields)) => {
            crate::native::accepts(key, &Type::named("Toybox.Lang.Symbol"))
                && fields.values().all(|ty| matches_with(value, ty, leaf))
        }
        (Type::Options(expected), Type::Options(actual)) => actual.iter().all(|(key, ty)| {
            expected
                .get(key)
                .is_none_or(|required| matches_with(required, ty, leaf))
        }),
        (Type::TupleValue(expected), Type::TupleValue(actual)) => {
            expected.slots().len() == actual.slots().len()
                && expected
                    .slots()
                    .iter()
                    .zip(actual.slots())
                    .all(|(e, a)| matches_with(&exact_type(e), &exact_type(a), leaf))
        }
        (Type::Array(element), Type::TupleValue(actual)) => actual
            .slots()
            .iter()
            .all(|slot| matches_with(element, &exact_type(slot), leaf)),
        (Type::Array(expected), Type::Array(actual)) => {
            array_elements_match_with(expected, actual, leaf)
        }
        _ => leaf(expected, actual),
    }
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

/// Compare invariant Array element domains while permitting nested tuple-to-Array
/// views. Every alternative on each side must be represented; ordinary scalar
/// or source element domains cannot widen through this adaptation.
pub(crate) fn array_elements_match(expected: &Type, actual: &Type) -> bool {
    array_elements_match_with(expected, actual, &crate::native::accepts)
}

fn array_elements_match_with(
    expected: &Type,
    actual: &Type,
    leaf: &impl Fn(&Type, &Type) -> bool,
) -> bool {
    let expected = crate::unions::alternatives(expected);
    let actual = crate::unions::alternatives(actual);
    let matches = |expected: &Type, actual: &Type| match (expected, actual) {
        (Type::Array(expected), Type::Array(actual)) => {
            array_elements_match_with(expected, actual, leaf)
        }
        (Type::Array(_), Type::TupleValue(_) | Type::Tuple(_)) => {
            matches_with(expected, actual, leaf)
        }
        _ => crate::structural::same_type(expected, actual),
    };
    actual
        .iter()
        .all(|a| expected.iter().any(|e| matches(e, a)))
        && expected
            .iter()
            .all(|e| actual.iter().any(|a| matches(e, a)))
}

/// Reverse each positional alternative, retaining exact slot contracts.
pub(crate) fn reverse_contract(receiver: &Type) -> Result<Type, &'static str> {
    let mut results = Vec::new();
    for part in crate::unions::alternatives(receiver) {
        let Type::TupleValue(tuple) = part else {
            return Err("reverse requires non-null tuple alternatives");
        };
        results.push(TupleType::new(tuple.slots.iter().rev().cloned().collect())?.api_type());
    }
    Ok(crate::unions::normalize(results))
}

/// A checked slice bound. Dynamic bounds cannot establish a positional shape.
#[derive(Clone, Copy)]
pub(crate) enum SliceBound {
    Constant(i32),
    Null,
    Dynamic,
}

/// Refine a validated native slice call using immediate Number/Null operands.
/// Other calls or non-positional receivers retain their ordinary native result.
pub(crate) fn refine_slice(
    receiver: &Type,
    member: &str,
    arguments: &[Type],
    integers: &[Option<i32>],
) -> Result<Option<Type>, &'static str> {
    if member != "slice" || read_contract(receiver, None).is_none() {
        return Ok(None);
    }
    let [a, b] = arguments else {
        return Err("tuple slice requires two bounds");
    };
    let [ai, bi] = integers else {
        return Err("tuple slice requires two bound constants");
    };
    let bound = |ty: &Type, integer| match integer {
        Some(n) => SliceBound::Constant(n),
        None if *ty == Type::Null => SliceBound::Null,
        _ => SliceBound::Dynamic,
    };
    slice_contract(receiver, [bound(a, *ai), bound(b, *bi)]).map(Some)
}

/// Refine a tuple slice after ordinary native argument validation. Literal bounds
/// clip to each alternative's length; a dynamic bound yields Array<Any>.
pub(crate) fn slice_contract(
    receiver: &Type,
    bounds: [SliceBound; 2],
) -> Result<Type, &'static str> {
    if bounds
        .iter()
        .any(|bound| matches!(bound, SliceBound::Dynamic))
    {
        return Ok(Type::Array(Box::new(Type::Any)));
    }
    let mut results = Vec::new();
    for part in crate::unions::alternatives(receiver) {
        let Type::TupleValue(tuple) = part else {
            return Err("slice requires non-null tuple alternatives");
        };
        let length = tuple.slots.len() as i64;
        let bound = |value, default| match value {
            SliceBound::Null => default,
            SliceBound::Constant(n) => {
                let n = i64::from(n);
                (if n < 0 { length + n } else { n }).clamp(0, length) as usize
            }
            SliceBound::Dynamic => unreachable!(),
        };
        let start = bound(bounds[0], 0);
        let end = bound(bounds[1], tuple.slots.len()).max(start);
        results.push(TupleType::new(tuple.slots[start..end].to_vec())?.api_type());
    }
    Ok(crate::unions::normalize(results))
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
        Type::TupleValue(_) if index.is_none() => {
            array_contract(std::slice::from_ref(receiver), std::slice::from_ref(value))
        }
        Type::Array(_) => {
            array_contract(std::slice::from_ref(receiver), std::slice::from_ref(value))
        }
        Type::TupleValue(tuple) => {
            let index = index.expect("known tuple index");
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

/// Return the erased view produced by a positional removal. Already-erased
/// Arrays retain that view so loop inference has a stable fixed point; ordinary
/// typed Arrays keep their existing invariant removal contracts.
pub(crate) fn removal_contract(receiver: &Type) -> Option<Type> {
    let parts = crate::unions::alternatives(receiver);
    (!parts.is_empty()
        && parts.iter().all(|part| match part {
            Type::TupleValue(_) => true,
            Type::Array(element) => **element == Type::Any,
            _ => false,
        }))
    .then(|| Type::Array(Box::new(Type::Any)))
}

/// Identify Boolean-producing native removal calls, including discarded results.
pub(crate) fn removal_receiver(op: &crate::Operation) -> Option<ValueId> {
    match op {
        crate::Operation::NativeCall {
            receiver, member, ..
        }
        | crate::Operation::NativeCallStatement {
            receiver, member, ..
        } if matches!(member.as_str(), "remove" | "removeAll") => Some(*receiver),
        _ => None,
    }
}

/// Join the contents of positional/Array inputs into an unknown-length Array.
/// Preserve native, nested, and exact source alternatives under their supported
/// element contracts. Reject domains exceeding their representation budgets.
pub(crate) fn array_contract(inputs: &[Type], extra: &[ReturnType]) -> Result<Type, &'static str> {
    fn elements(ty: &Type, out: &mut Vec<ReturnType>) -> Result<(), &'static str> {
        match ty {
            Type::TupleValue(tuple) => out.extend_from_slice(tuple.slots()),
            Type::Array(element) => out.push(
                crate::unions::result(*element.clone())
                    .ok_or("unsupported Array element contract")?,
            ),
            Type::Union(parts) => {
                for part in parts {
                    elements(part, out)?;
                }
            }
            _ => return Err("shape loss requires non-null Array or tuple inputs"),
        }
        Ok(())
    }
    let mut slots = extra.to_vec();
    for input in inputs {
        elements(input, &mut slots)?;
    }
    // An explicit erased alternative absorbs even source/nested slot contracts;
    // this does not invent a representable element type for un-erased inputs.
    if slots.contains(&ReturnType::Any) {
        return Ok(Type::Array(Box::new(Type::Any)));
    }
    let ty = if slots.is_empty() {
        Type::Any
    } else {
        crate::unions::normalize(
            slots
                .iter()
                .map(|slot| {
                    slot.source_contract()
                        .map(|c| Type::SourceValue(Box::new(c)))
                        .unwrap_or_else(|| slot.api_type())
                })
                .collect(),
        )
    };
    let element = crate::ArrayElement::from_api_type(&ty)
        .ok_or("shape loss requires a supported native Array element domain")?;
    Ok(Type::Array(Box::new(element.api_type())))
}
