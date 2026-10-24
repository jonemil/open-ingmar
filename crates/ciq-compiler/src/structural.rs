//! Source structural contracts, independent of runtime class identities.
use crate::{ParameterType, ReturnType, api::Type};
use std::collections::BTreeMap;

/// Compare value contracts by shape, retaining exact source classes while
/// ignoring structural declaration provenance at any nesting depth.
pub(crate) fn same_result(a: &ReturnType, b: &ReturnType) -> bool {
    match (a.source_contract(), b.source_contract()) {
        (Some(a), Some(b)) => {
            a.classes == b.classes
                && a.nullable == b.nullable
                && same_alternatives(&a.native, &b.native)
        }
        (None, None) => same_type(&a.api_type(), &b.api_type()),
        _ => false,
    }
}

fn same_parameter(a: &ParameterType, b: &ParameterType) -> bool {
    match (a.source_contract(), b.source_contract()) {
        (Some(a), Some(b)) => {
            a.classes == b.classes
                && a.nullable == b.nullable
                && same_alternatives(&a.native, &b.native)
        }
        (None, None) => same_type(&a.api_type(), &b.api_type()),
        _ => false,
    }
}

fn same_alternatives(a: &[Type], b: &[Type]) -> bool {
    a.iter().all(|a| b.iter().any(|b| same_type(a, b)))
        && b.iter().all(|b| a.iter().any(|a| same_type(a, b)))
}

/// Compare invariant type shapes without making static flow provenance nominal.
/// This is equality, not widening or subtype assignability.
pub(crate) fn same_type(a: &Type, b: &Type) -> bool {
    if a == b {
        return true;
    }
    match (a, b) {
        (Type::SourceValue(a), Type::SourceValue(b)) => {
            same_result(&a.clone().result(), &b.clone().result())
        }
        (Type::Structural(a), Type::Structural(b)) => {
            a.fields.len() == b.fields.len()
                && a.methods.len() == b.methods.len()
                && a.fields.iter().all(|(name, ty)| {
                    b.fields
                        .get(name)
                        .is_some_and(|other| same_result(ty, other))
                })
                && a.methods.iter().all(|(name, method)| {
                    b.methods
                        .get(name)
                        .is_some_and(|other| method.accepts(other))
                })
        }
        (Type::Callback(a), Type::Callback(b)) => a.accepts(b),
        (Type::Array(a), Type::Array(b)) => same_type(a, b),
        (Type::Dictionary { key: a, value: av }, Type::Dictionary { key: b, value: bv }) => {
            same_type(a, b) && same_type(av, bv)
        }
        (Type::TupleValue(a), Type::TupleValue(b)) => {
            a.slots.len() == b.slots.len()
                && a.slots.iter().zip(&b.slots).all(|(a, b)| same_result(a, b))
        }
        (Type::Tuple(a), Type::Tuple(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| same_type(a, b))
        }
        (Type::Union(a), Type::Union(b)) => same_alternatives(a, b),
        (Type::Union(a), b) => !a.is_empty() && a.iter().all(|a| same_type(a, b)),
        (a, Type::Union(b)) => !b.is_empty() && b.iter().all(|b| same_type(a, b)),
        (Type::Options(a), Type::Options(b)) => {
            a.len() == b.len()
                && a.iter()
                    .all(|(name, ty)| b.get(name).is_some_and(|other| same_type(ty, other)))
        }
        _ => false,
    }
}

/// Extract one finite structural contract and its optional Null alternative.
pub(crate) fn value_shape(ty: &Type) -> Option<(&crate::StructuralInterface, bool)> {
    match ty {
        Type::Structural(shape) => Some((shape, false)),
        Type::Union(parts) if parts.len() == 2 && parts.contains(&Type::Null) => {
            let Type::Structural(shape) = parts.iter().find(|ty| **ty != Type::Null)? else {
                return None;
            };
            Some((shape, true))
        }
        _ => None,
    }
}

