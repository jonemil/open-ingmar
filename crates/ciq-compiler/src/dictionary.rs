//! Bounded Dictionary contracts shared by source checking and semantic IR.
use crate::api::Type;

/// A scalar token stored in a flat Array or Dictionary constant. Numeric/Char values retain
/// their exact VM bits; String values contain text, not DATA addresses or pool aliases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DictionaryScalar {
    Number(u32),
    Double(u64),
    Float(u32),
    Long(u64),
    String(String),
    Boolean(bool),
    Char(u32),
    Null,
}

impl DictionaryScalar {
    pub(crate) fn api_type(&self) -> Type {
        match self {
            Self::Number(_) => Type::named("Toybox.Lang.Number"),
            Self::Double(_) => Type::named("Toybox.Lang.Double"),
            Self::Float(_) => Type::named("Toybox.Lang.Float"),
            Self::Long(_) => Type::named("Toybox.Lang.Long"),
            Self::String(_) => Type::named("Toybox.Lang.String"),
            Self::Boolean(_) => Type::named("Toybox.Lang.Boolean"),
            Self::Char(_) => Type::named("Toybox.Lang.Char"),
            Self::Null => Type::Null,
        }
    }

    pub(crate) fn text(&self) -> String {
        match self {
            Self::Number(n) => format!("number 0x{n:08x}"),
            Self::Double(n) => format!("double 0x{n:016x}"),
            Self::Float(n) => format!("float 0x{n:08x}"),
            Self::Long(n) => format!("long 0x{n:016x}"),
            Self::Char(n) => format!("char 0x{n:08x}"),
            Self::Boolean(b) => format!("boolean {b}"),
            Self::String(s) => format!("string {s:?}"),
            Self::Null => "null".into(),
        }
    }

    pub(crate) fn parse(fields: &[String], at: &mut usize) -> Option<Self> {
        let kind = fields.get(*at)?;
        *at += 1;
        if kind == "null" {
            return Some(Self::Null);
        }
        let value = fields.get(*at)?;
        *at += 1;
        Some(match kind.as_str() {
            "number" => Self::Number(u32::from_str_radix(value.strip_prefix("0x")?, 16).ok()?),
            "long" => Self::Long(u64::from_str_radix(value.strip_prefix("0x")?, 16).ok()?),
            "float" => Self::Float(u32::from_str_radix(value.strip_prefix("0x")?, 16).ok()?),
            "double" => Self::Double(u64::from_str_radix(value.strip_prefix("0x")?, 16).ok()?),
            "char" => Self::Char(u32::from_str_radix(value.strip_prefix("0x")?, 16).ok()?),
            "boolean" => Self::Boolean(match value.as_str() {
                "true" => true,
                "false" => false,
                _ => return None,
            }),
            "string" => Self::String(value.strip_prefix('"')?.into()),
            _ => return None,
        })
    }
}

/// Infer homogeneous scalar contracts or check scalar entries against an expected
/// invariant contract, including native interface witnesses. Null-only values
/// need context; Null keys are unsupported.
pub(crate) fn constant_type(
    entries: &[(DictionaryScalar, DictionaryScalar)],
    expected: Option<DictionaryType>,
) -> Result<DictionaryType, String> {
    let fail = || {
        "Dictionary constants require homogeneous scalar keys and compatible scalar/Null values"
            .to_owned()
    };
    let (first, _) = entries.first().ok_or_else(fail)?;
    let ty = if let Some(expected) = expected {
        expected
    } else {
        let value = entries
            .iter()
            .map(|(_, value)| value)
            .find(|v| !matches!(v, DictionaryScalar::Null))
            .ok_or("Null-only Dictionary values require a typed context")?;
        let value = if entries
            .iter()
            .any(|(_, v)| matches!(v, DictionaryScalar::Null))
        {
            Type::Union(vec![value.api_type(), Type::Null])
        } else {
            value.api_type()
        };
        DictionaryType::from_api_type(&Type::dictionary(first.api_type(), value))
            .ok_or_else(fail)?
    };
    if matches!(
        ty.key,
        DictionaryKey::Interface(_) | crate::DictionaryKey::Structural(_)
    ) {
        return Err("interface Dictionary keys require runtime construction".into());
    }
    let Type::Dictionary { key, value } = ty.api_type() else {
        unreachable!()
    };
    if entries.iter().any(|(k, v)| {
        !crate::native::accepts(&key, &k.api_type())
            || !crate::native::accepts(&value, &v.api_type())
    }) {
        return Err(fail());
    }
    constant_record("constant".into(), entries)
        .encoded_length()
        .map_err(|e| e.to_string())?;
    Ok(ty)
}

