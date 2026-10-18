//! Restricted Monkey C frontend.
//!
//! [`compile`] parses Monkey C source and [`parse_ir`] parses saved IR text.
//! Both produce a validated [`Function`], which [`backend::lower`] converts
//! into typed VM assembly. Typed IR construction uses the same validation.
//! [`compile_sources`] collects global/module functions and instance classes
//! into a [`CompilationUnit`] and checks module/current-receiver calls.
//! [`parse_unit_ir`] resumes the same validated unit from saved text.
//! [`application::build_console_application`] adds generated lifecycle/metadata
//! and returns an unsigned PRG for the supported console bootstrap profile.

pub mod api;
pub mod application;
pub mod backend;
mod bindings;
mod builders;
mod casts;
mod control_flow;
mod dictionary;
pub use bindings::{array_bindings, dictionary_bindings, string_bindings, symbol_bindings};
pub use dictionary::{DictionaryKey, DictionaryScalar, DictionaryType, DictionaryValue};
mod ir_text;
mod joins;
mod names;
mod native;
mod operators;
mod parser;
mod runtime_types;
mod source_ir;
mod source_objects;
mod source_types;
mod type_flow;
mod unit;
pub use ir_text::{parse_ir, parse_ir_with_api};
pub use unit::{
    ClassBase, ClassDeclaration, CompilationUnit, FieldKind, FunctionDeclaration, InstanceField,
    SourceDiagnostic, UnitError, compile_sources, compile_sources_with_api, parse_unit_ir,
    parse_unit_ir_with_api,
};

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub offset: usize,
    pub line: usize,
    pub column: usize,
    pub message: String,
}

impl Diagnostic {
    fn at(source: &str, offset: usize, message: impl Into<String>) -> Self {
        let prefix = &source[..offset];
        Self {
            offset,
            line: prefix.bytes().filter(|b| *b == b'\n').count() + 1,
            column: prefix.rsplit('\n').next().unwrap().chars().count() + 1,
            message: message.into(),
        }
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}: {}", self.line, self.column, self.message)
    }
}

impl std::error::Error for Diagnostic {}

/// A semantic value identity, independent of VM stack slots and symbols.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ValueId(pub u32);

/// Semantic local identity; the backend independently assigns VM slots.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LocalId(pub u32);

/// Private container-construction identity, separate from values and ordinary locals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BuilderId(pub u32);

/// A semantic block identity, independent of assembly labels and CODE offsets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BlockId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Comparisons producing branch conditions; equality also supports null checks.
pub enum ComparisonOp {
    Equal,
    Greater,
    Less,
    LessEqual,
    GreaterEqual,
    NotEqual,
}
impl ComparisonOp {
    pub(crate) fn is_equality(self) -> bool {
        matches!(self, Self::Equal | Self::NotEqual)
    }
}

pub(crate) const MAX_PARAMETERS: usize = 254;
pub(crate) const MAX_LOCALS: usize = 255;
pub(crate) const MAX_OPERATIONS: usize = 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    BitAnd,
    BitOr,
    BitXor,
    ShiftLeft,
    ShiftRight,
}

/// Native class identities supported as non-null object references.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeClass {
    View,
    BehaviorDelegate,
    Dc,
}

impl NativeClass {
    /// Return this class's canonical API identity, independent of symbol binding.
    pub fn name(self) -> &'static str {
        match self {
            Self::View => "Toybox.WatchUi.View",
            Self::BehaviorDelegate => "Toybox.WatchUi.BehaviorDelegate",
            Self::Dc => "Toybox.Graphics.Dc",
        }
    }
    fn from_name(name: &str) -> Option<Self> {
        [Self::View, Self::BehaviorDelegate, Self::Dc]
            .into_iter()
            .find(|class| class.name() == name)
    }
}

/// Supported invariant Array element contracts, independent of VM storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrayElement {
    Number,
    Long,
    NullableLong,
    Float,
    NullableFloat,
    Double,
    NullableDouble,
    NullableNumber,
    NullableChar,
    NullableBoolean,
    Char,
    Boolean,
    String,
    NullableString,
    Object(NativeClass),
}

impl ArrayElement {
    /// Return the API element contract, including its Null alternative if present.
    pub fn api_type(self) -> api::Type {
        let base = api::Type::named(match self {
            Self::Number | Self::NullableNumber => "Toybox.Lang.Number",
            Self::Double | Self::NullableDouble => "Toybox.Lang.Double",
            Self::Float | Self::NullableFloat => "Toybox.Lang.Float",
            Self::Long | Self::NullableLong => "Toybox.Lang.Long",
            Self::Char | Self::NullableChar => "Toybox.Lang.Char",
            Self::Boolean | Self::NullableBoolean => "Toybox.Lang.Boolean",
            Self::String | Self::NullableString => "Toybox.Lang.String",
            Self::Object(class) => class.name(),
        });
        if self.nullable_base().is_some() {
            api::Type::Union(vec![base, api::Type::Null])
        } else {
            base
        }
    }

    fn nullable_base(self) -> Option<Self> {
        match self {
            Self::NullableNumber => Some(Self::Number),
            Self::NullableDouble => Some(Self::Double),
            Self::NullableFloat => Some(Self::Float),
            Self::NullableLong => Some(Self::Long),
            Self::NullableChar => Some(Self::Char),
            Self::NullableBoolean => Some(Self::Boolean),
            Self::NullableString => Some(Self::String),
            _ => None,
        }
    }

    fn from_api_type(ty: &api::Type) -> Option<Self> {
        [
            Self::Number,
            Self::NullableNumber,
            Self::Double,
            Self::NullableDouble,
            Self::Float,
            Self::NullableFloat,
            Self::Long,
            Self::NullableLong,
            Self::NullableChar,
            Self::NullableBoolean,
            Self::Char,
            Self::Boolean,
            Self::String,
            Self::NullableString,
            Self::Object(NativeClass::View),
        ]
        .into_iter()
        .find(|element| element.api_type() == *ty)
    }

    fn text(self) -> &'static str {
        match self {
            Self::Number => "number",
            Self::Double => "double",
            Self::NullableDouble => "nullable-double",
            Self::Float => "float",
            Self::NullableFloat => "nullable-float",
            Self::Long => "long",
            Self::NullableLong => "nullable-long",
            Self::NullableNumber => "nullable-number",
            Self::NullableChar => "nullable-char",
            Self::NullableBoolean => "nullable-boolean",
            Self::Char => "char",
            Self::Boolean => "boolean",
            Self::String => "string",
            Self::NullableString => "nullable-string",
            Self::Object(NativeClass::View) => "view",
            Self::Object(NativeClass::Dc) => "dc",
            Self::Object(NativeClass::BehaviorDelegate) => "behavior-delegate",
        }
    }

    fn array_text(self) -> &'static str {
        match self {
            Self::Number => "number-array",
            Self::Double => "double-array",
            Self::NullableDouble => "nullable-double-array",
            Self::Float => "float-array",
            Self::NullableFloat => "nullable-float-array",
            Self::Long => "long-array",
            Self::NullableLong => "nullable-long-array",
            Self::NullableNumber => "nullable-number-array",
            Self::NullableChar => "nullable-char-array",
            Self::NullableBoolean => "nullable-boolean-array",
            Self::Char => "char-array",
            Self::Boolean => "boolean-array",
            Self::String => "string-array",
            Self::NullableString => "nullable-string-array",
            Self::Object(NativeClass::View) => "view-array",
            Self::Object(NativeClass::Dc) => "dc-array",
            Self::Object(NativeClass::BehaviorDelegate) => "behavior-delegate-array",
        }
    }
}

/// Payload of a homogeneous Array constant. Bits are interpreted as Number or
/// Char by the element contract; Long/Float/Double retain their exact-width bits.
/// String contains text rather than a pool alias.
/// Boolean retains its logical value; Null requires a nullable element contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArrayConstantValue {
    Boolean(bool),
    Bits(u32),
    Double(u64),
    Float(u32),
    Long(u64),
    String(String),
    Null,
}

/// Supported positional native callback results, independent of runtime storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TupleShape {
    OneView,
    ViewDelegate,
}

impl TupleShape {
    /// Return the exact API slot types represented by this shape.
    pub fn api_type(self) -> api::Type {
        match self {
            Self::OneView => api::Type::Tuple(vec![api::Type::named(NativeClass::View.name())]),
            Self::ViewDelegate => api::Type::Tuple(vec![
                api::Type::named(NativeClass::View.name()),
                api::Type::named("Toybox.WatchUi.InputDelegate"),
            ]),
        }
    }
}

/// An exact source-class identity and its native API projection.
/// Unit validation checks both fields against the class declaration; a native
/// base alone cannot establish source-class assignability.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourceClass {
    /// Unqualified source identity, resolved against unit declarations.
    pub name: String,
    /// Native base used for API checking, independently of exact identity.
    pub base: ClassBase,
}

impl SourceClass {
    /// Return the unqualified source identity without its native projection.
    pub fn as_str(&self) -> &str {
        &self.name
    }

    /// Parse the saved reference contract `Class@Native.Base`.
    /// Reject malformed names and bases that cannot be source-constructed.
    pub(crate) fn parse(text: &str) -> Option<Self> {
        let (name, base) = text.split_once('@')?;
        validate_call_names(None, name).ok()?;
        let base = match base {
            "Lang.Object" => ClassBase::LangObject,
            "WatchUi.View" => ClassBase::WatchUiView,
            "WatchUi.BehaviorDelegate" => ClassBase::WatchUiBehaviorDelegate,
            _ => return None,
        };
        Some(Self {
            name: name.into(),
            base,
        })
    }
}

impl fmt::Display for SourceClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{}", self.name, self.base.text())
    }
}

/// A supported non-null structural comparison interface.
/// Source implementations require a unit-level member/signature proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterfaceType {
    Comparable,
    Comparator,
}

impl InterfaceType {
    /// Return the canonical API interface name without implying a runtime class.
    pub fn name(self) -> &'static str {
        match self {
            Self::Comparable => "Toybox.Lang.Comparable",
            Self::Comparator => "Toybox.Lang.Comparator",
        }
    }

    pub(crate) fn tag(self) -> &'static str {
        match self {
            Self::Comparable => "comparable",
            Self::Comparator => "comparator",
        }
    }

    pub(crate) fn from_name(name: &str) -> Option<Self> {
        match name {
            "Toybox.Lang.Comparable" => Some(Self::Comparable),
            "Toybox.Lang.Comparator" => Some(Self::Comparator),
            _ => None,
        }
    }
}

/// The supported function result contract, declared explicitly or inferred
/// from uniform supported value/Void returns.
/// Void functions produce no semantic value; parameter contracts are separate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReturnType {
    /// Non-null structural reference with checked comparison members.
    Interface(InterfaceType),
    /// Non-null Dictionary with invariant bounded key/value contracts.
    TypedDictionary(DictionaryType),
    Dictionary,
    /// An opaque Lang.Object value; concrete members require a narrower contract.
    LangObject,
    /// An opaque Lang.Object value or Null, as returned by WeakReference.get.
    NullableObject,
    /// A non-null ordinary WeakReference, distinct from immutable weak results.
    WeakReference,
    /// A non-null bound Method with erased argument and result contracts.
    Method,
    /// A symbol identity assigned a numeric ID during binding.
    Symbol,
    Integer,
    Long,
    NullableLong,
    Float,
    NullableFloat,
    Double,
    NullableDouble,
    NullableInteger,
    String,
    NullableString,
    NullableBoolean,
    NullableChar,
    Boolean,
    Char,
    Object(NativeClass),
    /// Exact non-null source class, resolved within a compilation unit.
    SourceObject(SourceClass),
    Tuple(TupleShape),
    /// Non-null Array with an invariant supported element contract.
    Array(ArrayElement),
    Void,
}

/// Supported explicit parameter contracts, independent of VM slot allocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParameterType {
    /// Non-null structural reference with checked comparison members.
    Interface(InterfaceType),
    /// Non-null Dictionary with invariant bounded key/value contracts.
    TypedDictionary(DictionaryType),
    Dictionary,
    /// An opaque Lang.Object value; concrete members require a narrower contract.
    LangObject,
    /// An opaque Lang.Object value or Null, as returned by WeakReference.get.
    NullableObject,
    /// A non-null ordinary WeakReference, distinct from immutable weak results.
    WeakReference,
    /// A non-null bound Method with erased argument and result contracts.
    Method,
    /// A symbol identity assigned a numeric ID during binding.
    Symbol,
    Integer,
    Long,
    NullableLong,
    Float,
    NullableFloat,
    Double,
    NullableDouble,
    NullableInteger,
    String,
    NullableString,
    NullableBoolean,
    NullableChar,
    Boolean,
    Char,
    Object(NativeClass),
    /// Exact non-null source class, resolved within a compilation unit.
    SourceObject(SourceClass),
    /// Non-null Array with an invariant supported element contract.
    Array(ArrayElement),
    /// An opaque native Dictionary or null; contents are not yet accessible.
    NullableDictionary,
}

impl ParameterType {
    /// Return the API type represented by this supported parameter contract.
    /// Source objects map to their declared native base for API checks; exact
    /// source assignability is checked separately against unit declarations.
    pub fn api_type(&self) -> api::Type {
        match self {
            Self::NullableInteger => ValueType::NullableInteger.api_type(),
            Self::NullableLong => ValueType::NullableLong.api_type(),
            Self::NullableDouble => ValueType::NullableDouble.api_type(),
            Self::NullableFloat => ValueType::NullableFloat.api_type(),
            Self::NullableString => ValueType::NullableString.api_type(),
            Self::NullableBoolean => ValueType::NullableBoolean.api_type(),
            Self::NullableChar => ValueType::NullableChar.api_type(),
            Self::Integer => ValueType::Integer.api_type(),
            Self::Long => ValueType::Long.api_type(),
            Self::Double => ValueType::Double.api_type(),
            Self::Float => ValueType::Float.api_type(),
            Self::String => ValueType::String.api_type(),
            Self::Boolean => ValueType::Boolean.api_type(),
            Self::Char => ValueType::Char.api_type(),
            Self::TypedDictionary(ty) => ty.api_type(),
            Self::Dictionary => ValueType::Dictionary.api_type(),
            Self::LangObject => ValueType::LangObject.api_type(),
            Self::NullableObject => ValueType::NullableObject.api_type(),
            Self::WeakReference => ValueType::WeakReference.api_type(),
            Self::Interface(interface) => api::Type::named(interface.name()),
            Self::Method => ValueType::Method.api_type(),
            Self::Symbol => ValueType::Symbol.api_type(),
            Self::Object(class) => ValueType::Object(*class).api_type(),
            Self::SourceObject(class) => api::Type::named(class.base.api_name()),
            Self::Array(element) => ValueType::Array(*element).api_type(),
            Self::NullableDictionary => ValueType::NullableDictionary.api_type(),
        }
    }
}

