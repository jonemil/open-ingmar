//! Bounded Dictionary contracts shared by source checking and semantic IR.
use crate::api::Type;

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
                _ => return None,
            },
        })
    }
}