/// Materialize a scalar constant with deterministic local string aliases.
/// Only equal scalar keys are coalesced; the last value replaces earlier ones.
pub(crate) fn constant_record(
    alias: String,
    entries: &[(DictionaryScalar, DictionaryScalar)],
) -> ciq_prg::data::dictionaries::Declaration {
    use ciq_prg::data::{
        StringDeclaration,
        dictionaries::{Declaration, Value},
    };
    let mut unique: Vec<&(DictionaryScalar, DictionaryScalar)> = Vec::new();
    for entry in entries {
        // Double NaN never matches a Dictionary key, even itself. Keep these
        // entries distinct; all other supported constant keys compare by bits/text.
        let repeatable =
            !matches!(&entry.0, DictionaryScalar::Double(bits) if f64::from_bits(*bits).is_nan());
        if let Some(position) = unique.iter().position(|old| repeatable && old.0 == entry.0) {
            unique[position] = entry;
        } else {
            unique.push(entry);
        }
    }
    let mut pool: Vec<StringDeclaration> = Vec::new();
    let mut scalar = |value: &DictionaryScalar| match value {
        DictionaryScalar::Null => Value::Null,
        DictionaryScalar::Number(n) => Value::Number(*n),
        DictionaryScalar::Double(n) => Value::Double(*n),
        DictionaryScalar::Float(n) => Value::Float(*n),
        DictionaryScalar::Long(n) => Value::Long(*n),
        DictionaryScalar::Char(n) => Value::Char(*n),
        DictionaryScalar::Boolean(b) => Value::Boolean(*b),
        DictionaryScalar::String(s) => {
            let index = if let Some(index) = pool.iter().position(|p| p.value == *s) {
                index
            } else {
                let index = pool.len();
                pool.push(StringDeclaration {
                    alias: format!("s{index}"),
                    at: None,
                    value: s.clone(),
                });
                index
            };
            Value::String(pool[index].alias.clone())
        }
    };
    let entries = unique
        .into_iter()
        .map(|(key, value)| (scalar(key), scalar(value)))
        .collect();
    Declaration {
        alias,
        at: None,
        pool: (!pool.is_empty()).then_some(pool),
        entries,
    }
}

/// Supported non-null scalar, Method, and interface Dictionary keys.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DictionaryKey {
    Callback(Box<crate::StructuralMethod>),
    /// A non-null source interface key; equality/hash dispatch remain runtime operations.
    Structural(Box<crate::StructuralInterface>),
    /// A non-null Numeric, Integer, or Decimal key, retaining its runtime type.
    Numeric(crate::NumericAlias),
    /// A ByteArray reference; Dictionary key comparison uses runtime identity.
    ByteArray,
    Method,
    Symbol,
    Interface(crate::InterfaceType),
    Number,
    Double,
    Float,
    Long,
    String,
    Boolean,
    Char,
}

impl DictionaryKey {
    /// Return the invariant key contract without erasing numeric alternatives.
    pub fn api_type(&self) -> Type {
        let name = match self {
            Self::Callback(signature) => return Type::Callback(signature.clone()),
            Self::Structural(shape) => return Type::Structural(shape.clone()),
            Self::Numeric(alias) => return alias.api_type(false),
            Self::Interface(crate::InterfaceType::Comparable) => "Comparable",
            Self::Interface(crate::InterfaceType::Comparator) => "Comparator",
            Self::Number => "Number",
            Self::Double => "Double",
            Self::Float => "Float",
            Self::Long => "Long",
            Self::String => "String",
            Self::Boolean => "Boolean",
            Self::Char => "Char",
            Self::Symbol => "Symbol",
            Self::Method => "Method",
            Self::ByteArray => "ByteArray",
        };
        Type::named(format!("Toybox.Lang.{name}"))
    }
}