/// Split one delimiter outside nested type parentheses, rejecting extra pairs.
pub(crate) fn split_pair<'a>(text: &'a str, delimiter: &str) -> Option<(&'a str, &'a str)> {
    let mut depth = 0usize;
    let mut found = None;
    for (at, ch) in text.char_indices() {
        match ch {
            '(' => {
                depth += 1;
                if depth > 64 {
                    return None;
                }
            }
            ')' => depth = depth.checked_sub(1)?,
            _ => {}
        }
        if depth == 0 && text[at..].starts_with(delimiter) {
            if found.is_some() {
                return None;
            }
            found = Some(at);
        }
    }
    if depth != 0 {
        return None;
    }
    let at = found?;
    Some((&text[..at], &text[at + delimiter.len()..]))
}

pub(crate) fn contains(ty: &Type) -> bool {
    match ty {
        Type::Structural(_) => true,
        Type::Union(parts) => parts.iter().any(contains),
        _ => false,
    }
}

/// Resolve a structural field without erasing exact source alternatives.
pub(crate) fn field_contract(receiver: &Type, member: &str) -> Result<ReturnType, String> {
    match receiver {
        Type::Structural(shape) => shape
            .fields
            .get(member)
            .cloned()
            .ok_or_else(|| format!("interface has no field {member}")),
        Type::Union(parts) => {
            let mut fields = parts.iter().map(|ty| field_contract(ty, member));
            let first = fields.next().ok_or("empty receiver union")??;
            fields.try_fold(first, |left, right| {
                crate::joins::call_result(&left, &right?)
                    .ok_or_else(|| "structural fields have incompatible contracts".into())
            })
        }
        _ => Err("field access requires a non-null structural receiver".into()),
    }
}

pub(crate) fn field_type(receiver: &Type, member: &str) -> Result<Type, String> {
    field_contract(receiver, member).map(|ty| ty.api_type())
}

/// Resolve one receiver's declared method without erasing source identities.
/// Inherited Object observers keep their ordinary native signatures.
pub(crate) fn method_contract(receiver: &Type, member: &str) -> Option<StructuralMethod> {
    if let Type::Callback(signature) = receiver {
        return if member == "invoke" {
            Some((**signature).clone())
        } else {
            method_contract(&Type::named("Toybox.Lang.Method"), member)
        };
    }
    if let Type::Structural(shape) = receiver {
        return shape
            .methods
            .get(member)
            .cloned()
            .or_else(|| method_contract(&Type::named("Toybox.Lang.Object"), member));
    }
    let method = crate::api::core().method_for_type(receiver, member)?;
    StructuralMethod::from_native(method.parameters, method.result)
}

/// Join every receiver alternative's method result, retaining source identities.
pub(crate) fn method_result(receiver: &Type, member: &str) -> Option<ReturnType> {
    let mut results = crate::unions::alternatives(receiver)
        .iter()
        .map(|ty| method_contract(ty, member).map(|method| method.result));
    let first = results.next()??;
    results.try_fold(first, |left, right| {
        crate::joins::call_result(&left, &right?)
    })
}

/// Resolve exact class references wherever a signature nests structural types.
pub(crate) fn resolve_parameter<E>(
    ty: &mut ParameterType,
    resolve: &mut impl FnMut(&mut crate::SourceClass) -> Result<(), E>,
) -> Result<(), E> {
    if let Some(mut contract) = ty.source_contract() {
        for class in &mut contract.classes {
            resolve(class)?;
        }
        for ty in &mut contract.native {
            resolve_api(ty, resolve)?;
        }
        *ty = contract.parameter();
    } else if let ParameterType::Tuple(tuple) = ty {
        for slot in &mut tuple.slots {
            resolve_result(slot, resolve)?;
        }
    } else if let ParameterType::Callback(signature) = ty {
        resolve_signature(signature, resolve)?;
    } else if let ParameterType::Structural(shape) = ty {
        shape.resolve_sources(resolve)?;
    } else if let ParameterType::Union(ty) = ty {
        resolve_api(ty, resolve)?;
    } else if let ParameterType::Array(element) = ty {
        resolve_element(element, resolve)?;
    } else if let ParameterType::TypedDictionary(dictionary) = ty {
        resolve_dictionary(dictionary, resolve)?;
    }

    Ok(())
}

