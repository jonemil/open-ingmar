//! Bounded Dictionary contracts shared by source checking and semantic IR.
use crate::api::Type;

/// A scalar token stored in a flat Dictionary constant. Number/Char values retain
/// their VM bits; String values contain text, not DATA addresses or pool aliases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DictionaryScalar {
    Number(u32),
    String(String),
    Boolean(bool),
    Char(u32),
    Null,
}

impl DictionaryScalar {
    fn api_type(&self) -> Type {
        match self {
            Self::Number(_) => Type::named("Toybox.Lang.Number"),
            Self::String(_) => Type::named("Toybox.Lang.String"),
            Self::Boolean(_) => Type::named("Toybox.Lang.Boolean"),
            Self::Char(_) => Type::named("Toybox.Lang.Char"),
            Self::Null => Type::Null,
        }
    }

    pub(crate) fn text(&self) -> String {
        match self {
            Self::Number(n) => format!("number 0x{n:08x}"),
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

/// Infer homogeneous scalar contracts or check all entries against an expected
/// invariant contract. Null-only values need context; Null keys are unsupported.
pub(crate) fn constant_type(
    entries: &[(DictionaryScalar, DictionaryScalar)],
    expected: Option<DictionaryType>,
) -> Result<DictionaryType, String> {
    let fail =
        || "Dictionary constants require homogeneous scalar keys and scalar/Null values".to_owned();
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
        if let Some(position) = unique.iter().position(|old| old.0 == entry.0) {
            unique[position] = entry;
        } else {
            unique.push(entry);
        }
    }
    let mut pool: Vec<StringDeclaration> = Vec::new();
    let mut scalar = |value: &DictionaryScalar| match value {
        DictionaryScalar::Null => Value::Null,
        DictionaryScalar::Number(n) => Value::Number(*n),
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

/// Supported non-null scalar Dictionary key contracts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DictionaryKey {
    Number,
    String,
    Boolean,
    Char,
}

/// Supported Dictionary value contracts, including explicit nullable values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DictionaryValue {
    Number,
    NullableNumber,
    String,
    NullableString,
    Boolean,
    NullableBoolean,
    Char,
    NullableChar,
}

/// Invariant key/value contracts for a non-null Dictionary reference.
/// This bounded profile excludes nested containers and heterogeneous unions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DictionaryType {
    pub key: DictionaryKey,
    pub value: DictionaryValue,
}

impl DictionaryType {
    /// Return the generic API type without erasing either mutable parameter.
    pub fn api_type(self) -> Type {
        let key = match self.key {
            DictionaryKey::Number => "Number",
            DictionaryKey::String => "String",
            DictionaryKey::Boolean => "Boolean",
            DictionaryKey::Char => "Char",
        };
        let (value, nullable) = match self.value {
            DictionaryValue::Number => ("Number", false),
            DictionaryValue::NullableNumber => ("Number", true),
            DictionaryValue::String => ("String", false),
            DictionaryValue::NullableString => ("String", true),
            DictionaryValue::Boolean => ("Boolean", false),
            DictionaryValue::NullableBoolean => ("Boolean", true),
            DictionaryValue::Char => ("Char", false),
            DictionaryValue::NullableChar => ("Char", true),
        };
        let value = Type::named(format!("Toybox.Lang.{value}"));
        Type::dictionary(
            Type::named(format!("Toybox.Lang.{key}")),
            if nullable {
                Type::Union(vec![value, Type::Null])
            } else {
                value
            },
        )
    }

    pub(crate) fn from_api_type(ty: &Type) -> Option<Self> {
        let Type::Dictionary { key, .. } = ty else {
            return None;
        };
        let key = match key.as_ref() {
            Type::Named(name) => match name.as_str() {
                "Toybox.Lang.Number" => DictionaryKey::Number,
                "Toybox.Lang.String" => DictionaryKey::String,
                "Toybox.Lang.Boolean" => DictionaryKey::Boolean,
                "Toybox.Lang.Char" => DictionaryKey::Char,
                _ => return None,
            },
            _ => return None,
        };
        for value_kind in [
            DictionaryValue::Number,
            DictionaryValue::NullableNumber,
            DictionaryValue::String,
            DictionaryValue::NullableString,
            DictionaryValue::Boolean,
            DictionaryValue::NullableBoolean,
            DictionaryValue::Char,
            DictionaryValue::NullableChar,
        ] {
            let candidate = Self {
                key,
                value: value_kind,
            };
            if candidate.api_type() == *ty {
                return Some(candidate);
            }
        }
        None
    }

    pub(crate) fn text(self) -> String {
        let key = match self.key {
            DictionaryKey::Number => "integer",
            DictionaryKey::String => "string",
            DictionaryKey::Boolean => "boolean",
            DictionaryKey::Char => "char",
        };
        let value = match self.value {
            DictionaryValue::Number => "integer",
            DictionaryValue::NullableNumber => "nullable-integer",
            DictionaryValue::String => "string",
            DictionaryValue::NullableString => "nullable-string",
            DictionaryValue::Boolean => "boolean",
            DictionaryValue::NullableBoolean => "nullable-boolean",
            DictionaryValue::Char => "char",
            DictionaryValue::NullableChar => "nullable-char",
        };
        format!("dictionary({key}=>{value})")
    }

    pub(crate) fn parse(text: &str) -> Option<Self> {
        let (key, value) = text
            .strip_prefix("dictionary(")?
            .strip_suffix(')')?
            .split_once("=>")?;
        Some(Self {
            key: match key {
                "integer" => DictionaryKey::Number,
                "string" => DictionaryKey::String,
                "boolean" => DictionaryKey::Boolean,
                "char" => DictionaryKey::Char,
                _ => return None,
            },
            value: match value {
                "integer" => DictionaryValue::Number,
                "nullable-integer" => DictionaryValue::NullableNumber,
                "string" => DictionaryValue::String,
                "nullable-string" => DictionaryValue::NullableString,
                "boolean" => DictionaryValue::Boolean,
                "nullable-boolean" => DictionaryValue::NullableBoolean,
                "char" => DictionaryValue::Char,
                "nullable-char" => DictionaryValue::NullableChar,
                _ => return None,
            },
        })
    }
}

pub(crate) fn record_entries(
    record: &ciq_prg::data::dictionaries::Declaration,
) -> Option<Vec<(DictionaryScalar, DictionaryScalar)>> {
    use ciq_prg::data::dictionaries::Value;
    let scalar = |value: &Value| {
        Some(match value {
            Value::Null => DictionaryScalar::Null,
            Value::Long(_) | Value::Float(_) | Value::Double(_) => return None,
            Value::Number(n) => DictionaryScalar::Number(*n),
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
    fn key(ty: &Type) -> Option<&Type> {
        match ty {
            Type::Named(_) => Some(ty),
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
        match ty {
            Type::Named(name)
                if matches!(
                    name.as_str(),
                    "Toybox.Lang.Number"
                        | "Toybox.Lang.String"
                        | "Toybox.Lang.Boolean"
                        | "Toybox.Lang.Char"
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
        first.clone(),
        if nullable {
            Type::Union(vec![value, Type::Null])
        } else {
            value
        },
    ))
}