/// Supported bounded Dictionary value contracts, including explicit nullable values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DictionaryValue {
    /// A retained nested container or recursive reference contract.
    Nested {
        contract: Box<Type>,
        nullable: bool,
    },
    Callback {
        signature: Box<crate::StructuralMethod>,
        nullable: bool,
    },
    /// A source interface value, optionally permitting explicitly stored Null.
    Structural {
        shape: Box<crate::StructuralInterface>,
        nullable: bool,
    },
    /// A non-null erased Object; lookups additionally permit absent-key Null.
    LangObject,
    /// An erased Object or an explicitly stored Null value.
    NullableObject,
    /// A numeric alias, optionally permitting explicitly stored Null values.
    Numeric {
        alias: crate::NumericAlias,
        nullable: bool,
    },
    /// A non-null mutable ByteArray reference.
    ByteArray,
    /// A ByteArray reference or Null; lookups still require a non-null proof.
    NullableByteArray,
    WeakReference,
    NullableWeakReference,
    Method,
    NullableMethod,
    Symbol,
    NullableSymbol,
    Interface(crate::InterfaceType),
    NullableInterface(crate::InterfaceType),
    Number,
    Double,
    NullableDouble,
    Float,
    NullableFloat,
    Long,
    NullableLong,
    NullableNumber,
    String,
    NullableString,
    Boolean,
    NullableBoolean,
    Char,
    NullableChar,
}

/// Invariant key/value contracts for a non-null Dictionary reference.
/// Nested container values retain their parameters; arbitrary value unions remain bounded.
/// Object values may hold nested references without retaining their parameters;
/// numeric typedefs retain their bounded scalar alternatives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DictionaryType {
    pub key: DictionaryKey,
    pub value: DictionaryValue,
}

impl DictionaryType {
    pub(crate) fn validate(&self) -> Result<(), String> {
        crate::recursive::validate(&self.api_type())?;

        if let DictionaryKey::Callback(signature) = &self.key {
            crate::callbacks::validate(signature)?;
        }
        if let DictionaryValue::Callback { signature, .. } = &self.value {
            crate::callbacks::validate(signature)?;
        }
        if let DictionaryKey::Structural(shape) = &self.key {
            shape.validate()?;
        }
        if let DictionaryValue::Structural { shape, .. } = &self.value {
            shape.validate()?;
        }
        Ok(())
    }

    /// Return the generic API type without erasing either mutable parameter.
    pub fn api_type(&self) -> Type {
        if let DictionaryValue::Nested { contract, nullable } = &self.value {
            return Type::dictionary(
                self.key.api_type(),
                if *nullable {
                    (**contract).clone().with_null()
                } else {
                    (**contract).clone()
                },
            );
        }

        let key = self.key.api_type();
        let (value, nullable) = match &self.value {
            DictionaryValue::Nested { .. } => unreachable!(),
            DictionaryValue::Callback {
                signature,
                nullable,
            } => {
                let ty = Type::Callback(signature.clone());
                return Type::dictionary(key, if *nullable { ty.with_null() } else { ty });
            }
            DictionaryValue::Structural { shape, nullable } => {
                let ty = Type::Structural(shape.clone());
                return Type::dictionary(key, if *nullable { ty.with_null() } else { ty });
            }
            DictionaryValue::LangObject => ("Object", false),
            DictionaryValue::NullableObject => ("Object", true),
            DictionaryValue::Numeric { alias, nullable } => {
                return Type::dictionary(key, alias.api_type(*nullable));
            }
            DictionaryValue::Interface(crate::InterfaceType::Comparable) => ("Comparable", false),
            DictionaryValue::Interface(crate::InterfaceType::Comparator) => ("Comparator", false),
            DictionaryValue::NullableInterface(crate::InterfaceType::Comparable) => {
                ("Comparable", true)
            }
            DictionaryValue::NullableInterface(crate::InterfaceType::Comparator) => {
                ("Comparator", true)
            }
            DictionaryValue::Number => ("Number", false),
            DictionaryValue::Double => ("Double", false),
            DictionaryValue::NullableDouble => ("Double", true),
            DictionaryValue::Float => ("Float", false),
            DictionaryValue::NullableFloat => ("Float", true),
            DictionaryValue::Long => ("Long", false),
            DictionaryValue::NullableLong => ("Long", true),
            DictionaryValue::NullableNumber => ("Number", true),
            DictionaryValue::String => ("String", false),
            DictionaryValue::NullableString => ("String", true),
            DictionaryValue::Boolean => ("Boolean", false),
            DictionaryValue::NullableBoolean => ("Boolean", true),
            DictionaryValue::Char => ("Char", false),
            DictionaryValue::Symbol => ("Symbol", false),
            DictionaryValue::Method => ("Method", false),
            DictionaryValue::WeakReference => ("WeakReference", false),
            DictionaryValue::ByteArray => ("ByteArray", false),
            DictionaryValue::NullableByteArray => ("ByteArray", true),
            DictionaryValue::NullableWeakReference => ("WeakReference", true),
            DictionaryValue::NullableMethod => ("Method", true),
            DictionaryValue::NullableSymbol => ("Symbol", true),
            DictionaryValue::NullableChar => ("Char", true),
        };
        let value = Type::named(format!("Toybox.Lang.{value}"));
        Type::dictionary(
            key,
            if nullable {
                Type::Union(vec![value, Type::Null])
            } else {
                value
            },
        )
    }