pub(crate) fn resolve_result<E>(
    ty: &mut ReturnType,
    resolve: &mut impl FnMut(&mut crate::SourceClass) -> Result<(), E>,
) -> Result<(), E> {
    if let Some(mut contract) = ty.source_contract() {
        for class in &mut contract.classes {
            resolve(class)?;
        }
        for ty in &mut contract.native {
            resolve_api(ty, resolve)?;
        }
        *ty = contract.result();
    } else if let ReturnType::Tuple(tuple) = ty {
        for slot in &mut tuple.slots {
            resolve_result(slot, resolve)?;
        }
    } else if let ReturnType::Callback(signature) = ty {
        resolve_signature(signature, resolve)?;
    } else if let ReturnType::Structural(shape) = ty {
        shape.resolve_sources(resolve)?;
    } else if let ReturnType::Union(ty) = ty {
        resolve_api(ty, resolve)?;
    } else if let ReturnType::Array(element) = ty {
        resolve_element(element, resolve)?;
    } else if let ReturnType::TypedDictionary(dictionary) = ty {
        resolve_dictionary(dictionary, resolve)?;
    }

    Ok(())
}

fn resolve_element<E>(
    element: &mut crate::ArrayElement,
    resolve: &mut impl FnMut(&mut crate::SourceClass) -> Result<(), E>,
) -> Result<(), E> {
    if let crate::ArrayElement::Source(contract) = element {
        for class in &mut contract.classes {
            resolve(class)?;
        }
        for ty in &mut contract.native {
            resolve_api(ty, resolve)?;
        }
    }
    if let crate::ArrayElement::Nested { contract, .. } = element {
        resolve_api(contract, resolve)?;
    }
    if let crate::ArrayElement::Callback { signature, .. } = element {
        resolve_signature(signature, resolve)?;
    }
    if let crate::ArrayElement::Structural { shape, .. } = element {
        shape.resolve_sources(resolve)?;
    }
    Ok(())
}
fn resolve_dictionary<E>(
    dictionary: &mut crate::DictionaryType,
    resolve: &mut impl FnMut(&mut crate::SourceClass) -> Result<(), E>,
) -> Result<(), E> {
    if let crate::DictionaryKey::Callback(signature) = &mut dictionary.key {
        resolve_signature(signature, resolve)?;
    }
    if let crate::DictionaryValue::Callback { signature, .. } = &mut dictionary.value {
        resolve_signature(signature, resolve)?;
    }
    if let crate::DictionaryKey::Structural(shape) = &mut dictionary.key {
        shape.resolve_sources(resolve)?;
    }
    if let crate::DictionaryValue::Structural { shape, .. } = &mut dictionary.value {
        shape.resolve_sources(resolve)?;
    }
    Ok(())
}

fn resolve_signature<E>(
    signature: &mut StructuralMethod,
    resolve: &mut impl FnMut(&mut crate::SourceClass) -> Result<(), E>,
) -> Result<(), E> {
    for parameter in &mut signature.parameters {
        resolve_parameter(parameter, resolve)?;
    }
    resolve_result(&mut signature.result, resolve)
}

