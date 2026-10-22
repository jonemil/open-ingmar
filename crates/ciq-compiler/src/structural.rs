//! Source structural contracts, independent of runtime class identities.
use crate::api::Type;
use std::collections::BTreeMap;

pub(crate) fn contains(ty: &Type) -> bool {
    match ty {
        Type::Structural(_) => true,
        Type::Union(parts) => parts.iter().any(contains),
        _ => false,
    }
}

pub(crate) fn field_type(receiver: &Type, member: &str) -> Result<Type, String> {
    match receiver {
        Type::Structural(shape) => shape
            .fields
            .get(member)
            .cloned()
            .ok_or_else(|| format!("interface has no field {member}")),
        Type::Union(parts) => parts
            .iter()
            .map(|ty| field_type(ty, member))
            .collect::<Result<Vec<_>, _>>()
            .map(crate::unions::normalize),
        _ => Err("field access requires a non-null structural receiver".into()),
    }
}

/// Required member types for a source `interface` expression.
/// Names are structural: declaration order and typedef names carry no identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuralInterface {
    /// Mutable member fields with invariant value contracts.
    pub fields: BTreeMap<String, Type>,
    /// Required instance methods, indexed by member name.
    pub methods: BTreeMap<String, StructuralMethod>,
}

/// A fixed-arity instance method required by a structural interface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuralMethod {
    pub parameters: Vec<Type>,
    pub result: Type,
}

impl StructuralInterface {
    /// Check supported names, arity, and member contracts before IR use.
    /// Interface declarations do not create runtime class values or method bodies.
    pub fn validate(&self) -> Result<(), String> {
        fn type_depth(ty: &Type, depth: usize) -> bool {
            if depth > 64 {
                return false;
            }
            match ty {
                Type::Structural(shape) => shape_depth(shape, depth + 1),
                Type::Union(parts) | Type::Tuple(parts) => {
                    parts.iter().all(|ty| type_depth(ty, depth + 1))
                }
                Type::Array(ty) => type_depth(ty, depth + 1),
                Type::Dictionary { key, value } => {
                    type_depth(key, depth + 1) && type_depth(value, depth + 1)
                }
                Type::Options(fields) => fields.values().all(|ty| type_depth(ty, depth + 1)),
                _ => true,
            }
        }
        fn shape_depth(shape: &StructuralInterface, depth: usize) -> bool {
            depth <= 64
                && shape
                    .fields
                    .values()
                    .chain(shape.methods.values().flat_map(|method| {
                        method
                            .parameters
                            .iter()
                            .chain(std::iter::once(&method.result))
                    }))
                    .all(|ty| type_depth(ty, depth + 1))
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
        for ty in self.fields.values() {
            if !crate::unions::parameter(ty.clone()).is_some_and(|p| p.api_type() == *ty) {
                return Err("unsupported interface field contract".into());
            }
        }
        for method in self.methods.values() {
            if method.parameters.len() > crate::MAX_PARAMETERS
                || method.parameters.iter().any(|ty| {
                    !crate::unions::parameter(ty.clone()).is_some_and(|p| p.api_type() == *ty)
                })
                || (method.result != Type::Void
                    && !crate::unions::result(method.result.clone())
                        .is_some_and(|r| r.api_type() == method.result))
            {
                return Err("unsupported interface method signature".into());
            }
        }
        Ok(())
    }

    pub(crate) fn accepts(&self, actual: &Type) -> bool {
        if let Type::Structural(other) = actual {
            return self
                .fields
                .iter()
                .all(|(name, ty)| other.fields.get(name) == Some(ty))
                && self.methods.iter().all(|(name, method)| {
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
                crate::api::core()
                    .method_for_type(actual, name)
                    .is_some_and(|actual| {
                        required.accepts(&StructuralMethod {
                            parameters: actual.parameters,
                            result: actual.result,
                        })
                    })
            })
            && crate::api::core()
                .method_for_type(actual, "toString")
                .is_some()
    }

    pub(crate) fn from_api(name: &str) -> Option<Self> {
        let declaration = crate::api::core().interface(name)?;
        Some(Self {
            fields: BTreeMap::new(),
            methods: declaration
                .methods
                .iter()
                .map(|method| {
                    (
                        method.name.clone(),
                        StructuralMethod {
                            parameters: method.parameters.clone(),
                            result: method.result.clone(),
                        },
                    )
                })
                .collect(),
        })
    }

    pub(crate) fn tag(&self) -> String {
        let fields = self.fields.iter().map(|(name, ty)| {
            format!(
                "f({name}={})",
                crate::type_text::result_tag(&crate::unions::result(ty.clone()).unwrap())
            )
        });
        let methods = self.methods.iter().map(|(name, method)| {
            let params = method
                .parameters
                .iter()
                .map(|ty| crate::type_text::result_tag(&crate::unions::result(ty.clone()).unwrap()))
                .collect::<Vec<_>>()
                .join(";");
            let result = if method.result == Type::Void {
                "void".into()
            } else {
                crate::type_text::result_tag(&crate::unions::result(method.result.clone()).unwrap())
                    .into_owned()
            };
            format!("m({name}({params})={result})")
        });
        format!(
            "interface({})",
            fields.chain(methods).collect::<Vec<_>>().join(";")
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
    fn value(tag: &str) -> Result<Type, &'static str> {
        let result = crate::type_text::call_result(tag)?;
        if result.source_contract().is_some() {
            return Err("exact source interface member contracts are not supported");
        }
        Ok(result.api_type())
    }
    let body = tag
        .strip_prefix("interface(")
        .and_then(|s| s.strip_suffix(')'))
        .ok_or("invalid interface tag")?;
    let mut shape = StructuralInterface {
        fields: BTreeMap::new(),
        methods: BTreeMap::new(),
    };
    if !body.is_empty() {
        for member in split(body, ';')? {
            if let Some(field) = member.strip_prefix("f(").and_then(|s| s.strip_suffix(')')) {
                let (name, tag) = field.split_once('=').ok_or("invalid interface field tag")?;
                if shape.fields.insert(name.into(), value(tag)?).is_some() {
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
                        .map(|tag| {
                            let parameter = crate::type_text::parameter(tag)?;
                            if parameter.source_contract().is_some() {
                                return Err(
                                    "exact source interface member contracts are not supported",
                                );
                            }
                            Ok(parameter.api_type())
                        })
                        .collect::<Result<_, _>>()?
                };
                let result = if result == "void" {
                    Type::Void
                } else {
                    value(result)?
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
    pub(crate) fn accepts(&self, actual: &Self) -> bool {
        self == actual
    }
}