    pub(crate) fn from_api_type(ty: &Type) -> Option<Self> {
        let Type::Dictionary { key, value } = ty else {
            return None;
        };
        let key = match key.as_ref() {
            Type::Callback(signature) => {
                crate::callbacks::validate(signature).ok()?;
                DictionaryKey::Callback(signature.clone())
            }
            Type::Structural(shape) => {
                shape.validate().ok()?;
                DictionaryKey::Structural(shape.clone())
            }
            ty if crate::NumericAlias::from_api_type(ty).is_some_and(|(_, nullable)| !nullable) => {
                DictionaryKey::Numeric(crate::NumericAlias::from_api_type(ty)?.0)
            }
            Type::Named(name) => match name.as_str() {
                "Toybox.Lang.Comparable" => {
                    DictionaryKey::Interface(crate::InterfaceType::Comparable)
                }
                "Toybox.Lang.Comparator" => {
                    DictionaryKey::Interface(crate::InterfaceType::Comparator)
                }
                "Toybox.Lang.Number" => DictionaryKey::Number,
                "Toybox.Lang.Double" => DictionaryKey::Double,
                "Toybox.Lang.Float" => DictionaryKey::Float,
                "Toybox.Lang.Long" => DictionaryKey::Long,
                "Toybox.Lang.String" => DictionaryKey::String,
                "Toybox.Lang.Boolean" => DictionaryKey::Boolean,
                "Toybox.Lang.Char" => DictionaryKey::Char,
                "Toybox.Lang.Symbol" => DictionaryKey::Symbol,
                "Toybox.Lang.Method" => DictionaryKey::Method,
                "Toybox.Lang.ByteArray" => DictionaryKey::ByteArray,
                _ => return None,
            },
            _ => return None,
        };
        let (nested, nullable) = match crate::unions::alternatives(value) {
            [ty, Type::Null] => (ty, true),
            [ty] => (ty, false),
            _ => (value.as_ref(), false),
        };
        if matches!(
            nested,
            Type::Recursive(_)
                | Type::RecursionRef(_)
                | Type::Array(_)
                | Type::Dictionary { .. }
                | Type::TupleValue(_)
        ) {
            return Some(Self {
                key,
                value: DictionaryValue::Nested {
                    contract: Box::new(nested.clone()),
                    nullable,
                },
            });
        }
        if let Some((alias, nullable)) = crate::NumericAlias::from_api_type(value) {
            return Some(Self {
                key,
                value: DictionaryValue::Numeric { alias, nullable },
            });
        }
        if let Some((signature, nullable)) = crate::callbacks::value_signature(value) {
            crate::callbacks::validate(signature).ok()?;
            return Some(Self {
                key,
                value: DictionaryValue::Callback {
                    signature: Box::new(signature.clone()),
                    nullable,
                },
            });
        }
        if let Some((shape, nullable)) = crate::structural::value_shape(value) {
            shape.validate().ok()?;
            return Some(Self {
                key,
                value: DictionaryValue::Structural {
                    shape: Box::new(shape.clone()),
                    nullable,
                },
            });
        }
        for value_kind in [
            DictionaryValue::LangObject,
            DictionaryValue::NullableObject,
            DictionaryValue::Interface(crate::InterfaceType::Comparable),
            DictionaryValue::Interface(crate::InterfaceType::Comparator),
            DictionaryValue::NullableInterface(crate::InterfaceType::Comparable),
            DictionaryValue::NullableInterface(crate::InterfaceType::Comparator),
            DictionaryValue::Number,
            DictionaryValue::Double,
            DictionaryValue::NullableDouble,
            DictionaryValue::Float,
            DictionaryValue::NullableFloat,
            DictionaryValue::Long,
            DictionaryValue::NullableLong,
            DictionaryValue::NullableNumber,
            DictionaryValue::String,
            DictionaryValue::NullableString,
            DictionaryValue::Boolean,
            DictionaryValue::NullableBoolean,
            DictionaryValue::Symbol,
            DictionaryValue::NullableSymbol,
            DictionaryValue::Method,
            DictionaryValue::NullableMethod,
            DictionaryValue::WeakReference,
            DictionaryValue::ByteArray,
            DictionaryValue::NullableByteArray,
            DictionaryValue::NullableWeakReference,
            DictionaryValue::Char,
            DictionaryValue::NullableChar,
        ] {
            let candidate = Self {
                key: key.clone(),
                value: value_kind,
            };
            if candidate.api_type() == *ty {
                return Some(candidate);
            }
        }
        None
    }