fn resolve_api<E>(
    ty: &mut Type,
    resolve: &mut impl FnMut(&mut crate::SourceClass) -> Result<(), E>,
) -> Result<(), E> {
    match ty {
        Type::SourceValue(contract) => {
            for class in &mut contract.classes {
                resolve(class)?;
            }
            for ty in &mut contract.native {
                resolve_api(ty, resolve)?;
            }
        }
        Type::TupleValue(tuple) => {
            for slot in &mut tuple.slots {
                resolve_result(slot, resolve)?;
            }
        }
        Type::Array(element) => resolve_api(element, resolve)?,
        Type::Dictionary { key, value } => {
            resolve_api(key, resolve)?;
            resolve_api(value, resolve)?;
        }
        Type::Callback(signature) => resolve_signature(signature, resolve)?,
        Type::Structural(shape) => shape.resolve_sources(resolve)?,
        Type::Union(parts) => {
            for ty in parts {
                resolve_api(ty, resolve)?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// Required member types for a source `interface` expression.
/// Assignability depends on member shapes. Optional declaration provenance
/// distinguishes tuple-field flow views without introducing a runtime class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuralInterface {
    /// Source declaration provenance for tuple-field flow. This participates in
    /// saved model identity, but structural assignability ignores it.
    pub declaration: Option<StructuralDeclaration>,
    /// Mutable member fields with invariant value contracts.
    pub fields: BTreeMap<String, ReturnType>,
    /// Required instance methods, indexed by member name.
    pub methods: BTreeMap<String, StructuralMethod>,
}

/// A structural declaration's location within the supplied source sequence.
/// Saved IR retains this static identity; it has no VM or runtime representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct StructuralDeclaration {
    /// Index in the source sequence supplied to compilation.
    pub source: usize,
    /// Byte offset of the interface declaration in that source.
    pub offset: usize,
}

/// A fixed-arity instance method required by a structural interface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuralMethod {
    pub parameters: Vec<ParameterType>,
    pub result: ReturnType,
}

pub(crate) fn type_depth(ty: &Type, depth: usize) -> bool {
    if depth > 64 {
        return false;
    }
    match ty {
        Type::SourceValue(contract) => contract.native().iter().all(|ty| type_depth(ty, depth + 1)),
        Type::TupleValue(tuple) => tuple.slots.iter().all(|ty| result_depth(ty, depth + 1)),
        Type::Structural(shape) => shape_depth(shape, depth + 1),
        Type::Callback(signature) => signature_depth(signature, depth + 1),
        Type::Union(parts) | Type::Tuple(parts) => parts.iter().all(|ty| type_depth(ty, depth + 1)),
        Type::Array(ty) => type_depth(ty, depth + 1),
        Type::Dictionary { key, value } => {
            type_depth(key, depth + 1) && type_depth(value, depth + 1)
        }
        Type::Options(fields) => fields.values().all(|ty| type_depth(ty, depth + 1)),
        _ => true,
    }
}
pub(crate) fn result_depth(ty: &ReturnType, depth: usize) -> bool {
    if depth > 64 {
        return false;
    }
    match ty {
        ReturnType::Tuple(tuple) => tuple.slots.iter().all(|ty| result_depth(ty, depth + 1)),
        ReturnType::Structural(shape) => shape_depth(shape, depth + 1),
        ReturnType::Callback(signature) => signature_depth(signature, depth + 1),
        ReturnType::Union(ty) => type_depth(ty, depth + 1),
        ReturnType::SourceUnion(contract) => {
            contract.native().iter().all(|ty| type_depth(ty, depth + 1))
        }
        ReturnType::Array(element) => element_depth(element, depth + 1),
        ReturnType::TypedDictionary(dictionary) => dictionary_depth(dictionary, depth + 1),
        _ => depth <= 64,
    }
}
fn parameter_depth(ty: &ParameterType, depth: usize) -> bool {
    match ty {
        ParameterType::Tuple(tuple) => tuple.slots.iter().all(|ty| result_depth(ty, depth + 1)),
        ParameterType::Structural(shape) => shape_depth(shape, depth + 1),
        ParameterType::Callback(signature) => signature_depth(signature, depth + 1),
        ParameterType::Union(ty) => type_depth(ty, depth + 1),
        ParameterType::SourceUnion(contract) => {
            contract.native().iter().all(|ty| type_depth(ty, depth + 1))
        }
        ParameterType::Array(element) => element_depth(element, depth + 1),
        ParameterType::TypedDictionary(dictionary) => dictionary_depth(dictionary, depth + 1),
        _ => depth <= 64,
    }
}
fn element_depth(element: &crate::ArrayElement, depth: usize) -> bool {
    depth <= 64
        && match element {
            crate::ArrayElement::Source(contract) => {
                contract.native().iter().all(|ty| type_depth(ty, depth + 1))
            }
            crate::ArrayElement::Nested { contract, .. } => type_depth(contract, depth + 1),
            crate::ArrayElement::Union(ty) => type_depth(ty, depth + 1),
            crate::ArrayElement::Callback { signature, .. } => {
                signature_depth(signature, depth + 1)
            }
            crate::ArrayElement::Structural { shape, .. } => shape_depth(shape, depth + 1),
            _ => true,
        }
}
fn dictionary_depth(dictionary: &crate::DictionaryType, depth: usize) -> bool {
    depth <= 64
        && match &dictionary.key {
            crate::DictionaryKey::Callback(signature) => signature_depth(signature, depth + 1),
            crate::DictionaryKey::Structural(shape) => shape_depth(shape, depth + 1),
            _ => true,
        }
        && match &dictionary.value {
            crate::DictionaryValue::Callback { signature, .. } => {
                signature_depth(signature, depth + 1)
            }
            crate::DictionaryValue::Structural { shape, .. } => shape_depth(shape, depth + 1),
            _ => true,
        }
}
fn shape_depth(shape: &StructuralInterface, depth: usize) -> bool {
    depth <= 64
        && shape.fields.values().all(|ty| result_depth(ty, depth + 1))
        && shape.methods.values().all(|method| {
            method
                .parameters
                .iter()
                .all(|ty| parameter_depth(ty, depth + 1))
                && result_depth(&method.result, depth + 1)
        })
}
pub(crate) fn signature_depth(signature: &StructuralMethod, depth: usize) -> bool {
    depth <= 64
        && signature
            .parameters
            .iter()
            .all(|p| parameter_depth(p, depth + 1))
        && result_depth(&signature.result, depth + 1)
}

impl StructuralInterface {
    fn resolve_sources<E>(
        &mut self,
        resolve: &mut impl FnMut(&mut crate::SourceClass) -> Result<(), E>,
    ) -> Result<(), E> {
        for ty in self.fields.values_mut() {
            resolve_result(ty, resolve)?;
        }
        for method in self.methods.values_mut() {
            for ty in &mut method.parameters {
                resolve_parameter(ty, resolve)?;
            }
            resolve_result(&mut method.result, resolve)?;
        }
        Ok(())
    }

    /// Iterate native projections for API availability checks.
    pub(crate) fn api_types(&self) -> impl Iterator<Item = Type> + '_ {
        self.fields
            .values()
            .map(ReturnType::api_type)
            .chain(self.methods.values().flat_map(|m| {
                m.parameters
                    .iter()
                    .map(ParameterType::api_type)
                    .chain(std::iter::once(m.result.api_type()))
            }))
    }

    /// Check supported names, arity, and member contracts before IR use.
    /// Interface declarations do not create runtime class values or method bodies.
    pub fn validate(&self) -> Result<(), String> {
        if self.declaration.is_some() && !self.fields.values().any(crate::field_flow::positional) {
            return Err("declaration provenance requires a tuple-valued field".into());
        }
        if !shape_depth(self, 0) {
            return Err("interface contract nesting exceeds 64 levels".into());
        }
        if self.fields.len() + self.methods.len() > 256 {
            return Err("interface exceeds 256 members".into());
        }
        for name in self.fields.keys().chain(self.methods.keys()) {
            crate::validate_call_names(None, name)?;
        }
        if self
            .fields
            .keys()
            .any(|name| self.methods.contains_key(name))
        {
            return Err("duplicate interface member".into());
        }
        fn valid_result(ty: &ReturnType) -> bool {
            if let Some(contract) = ty.source_contract() {
                crate::SourceUnion::with_native(
                    contract.classes.clone(),
                    contract.native.clone(),
                    contract.nullable,
                )
                .is_ok_and(|contract| contract.result() == *ty)
            } else {
                crate::unions::result(ty.api_type()).as_ref() == Some(ty)
            }
        }
        fn valid_parameter(ty: &ParameterType) -> bool {
            if let Some(contract) = ty.source_contract() {
                crate::SourceUnion::with_native(
                    contract.classes.clone(),
                    contract.native.clone(),
                    contract.nullable,
                )
                .is_ok_and(|contract| contract.parameter() == *ty)
            } else {
                crate::unions::parameter(ty.api_type()).as_ref() == Some(ty)
            }
        }
        for ty in self.fields.values() {
            if !valid_result(ty) {
                return Err("unsupported interface field contract".into());
            }
        }
        for method in self.methods.values() {
            if method.parameters.len() > crate::MAX_PARAMETERS
                || method.parameters.iter().any(|ty| !valid_parameter(ty))
                || (method.result != ReturnType::Void && !valid_result(&method.result))
            {
                return Err("unsupported interface method signature".into());
            }
        }
        Ok(())
    }

    pub(crate) fn accepts(&self, actual: &Type) -> bool {
        if let Type::Structural(other) = actual {
            return self.fields.iter().all(|(name, ty)| {
                other
                    .fields
                    .get(name)
                    .is_some_and(|other| same_result(ty, other))
            }) && self.methods.iter().all(|(name, method)| {
                other
                    .methods
                    .get(name)
                    .is_some_and(|other| method.accepts(other))
            });
        }
        if let Type::Named(name) = actual {
            if let Some(actual) = Self::from_api(name) {
                return self.accepts(&Type::Structural(Box::new(actual)));
            }
        }
        // Native classes expose no general field catalogue in this profile.
        self.fields.is_empty()
            && self.methods.iter().all(|(name, required)| {
                method_contract(actual, name).is_some_and(|method| required.accepts(&method))
            })
            && crate::native::accepts(&Type::named("Toybox.Lang.Object"), actual)
    }

    pub(crate) fn from_api(name: &str) -> Option<Self> {
        let declaration = crate::api::core().interface(name)?;
        Some(Self {
            declaration: None,
            fields: BTreeMap::new(),
            methods: declaration
                .methods
                .iter()
                .map(|method| {
                    Some((
                        method.name.clone(),
                        StructuralMethod::from_native(
                            method.parameters.clone(),
                            method.result.clone(),
                        )?,
                    ))
                })
                .collect::<Option<_>>()?,
        })
    }

    pub(crate) fn tag(&self) -> String {
        let declaration = self
            .declaration
            .into_iter()
            .map(|d| format!("d({}@{})", d.source, d.offset));
        let fields = self
            .fields
            .iter()
            .map(|(name, ty)| format!("f({name}={})", crate::type_text::result_tag(ty)));
        let methods = self.methods.iter().map(|(name, method)| {
            let params = method
                .parameters
                .iter()
                .map(crate::type_text::parameter_tag)
                .collect::<Vec<_>>()
                .join(";");
            let result = if method.result == ReturnType::Void {
                "void".into()
            } else {
                crate::type_text::result_tag(&method.result).into_owned()
            };
            format!("m({name}({params})={result})")
        });
        format!(
            "interface({})",
            declaration
                .chain(fields)
                .chain(methods)
                .collect::<Vec<_>>()
                .join(";")
        )
    }
}