impl ReturnType {
    /// Return the API type represented by this supported result contract.
    /// Source objects map to their declared native base without erasing their
    /// separate source-class return requirement.
    pub fn api_type(&self) -> api::Type {
        match self {
            Self::NullableInteger => ValueType::NullableInteger.api_type(),
            Self::NullableLong => ValueType::NullableLong.api_type(),
            Self::NullableDouble => ValueType::NullableDouble.api_type(),
            Self::NullableFloat => ValueType::NullableFloat.api_type(),
            Self::NullableString => ValueType::NullableString.api_type(),
            Self::NullableBoolean => ValueType::NullableBoolean.api_type(),
            Self::NullableChar => ValueType::NullableChar.api_type(),
            Self::Integer => ValueType::Integer.api_type(),
            Self::Long => ValueType::Long.api_type(),
            Self::Double => ValueType::Double.api_type(),
            Self::Float => ValueType::Float.api_type(),
            Self::String => ValueType::String.api_type(),
            Self::Boolean => ValueType::Boolean.api_type(),
            Self::Char => ValueType::Char.api_type(),
            Self::TypedDictionary(ty) => ty.api_type(),
            Self::Dictionary => ValueType::Dictionary.api_type(),
            Self::LangObject => ValueType::LangObject.api_type(),
            Self::NullableObject => ValueType::NullableObject.api_type(),
            Self::WeakReference => ValueType::WeakReference.api_type(),
            Self::Interface(interface) => api::Type::named(interface.name()),
            Self::Method => ValueType::Method.api_type(),
            Self::Symbol => ValueType::Symbol.api_type(),
            Self::Object(class) => ValueType::Object(*class).api_type(),
            Self::SourceObject(class) => api::Type::named(class.base.api_name()),
            Self::Tuple(shape) => shape.api_type(),
            Self::Array(element) => ValueType::Array(*element).api_type(),
            Self::Void => api::Type::Void,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ValueType {
    Interface(InterfaceType),
    TypedDictionary(DictionaryType),
    Dictionary,
    /// An opaque Lang.Object value; concrete members require a narrower contract.
    LangObject,
    /// An opaque Lang.Object value or Null, as returned by WeakReference.get.
    NullableObject,
    /// A non-null ordinary WeakReference, distinct from immutable weak results.
    WeakReference,
    /// A non-null bound Method with erased argument and result contracts.
    Method,
    /// A symbol identity assigned a numeric ID during binding.
    Symbol,
    Integer,
    Long,
    NullableLong,
    Float,
    NullableFloat,
    Double,
    NullableDouble,
    NullableInteger,
    String,
    NullableString,
    NullableBoolean,
    NullableChar,
    Boolean,
    Char,
    Object(NativeClass),
    Tuple(TupleShape),
    FontDefinition,
    Array(ArrayElement),
    Null,
    NullableDictionary,
}

impl ValueType {
    /// Return the non-null alternative of a supported nullable value.
    fn without_null(self) -> Self {
        match self {
            Self::NullableInteger => Self::Integer,
            Self::NullableLong => Self::Long,
            Self::NullableDouble => Self::Double,
            Self::NullableFloat => Self::Float,
            Self::NullableString => Self::String,
            Self::NullableBoolean => Self::Boolean,
            Self::NullableChar => Self::Char,
            Self::NullableObject => Self::LangObject,
            Self::NullableDictionary => Self::Dictionary,
            _ => self,
        }
    }

    /// Map a supported value category to its core API type without changing storage.
    fn api_type(self) -> api::Type {
        match self {
            Self::Integer => api::Type::named("Toybox.Lang.Number"),
            Self::Long => api::Type::named("Toybox.Lang.Long"),
            Self::Double => api::Type::named("Toybox.Lang.Double"),
            Self::NullableDouble => api::Type::Union(vec![
                api::Type::named("Toybox.Lang.Double"),
                api::Type::Null,
            ]),
            Self::Float => api::Type::named("Toybox.Lang.Float"),
            Self::NullableFloat => {
                api::Type::Union(vec![api::Type::named("Toybox.Lang.Float"), api::Type::Null])
            }
            Self::NullableLong => {
                api::Type::Union(vec![api::Type::named("Toybox.Lang.Long"), api::Type::Null])
            }
            Self::NullableInteger => api::Type::Union(vec![
                api::Type::named("Toybox.Lang.Number"),
                api::Type::Null,
            ]),
            Self::String => api::Type::named("Toybox.Lang.String"),
            Self::NullableString => api::Type::Union(vec![
                api::Type::named("Toybox.Lang.String"),
                api::Type::Null,
            ]),
            Self::NullableBoolean => api::Type::Union(vec![
                api::Type::named("Toybox.Lang.Boolean"),
                api::Type::Null,
            ]),
            Self::NullableChar => {
                api::Type::Union(vec![api::Type::named("Toybox.Lang.Char"), api::Type::Null])
            }
            Self::Boolean => api::Type::named("Toybox.Lang.Boolean"),
            Self::Char => api::Type::named("Toybox.Lang.Char"),
            Self::TypedDictionary(ty) => ty.api_type(),
            Self::Dictionary => api::Type::named("Toybox.Lang.Dictionary"),
            Self::LangObject => api::Type::named("Toybox.Lang.Object"),
            Self::NullableObject => {
                api::Type::Union(vec![Self::LangObject.api_type(), api::Type::Null])
            }
            Self::WeakReference => api::Type::named("Toybox.Lang.WeakReference"),
            Self::Interface(interface) => api::Type::named(interface.name()),
            Self::Method => api::Type::named("Toybox.Lang.Method"),
            Self::Symbol => api::Type::named("Toybox.Lang.Symbol"),
            Self::Object(class) => api::Type::named(class.name()),
            Self::Tuple(shape) => shape.api_type(),
            Self::FontDefinition => api::Type::named("Toybox.Graphics.FontDefinition"),
            Self::Array(element) => api::Type::Array(Box::new(element.api_type())),
            Self::Null => api::Type::Null,
            Self::NullableDictionary => api::Type::Union(vec![
                api::Type::named("Toybox.Lang.Dictionary"),
                api::Type::Null,
            ]),
        }
    }

    /// Adapt an API type to the fixed executable value subset; other types
    /// remain unsupported even if the catalogue contains their declarations.
    fn from_api_type(ty: &api::Type) -> Option<Self> {
        match ty {
            ty if *ty == TupleShape::OneView.api_type() => Some(Self::Tuple(TupleShape::OneView)),
            ty if *ty == TupleShape::ViewDelegate.api_type() => {
                Some(Self::Tuple(TupleShape::ViewDelegate))
            }
            api::Type::Dictionary { .. } => {
                DictionaryType::from_api_type(ty).map(Self::TypedDictionary)
            }
            api::Type::Array(element) => ArrayElement::from_api_type(element).map(Self::Array),
            api::Type::Named(name) => match name.as_str() {
                "Toybox.Graphics.FontDefinition" => Some(Self::FontDefinition),
                "Toybox.Lang.Number" => Some(Self::Integer),
                "Toybox.Lang.Long" => Some(Self::Long),
                "Toybox.Lang.Double" => Some(Self::Double),
                "Toybox.Lang.Float" => Some(Self::Float),
                "Toybox.Lang.String" => Some(Self::String),
                "Toybox.Lang.Boolean" => Some(Self::Boolean),
                "Toybox.Lang.Char" => Some(Self::Char),
                "Toybox.Lang.Dictionary" => Some(Self::Dictionary),
                "Toybox.Lang.Object" => Some(Self::LangObject),
                "Toybox.Lang.WeakReference" => Some(Self::WeakReference),
                "Toybox.Lang.Method" => Some(Self::Method),
                "Toybox.Lang.Symbol" => Some(Self::Symbol),
                name if InterfaceType::from_name(name).is_some() => {
                    InterfaceType::from_name(name).map(Self::Interface)
                }
                name => NativeClass::from_name(name).map(Self::Object),
            },
            api::Type::Null => Some(Self::Null),
            _ if *ty == Self::NullableObject.api_type() => Some(Self::NullableObject),
            _ if *ty == Self::NullableDictionary.api_type() => Some(Self::NullableDictionary),
            _ if *ty == Self::NullableInteger.api_type() => Some(Self::NullableInteger),
            _ if *ty == Self::NullableLong.api_type() => Some(Self::NullableLong),
            _ if *ty == Self::NullableDouble.api_type() => Some(Self::NullableDouble),
            _ if *ty == Self::NullableFloat.api_type() => Some(Self::NullableFloat),
            _ if *ty == Self::NullableString.api_type() => Some(Self::NullableString),
            _ if *ty == Self::NullableBoolean.api_type() => Some(Self::NullableBoolean),
            _ if *ty == Self::NullableChar.api_type() => Some(Self::NullableChar),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operation {
    /// Push a named symbol; assembly/application binding supplies its ID.
    Symbol {
        result: ValueId,
        name: String,
    },
    /// Allocate a private Dictionary before evaluating its entries. Length is the
    /// source entry count passed to newd, not the final number of distinct keys.
    DictionaryBegin {
        builder: BuilderId,
        dictionary_type: DictionaryType,
        length: u32,
    },
    /// Insert consecutive source entries, evaluating each key then value before
    /// its store. Duplicate keys still execute every expression and insertion.
    DictionaryFill {
        builder: BuilderId,
        start: u32,
        entries: Vec<(ValueId, ValueId)>,
    },
    /// Expose a fully initialized Dictionary after all source entries are inserted.
    DictionaryFinish {
        result: ValueId,
        builder: BuilderId,
    },
    /// Read a typed Dictionary key, producing its value contract or Null.
    /// Evaluate receiver before key; missing keys do not raise Array bounds errors.
    DictionaryRead {
        result: ValueId,
        dictionary: ValueId,
        key: ValueId,
    },
    /// Insert or replace a typed Dictionary entry without producing a value.
    /// Evaluate receiver, key, and value once in source order. Null values require
    /// a nullable value contract and remain distinct from removing the key.
    DictionaryWrite {
        dictionary: ValueId,
        key: ValueId,
        value: ValueId,
    },
    /// Load a fresh Dictionary from a nonempty flat scalar constant. The type
    /// checks every entry before last-value replacement of equal scalar keys.
    DictionaryConstant {
        result: ValueId,
        dictionary_type: DictionaryType,
        entries: Vec<(DictionaryScalar, DictionaryScalar)>,
    },
    /// Allocate a fresh empty Dictionary with optional invariant key/value contracts.
    DictionaryNew {
        result: ValueId,
        dictionary_type: Option<DictionaryType>,
    },
    /// Define the single incoming stack value immediately after a block marker.
    /// The entry block cannot have a parameter; every predecessor supplies one
    /// assignable value via `JumpValue`. Supported types follow the join profile.
    /// Consume it before call/field/array setup, staging in a local when needed.
    BlockParameter {
        result: ValueId,
        value_type: api::Type,
    },
    /// Transfer one value to a parameterized block, consuming it in this block.
    JumpValue {
        target: BlockId,
        value: ValueId,
    },
    /// Step a numeric value by one or increment a Char, preserving its type.
    /// Nullable operands and Char decrement are rejected.
    /// Target capture and write-back are separate ordered IR operations.
    Step {
        result: ValueId,
        operand: ValueId,
        increment: bool,
    },
    /// Load a fresh mutable flat scalar array from DATA.
    /// Values retain raw Number/Char bits or String text. Arrays must be nonempty
    /// and homogeneous; the payload kind must agree with the element contract.
    ArrayConstant {
        result: ValueId,
        element_type: ArrayElement,
        values: Vec<ArrayConstantValue>,
    },
    /// Allocate a private array builder before its elements are evaluated.
    /// The builder cannot escape as a value; fills must cover every index in order.
    ArrayBegin {
        builder: BuilderId,
        element_type: ArrayElement,
        length: u32,
    },
    /// Initialize the next builder element from one checked value.
    ArrayFill {
        builder: BuilderId,
        index: u32,
        value: ValueId,
    },
    /// Fill consecutive private-builder slots, evaluating and storing each value
    /// before the next. Operands must be single-use values in the same block.
    ArrayFillMany {
        builder: BuilderId,
        start: u32,
        values: Vec<ValueId>,
    },
    /// Expose a completed builder as a typed array value, consuming the builder.
    ArrayFinish {
        result: ValueId,
        builder: BuilderId,
    },
    /// Invoke a source instance method on a receiver with proven constructor
    /// class. Unit validation checks ownership and the declared call contract.
    ObjectCall {
        result: ValueId,
        result_type: ReturnType,
        receiver: ValueId,
        class: String,
        member: String,
        arguments: Vec<ValueId>,
    },
    /// Invoke a proven source instance method and discard its runtime result.
    ObjectCallStatement {
        receiver: ValueId,
        class: String,
        member: String,
        arguments: Vec<ValueId>,
    },
    /// Read the current instance as an opaque Object; requires an instance function.
    SelfValue {
        result: ValueId,
    },
    /// Read a Number or non-null source reference on the current instance.
    /// Unit validation checks the claimed result against the field declaration
    /// and prevents reference reads before constructor initialization.
    SelfFieldRead {
        result: ValueId,
        field: String,
        result_type: ReturnType,
    },
    /// Replace a declared field on the current instance, preserving its type.
    SelfFieldAssign {
        field: String,
        value: ValueId,
    },
    /// Define a supported native constant with its declared enum/value contract.
    NativeConstant {
        result: ValueId,
        module: String,
        member: String,
    },
    /// Allocate a supported source object and invoke its initializer with ordered arguments.
    /// Unit validation resolves the class and checks its initializer contract.
    SourceNew {
        result: ValueId,
        class: String,
        /// Native base projection; unit validation checks the class declaration.
        base: ClassBase,
        arguments: Vec<ValueId>,
    },
    /// Invoke a catalogue-resolved Void member on a value receiver for effects.
    NativeCallStatement {
        receiver: ValueId,
        member: String,
        arguments: Vec<ValueId>,
    },
    /// Construct a fresh View result, optionally followed by a non-null delegate.
    /// Slots must be already evaluated local/parameter reads, so allocation
    /// cannot move before operand effects. The shape has no Array mutation profile.
    TupleNew {
        result: ValueId,
        elements: Vec<ValueId>,
    },
    /// Allocate and initialize one supported native object with no arguments.
    /// Constructor availability is checked separately from class availability;
    /// allocation and initialization failures belong to the runtime.
    NativeNew {
        result: ValueId,
        class: NativeClass,
    },
    /// Allocate an empty array with the specified element contract.
    /// The element contract is explicit; allocation failures belong to the VM.
    ArrayNew {
        result: ValueId,
        element_type: ArrayElement,
    },
    /// Read one element from a non-null typed array using a non-null Number index.
    /// Evaluate array before index, once each. Runtime bounds failures remain
    /// observable; indexes are not clamped or adjusted by the compiler.
    ArrayRead {
        result: ValueId,
        array: ValueId,
        index: ValueId,
    },
    /// Replace a supported array element without producing a value or resizing.
    /// Require a non-null Number index and a value matching the element type.
    /// Evaluate receiver, index, and value once in that order;
    /// runtime bounds checks occur at the write, after operand effects.
    ArrayWrite {
        array: ValueId,
        index: ValueId,
        value: ValueId,
    },
    /// Invert a Boolean or complement a Number/Long, evaluating it once.
    /// Preserve its scalar type and integer width without coercing Null.
    Not {
        result: ValueId,
        operand: ValueId,
    },
    /// Complement all bits of a Number/Long, evaluating the operand once.
    /// Nullable values require a non-null proof; other value types are rejected.
    Complement {
        result: ValueId,
        operand: ValueId,
    },
    /// Return a non-null numeric operand unchanged after evaluating it once.
    /// Other operand types require a separate contract and are rejected.
    Positive {
        result: ValueId,
        operand: ValueId,
    },
    /// Negate one numeric operand with a same-type zero subtraction.
    /// Nullable values require a non-null proof before use.
    Negate {
        result: ValueId,
        operand: ValueId,
    },
    /// Produce a Float from exact IEEE 754 binary32 bits, preserving signed zero
    /// and NaN payloads.
    Float {
        result: ValueId,
        bits: u32,
    },
    /// Produce a Double from exact IEEE 754 binary64 bits, preserving signed zero
    /// and NaN payloads.
    Double {
        result: ValueId,
        bits: u64,
    },
    /// Produce a signed 64-bit Long without narrowing to a Number.
    Long {
        result: ValueId,
        value: i64,
    },
    /// Define a Char from its raw 32-bit representation. This preserves VM bits,
    /// including values outside Unicode scalar ranges; source literals are narrower.
    Char {
        result: ValueId,
        value: u32,
    },
    /// Invoke a catalogue-resolved native module function as a Void statement.
    /// Names are canonical API identities, independent of executable bindings.
    NativeModuleCallStatement {
        module: String,
        member: String,
        arguments: Vec<ValueId>,
    },
    /// Define a Boolean literal, distinct from integer zero/one and null.
    Boolean {
        result: ValueId,
        value: bool,
    },
    /// Start a uniquely named block; the first declared block is entry.
    Block {
        block: BlockId,
    },
    /// Compare supported operands, defining a Boolean in this block.
    /// Assert an expression's lexical type without converting or checking its
    /// runtime value. The caller is responsible for the asserted contract.
    Cast {
        result: ValueId,
        operand: ValueId,
        target: api::Type,
    },
    /// Assert an exact source identity and native projection without a runtime
    /// conversion or check. Unit checking resolves the target declaration.
    SourceCast {
        result: ValueId,
        operand: ValueId,
        class: SourceClass,
    },
    /// Test a value against a unit-resolved source class's runtime class object.
    SourceInstanceOf {
        result: ValueId,
        operand: ValueId,
        class: SourceClass,
    },
    /// Test the runtime class of a value without changing its semantic type.
    InstanceOf {
        result: ValueId,
        operand: ValueId,
        class: String,
    },
    /// Test member existence on a value using a non-null Symbol selector.
    Has {
        result: ValueId,
        receiver: ValueId,
        symbol: ValueId,
    },
    Compare {
        result: ValueId,
        kind: ComparisonOp,
        left: ValueId,
        right: ValueId,
    },
    /// Consume a Boolean and terminate the block with two block targets.
    Branch {
        condition: ValueId,
        on_true: BlockId,
        on_false: BlockId,
    },
    /// Terminate the block with an unconditional edge.
    Jump {
        target: BlockId,
    },
    Integer {
        result: ValueId,
        value: i32,
    },
    Null {
        result: ValueId,
    },
    String {
        result: ValueId,
        value: String,
    },
    Println {
        value: ValueId,
    },
    /// Evaluate and consume a supported non-Void value without retaining it.
    /// Calls and operand effects still execute once in their validated order.
    Discard {
        value: ValueId,
    },
    /// Invoke the native superclass initializer on the current instance.
    /// The supported call has no explicit arguments and no semantic result.
    NativeBaseInitialize {
        base: ClassBase,
    },
    Binary {
        result: ValueId,
        kind: BinaryOp,
        left: ValueId,
        right: ValueId,
    },
    ParameterRead {
        result: ValueId,
        index: usize,
    },
    ModuleCall {
        result: ValueId,
        result_type: ReturnType,
        module: String,
        member: String,
        arguments: Vec<ValueId>,
    },
    /// Invoke a runtime-provided member on an explicit value receiver.
    /// Validation resolves its inherited contract and supported native lowering.
    NativeCall {
        result: ValueId,
        receiver: ValueId,
        member: String,
        arguments: Vec<ValueId>,
    },
    /// Invoke a qualified user method for effects, discarding any VM result.
    /// Unit validation checks its parameter contracts and exact arity.
    ModuleCallStatement {
        module: String,
        member: String,
        arguments: Vec<ValueId>,
    },
    /// Invoke a value-returning member on this method's instance receiver.
    SelfCall {
        result: ValueId,
        result_type: ReturnType,
        member: String,
        arguments: Vec<ValueId>,
    },
    /// Invoke a member on this method's receiver and discard its result.
    SelfCallStatement {
        member: String,
        arguments: Vec<ValueId>,
    },
    LocalInitialize {
        local: LocalId,
        value: ValueId,
    },
    /// Replace an initialized local's value without changing its supported
    /// type or allocating a new local. The right-hand value is consumed first.
    LocalAssign {
        local: LocalId,
        value: ValueId,
    },
    LocalRead {
        result: ValueId,
        local: LocalId,
    },
    /// Move a local value onto its consuming expression and empty the slot.
    /// Subsequent reads require a new initialization; used to release hidden owners.
    LocalTake {
        result: ValueId,
        local: LocalId,
    },
    Return {
        value: ValueId,
    },
    /// Terminate the block without producing a semantic result.
    ReturnVoid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocatedOperation {
    pub operation: Operation,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrError {
    pub operation: Option<usize>,
    pub message: String,
}
impl fmt::Display for IrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}
impl std::error::Error for IrError {}

/// A validated semantic IR function with a name, parameters, and operations.
///
/// Expression values have one use within their defining block and refer only to
/// earlier definitions. Explicit blocks may have cyclic edges and terminate with a
/// return, branch, or jump; an unmarked body is one implicit block. Local reads
/// and assignments require initialization on every incoming path. Assignments
/// retain each local's initializer type. Calls, prints, and assignments require
/// operations in expression evaluation order. Operations cannot be mutated
/// after validation; VM frame slots are assigned during lowering.
/// Locals retain supported value types; printing accepts non-null numeric,
/// String, Boolean, or Char values. Operator contracts determine arithmetic result types. User calls carry
/// result contracts; unit validation checks them against callee signatures.
/// Native callback parameters may carry opaque nullable Dictionary state.
/// Returns consistently produce one supported type or no value. Statement calls
/// discard results; unit validation rejects using a void callee as a value.
/// Instance methods permit self calls and require class ownership at unit
/// validation. Other functions cannot assume an instance receiver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    name: String,
    parameter_count: usize,
    parameter_types: Vec<ParameterType>,
    return_type: ReturnType,
    instance_method: bool,
    operations: Vec<LocatedOperation>,
    value_types: std::collections::BTreeMap<ValueId, ValueType>,
}

impl Function {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn parameter_count(&self) -> usize {
        self.parameter_count
    }

    /// Return the validated result contract shared by all return terminators.
    pub fn return_type(&self) -> ReturnType {
        self.return_type.clone()
    }

    /// Whether this body requires an instance receiver in its hidden context slot.
    pub fn is_instance_method(&self) -> bool {
        self.instance_method
    }

    pub fn operations(&self) -> &[LocatedOperation] {
        &self.operations
    }

    /// Check this immutable function's known unguarded API references against
    /// explicit target and deployment levels. Returns an operation-indexed error
    /// where possible; does not change IR or establish device compatibility.
    pub fn validate_api(&self, context: api::ApiContext) -> Result<(), IrError> {
        Self::construct_for_api(
            self.name.clone(),
            self.parameter_types.clone(),
            self.operations.clone(),
            self.instance_method,
            Some(context),
            Some(self.return_type.clone()),
        )
        .map(|_| ())
    }

    /// Construct and validate a zero-parameter function from IR operations.
    ///
    /// Uses [`Self::with_parameters`] with a parameter count of zero.
    pub fn new(name: String, operations: Vec<LocatedOperation>) -> Result<Self, IrError> {
        Self::with_parameters(name, 0, operations)
    }

    /// Construct and validate a function with positional parameters.
    ///
    /// Parameter count includes unused parameters and excludes the VM context.
    /// Rejects invalid names, spans, value/local dependencies, effect ordering,
    /// incompatible value types or inconsistent returns, malformed or
    /// unreachable blocks, cross-block expression uses, and unsupported
    /// operation or parameter/local counts.
    /// External callee arity and types are not checked here.
    pub fn with_parameters(
        name: String,
        parameter_count: usize,
        operations: Vec<LocatedOperation>,
    ) -> Result<Self, IrError> {
        Self::integer_parameters(name, parameter_count, operations, false)
    }

    /// Construct and validate an instance method with integer positional parameters.
    ///
    /// Enables current-receiver calls. The receiver occupies the hidden VM context
    /// slot, not an explicit parameter. Class ownership and callee signatures are
    /// checked by compilation-unit validation; other Function invariants apply.
    pub fn method(
        name: String,
        parameter_count: usize,
        operations: Vec<LocatedOperation>,
    ) -> Result<Self, IrError> {
        Self::integer_parameters(name, parameter_count, operations, true)
    }

    /// Construct instance-method IR with explicit parameter contracts.
    /// Unit validation checks these against native callbacks or user signatures.
    pub fn method_with_parameter_types(
        name: String,
        parameter_types: Vec<ParameterType>,
        operations: Vec<LocatedOperation>,
    ) -> Result<Self, IrError> {
        Self::construct(name, parameter_types, operations, true)
    }

    /// Construct and validate function IR with explicit positional types.
    /// Callee parameter and result contracts are checked by unit validation.
    pub fn with_parameter_types(
        name: String,
        parameter_types: Vec<ParameterType>,
        operations: Vec<LocatedOperation>,
    ) -> Result<Self, IrError> {
        Self::construct(name, parameter_types, operations, false)
    }

    /// Construct function IR with explicit parameter and result contracts.
    /// Every return must be assignable to the result; nullable contracts accept
    /// Null, their scalar type, or the same union. Other validation is unchanged.
    pub fn with_signature(
        name: String,
        parameter_types: Vec<ParameterType>,
        result: ReturnType,
        operations: Vec<LocatedOperation>,
    ) -> Result<Self, IrError> {
        Self::construct_for_api(name, parameter_types, operations, false, None, Some(result))
    }

    /// Construct instance-method IR with explicit parameter and result contracts.
    /// Unit validation checks ownership, callees, and native callback contracts.
    pub fn method_with_signature(
        name: String,
        parameter_types: Vec<ParameterType>,
        result: ReturnType,
        operations: Vec<LocatedOperation>,
    ) -> Result<Self, IrError> {
        Self::construct_for_api(name, parameter_types, operations, true, None, Some(result))
    }

    /// Return explicit parameter types in source order, excluding the receiver.
    pub fn parameter_types(&self) -> &[ParameterType] {
        &self.parameter_types
    }

    fn integer_parameters(
        name: String,
        count: usize,
        operations: Vec<LocatedOperation>,
        instance: bool,
    ) -> Result<Self, IrError> {
        if count > MAX_PARAMETERS {
            return Err(IrError {
                operation: None,
                message: "function exceeds 254-parameter limit".into(),
            });
        }
        Self::construct(
            name,
            vec![ParameterType::Integer; count],
            operations,
            instance,
        )
    }

    fn construct(
        name: String,
        parameter_types: Vec<ParameterType>,
        operations: Vec<LocatedOperation>,
        instance_method: bool,
    ) -> Result<Self, IrError> {
        Self::construct_for_api(
            name,
            parameter_types,
            operations,
            instance_method,
            None,
            None,
        )
    }

    fn construct_for_api(
        name: String,
        parameter_types: Vec<ParameterType>,
        operations: Vec<LocatedOperation>,
        instance_method: bool,
        context: Option<api::ApiContext>,
        declared_result: Option<ReturnType>,
    ) -> Result<Self, IrError> {
        Self::construct_with_join_inference(
            name,
            parameter_types,
            operations,
            instance_method,
            context,
            declared_result,
            false,
        )
    }

    /// Validate a function, optionally resolving source joins from checked edge
    /// operands. Source joins have forward edges; native/text contracts remain
    /// explicit. Resolved types are stored in the returned immutable function.
    fn construct_with_join_inference(
        name: String,
        parameter_types: Vec<ParameterType>,
        mut operations: Vec<LocatedOperation>,
        instance_method: bool,
        context: Option<api::ApiContext>,
        declared_result: Option<ReturnType>,
        infer_joins: bool,
    ) -> Result<Self, IrError> {
        let source_parameters = parameter_types.iter().filter_map(|ty| {
            if let ParameterType::SourceObject(class) = ty {
                Some(class.as_str())
            } else {
                None
            }
        });
        let source_result = declared_result.as_ref().and_then(|ty| {
            if let ReturnType::SourceObject(class) = ty {
                Some(class.as_str())
            } else {
                None
            }
        });
        for class in source_parameters.chain(source_result) {
            validate_call_names(None, class).map_err(|message| IrError {
                operation: None,
                message: message.into(),
            })?;
        }
        if parameter_types.iter().any(|ty| {
            matches!(ty, ParameterType::SourceObject(class) if class.base.constructed_type().is_none())
        }) || matches!(&declared_result, Some(ReturnType::SourceObject(class)) if class.base.constructed_type().is_none())
        {
            return Err(IrError {
                operation: None,
                message: "source reference requires a constructible native base".into(),
            });
        }
        if parameter_types.iter().any(|ty| {
            matches!(
                ty,
                ParameterType::Array(ArrayElement::Object(class)) if *class != NativeClass::View
            )
        }) || matches!(declared_result, Some(ReturnType::Array(ArrayElement::Object(class))) if class != NativeClass::View)
        {
            return Err(IrError {
                operation: None,
                message: "object arrays support only the View element contract".into(),
            });
        }
        if instance_method && native::callback(ClassBase::ApplicationAppBase, &name).is_some() {
            if let Some(context) = context {
                context
                    .method(api::core(), "Toybox.Application.AppBase", &name)
                    .map_err(|message| IrError {
                        operation: None,
                        message,
                    })?;
            }
        }
        if let Some(context) = context {
            for ty in parameter_types.iter().map(|ty| ty.api_type()) {
                context
                    .check_type(api::core(), &ty)
                    .map_err(|message| IrError {
                        operation: None,
                        message,
                    })?;
            }
        }
        let parameter_count = parameter_types.len();
        if parameter_count > MAX_PARAMETERS {
            return Err(IrError {
                operation: None,
                message: "function exceeds 254-parameter limit".into(),
            });
        }
        let valid_name = name
            .bytes()
            .next()
            .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
            && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_');
        if !valid_name {
            return Err(IrError {
                operation: None,
                message: "invalid function name".into(),
            });
        }
        if operations.is_empty() || operations.len() > MAX_OPERATIONS {
            return Err(IrError {
                operation: None,
                message: "IR requires 1..=1024 operations with a final return".into(),
            });
        }
        let mut return_type = None;
        let mut definitions = std::collections::BTreeMap::<ValueId, usize>::new();
        let structure = control_flow::validate(&operations)?;
        let join_types = joins::validate(&operations)?;
        let incoming_values = if infer_joins {
            joins::incoming_values(&operations)
        } else {
            Default::default()
        };
        let mut inferred_types = std::collections::BTreeMap::new();
        let builders = builders::validate(&structure, &operations)?;
        if parameter_count + builders.len() > MAX_LOCALS {
            return Err(IrError {
                operation: None,
                message: "parameters plus builders exceed 255-slot limit".into(),
            });
        }
        let mut value_blocks = std::collections::BTreeMap::new();
        let mut available_locals = std::collections::BTreeSet::new();
        let local_inputs = control_flow::local_inputs(&structure, &operations);
        let refinement_inputs = type_flow::inputs(&structure, &operations);
        let mut refinements = std::collections::BTreeSet::new();
        let mut types = std::collections::BTreeMap::new();
        let mut locals = std::collections::BTreeSet::new();
        let mut uses = std::collections::BTreeMap::new();
        let mut consumers = std::collections::BTreeMap::new();
        let mut local_types = std::collections::BTreeMap::new();
        let mut origins = source_objects::Origins::new(parameter_types.clone());
        for (index, located) in operations.iter().enumerate() {
            let fail = |message: &str| IrError {
                operation: Some(index),
                message: message.into(),
            };
            if located.span.is_some_and(|span| span.start > span.end) {
                return Err(fail("source span ends before it starts"));
            }
            let type_of = |v| types.get(&v).copied().unwrap_or(ValueType::Integer);
            origins
                .observe(&located.operation, &refinements)
                .map_err(fail)?;
            let (result, operands) = match located.operation {
                Operation::BlockParameter {
                    result,
                    ref value_type,
                } => {
                    let inferred;
                    let value_type = if infer_joins {
                        inferred =
                            joins::infer_type(&incoming_values[&structure.blocks[index]], &types)
                                .map_err(fail)?;
                        inferred_types.insert(index, inferred.clone());
                        &inferred
                    } else {
                        value_type
                    };
                    types.insert(result, ValueType::from_api_type(value_type).unwrap());
                    (Some(result), vec![])
                }
                Operation::JumpValue { target, value } => {
                    if !infer_joins
                        && !native::accepts(&join_types[&target], &type_of(value).api_type())
                    {
                        return Err(fail(
                            "edge value does not match target block parameter type",
                        ));
                    }
                    (None, vec![value])
                }
                Operation::DictionaryConstant {
                    result,
                    dictionary_type,
                    ref entries,
                } => {
                    dictionary::constant_type(entries, Some(dictionary_type))
                        .map_err(|e| fail(&e))?;
                    types.insert(result, ValueType::TypedDictionary(dictionary_type));
                    (Some(result), vec![])
                }
                Operation::DictionaryNew {
                    result,
                    dictionary_type,
                } => {
                    types.insert(
                        result,
                        dictionary_type
                            .map(ValueType::TypedDictionary)
                            .unwrap_or(ValueType::Dictionary),
                    );
                    (Some(result), vec![])
                }
                Operation::DictionaryBegin { .. } | Operation::ArrayBegin { .. } => (None, vec![]),
                Operation::DictionaryFill {
                    builder,
                    ref entries,
                    ..
                } => {
                    let dictionary_type = builders[&builder]
                        .0
                        .dictionary()
                        .expect("validated Dictionary builder");
                    for &(key, value) in entries {
                        native::value_call(
                            &dictionary_type.api_type(),
                            "put",
                            &[type_of(key).api_type(), type_of(value).api_type()],
                            context,
                        )
                        .map_err(|e| fail(&e))?;
                    }
                    (
                        None,
                        entries
                            .iter()
                            .flat_map(|(key, value)| [*key, *value])
                            .collect(),
                    )
                }
                Operation::DictionaryFinish { result, builder } => {
                    types.insert(
                        result,
                        ValueType::TypedDictionary(
                            builders[&builder]
                                .0
                                .dictionary()
                                .expect("validated Dictionary builder"),
                        ),
                    );
                    (Some(result), vec![])
                }
                Operation::ArrayFill { builder, value, .. } => {
                    let expected = builders[&builder]
                        .0
                        .array()
                        .expect("validated Array builder")
                        .api_type();
                    if !native::accepts(&expected, &type_of(value).api_type()) {
                        return Err(fail("array builder element must match its declared type"));
                    }
                    (None, vec![value])
                }
                Operation::ArrayFillMany {
                    builder,
                    ref values,
                    ..
                } => {
                    let expected = builders[&builder]
                        .0
                        .array()
                        .expect("validated Array builder")
                        .api_type();
                    if values
                        .iter()
                        .any(|v| !native::accepts(&expected, &type_of(*v).api_type()))
                    {
                        return Err(fail(
                            "array builder elements must match their declared type",
                        ));
                    }
                    (None, values.clone())
                }
                Operation::ArrayFinish { result, builder } => {
                    types.insert(
                        result,
                        ValueType::Array(
                            builders[&builder]
                                .0
                                .array()
                                .expect("validated Array builder"),
                        ),
                    );
                    (Some(result), vec![])
                }
                Operation::ObjectCall {
                    receiver,
                    ref class,
                    ref member,
                    ref arguments,
                    ..
                }
                | Operation::ObjectCallStatement {
                    receiver,
                    ref class,
                    ref member,
                    ref arguments,
                } => {
                    validate_call_names(Some(class), member).map_err(fail)?;
                    validate_call_arguments(arguments, &types).map_err(fail)?;
                    if !matches!(
                        type_of(receiver),
                        ValueType::Object(NativeClass::View | NativeClass::BehaviorDelegate)
                            | ValueType::LangObject
                    ) {
                        return Err(fail(
                            "source object calls require a supported non-null object receiver",
                        ));
                    }
                    let result = if let Operation::ObjectCall {
                        result,
                        ref result_type,
                        ..
                    } = located.operation
                    {
                        let ty = call_result_type(result_type.clone()).map_err(fail)?;
                        types.insert(result, ty);
                        Some(result)
                    } else {
                        None
                    };
                    (
                        result,
                        std::iter::once(receiver)
                            .chain(arguments.iter().copied())
                            .collect(),
                    )
                }
                Operation::Block { block } => {
                    available_locals = local_inputs[&block].clone();
                    refinements = refinement_inputs[&block].clone();
                    (None, vec![])
                }
                Operation::SourceCast {
                    result,
                    operand,
                    ref class,
                }
                | Operation::SourceInstanceOf {
                    result,
                    operand,
                    ref class,
                } => {
                    let projection =
                        call_result_type(ReturnType::SourceObject(class.clone())).map_err(fail)?;
                    if class.base.constructed_type().is_none() {
                        return Err(fail(
                            "source type target requires a constructible class base",
                        ));
                    }
                    if let Some(context) = context {
                        context
                            .check_class(api::core(), class.base.api_name())
                            .map_err(|message| IrError {
                                operation: Some(index),
                                message,
                            })?;
                    }
                    let ty = if matches!(located.operation, Operation::SourceInstanceOf { .. }) {
                        ValueType::Boolean
                    } else {
                        projection
                    };
                    types.insert(result, ty);
                    (Some(result), vec![operand])
                }
                Operation::Cast {
                    result,
                    operand,
                    ref target,
                } => {
                    if casts::type_tag(target).is_none() {
                        return Err(fail("unsupported lexical cast target"));
                    }
                    if let Some(context) = context {
                        context
                            .check_type(api::core(), target)
                            .map_err(|message| IrError {
                                operation: Some(index),
                                message,
                            })?;
                    }
                    types.insert(
                        result,
                        ValueType::from_api_type(target).expect("supported cast target"),
                    );
                    (Some(result), vec![operand])
                }
                Operation::InstanceOf {
                    result,
                    operand,
                    ref class,
                } => {
                    if runtime_types::symbol(class).is_none() {
                        return Err(fail("unsupported runtime class test"));
                    }
                    if let Some(context) = context {
                        context
                            .check_class(api::core(), class)
                            .map_err(|message| IrError {
                                operation: Some(index),
                                message,
                            })?;
                    }
                    types.insert(result, ValueType::Boolean);
                    (Some(result), vec![operand])
                }
                Operation::Has {
                    result,
                    receiver,
                    symbol,
                } => {
                    let ty = type_of(receiver);
                    if ty == ValueType::Null || ty.without_null() != ty {
                        return Err(fail("has requires a non-null receiver"));
                    }
                    if type_of(symbol) != ValueType::Symbol {
                        return Err(fail("has requires a non-null Symbol selector"));
                    }
                    types.insert(result, ValueType::Boolean);
                    (Some(result), vec![receiver, symbol])
                }
                Operation::Compare {
                    result,
                    left,
                    right,
                    kind,
                } => {
                    let checked_type = |value| {
                        if kind.is_equality()
                            && (type_of(left) == ValueType::Null
                                || type_of(right) == ValueType::Null)
                        {
                            if let Some(&at) = definitions.get(&value) {
                                let declared = match operations[at].operation {
                                    Operation::LocalRead { local, .. }
                                    | Operation::LocalTake { local, .. } => {
                                        local_types.get(&local).copied()
                                    }
                                    Operation::ParameterRead { index, .. } => parameter_types
                                        .get(index)
                                        .and_then(|ty| ValueType::from_api_type(&ty.api_type())),
                                    _ => None,
                                };
                                if let Some(
                                    ty @ (ValueType::NullableInteger
                                    | ValueType::NullableString
                                    | ValueType::NullableBoolean
                                    | ValueType::NullableChar
                                    | ValueType::NullableLong
                                    | ValueType::NullableDouble
                                    | ValueType::NullableFloat
                                    | ValueType::NullableObject
                                    | ValueType::NullableDictionary),
                                ) = declared
                                {
                                    return ty;
                                }
                            }
                        }
                        type_of(value)
                    };
                    let result_type = operators::comparison_result(
                        kind,
                        &checked_type(left).api_type(),
                        &checked_type(right).api_type(),
                    )
                    .and_then(|ty| ValueType::from_api_type(&ty))
                    .ok_or_else(|| {
                        fail("operand types do not match a supported comparison contract")
                    })?;
                    types.insert(result, result_type);
                    (Some(result), vec![left, right])
                }
                Operation::Branch { condition, .. } => {
                    if type_of(condition) != ValueType::Boolean {
                        return Err(fail("branch requires a Boolean value"));
                    }
                    (None, vec![condition])
                }
                Operation::Jump { .. } => (None, vec![]),
                Operation::Char { result, .. } => {
                    types.insert(result, ValueType::Char);
                    (Some(result), vec![])
                }
                Operation::Boolean { result, .. } => {
                    types.insert(result, ValueType::Boolean);
                    (Some(result), vec![])
                }
                Operation::SelfValue { result } => {
                    if !instance_method {
                        return Err(fail("self values require an instance receiver"));
                    }
                    types.insert(result, ValueType::LangObject);
                    (Some(result), vec![])
                }
                Operation::SelfFieldRead {
                    result,
                    ref field,
                    ref result_type,
                } => {
                    if !instance_method {
                        return Err(fail("self field reads require an instance receiver"));
                    }
                    validate_call_names(None, field).map_err(fail)?;
                    if !matches!(
                        result_type,
                        ReturnType::Integer | ReturnType::SourceObject(_)
                    ) {
                        return Err(fail("unsupported instance field result contract"));
                    }
                    types.insert(result, call_result_type(result_type.clone()).map_err(fail)?);
                    (Some(result), vec![])
                }
                Operation::SelfFieldAssign { ref field, value } => {
                    if !instance_method {
                        return Err(fail("self field assignments require an instance receiver"));
                    }
                    validate_call_names(None, field).map_err(fail)?;
                    if type_of(value) != ValueType::Integer && origins.class(value).is_none() {
                        return Err(fail(
                            "instance fields require Number or exact source values",
                        ));
                    }
                    (None, vec![value])
                }
                Operation::Integer { result, .. } => (Some(result), vec![]),
                Operation::Float { result, .. } => {
                    types.insert(result, ValueType::Float);
                    (Some(result), vec![])
                }
                Operation::Double { result, .. } => {
                    types.insert(result, ValueType::Double);
                    (Some(result), vec![])
                }
                Operation::Long { result, .. } => {
                    types.insert(result, ValueType::Long);
                    (Some(result), vec![])
                }
                Operation::Symbol { result, ref name } => {
                    if !ciq_prg::symbols::source::valid_alias(name) {
                        return Err(fail("symbol literal requires an identifier"));
                    }
                    types.insert(result, ValueType::Symbol);
                    (Some(result), vec![])
                }
                Operation::Null { result } => {
                    types.insert(result, ValueType::Null);
                    (Some(result), vec![])
                }
                Operation::String { result, ref value } => {
                    if value.len() > u16::MAX as usize {
                        return Err(fail("string UTF-8 byte length exceeds u16"));
                    }
                    types.insert(result, ValueType::String);
                    (Some(result), vec![])
                }
                Operation::NativeBaseInitialize { base } => {
                    if base == ClassBase::LangObject {
                        return Err(fail("Object has no native initialize method"));
                    }
                    if let Some(context) = context {
                        context
                            .method(api::core(), base.api_name(), "initialize")
                            .map_err(|message| fail(&message))?;
                    }
                    if !instance_method {
                        return Err(fail(
                            "native base initialization requires an instance method",
                        ));
                    }
                    (None, vec![])
                }
                Operation::Println { value } => {
                    if let Some(context) = context {
                        context
                            .function(api::core(), "Toybox.System", "println")
                            .map_err(|message| fail(&message))?;
                    }
                    if !matches!(
                        type_of(value),
                        ValueType::Integer
                            | ValueType::String
                            | ValueType::Boolean
                            | ValueType::Char
                            | ValueType::Long
                            | ValueType::Float
                            | ValueType::Double
                    ) {
                        return Err(fail("println requires a supported non-null scalar"));
                    }
                    (None, vec![value])
                }
                Operation::NativeModuleCallStatement {
                    ref module,
                    ref member,
                    ref arguments,
                } => {
                    let types = arguments
                        .iter()
                        .map(|v| type_of(*v).api_type())
                        .collect::<Vec<_>>();
                    native::module_statement(module, member, &types, context)
                        .map_err(|message| fail(&message))?;
                    (None, arguments.clone())
                }
                Operation::NativeConstant {
                    result,
                    ref module,
                    ref member,
                } => {
                    native::constant(module, member, context).map_err(|message| fail(&message))?;
                    types.insert(result, ValueType::FontDefinition);
                    (Some(result), vec![])
                }
                Operation::SourceNew {
                    result,
                    ref class,
                    base,
                    ref arguments,
                } => {
                    validate_call_names(None, class).map_err(fail)?;
                    let ty = base.constructed_type().ok_or_else(|| {
                        fail("native base has no supported source construction profile")
                    })?;
                    if base == ClassBase::LangObject {
                        if let Some(context) = context {
                            context
                                .check_class(api::core(), base.api_name())
                                .map_err(|message| fail(&message))?;
                        }
                    } else {
                        native::constructor(
                            NativeClass::from_name(base.api_name()).unwrap(),
                            context,
                        )
                        .map_err(|message| fail(&message))?;
                    }
                    validate_call_arguments(arguments, &types).map_err(fail)?;
                    types.insert(result, ValueType::from_api_type(&ty).unwrap());
                    (Some(result), arguments.clone())
                }
                Operation::NativeCallStatement {
                    receiver,
                    ref member,
                    ref arguments,
                } => {
                    let arguments_types = origins.native_arguments(
                        &type_of(receiver).api_type(),
                        member,
                        arguments.iter().map(|v| (*v, type_of(*v).api_type())),
                    );
                    let result = native::value_call(
                        &type_of(receiver).api_type(),
                        member,
                        &arguments_types,
                        context,
                    )
                    .map_err(|message| fail(&message))?;
                    if result != api::Type::Void {
                        return Err(fail("native statement requires a Void result"));
                    }
                    (
                        None,
                        std::iter::once(receiver)
                            .chain(arguments.iter().copied())
                            .collect(),
                    )
                }
                Operation::TupleNew {
                    result,
                    ref elements,
                } => {
                    let shape = match elements.len() {
                        1 => TupleShape::OneView,
                        2 => TupleShape::ViewDelegate,
                        _ => return Err(fail("initial View tuple requires one or two slots")),
                    };
                    let api::Type::Tuple(expected) = shape.api_type() else {
                        unreachable!()
                    };
                    if !expected.iter().zip(elements).all(|(expected, value)| {
                        native::accepts(expected, &type_of(*value).api_type())
                    }) {
                        return Err(fail("initial View tuple slot types do not match its shape"));
                    }
                    if !elements.iter().all(|value| {
                        definitions.get(value).is_some_and(|at| {
                            matches!(
                                operations[*at].operation,
                                Operation::LocalRead { .. }
                                    | Operation::LocalTake { .. }
                                    | Operation::ParameterRead { .. }
                            )
                        })
                    }) {
                        return Err(fail(
                            "tuple slots require already evaluated local or parameter reads",
                        ));
                    }
                    types.insert(result, ValueType::Tuple(shape));
                    (Some(result), elements.clone())
                }
                Operation::NativeNew { result, class } => {
                    native::constructor(class, context).map_err(|message| fail(&message))?;
                    types.insert(result, ValueType::Object(class));
                    (Some(result), vec![])
                }
                Operation::ArrayConstant {
                    result,
                    element_type,
                    ref values,
                } => {
                    if values.is_empty()
                        || !values.iter().all(|value| {
                            matches!(
                                (element_type, value),
                                (
                                    ArrayElement::Boolean | ArrayElement::NullableBoolean,
                                    ArrayConstantValue::Boolean(_)
                                ) | (
                                    ArrayElement::Number
                                        | ArrayElement::NullableNumber
                                        | ArrayElement::Char
                                        | ArrayElement::NullableChar,
                                    ArrayConstantValue::Bits(_)
                                ) | (
                                    ArrayElement::Long | ArrayElement::NullableLong,
                                    ArrayConstantValue::Long(_)
                                ) | (
                                    ArrayElement::Float | ArrayElement::NullableFloat,
                                    ArrayConstantValue::Float(_)
                                ) | (
                                    ArrayElement::Double | ArrayElement::NullableDouble,
                                    ArrayConstantValue::Double(_)
                                ) | (
                                    ArrayElement::String | ArrayElement::NullableString,
                                    ArrayConstantValue::String(_)
                                ) | (
                                    ArrayElement::NullableNumber
                                        | ArrayElement::NullableFloat
                                        | ArrayElement::NullableDouble
                                        | ArrayElement::NullableLong
                                        | ArrayElement::NullableString
                                        | ArrayElement::NullableChar
                                        | ArrayElement::NullableBoolean,
                                    ArrayConstantValue::Null
                                )
                            )
                        })
                    {
                        return Err(fail(
                            "constant payloads must match a nonempty supported Array contract",
                        ));
                    }
                    let ty = ValueType::Array(element_type);
                    if let Some(context) = context {
                        context
                            .check_type(api::core(), &ty.api_type())
                            .map_err(|message| fail(&message))?;
                    }
                    types.insert(result, ty);
                    (Some(result), vec![])
                }
                Operation::ArrayNew {
                    result,
                    element_type,
                } => {
                    if matches!(element_type, ArrayElement::Object(class) if class != NativeClass::View)
                    {
                        return Err(fail("object arrays support only the View element contract"));
                    }
                    let ty = ValueType::Array(element_type);
                    if let Some(context) = context {
                        context
                            .check_type(api::core(), &ty.api_type())
                            .map_err(|message| fail(&message))?;
                    }
                    types.insert(result, ty);
                    (Some(result), vec![])
                }
                Operation::DictionaryRead {
                    result,
                    dictionary,
                    key,
                } => {
                    if !matches!(type_of(dictionary), ValueType::TypedDictionary(_)) {
                        return Err(fail("Dictionary reads require a typed non-null Dictionary"));
                    }
                    let ty = native::value_call(
                        &type_of(dictionary).api_type(),
                        "get",
                        &[type_of(key).api_type()],
                        context,
                    )
                    .map_err(|e| fail(&e))?;
                    types.insert(
                        result,
                        ValueType::from_api_type(&ty)
                            .ok_or_else(|| fail("unsupported Dictionary lookup result"))?,
                    );
                    (Some(result), vec![dictionary, key])
                }
                Operation::DictionaryWrite {
                    dictionary,
                    key,
                    value,
                } => {
                    if !matches!(type_of(dictionary), ValueType::TypedDictionary(_)) {
                        return Err(fail(
                            "Dictionary writes require a typed non-null Dictionary",
                        ));
                    }
                    native::value_call(
                        &type_of(dictionary).api_type(),
                        "put",
                        &[type_of(key).api_type(), type_of(value).api_type()],
                        context,
                    )
                    .map_err(|e| fail(&e))?;
                    (None, vec![dictionary, key, value])
                }
                Operation::ArrayRead {
                    result,
                    array,
                    index,
                } => {
                    let ValueType::Array(element) = type_of(array) else {
                        return Err(fail("array reads require a supported array receiver"));
                    };
                    if type_of(index) != ValueType::Integer {
                        return Err(fail("array reads require a non-null Number index"));
                    }
                    types.insert(
                        result,
                        ValueType::from_api_type(&element.api_type()).unwrap(),
                    );
                    (Some(result), vec![array, index])
                }
                Operation::ArrayWrite {
                    array,
                    index,
                    value,
                } => {
                    let ValueType::Array(element) = type_of(array) else {
                        return Err(fail("array writes require a supported array receiver"));
                    };
                    if type_of(index) != ValueType::Integer {
                        return Err(fail("array writes require a non-null Number index"));
                    }
                    if !native::accepts(&element.api_type(), &type_of(value).api_type()) {
                        return Err(fail(
                            "array write value must match the receiver's element type",
                        ));
                    }
                    (None, vec![array, index, value])
                }
                Operation::Not { result, operand } => {
                    let result_type = operators::not_result(&type_of(operand).api_type())
                        .and_then(|ty| ValueType::from_api_type(&ty))
                        .ok_or_else(|| {
                            fail("unary ! requires a non-null Boolean, Number, or Long")
                        })?;
                    types.insert(result, result_type);
                    (Some(result), vec![operand])
                }
                Operation::Step {
                    result,
                    operand,
                    increment,
                } => {
                    let result_type =
                        operators::step_result(&type_of(operand).api_type(), increment)
                            .and_then(|ty| ValueType::from_api_type(&ty))
                            .ok_or_else(|| {
                                fail(
                                    "step requires a non-null numeric value, or Char for increment",
                                )
                            })?;
                    types.insert(result, result_type);
                    (Some(result), vec![operand])
                }
                Operation::Positive { result, operand } => {
                    let result_type = operators::sign_result(&type_of(operand).api_type())
                        .and_then(|ty| ValueType::from_api_type(&ty))
                        .ok_or_else(|| fail("unary plus requires a non-null numeric value"))?;
                    types.insert(result, result_type);
                    (Some(result), vec![operand])
                }
                Operation::Complement { result, operand } => {
                    let result_type = operators::complement_result(&type_of(operand).api_type())
                        .and_then(|ty| ValueType::from_api_type(&ty))
                        .ok_or_else(|| fail("complement requires a non-null Number or Long"))?;
                    types.insert(result, result_type);
                    (Some(result), vec![operand])
                }
                Operation::Negate { result, operand } => {
                    let result_type = operators::sign_result(&type_of(operand).api_type())
                        .and_then(|ty| ValueType::from_api_type(&ty))
                        .ok_or_else(|| fail("negation requires a non-null numeric value"))?;
                    types.insert(result, result_type);
                    (Some(result), vec![operand])
                }
                Operation::Binary {
                    result,
                    left,
                    right,
                    kind,
                } => {
                    let result_type = operators::binary_result(
                        kind,
                        &type_of(left).api_type(),
                        &type_of(right).api_type(),
                    )
                    .and_then(|ty| ValueType::from_api_type(&ty))
                    .ok_or_else(|| {
                        fail("operand types do not match a supported arithmetic contract")
                    })?;
                    types.insert(result, result_type);
                    (Some(result), vec![left, right])
                }
                Operation::ParameterRead { result, index } => {
                    if index >= parameter_count {
                        return Err(fail("parameter index is outside the function signature"));
                    }
                    let ty = ValueType::from_api_type(&parameter_types[index].api_type())
                        .expect("parameter contracts have supported value types");
                    types.insert(
                        result,
                        type_flow::refine(ty, type_flow::Place::Parameter(index), &refinements),
                    );
                    (Some(result), vec![])
                }
                Operation::Discard { value } => (None, vec![value]),
                Operation::NativeCall {
                    result,
                    receiver,
                    ref member,
                    ref arguments,
                } => {
                    let argument_types = origins.native_arguments(
                        &type_of(receiver).api_type(),
                        member,
                        arguments.iter().map(|v| (*v, type_of(*v).api_type())),
                    );
                    let result_type = native::value_call(
                        &type_of(receiver).api_type(),
                        member,
                        &argument_types,
                        context,
                    )
                    .map_err(|message| fail(&message))?;
                    let result_type = ValueType::from_api_type(&result_type)
                        .ok_or_else(|| fail("native call result type has no supported lowering"))?;
                    types.insert(result, result_type);
                    let mut operands = vec![receiver];
                    operands.extend(arguments);
                    (Some(result), operands)
                }
                Operation::ModuleCall {
                    ref module,
                    ref member,
                    ref arguments,
                    ..
                }
                | Operation::ModuleCallStatement {
                    ref module,
                    ref member,
                    ref arguments,
                } => {
                    validate_call_names(Some(module), member).map_err(fail)?;
                    validate_call_arguments(arguments, &types).map_err(fail)?;
                    let result = if let Operation::ModuleCall {
                        result,
                        ref result_type,
                        ..
                    } = located.operation
                    {
                        types.insert(result, call_result_type(result_type.clone()).map_err(fail)?);
                        Some(result)
                    } else {
                        None
                    };
                    (result, arguments.clone())
                }
                Operation::SelfCall {
                    ref member,
                    ref arguments,
                    ..
                }
                | Operation::SelfCallStatement {
                    ref member,
                    ref arguments,
                } => {
                    if !instance_method {
                        return Err(fail("self calls require an instance method"));
                    }
                    validate_call_names(None, member).map_err(fail)?;
                    validate_call_arguments(arguments, &types).map_err(fail)?;
                    let result = if let Operation::SelfCall {
                        result,
                        ref result_type,
                        ..
                    } = located.operation
                    {
                        types.insert(result, call_result_type(result_type.clone()).map_err(fail)?);
                        Some(result)
                    } else {
                        None
                    };
                    (result, arguments.clone())
                }
                Operation::LocalInitialize { local, value } => {
                    refinements.retain(|fact: &type_flow::Fact| {
                        fact.place() != type_flow::Place::Local(local)
                    });
                    local_types.insert(local, type_of(value));
                    if !locals.insert(local) {
                        return Err(fail("duplicate local initialization"));
                    }
                    available_locals.insert(local);
                    if locals.len() > MAX_LOCALS {
                        return Err(fail("function exceeds 255-local limit"));
                    }
                    if parameter_count + locals.len() + builders.len() > MAX_LOCALS {
                        return Err(fail("parameters plus locals exceed 255-slot limit"));
                    }
                    (None, vec![value])
                }
                Operation::LocalAssign { local, value } => {
                    if !locals.contains(&local) || !available_locals.contains(&local) {
                        return Err(fail(
                            "assignment requires an initialized local on every incoming path",
                        ));
                    }
                    let incoming = type_of(value);
                    if local_types[&local] != incoming
                        && !(matches!(
                            local_types[&local],
                            ValueType::LangObject
                                | ValueType::NullableObject
                                | ValueType::Interface(_)
                        ) && native::accepts(
                            &local_types[&local].api_type(),
                            &incoming.api_type(),
                        ))
                        && !(local_types[&local] == ValueType::Dictionary
                            && matches!(incoming, ValueType::TypedDictionary(_)))
                        && !(local_types[&local] == ValueType::NullableDictionary
                            && incoming == ValueType::Null)
                        && !(local_types[&local] == ValueType::NullableFloat
                            && matches!(incoming, ValueType::Float | ValueType::Null))
                        && !(local_types[&local] == ValueType::NullableDouble
                            && matches!(incoming, ValueType::Double | ValueType::Null))
                        && !(local_types[&local] == ValueType::NullableLong
                            && matches!(incoming, ValueType::Long | ValueType::Null))
                        && !(local_types[&local] == ValueType::NullableInteger
                            && matches!(incoming, ValueType::Integer | ValueType::Null))
                        && !(local_types[&local] == ValueType::NullableBoolean
                            && matches!(incoming, ValueType::Boolean | ValueType::Null))
                        && !(local_types[&local] == ValueType::NullableChar
                            && matches!(incoming, ValueType::Char | ValueType::Null))
                        && !(local_types[&local] == ValueType::NullableString
                            && matches!(incoming, ValueType::String | ValueType::Null))
                    {
                        return Err(fail("assignment must preserve the local's value type"));
                    }
                    refinements.retain(|fact: &type_flow::Fact| {
                        fact.place() != type_flow::Place::Local(local)
                    });
                    (None, vec![value])
                }
                Operation::LocalRead { result, local } | Operation::LocalTake { result, local } => {
                    if !locals.contains(&local) || !available_locals.contains(&local) {
                        return Err(fail(
                            "local read must refer to an earlier initialized local",
                        ));
                    }
                    if matches!(located.operation, Operation::LocalTake { .. }) {
                        available_locals.remove(&local);
                    }
                    let ty = local_types[&local];
                    types.insert(
                        result,
                        type_flow::refine(ty, type_flow::Place::Local(local), &refinements),
                    );
                    (Some(result), vec![])
                }
                Operation::ReturnVoid => {
                    if declared_result
                        .as_ref()
                        .is_some_and(|ty| *ty != ReturnType::Void)
                    {
                        return Err(fail("void return does not match function result contract"));
                    }
                    if return_type
                        .replace(ReturnType::Void)
                        .is_some_and(|previous| previous != ReturnType::Void)
                    {
                        return Err(fail("function cannot mix value and void returns"));
                    }
                    if !structure.explicit && index + 1 != operations.len() {
                        return Err(fail("return must be the final operation"));
                    }
                    (None, vec![])
                }
                Operation::Return { value } => {
                    let result = if let Some(contract) = &declared_result {
                        if let ReturnType::SourceObject(class) = contract {
                            if origins.class(value) != Some(class.as_str()) {
                                return Err(fail(
                                    "return does not preserve its declared source class",
                                ));
                            }
                        }
                        let source_witness = matches!(contract, ReturnType::Interface(_))
                            && origins.class(value).is_some();
                        if !source_witness
                            && !native::accepts(&contract.api_type(), &type_of(value).api_type())
                        {
                            return Err(fail("return type does not match function contract"));
                        }
                        contract.clone()
                    } else {
                        match type_of(value) {
                            ValueType::Integer => ReturnType::Integer,
                            ValueType::Long => ReturnType::Long,
                            ValueType::Double => ReturnType::Double,
                            ValueType::Float => ReturnType::Float,
                            ValueType::String => ReturnType::String,
                            ValueType::Boolean => ReturnType::Boolean,
                            ValueType::Char => ReturnType::Char,
                            ValueType::TypedDictionary(ty) => ReturnType::TypedDictionary(ty),
                            ValueType::Dictionary => ReturnType::Dictionary,
                            ValueType::LangObject => ReturnType::LangObject,
                            ValueType::WeakReference => ReturnType::WeakReference,
                            ValueType::Interface(interface) => ReturnType::Interface(interface),
                            ValueType::Method => ReturnType::Method,
                            ValueType::Symbol => ReturnType::Symbol,
                            ValueType::Object(class) => ReturnType::Object(class),
                            ValueType::Tuple(shape) => ReturnType::Tuple(shape),
                            ValueType::Array(element) => ReturnType::Array(element),
                            _ => {
                                return Err(fail(
                                    "nullable returns require an explicit result contract",
                                ));
                            }
                        }
                    };
                    if return_type
                        .replace(result.clone())
                        .is_some_and(|previous| previous != result)
                    {
                        return Err(fail("function cannot mix different return types"));
                    }
                    if !structure.explicit && index + 1 != operations.len() {
                        return Err(fail("return must be the final operation"));
                    }
                    (None, vec![value])
                }
            };
            if let Some(result) = result {
                if let Some(context) = context {
                    if let Some(ty) = types.get(&result) {
                        context
                            .check_type(api::core(), &ty.api_type())
                            .map_err(|message| fail(&message))?;
                    }
                }
                types.entry(result).or_insert(ValueType::Integer);
            }
            for operand in operands {
                if !definitions.contains_key(&operand) {
                    return Err(fail("operand must refer to an earlier defined value"));
                }
                if value_blocks[&operand] != structure.blocks[index] {
                    return Err(fail(
                        "expression values cannot cross block boundaries; use local reads",
                    ));
                }
                *uses.entry(operand).or_insert(0usize) += 1;
                consumers.insert(operand, index);
            }
            if let Some(result) = result {
                value_blocks.insert(result, structure.blocks[index]);
                if definitions.insert(result, index).is_some() {
                    return Err(fail("duplicate value definition"));
                }
            }
        }
        for (index, inferred) in inferred_types {
            if let Operation::BlockParameter { value_type, .. } = &mut operations[index].operation {
                *value_type = inferred;
            }
        }
        if let (Some(context), Some(result)) = (context, declared_result) {
            context
                .check_type(api::core(), &result.api_type())
                .map_err(|message| IrError {
                    operation: None,
                    message,
                })?;
        }
        if !matches!(
            operations.last().unwrap().operation,
            Operation::Return { .. } | Operation::ReturnVoid
        ) {
            return Err(IrError {
                operation: None,
                message: "IR requires a final return".into(),
            });
        }
        for (value, index) in &definitions {
            if uses.get(value) != Some(&1) {
                return Err(IrError {
                    operation: Some(*index),
                    message: "supported expression IR requires exactly one use per defined value"
                        .into(),
                });
            }
        }
        joins::validate_stack_uses(&operations, &consumers)?;
        if operations.iter().any(|located| {
            matches!(
                located.operation,
                Operation::BlockParameter { .. }
                    | Operation::LocalTake { .. }
                    | Operation::DictionaryNew { .. }
                    | Operation::ModuleCall { .. }
                    | Operation::Binary {
                        kind: BinaryOp::Divide | BinaryOp::Remainder,
                        ..
                    }
                    | Operation::DictionaryRead { .. }
                    | Operation::DictionaryWrite { .. }
                    | Operation::ArrayRead { .. }
                    | Operation::ArrayWrite { .. }
                    | Operation::ArrayNew { .. }
                    | Operation::ArrayConstant { .. }
                    | Operation::DictionaryConstant { .. }
                    | Operation::DictionaryBegin { .. }
                    | Operation::DictionaryFill { .. }
                    | Operation::ArrayBegin { .. }
                    | Operation::ArrayFill { .. }
                    | Operation::ArrayFillMany { .. }
                    | Operation::NativeNew { .. }
                    | Operation::TupleNew { .. }
                    | Operation::SourceNew { .. }
                    | Operation::NativeCallStatement { .. }
                    | Operation::NativeCall { .. }
                    | Operation::ObjectCall { .. }
                    | Operation::ObjectCallStatement { .. }
                    | Operation::NativeModuleCallStatement { .. }
                    | Operation::ModuleCallStatement { .. }
                    | Operation::SelfCall { .. }
                    | Operation::SelfCallStatement { .. }
                    | Operation::Println { .. }
                    | Operation::Discard { .. }
                    | Operation::LocalAssign { .. }
                    | Operation::SelfFieldRead { .. }
                    | Operation::SelfFieldAssign { .. }
                    | Operation::NativeBaseInitialize { .. }
            )
        }) {
            // Calls, prints, and writes make operation order observable. A read
            // defined before a write must not be emitted after that write.
            enum Task {
                Value(ValueId),
                Emit(usize),
            }
            let mut schedule = Vec::new();
            for (index, located) in operations.iter().enumerate() {
                let roots = match &located.operation {
                    Operation::DictionaryWrite {
                        dictionary: array,
                        key: index,
                        value,
                    }
                    | Operation::ArrayWrite {
                        array,
                        index,
                        value,
                    } => vec![*array, *index, *value],
                    Operation::DictionaryBegin { .. } | Operation::ArrayBegin { .. } => vec![],
                    Operation::DictionaryFill { entries, .. } => entries
                        .iter()
                        .flat_map(|(key, value)| [*key, *value])
                        .collect(),
                    Operation::ArrayFillMany { values, .. } => values.clone(),
                    Operation::JumpValue { value, .. } => vec![*value],
                    Operation::ArrayFill { value, .. }
                    | Operation::LocalInitialize { value, .. }
                    | Operation::LocalAssign { value, .. }
                    | Operation::SelfFieldAssign { value, .. }
                    | Operation::Return { value }
                    | Operation::Println { value }
                    | Operation::Discard { value } => vec![*value],
                    Operation::Branch { condition, .. } => vec![*condition],
                    Operation::ObjectCallStatement {
                        receiver,
                        arguments,
                        ..
                    }
                    | Operation::NativeCallStatement {
                        receiver,
                        arguments,
                        ..
                    } => std::iter::once(*receiver)
                        .chain(arguments.iter().copied())
                        .collect(),
                    Operation::ModuleCallStatement { arguments, .. }
                    | Operation::NativeModuleCallStatement { arguments, .. }
                    | Operation::SelfCallStatement { arguments, .. } => arguments.clone(),
                    Operation::Block { .. }
                    | Operation::Jump { .. }
                    | Operation::ReturnVoid
                    | Operation::NativeBaseInitialize { .. } => {
                        schedule.push(index);
                        continue;
                    }
                    _ => continue,
                };
                let mut tasks: Vec<_> = roots.into_iter().rev().map(Task::Value).collect();
                while let Some(task) = tasks.pop() {
                    match task {
                        Task::Emit(at) => schedule.push(at),
                        Task::Value(value) => {
                            let at = definitions[&value];
                            tasks.push(Task::Emit(at));
                            match &operations[at].operation {
                                Operation::Step { operand, .. }
                                | Operation::Positive { operand, .. }
                                | Operation::Complement { operand, .. }
                                | Operation::Negate { operand, .. }
                                | Operation::SourceCast { operand, .. }
                                | Operation::SourceInstanceOf { operand, .. }
                                | Operation::Cast { operand, .. }
                                | Operation::InstanceOf { operand, .. }
                                | Operation::Not { operand, .. } => {
                                    tasks.push(Task::Value(*operand))
                                }
                                Operation::DictionaryRead {
                                    dictionary: left,
                                    key: right,
                                    ..
                                }
                                | Operation::ArrayRead {
                                    array: left,
                                    index: right,
                                    ..
                                }
                                | Operation::Binary { left, right, .. }
                                | Operation::Has {
                                    receiver: left,
                                    symbol: right,
                                    ..
                                }
                                | Operation::Compare { left, right, .. } => {
                                    tasks.extend([Task::Value(*right), Task::Value(*left)])
                                }
                                Operation::ObjectCall {
                                    receiver,
                                    arguments,
                                    ..
                                }
                                | Operation::NativeCall {
                                    receiver,
                                    arguments,
                                    ..
                                } => {
                                    tasks.extend(arguments.iter().rev().map(|v| Task::Value(*v)));
                                    tasks.push(Task::Value(*receiver));
                                }
                                Operation::TupleNew {
                                    elements: arguments,
                                    ..
                                }
                                | Operation::SourceNew { arguments, .. }
                                | Operation::ModuleCall { arguments, .. }
                                | Operation::SelfCall { arguments, .. } => tasks.extend(
                                    arguments.iter().rev().map(|value| Task::Value(*value)),
                                ),
                                _ => {}
                            }
                        }
                    }
                }
                schedule.push(index);
            }
            if let Some((expected, actual)) = schedule
                .iter()
                .enumerate()
                .find(|(expected, actual)| *expected != **actual)
            {
                return Err(IrError {
                    operation: Some(*actual),
                    message: format!(
                        "functions with calls, division/remainder, array reads, prints, or assignments require expression evaluation order in IR (expected operation {expected})"
                    ),
                });
            }
        }
        Ok(Self {
            name,
            parameter_count,
            parameter_types,
            instance_method,
            return_type: return_type.expect("final return establishes result type"),
            operations,
            value_types: types,
        })
    }

    /// Lower this IR function into a typed VM assembly program.
    ///
    /// [`backend::lower`] selects instructions and assigns frame slots. Module
    /// and member aliases remain unresolved for later binding. Deduplicated
    /// string declarations carry content without DATA placement; bind them with
    /// [`string_bindings`]. Container declarations use [`array_bindings`] or
    /// [`dictionary_bindings`]; declarations retain separate identities until
    /// CODE encoding. Output is not a complete PRG.
    pub fn lower(&self) -> ciq_assembly::Program {
        backend::lower(self)
    }

    /// Lower this function and serialize its VM assembly as `.mcasm` text.
    pub fn assembly(&self) -> String {
        self.lower().to_text()
    }

    /// Serialize this function as provisional `ciq-ir` text.
    ///
    /// Retains semantic IDs, operation order, receiver metadata, and optional
    /// source spans.
    /// It can be read by [`parse_ir`]; it is not Garmin MIR.
    pub fn to_ir_text(&self) -> String {
        let parameters = if self.parameter_count == 0 {
            String::new()
        } else {
            format!(" parameters={}", self.parameter_count)
        };
        let receiver = if self.instance_method {
            " receiver=instance"
        } else {
            ""
        };
        let parameter_types = if self
            .parameter_types
            .iter()
            .all(|t| *t == ParameterType::Integer)
        {
            String::new()
        } else {
            format!(
                " parameter-types={}",
                self.parameter_types
                    .iter()
                    .map(|t| {
                        match t {
                            ParameterType::SourceObject(class) => {
                                return format!("source({class})");
                            }
                            ParameterType::NullableInteger => "nullable-integer",
                            ParameterType::NullableLong => "nullable-long",
                            ParameterType::NullableDouble => "nullable-double",
                            ParameterType::NullableFloat => "nullable-float",
                            ParameterType::NullableString => "nullable-string",
                            ParameterType::NullableBoolean => "nullable-boolean",
                            ParameterType::NullableChar => "nullable-char",
                            ParameterType::Integer => "integer",
                            ParameterType::Long => "long",
                            ParameterType::Double => "double",
                            ParameterType::Float => "float",
                            ParameterType::String => "string",
                            ParameterType::Boolean => "boolean",
                            ParameterType::Char => "char",
                            ParameterType::TypedDictionary(ty) => return ty.text(),
                            ParameterType::Dictionary => "dictionary",
                            ParameterType::LangObject => "object",
                            ParameterType::NullableObject => "nullable-object",
                            ParameterType::WeakReference => "weak-reference",
                            ParameterType::Interface(interface) => interface.tag(),
                            ParameterType::Method => "method",
                            ParameterType::Symbol => "symbol",
                            ParameterType::Object(NativeClass::View) => "view",
                            ParameterType::Object(NativeClass::Dc) => "dc",
                            ParameterType::Object(NativeClass::BehaviorDelegate) => {
                                "behavior-delegate"
                            }
                            ParameterType::Array(element) => element.array_text(),
                            ParameterType::NullableDictionary => "nullable-dictionary",
                        }
                        .to_owned()
                    })
                    .collect::<Vec<_>>()
                    .join(":")
            )
        };
        let result = match self.return_type {
            ReturnType::LangObject
            | ReturnType::NullableObject
            | ReturnType::WeakReference
            | ReturnType::Interface(_)
            | ReturnType::Method
            | ReturnType::Symbol
            | ReturnType::Dictionary
            | ReturnType::TypedDictionary(_)
            | ReturnType::NullableInteger
            | ReturnType::NullableString
            | ReturnType::NullableBoolean
            | ReturnType::NullableChar
            | ReturnType::NullableLong
            | ReturnType::NullableDouble
            | ReturnType::NullableFloat
            | ReturnType::SourceObject(_) => call_type_text(&self.return_type),
            _ => String::new(),
        };
        let mut text = format!(
            "ciq-ir\n.function {}{parameters}{receiver}{parameter_types}{result}\n",
            self.name
        );
        for operation in &self.operations {
            text.push_str(&match operation.operation {
                Operation::DictionaryConstant {
                    result,
                    dictionary_type,
                    ref entries,
                } => format!(
                    ".dictionary-constant %{} {} {}",
                    result.0,
                    dictionary_type.text(),
                    entries
                        .iter()
                        .map(|(key, value)| format!("{} {}", key.text(), value.text()))
                        .collect::<Vec<_>>()
                        .join(" ")
                ),
                Operation::DictionaryNew {
                    result,
                    dictionary_type,
                } => format!(
                    ".dictionary-new %{}{}",
                    result.0,
                    dictionary_type
                        .map(|ty| format!(" type={}", ty.text()))
                        .unwrap_or_default()
                ),
                Operation::Block { block } => format!(".block ^{}", block.0),
                Operation::BlockParameter {
                    result,
                    ref value_type,
                } => format!(
                    ".block-value %{} {}",
                    result.0,
                    joins::type_tag(value_type).unwrap()
                ),
                Operation::JumpValue { target, value } => {
                    format!(".jump-value ^{} %{}", target.0, value.0)
                }
                Operation::Branch {
                    condition,
                    on_true,
                    on_false,
                } => format!(".branch %{} ^{} ^{}", condition.0, on_true.0, on_false.0),
                Operation::Jump { target } => format!(".jump ^{}", target.0),
                Operation::Compare {
                    result,
                    kind,
                    left,
                    right,
                } => format!(
                    ".{} %{} %{} %{}",
                    match kind {
                        ComparisonOp::Equal => "equal",
                        ComparisonOp::Greater => "greater",
                        ComparisonOp::Less => "less",
                        ComparisonOp::LessEqual => "less-equal",
                        ComparisonOp::GreaterEqual => "greater-equal",
                        ComparisonOp::NotEqual => "not-equal",
                    },
                    result.0,
                    left.0,
                    right.0
                ),
                Operation::SourceCast {
                    result,
                    operand,
                    ref class,
                } => format!(".cast %{} %{} source({class})", result.0, operand.0),
                Operation::SourceInstanceOf {
                    result,
                    operand,
                    ref class,
                } => format!(".instanceof %{} %{} source({class})", result.0, operand.0),
                Operation::Cast {
                    result,
                    operand,
                    ref target,
                } => format!(
                    ".cast %{} %{} {}",
                    result.0,
                    operand.0,
                    casts::type_tag(target).expect("validated cast target")
                ),
                Operation::InstanceOf {
                    result,
                    operand,
                    ref class,
                } => format!(".instanceof %{} %{} {class}", result.0, operand.0),
                Operation::Has {
                    result,
                    receiver,
                    symbol,
                } => format!(".has %{} %{} %{}", result.0, receiver.0, symbol.0),
                Operation::Null { result } => format!(".null %{}", result.0),
                Operation::NativeModuleCallStatement {
                    ref module,
                    ref member,
                    ref arguments,
                } => {
                    let mut line = format!(".call-native-module-discard {module} {member}");
                    for argument in arguments {
                        line.push_str(&format!(" %{}", argument.0));
                    }
                    line
                }
                Operation::Char { result, value } => format!(".char %{} 0x{value:08x}", result.0),
                Operation::Boolean { result, value } => format!(".boolean %{} {value}", result.0),
                Operation::SelfFieldRead {
                    result,
                    ref field,
                    ref result_type,
                } => {
                    format!(
                        ".read-self %{} {field}{}",
                        result.0,
                        call_type_text(result_type)
                    )
                }
                Operation::SelfFieldAssign { ref field, value } => {
                    format!(".assign-self {field} %{}", value.0)
                }
                Operation::SelfValue { result } => format!(".self %{}", result.0),
                Operation::Symbol { result, ref name } => format!(".symbol %{} {name}", result.0),
                Operation::Integer { result, value } => format!(".integer %{} {value}", result.0),
                Operation::Long { result, value } => format!(".long %{} {value}", result.0),
                Operation::Double { result, bits } => {
                    format!(".double %{} 0x{bits:016x}", result.0)
                }
                Operation::Float { result, bits } => format!(".float %{} 0x{bits:08x}", result.0),
                Operation::String { result, ref value } => {
                    format!(".string %{} {value:?}", result.0)
                }
                Operation::Println { value } => format!(".println %{}", value.0),
                Operation::NativeConstant {
                    result,
                    ref module,
                    ref member,
                } => format!(".constant %{} {module} {member}", result.0),
                Operation::SourceNew {
                    result,
                    ref class,
                    base,
                    ref arguments,
                } => {
                    let args = arguments
                        .iter()
                        .map(|value| format!(" %{}", value.0))
                        .collect::<String>();
                    format!(".new-source %{} {class} {}{args}", result.0, base.text())
                }
                Operation::ObjectCall {
                    result,
                    ref result_type,
                    receiver,
                    ref class,
                    ref member,
                    ref arguments,
                } => {
                    let args = arguments
                        .iter()
                        .map(|v| format!(" %{}", v.0))
                        .collect::<String>();
                    format!(
                        ".call-object %{} %{} {class} {member}{}{args}",
                        result.0,
                        receiver.0,
                        call_type_text(result_type)
                    )
                }
                Operation::ObjectCallStatement {
                    receiver,
                    ref class,
                    ref member,
                    ref arguments,
                } => {
                    let args = arguments
                        .iter()
                        .map(|v| format!(" %{}", v.0))
                        .collect::<String>();
                    format!(
                        ".call-object-discard %{} {class} {member}{args}",
                        receiver.0
                    )
                }
                Operation::NativeCallStatement {
                    receiver,
                    ref member,
                    ref arguments,
                } => format!(
                    ".call-native-discard %{} {member}{}",
                    receiver.0,
                    arguments
                        .iter()
                        .map(|v| format!(" %{}", v.0))
                        .collect::<String>()
                ),
                Operation::TupleNew {
                    result,
                    ref elements,
                } => format!(
                    ".tuple-new %{}{}",
                    result.0,
                    elements
                        .iter()
                        .map(|value| format!(" %{}", value.0))
                        .collect::<String>()
                ),
                Operation::NativeNew { result, class } => {
                    format!(".new-native %{} {}", result.0, class.name())
                }
                Operation::DictionaryBegin {
                    builder,
                    dictionary_type,
                    length,
                } => format!(
                    ".dictionary-begin &{} {} {}",
                    builder.0,
                    dictionary_type.text(),
                    length
                ),
                Operation::DictionaryFill {
                    builder,
                    start,
                    ref entries,
                } => format!(
                    ".dictionary-fill &{} {}{}",
                    builder.0,
                    start,
                    entries
                        .iter()
                        .map(|(key, value)| format!(" %{} %{}", key.0, value.0))
                        .collect::<String>()
                ),
                Operation::DictionaryFinish { result, builder } => {
                    format!(".dictionary-finish %{} &{}", result.0, builder.0)
                }
                Operation::ArrayBegin {
                    builder,
                    element_type,
                    length,
                } => format!(
                    ".array-begin &{} {} {}",
                    builder.0,
                    element_type.text(),
                    length
                ),
                Operation::ArrayFill {
                    builder,
                    index,
                    value,
                } => format!(".array-fill &{} {} %{}", builder.0, index, value.0),
                Operation::ArrayFillMany {
                    builder,
                    start,
                    ref values,
                } => format!(
                    ".array-fill-many &{} {}{}",
                    builder.0,
                    start,
                    values
                        .iter()
                        .map(|v| format!(" %{}", v.0))
                        .collect::<String>()
                ),
                Operation::ArrayConstant {
                    result,
                    element_type,
                    ref values,
                } => format!(
                    ".array-constant %{} {}{}",
                    result.0,
                    element_type.text(),
                    values
                        .iter()
                        .map(|v| match v {
                            ArrayConstantValue::Bits(n) => format!(" 0x{n:08x}"),
                            ArrayConstantValue::Double(n) => format!(" 0x{n:016x}"),
                            ArrayConstantValue::Float(n) => format!(" 0x{n:08x}"),
                            ArrayConstantValue::Long(n) => format!(" 0x{n:016x}"),
                            ArrayConstantValue::String(s) => format!(" {s:?}"),
                            ArrayConstantValue::Null => " null".into(),
                            ArrayConstantValue::Boolean(b) => format!(" {b}"),
                        })
                        .collect::<String>()
                ),
                Operation::ArrayFinish { result, builder } => {
                    format!(".array-finish %{} &{}", result.0, builder.0)
                }
                Operation::ArrayNew {
                    result,
                    element_type,
                } => format!(".array-new %{} {}", result.0, element_type.text()),
                Operation::DictionaryRead {
                    result,
                    dictionary,
                    key,
                } => format!(
                    ".dictionary-read %{} %{} %{}",
                    result.0, dictionary.0, key.0
                ),
                Operation::DictionaryWrite {
                    dictionary,
                    key,
                    value,
                } => format!(
                    ".dictionary-write %{} %{} %{}",
                    dictionary.0, key.0, value.0
                ),
                Operation::ArrayRead {
                    result,
                    array,
                    index,
                } => format!(".array-read %{} %{} %{}", result.0, array.0, index.0),
                Operation::ArrayWrite {
                    array,
                    index,
                    value,
                } => format!(".array-write %{} %{} %{}", array.0, index.0, value.0),
                Operation::Not { result, operand } => {
                    format!(".not %{} %{}", result.0, operand.0)
                }
                Operation::Step {
                    result,
                    operand,
                    increment,
                } => format!(
                    ".{} %{} %{}",
                    if increment { "increment" } else { "decrement" },
                    result.0,
                    operand.0
                ),
                Operation::Positive { result, operand } => {
                    format!(".positive %{} %{}", result.0, operand.0)
                }
                Operation::Complement { result, operand } => {
                    format!(".complement %{} %{}", result.0, operand.0)
                }
                Operation::Negate { result, operand } => {
                    format!(".negate %{} %{}", result.0, operand.0)
                }
                Operation::Binary {
                    result,
                    kind,
                    left,
                    right,
                } => format!(
                    ".{} %{} %{} %{}",
                    match kind {
                        BinaryOp::Add => "add",
                        BinaryOp::Subtract => "subtract",
                        BinaryOp::Multiply => "multiply",
                        BinaryOp::Divide => "divide",
                        BinaryOp::Remainder => "remainder",
                        BinaryOp::BitAnd => "bit-and",
                        BinaryOp::BitOr => "bit-or",
                        BinaryOp::BitXor => "bit-xor",
                        BinaryOp::ShiftLeft => "shift-left",
                        BinaryOp::ShiftRight => "shift-right",
                    },
                    result.0,
                    left.0,
                    right.0
                ),
                Operation::Discard { value } => format!(".discard %{}", value.0),
                Operation::NativeCall {
                    result,
                    receiver,
                    ref member,
                    ref arguments,
                } => {
                    let arguments = arguments
                        .iter()
                        .map(|v| format!(" %{}", v.0))
                        .collect::<String>();
                    format!(
                        ".call-native %{} %{} {member}{arguments}",
                        result.0, receiver.0
                    )
                }
                Operation::ModuleCall {
                    result,
                    ref result_type,
                    ref module,
                    ref member,
                    ref arguments,
                } => {
                    let arguments = arguments
                        .iter()
                        .map(|value| format!(" %{}", value.0))
                        .collect::<String>();
                    format!(
                        ".call %{} {module} {member}{}{arguments}",
                        result.0,
                        call_type_text(result_type)
                    )
                }
                Operation::ModuleCallStatement {
                    ref module,
                    ref member,
                    ref arguments,
                } => {
                    let arguments = arguments
                        .iter()
                        .map(|value| format!(" %{}", value.0))
                        .collect::<String>();
                    format!(".call-discard {module} {member}{arguments}")
                }
                Operation::SelfCall {
                    result,
                    ref result_type,
                    ref member,
                    ref arguments,
                } => {
                    let arguments = arguments
                        .iter()
                        .map(|v| format!(" %{}", v.0))
                        .collect::<String>();
                    format!(
                        ".call-self %{} {member}{}{arguments}",
                        result.0,
                        call_type_text(result_type)
                    )
                }
                Operation::SelfCallStatement {
                    ref member,
                    ref arguments,
                } => {
                    let arguments = arguments
                        .iter()
                        .map(|v| format!(" %{}", v.0))
                        .collect::<String>();
                    format!(".call-self-discard {member}{arguments}")
                }
                Operation::NativeBaseInitialize { base } => {
                    format!(".base-initialize {}", base.text())
                }
                Operation::ReturnVoid => ".return-void".into(),
                Operation::ParameterRead { result, index } => {
                    format!(".parameter %{} {index}", result.0)
                }
                Operation::LocalInitialize { local, value } => {
                    format!(".local ${} %{}", local.0, value.0)
                }
                Operation::LocalAssign { local, value } => {
                    format!(".assign ${} %{}", local.0, value.0)
                }
                Operation::LocalRead { result, local } => {
                    format!(".read %{} ${}", result.0, local.0)
                }
                Operation::LocalTake { result, local } => {
                    format!(".take %{} ${}", result.0, local.0)
                }
                Operation::Return { value } => format!(".return %{}", value.0),
            });
            if let Some(span) = operation.span {
                text.push_str(&format!(" span={}:{}", span.start, span.end));
            }
            text.push('\n');
        }
        text.push_str(".end\n");
        text
    }
}

/// Parse one supported Monkey C function into validated semantic IR.
///
/// Resolves supported leading native name declarations and their aliases
/// before IR construction. Source-class annotations require `compile_sources`
/// so the native base can be resolved from declarations.
/// Resolves parameters and scoped locals, lowers conditions and while loops
/// (including innermost break/continue) to blocks, and retains source spans.
/// Value functions require a consistent typed return on every path;
/// otherwise bare returns and fallthrough produce void returns. Mixed return
/// types, unreachable statements, and local/parameter shadowing are rejected.
/// Assignments require a visible initialized local and preserve its supported type.
/// Unsupported syntax, unresolved names, and invalid IR produce source
/// diagnostics rather than partial output. VM instruction selection and symbol
/// binding occur later.
pub fn compile(source: &str) -> Result<Function, Diagnostic> {
    source_ir::build(source, parser::parse(source)?, None)
}

/// Parse a supported function and check unguarded API references against explicit
/// target and deployment levels before lowering support. No device is inferred.
pub fn compile_with_api(source: &str, context: api::ApiContext) -> Result<Function, Diagnostic> {
    source_ir::build(source, parser::parse(source)?, Some(context))
}

fn validate_call_names(module: Option<&str>, member: &str) -> Result<(), &'static str> {
    let valid = |name: &str| {
        name.bytes()
            .next()
            .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
            && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
    };
    if !valid(member) || module.is_some_and(|m| !valid(m)) {
        return Err("invalid module or member name");
    }
    if module == Some("System") {
        return Err("System calls require supported native statement forms");
    }
    if module == Some("self") {
        return Err("self calls require explicit receiver IR operations");
    }
    Ok(())
}
fn validate_call_arguments(
    arguments: &[ValueId],
    types: &std::collections::BTreeMap<ValueId, ValueType>,
) -> Result<(), &'static str> {
    if arguments.iter().any(|v| {
        types.get(v).is_some_and(|t| {
            !matches!(
                t,
                ValueType::Integer
                    | ValueType::String
                    | ValueType::Boolean
                    | ValueType::Char
                    | ValueType::Long
                    | ValueType::Float
                    | ValueType::Double
                    | ValueType::Object(_)
                    | ValueType::Array(_)
                    | ValueType::LangObject
                    | ValueType::NullableObject
                    | ValueType::WeakReference
                    | ValueType::Interface(_)
                    | ValueType::Method
                    | ValueType::Symbol
                    | ValueType::Dictionary
                    | ValueType::TypedDictionary(_)
                    | ValueType::Null
                    | ValueType::NullableInteger
                    | ValueType::NullableString
                    | ValueType::NullableBoolean
                    | ValueType::NullableChar
                    | ValueType::NullableLong
                    | ValueType::NullableDouble
                    | ValueType::NullableFloat
            )
        })
    }) {
        return Err("user calls require supported scalar, array, or nullable arguments");
    }
    if arguments.len() > MAX_PARAMETERS {
        return Err("call exceeds 254-argument limit");
    }
    Ok(())
}

fn call_result_type(result: ReturnType) -> Result<ValueType, &'static str> {
    match result {
        ReturnType::NullableInteger => Ok(ValueType::NullableInteger),
        ReturnType::NullableLong => Ok(ValueType::NullableLong),
        ReturnType::NullableDouble => Ok(ValueType::NullableDouble),
        ReturnType::NullableFloat => Ok(ValueType::NullableFloat),
        ReturnType::NullableString => Ok(ValueType::NullableString),
        ReturnType::NullableBoolean => Ok(ValueType::NullableBoolean),
        ReturnType::NullableChar => Ok(ValueType::NullableChar),
        ReturnType::Integer => Ok(ValueType::Integer),
        ReturnType::Long => Ok(ValueType::Long),
        ReturnType::Double => Ok(ValueType::Double),
        ReturnType::Float => Ok(ValueType::Float),
        ReturnType::String => Ok(ValueType::String),
        ReturnType::Boolean => Ok(ValueType::Boolean),
        ReturnType::Char => Ok(ValueType::Char),
        ReturnType::TypedDictionary(ty) => Ok(ValueType::TypedDictionary(ty)),
        ReturnType::Dictionary => Ok(ValueType::Dictionary),
        ReturnType::LangObject => Ok(ValueType::LangObject),
        ReturnType::NullableObject => Ok(ValueType::NullableObject),
        ReturnType::WeakReference => Ok(ValueType::WeakReference),
        ReturnType::Interface(interface) => Ok(ValueType::Interface(interface)),
        ReturnType::Method => Ok(ValueType::Method),
        ReturnType::Symbol => Ok(ValueType::Symbol),
        ReturnType::Object(class) => Ok(ValueType::Object(class)),
        ReturnType::SourceObject(class) => {
            validate_call_names(None, class.as_str())?;
            ValueType::from_api_type(&api::Type::named(class.base.api_name()))
                .ok_or("source reference requires a constructible native base")
        }
        ReturnType::Tuple(shape) => Ok(ValueType::Tuple(shape)),
        ReturnType::Array(element) => Ok(ValueType::Array(element)),
        ReturnType::Void => Err("void function cannot be used as a value"),
    }
}

fn call_type_text(result: &ReturnType) -> String {
    match result {
        ReturnType::SourceObject(class) => return format!(" result=source({class})"),
        ReturnType::Interface(interface) => return format!(" result={}", interface.tag()),
        ReturnType::NullableInteger => " result=nullable-integer",
        ReturnType::NullableLong => " result=nullable-long",
        ReturnType::NullableDouble => " result=nullable-double",
        ReturnType::NullableFloat => " result=nullable-float",
        ReturnType::NullableString => " result=nullable-string",
        ReturnType::NullableBoolean => " result=nullable-boolean",
        ReturnType::NullableChar => " result=nullable-char",
        ReturnType::Integer => "",
        ReturnType::Long => " result=long",
        ReturnType::Double => " result=double",
        ReturnType::Float => " result=float",
        ReturnType::String => " result=string",
        ReturnType::Boolean => " result=boolean",
        ReturnType::Char => " result=char",
        ReturnType::TypedDictionary(ty) => return format!(" result={}", ty.text()),
        ReturnType::Dictionary => " result=dictionary",
        ReturnType::LangObject => " result=object",
        ReturnType::NullableObject => " result=nullable-object",
        ReturnType::WeakReference => " result=weak-reference",
        ReturnType::Method => " result=method",
        ReturnType::Symbol => " result=symbol",
        ReturnType::Object(NativeClass::View) => " result=view",
        ReturnType::Object(NativeClass::Dc) => " result=dc",
        ReturnType::Object(NativeClass::BehaviorDelegate) => " result=behavior-delegate",
        ReturnType::Tuple(TupleShape::OneView) => " result=one-view",
        ReturnType::Tuple(TupleShape::ViewDelegate) => " result=view-delegate",
        ReturnType::Array(ArrayElement::Char) => " result=char-array",
        ReturnType::Array(ArrayElement::Boolean) => " result=boolean-array",
        ReturnType::Array(ArrayElement::Number) => " result=number-array",
        ReturnType::Array(ArrayElement::Double) => " result=double-array",
        ReturnType::Array(ArrayElement::NullableDouble) => " result=nullable-double-array",
        ReturnType::Array(ArrayElement::Float) => " result=float-array",
        ReturnType::Array(ArrayElement::NullableFloat) => " result=nullable-float-array",
        ReturnType::Array(ArrayElement::Long) => " result=long-array",
        ReturnType::Array(ArrayElement::NullableLong) => " result=nullable-long-array",
        ReturnType::Array(ArrayElement::NullableNumber) => " result=nullable-number-array",
        ReturnType::Array(ArrayElement::NullableChar) => " result=nullable-char-array",
        ReturnType::Array(ArrayElement::NullableBoolean) => " result=nullable-boolean-array",
        ReturnType::Array(ArrayElement::String) => " result=string-array",
        ReturnType::Array(ArrayElement::NullableString) => " result=nullable-string-array",
        ReturnType::Array(ArrayElement::Object(NativeClass::View)) => " result=view-array",
        ReturnType::Array(ArrayElement::Object(NativeClass::Dc)) => " result=dc-array",
        ReturnType::Array(ArrayElement::Object(NativeClass::BehaviorDelegate)) => {
            " result=behavior-delegate-array"
        }
        ReturnType::Void => unreachable!("validated value calls cannot be void"),
    }
    .to_owned()
}