    pub(crate) fn text(&self) -> String {
        let key = match &self.key {
            DictionaryKey::Callback(signature) => {
                return format!(
                    "dictionary({}=>{})",
                    crate::callbacks::tag(signature),
                    self.value_text()
                );
            }
            DictionaryKey::Structural(shape) => {
                return format!("dictionary({}=>{})", shape.tag(), self.value_text());
            }
            DictionaryKey::Numeric(alias) => alias.value_tag(false),
            DictionaryKey::Interface(crate::InterfaceType::Comparable) => "comparable",
            DictionaryKey::Interface(crate::InterfaceType::Comparator) => "comparator",
            DictionaryKey::Number => "integer",
            DictionaryKey::Double => "double",
            DictionaryKey::Float => "float",
            DictionaryKey::Long => "long",
            DictionaryKey::String => "string",
            DictionaryKey::Boolean => "boolean",
            DictionaryKey::Char => "char",
            DictionaryKey::Symbol => "symbol",
            DictionaryKey::Method => "method",
            DictionaryKey::ByteArray => "byte-array",
        };
        let value = self.value_text();
        format!("dictionary({key}=>{value})")
    }

    fn value_text(&self) -> String {
        if let DictionaryValue::Nested { contract, nullable } = &self.value {
            let ty = if *nullable {
                (**contract).clone().with_null()
            } else {
                (**contract).clone()
            };
            return crate::type_text::parameter_tag(
                &crate::unions::parameter(ty).expect("supported nested Dictionary value"),
            )
            .into_owned();
        }

        if let DictionaryValue::Callback {
            signature,
            nullable,
        } = &self.value
        {
            let tag = crate::callbacks::tag(signature);
            return if *nullable {
                format!("union({tag}|null)")
            } else {
                tag
            };
        }
        if let DictionaryValue::Structural { shape, nullable } = &self.value {
            return if *nullable {
                format!("union({}|null)", shape.tag())
            } else {
                shape.tag()
            };
        }
        let value = match &self.value {
            DictionaryValue::Nested { .. }
            | DictionaryValue::Structural { .. }
            | DictionaryValue::Callback { .. } => unreachable!(),
            DictionaryValue::LangObject => "object",
            DictionaryValue::NullableObject => "nullable-object",
            DictionaryValue::Numeric { alias, nullable } => alias.value_tag(*nullable),
            DictionaryValue::Interface(crate::InterfaceType::Comparable) => "comparable",
            DictionaryValue::Interface(crate::InterfaceType::Comparator) => "comparator",
            DictionaryValue::NullableInterface(crate::InterfaceType::Comparable) => {
                "nullable-comparable"
            }
            DictionaryValue::NullableInterface(crate::InterfaceType::Comparator) => {
                "nullable-comparator"
            }
            DictionaryValue::Number => "integer",
            DictionaryValue::Double => "double",
            DictionaryValue::NullableDouble => "nullable-double",
            DictionaryValue::Float => "float",
            DictionaryValue::NullableFloat => "nullable-float",
            DictionaryValue::Long => "long",
            DictionaryValue::NullableLong => "nullable-long",
            DictionaryValue::NullableNumber => "nullable-integer",
            DictionaryValue::String => "string",
            DictionaryValue::NullableString => "nullable-string",
            DictionaryValue::Boolean => "boolean",
            DictionaryValue::NullableBoolean => "nullable-boolean",
            DictionaryValue::Char => "char",
            DictionaryValue::Symbol => "symbol",
            DictionaryValue::Method => "method",
            DictionaryValue::WeakReference => "weak-reference",
            DictionaryValue::ByteArray => "byte-array",
            DictionaryValue::NullableByteArray => "nullable-byte-array",
            DictionaryValue::NullableWeakReference => "nullable-weak-reference",
            DictionaryValue::NullableMethod => "nullable-method",
            DictionaryValue::NullableSymbol => "nullable-symbol",
            DictionaryValue::NullableChar => "nullable-char",
        };
        value.into()
    }