/// Split saved type components without splitting a nested type expression.
pub(crate) fn split(text: &str, separator: char) -> Result<Vec<&str>, &'static str> {
    let mut depth = 0usize;
    let mut start = 0;
    let mut parts = Vec::new();
    for (at, ch) in text.char_indices() {
        match ch {
            '(' => {
                depth += 1;
                if depth > 64 {
                    return Err("interface type nesting exceeds 64 levels");
                }
            }
            ')' => depth = depth.checked_sub(1).ok_or("unbalanced type tag")?,
            ch if ch == separator && depth == 0 => {
                parts.push(&text[start..at]);
                start = at + ch.len_utf8();
            }
            _ => {}
        }
    }
    if depth != 0 {
        return Err("unbalanced type tag");
    }
    parts.push(&text[start..]);
    Ok(parts)
}

pub(crate) fn parse(tag: &str) -> Result<StructuralInterface, &'static str> {
    let body = tag
        .strip_prefix("interface(")
        .and_then(|s| s.strip_suffix(')'))
        .ok_or("invalid interface tag")?;
    let mut shape = StructuralInterface {
        declaration: None,
        fields: BTreeMap::new(),
        methods: BTreeMap::new(),
    };
    if !body.is_empty() {
        for member in split(body, ';')? {
            if let Some(origin) = member.strip_prefix("d(").and_then(|s| s.strip_suffix(')')) {
                let (source, offset) = origin
                    .split_once('@')
                    .ok_or("invalid structural declaration")?;
                let number = |text: &str| {
                    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
                        return Err("invalid structural declaration location");
                    }
                    text.parse::<usize>()
                        .map_err(|_| "structural declaration location exceeds usize")
                };
                if shape
                    .declaration
                    .replace(StructuralDeclaration {
                        source: number(source)?,
                        offset: number(offset)?,
                    })
                    .is_some()
                {
                    return Err("duplicate structural declaration provenance");
                }
            } else if let Some(field) = member.strip_prefix("f(").and_then(|s| s.strip_suffix(')'))
            {
                let (name, tag) = field.split_once('=').ok_or("invalid interface field tag")?;
                if shape
                    .fields
                    .insert(name.into(), crate::type_text::call_result(tag)?)
                    .is_some()
                {
                    return Err("duplicate interface field");
                }
            } else if let Some(method) = member.strip_prefix("m(").and_then(|s| s.strip_suffix(')'))
            {
                let (name, rest) = method
                    .split_once('(')
                    .ok_or("invalid interface method tag")?;
                let mut depth = 1usize;
                let mut close = None;
                for (at, ch) in rest.char_indices() {
                    if ch == '(' {
                        depth += 1;
                    }
                    if ch == ')' {
                        depth -= 1;
                        if depth == 0 {
                            close = Some(at);
                            break;
                        }
                    }
                }
                let close = close.ok_or("invalid interface method result")?;
                let parameters = &rest[..close];
                let result = rest[close + 1..]
                    .strip_prefix('=')
                    .ok_or("invalid interface method result")?;
                let parameters = if parameters.is_empty() {
                    Vec::new()
                } else {
                    split(parameters, ';')?
                        .into_iter()
                        .map(crate::type_text::parameter)
                        .collect::<Result<_, _>>()?
                };
                let result = if result == "void" {
                    ReturnType::Void
                } else {
                    crate::type_text::call_result(result)?
                };
                if shape
                    .methods
                    .insert(name.into(), StructuralMethod { parameters, result })
                    .is_some()
                {
                    return Err("duplicate interface method");
                }
            } else {
                return Err("invalid interface member tag");
            }
        }
    }
    shape.validate().map_err(|_| "invalid interface contract")?;
    Ok(shape)
}

impl StructuralMethod {
    pub(crate) fn from_native(parameters: Vec<Type>, result: Type) -> Option<Self> {
        Some(Self {
            parameters: parameters
                .into_iter()
                .map(crate::unions::parameter)
                .collect::<Option<_>>()?,
            result: if result == Type::Void {
                ReturnType::Void
            } else {
                crate::unions::result(result)?
            },
        })
    }

    pub(crate) fn accepts(&self, actual: &Self) -> bool {
        self.parameters.len() == actual.parameters.len()
            && self
                .parameters
                .iter()
                .zip(&actual.parameters)
                .all(|(a, b)| same_parameter(a, b))
            && same_result(&self.result, &actual.result)
    }
}