    pub(crate) fn parse(text: &str) -> Option<Self> {
        let body = text.strip_prefix("dictionary(")?.strip_suffix(')')?;
        let (key, value) = crate::structural::split_pair(body, "=>")?;
        let key = crate::type_text::call_result(key).ok()?.api_type();
        let value = crate::type_text::call_result(value).ok()?.api_type();
        Self::from_api_type(&Type::dictionary(key, value))
    }
}

pub(crate) fn record_entries(
    record: &ciq_prg::data::dictionaries::Declaration,
) -> Option<Vec<(DictionaryScalar, DictionaryScalar)>> {
    use ciq_prg::data::dictionaries::Value;
    let scalar = |value: &Value| {
        Some(match value {
            Value::Null => DictionaryScalar::Null,
            Value::Number(n) => DictionaryScalar::Number(*n),
            Value::Double(n) => DictionaryScalar::Double(*n),
            Value::Float(n) => DictionaryScalar::Float(*n),
            Value::Long(n) => DictionaryScalar::Long(*n),
            Value::Char(n) => DictionaryScalar::Char(*n),
            Value::Boolean(b) => DictionaryScalar::Boolean(*b),
            Value::String(alias) => DictionaryScalar::String(
                record
                    .pool
                    .as_ref()?
                    .iter()
                    .find(|s| s.alias == *alias)?
                    .value
                    .clone(),
            ),
        })
    };
    record
        .entries
        .iter()
        .map(|(k, v)| Some((scalar(k)?, scalar(v)?)))
        .collect()
}

/// Infer bounded contracts from source candidate types. Nullable key candidates
/// select their non-null class; the IR checker must prove every actual key safe.
pub(crate) fn inferred_type(entries: &[(Type, Type)]) -> Option<DictionaryType> {
    fn key(ty: &Type) -> Option<Type> {
        if let Some((alias, _)) = crate::NumericAlias::from_api_type(ty) {
            return Some(alias.api_type(false));
        }
        match ty {
            Type::Named(_) => Some(ty.clone()),
            Type::Union(types) => {
                let mut non_null = types.iter().filter(|ty| **ty != Type::Null);
                let first = non_null.next()?;
                if non_null.next().is_none() {
                    key(first)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
    fn value(ty: &Type, scalar: &mut Option<Type>, nullable: &mut bool) -> Option<()> {
        if let Some((alias, has_null)) = crate::NumericAlias::from_api_type(ty) {
            let base = alias.api_type(false);
            if scalar.as_ref().is_some_and(|previous| previous != &base) {
                return None;
            }
            *scalar = Some(base);
            *nullable |= has_null;
            return Some(());
        }
        match ty {
            Type::Named(name)
                if matches!(
                    name.as_str(),
                    "Toybox.Lang.Number"
                        | "Toybox.Lang.Object"
                        | "Toybox.Lang.String"
                        | "Toybox.Lang.Boolean"
                        | "Toybox.Lang.Char"
                        | "Toybox.Lang.Symbol"
                        | "Toybox.Lang.Method"
                        | "Toybox.Lang.WeakReference"
                        | "Toybox.Lang.ByteArray"
                ) =>
            {
                if scalar.as_ref().is_some_and(|previous| previous != ty) {
                    return None;
                }
                *scalar = Some(ty.clone());
            }
            Type::Null => *nullable = true,
            Type::Union(types) => {
                for ty in types {
                    value(ty, scalar, nullable)?;
                }
            }
            _ => return None,
        }
        Some(())
    }
    let first = key(&entries.first()?.0)?;
    let (mut scalar, mut nullable) = (None, false);
    for (k, v) in entries {
        if key(k)? != first {
            return None;
        }
        value(v, &mut scalar, &mut nullable)?;
    }
    let value = scalar?;
    DictionaryType::from_api_type(&Type::dictionary(
        first,
        if nullable { value.with_null() } else { value },
    ))
}
