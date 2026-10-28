//! Declaration collection, cross-function checks, and unit text serialization.
use crate::{Diagnostic, Function, Operation, ReturnType, backend, parser, source_ir};
use ciq_assembly::Program;
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

pub(crate) const MAX_FUNCTIONS: usize = 256;

/// Match a source method's arity, parameter contracts, and result against a
/// fixed native signature. Exact source identities cannot stand in for Object.
fn signature_matches(implementation: &Function, required: &crate::api::Method) -> bool {
    required.variadic.is_none()
        && implementation.parameter_types().len() == required.parameters.len()
        && implementation
            .parameter_types()
            .iter()
            .zip(&required.parameters)
            .all(|(actual, expected)| {
                !matches!(
                    actual,
                    crate::ParameterType::SourceObject(_)
                        | crate::ParameterType::NullableSourceObject(_)
                        | crate::ParameterType::SourceUnion(_)
                ) && &actual.api_type() == expected
            })
        && required.result == implementation.return_type().api_type()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Owner<'a> {
    Global,
    Module(&'a str),
    Class(&'a str),
}
impl<'a> Owner<'a> {
    fn name(self) -> &'a str {
        match self {
            Self::Global => "",
            Self::Module(name) | Self::Class(name) => name,
        }
    }
}

fn source_callee<'a>(owner: Owner<'a>, operation: &'a Operation) -> Option<(Owner<'a>, &'a str)> {
    match operation {
        Operation::GlobalCall { member, .. } | Operation::GlobalCallStatement { member, .. } => {
            Some((Owner::Global, member))
        }
        Operation::ModuleCall { module, member, .. }
        | Operation::ModuleCallStatement { module, member, .. } => {
            Some((Owner::Module(module), member))
        }
        Operation::SelfCall { member, .. } | Operation::SelfCallStatement { member, .. } => {
            Some((owner, member))
        }
        Operation::SourceNew { class, .. } => Some((Owner::Class(class), "initialize")),
        _ => None,
    }
}

/// List every possible source body; each contributes call effects and contracts.
fn source_callees<'a>(owner: Owner<'a>, operation: &'a Operation) -> Vec<(Owner<'a>, &'a str)> {
    match operation {
        Operation::ObjectCall {
            classes, member, ..
        }
        | Operation::ObjectCallStatement {
            classes, member, ..
        } => classes
            .iter()
            .map(|class| (Owner::Class(class.as_str()), member.as_str()))
            .collect(),
        _ => source_callee(owner, operation).into_iter().collect(),
    }
}

/// Include allocation-time bodies activated by a module lookup, independently
/// of the selected member. Same-owner access must not restart initialization.
fn activation_callees<'a>(
    owner: Owner<'a>,
    operation: &'a Operation,
    bodies: &[(Owner<'a>, &'a Function)],
) -> Vec<(Owner<'a>, &'a str)> {
    let mut calls = effect_callees(owner, operation);
    let module = match operation {
        Operation::SharedRead {
            module: Some(module),
            ..
        }
        | Operation::ModuleCall { module, .. }
        | Operation::ModuleCallStatement { module, .. } => Some(module.as_str()),
        _ => None,
    };
    if let Some(module) = module.filter(|module| owner != Owner::Module(module)) {
        calls.extend(
            bodies
                .iter()
                .filter(|(o, f)| {
                    *o == Owner::Module(module) && f.name().starts_with("__ciq_initialize_shared_")
                })
                .map(|(owner, f)| (*owner, f.name())),
        );
    }
    calls
}

fn requests_exit(operation: &Operation) -> bool {
    matches!(operation, Operation::NativeModuleCallStatement { module, member, .. } if module == "Toybox.System" && member == "exit")
}

/// Identify tuple contents whose equality, printing, or default ordering can
/// dispatch through source or erased object contracts, including nested values.
fn tuple_callback_elements(tuple: &crate::TupleType) -> bool {
    tuple
        .slots()
        .iter()
        .any(|slot| slot.source_contract().is_some() || callback_value_type(&slot.api_type()))
}

fn effect_callees<'a>(owner: Owner<'a>, operation: &'a Operation) -> Vec<(Owner<'a>, &'a str)> {
    let mut calls = source_callees(owner, operation);
    if let Operation::SourceBegin { class, .. } = operation {
        calls.push((Owner::Class(class), FIELD_INITIALIZER));
    }
    calls
}

/// Check erased value contracts that can expose source equality or observer
/// callbacks. Native/source base erasure does not prove a pure implementation.
fn callback_value_type(ty: &crate::api::Type) -> bool {
    use crate::api::Type;
    match ty {
        crate::api::Type::Options(_) => true,
        Type::SourceValue(_) => true,
        Type::Any | Type::Structural(_) => true,
        Type::Named(name) => {
            [
                ClassBase::LangObject,
                ClassBase::WatchUiView,
                ClassBase::WatchUiBehaviorDelegate,
            ]
            .iter()
            .any(|base| base.api_name() == name)
                || crate::InterfaceType::from_name(name).is_some()
        }
        Type::TupleValue(tuple) => tuple_callback_elements(tuple),
        Type::Union(parts) | Type::Tuple(parts) => parts.iter().any(callback_value_type),
        Type::Array(element) => callback_value_type(element),
        Type::Dictionary { key, value } => callback_value_type(key) || callback_value_type(value),
        _ => false,
    }
}

/// Treat erased comparison/element callbacks as possible shutdown requests.
/// No exact body remains available for the reference-initialization proof.
fn erased_callbacks(function: &Function, operation: &Operation) -> bool {
    let interface_keys = |ty: &crate::ValueType| {
        crate::unions::alternatives(&ty.api_type()).iter().any(|p| {
            matches!(
                crate::DictionaryType::from_api_type(p),
                Some(crate::DictionaryType {
                    key: crate::DictionaryKey::Interface(_),
                    ..
                })
            )
        })
    };
    match operation {
        Operation::ArrayRead { array, .. } | Operation::ArrayWrite { array, .. }
            if function.value_types[array] == crate::ValueType::Any => true,
        Operation::ArrayRead{array,..}|Operation::ArrayWrite{array,..} if matches!(function.value_types[array],crate::ValueType::Union(_))=>interface_keys(&function.value_types[array]),
        Operation::DictionaryRead { dictionary, .. }
        | Operation::DictionaryWrite { dictionary, .. } => interface_keys(&function.value_types[dictionary]),
        Operation::DictionaryFill { builder, entries, .. } => entries.iter().any(|(key,_)| function.value_types[key] == crate::ValueType::Any) || function.operations().iter().any(|located| {
            matches!(located.operation, Operation::DictionaryBegin { builder: id, ref dictionary_type, .. }
                if id == *builder && matches!(dictionary_type.key, crate::DictionaryKey::Interface(_)))
        }),
        Operation::NativeModuleCall { module, member, .. }
            if module == "Toybox.Lang" && member == "format" => true,
        Operation::NativeCall {
            receiver,
            member,
            arguments,
            ..
        }
        | Operation::NativeCallStatement {
            receiver,
            member,
            arguments,
        } => {
            let receiver_type=function.value_types[receiver].api_type();
            crate::unions::alternatives(&receiver_type).iter().filter_map(crate::ValueType::from_api_type).any(|ty| {
            matches!(
                ty,
                crate::ValueType::Any | crate::ValueType::Interface(_) | crate::ValueType::NullableInterface(_) | crate::ValueType::Structural(_)
            ) || (member == "sort"
                && arguments.iter().any(|a| {
                    (crate::source_objects::class(function.operations(), *a, function.parameter_types().to_vec()).is_none()
                        && function.value_types[a] != crate::ValueType::Null)
                    || matches!(
                        function.value_types[a],
                        crate::ValueType::Interface(_) | crate::ValueType::NullableInterface(_)
                    )
                }))
                || matches!(&ty, crate::ValueType::Tuple(tuple)
                    if (tuple_callback_elements(tuple)
                        || (matches!(member.as_str(), "indexOf" | "remove" | "removeAll")
                            && arguments.iter().any(|argument| callback_value_type(&function.value_types[argument].api_type()))))
                    && (matches!(member.as_str(), "toString" | "indexOf" | "remove" | "removeAll")
                        || (member == "sort" && arguments.iter().any(|argument|
                            matches!(function.value_types[argument], crate::ValueType::Null
                                | crate::ValueType::NullableInterface(crate::InterfaceType::Comparator))))))
                || matches!(ty, crate::ValueType::Array(
                    crate::ArrayElement::Any | crate::ArrayElement::Interface(_) | crate::ArrayElement::NullableInterface(_))
                    if matches!(member.as_str(), "toString" | "indexOf" | "remove" | "removeAll" | "sort"))
                || matches!(&ty, crate::ValueType::Array(element @ (crate::ArrayElement::Source(_) | crate::ArrayElement::Nested { .. } | crate::ArrayElement::Union(_)))
                    if callback_value_type(&element.api_type())
                        && matches!(member.as_str(), "toString" | "indexOf" | "remove" | "removeAll" | "sort"))
                || matches!(ty, crate::ValueType::Array(crate::ArrayElement::LangObject)
                    if member == "sort" && arguments.iter().any(|argument|
                        matches!(function.value_types[argument], crate::ValueType::Null
                            | crate::ValueType::NullableInterface(crate::InterfaceType::Comparator))))
                || matches!(ty, crate::ValueType::TypedDictionary(
                    crate::DictionaryType {value: crate::DictionaryValue::Interface(_) | crate::DictionaryValue::NullableInterface(_), ..})
                    if member == "toString")
                || (interface_keys(&ty)
                    && matches!(member.as_str(), "get" | "put" | "hasKey" | "remove" | "toString"))
                || matches!(ty, crate::ValueType::Dictionary
                    | crate::ValueType::Options(_)
                    | crate::ValueType::NullableDictionary | crate::ValueType::LangObject
                    | crate::ValueType::NullableObject if member == "toString")
                || matches!(ty, crate::ValueType::Method
                    | crate::ValueType::NullableMethod | crate::ValueType::Callback(_) if member == "invoke")
            })
        }
        _ => false,
    }
}

/// Resolve a checked sort callback to its source body for effect propagation.
/// The runtime selects the same pinned compare member checked by the unit.
fn comparator_callee<'a>(
    function: &Function,
    operation: &Operation,
    bodies: &[(Owner<'a>, &'a Function)],
) -> Option<(Owner<'a>, &'a str)> {
    let (receiver, arguments) = match operation {
        Operation::NativeCall {
            receiver,
            member,
            arguments,
            ..
        }
        | Operation::NativeCallStatement {
            receiver,
            member,
            arguments,
        } if member == "sort" => (*receiver, arguments),
        _ => return None,
    };
    if !crate::unions::alternatives(&function.value_types[&receiver].api_type())
        .iter()
        .all(|ty| {
            matches!(
                ty,
                crate::api::Type::Array(_) | crate::api::Type::TupleValue(_)
            )
        })
    {
        return None;
    }
    let [argument] = arguments.as_slice() else {
        return None;
    };
    let class = crate::source_objects::class(
        function.operations(),
        *argument,
        function.parameter_types().to_vec(),
    )?;
    bodies.iter().find_map(|&(owner, method)| {
        (owner == Owner::Class(&class) && method.name() == "compare")
            .then_some((owner, method.name()))
    })
}

/// Find bodies that can request shutdown, including through recursive source
/// calls. Conservatively include conditional calls; this does not prove safety
/// against runtime failures or unknown native callback effects.
fn exit_callers<'a>(bodies: &[(Owner<'a>, &'a Function)]) -> BTreeSet<(Owner<'a>, &'a str)> {
    let mut exits = BTreeSet::new();
    loop {
        let mut changed = false;
        for &(owner, function) in bodies {
            if function.operations().iter().any(|located| {
                requests_exit(&located.operation)
                    || erased_callbacks(function, &located.operation)
                    || activation_callees(owner, &located.operation, bodies)
                        .iter()
                        .any(|key| exits.contains(key))
                    || comparator_callee(function, &located.operation, bodies)
                        .is_some_and(|key| exits.contains(&key))
            }) {
                changed |= exits.insert((owner, function.name()));
            }
        }
        if !changed {
            return exits;
        }
    }
}

/// A global function or a member of a flat source module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionDeclaration {
    /// None for a global function, otherwise its unqualified module name.
    pub module: Option<String>,
    /// Validated function body with explicit parameter and result contracts.
    pub function: Function,
}

impl FunctionDeclaration {
    /// Return the deterministic CODE label for this declaration.
    ///
    /// Length-prefixed names keep global/module identities and internal aliases
    /// distinct even when source names contain underscores. Metadata producers
    /// must bind methods to this label rather than the unqualified source name.
    pub fn code_label(&self) -> String {
        let name = self.function.name();
        match &self.module {
            None => format!("__ciq_fn_g_{}_{}", name.len(), name),
            Some(module) => format!(
                "__ciq_fn_m_{}_{}_{}_{}",
                module.len(),
                module,
                name.len(),
                name
            ),
        }
    }
}

/// Allocation-only field initialization body, excluded from callable members.
pub(crate) const FIELD_INITIALIZER: &str = "__ciq_initialize_fields";

/// Supported native superclass identity, independent of an import-table index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClassBase {
    LangObject,
    ApplicationAppBase,
    WatchUiView,
    WatchUiBehaviorDelegate,
}
impl ClassBase {
    /// Return the canonical native API identity of this superclass.
    pub fn api_name(self) -> &'static str {
        match self {
            Self::LangObject => "Toybox.Lang.Object",
            Self::ApplicationAppBase => "Toybox.Application.AppBase",
            Self::WatchUiView => "Toybox.WatchUi.View",
            Self::WatchUiBehaviorDelegate => "Toybox.WatchUi.BehaviorDelegate",
        }
    }
    /// Return the API value type produced by supported source construction.
    /// AppBase is created by application startup, not by a source expression.
    pub fn constructed_type(self) -> Option<crate::api::Type> {
        (self != Self::ApplicationAppBase).then(|| self.reference_type())
    }
    /// Return the native projection of an existing source instance. This does
    /// not authorize construction; application startup supplies AppBase instances.
    pub fn reference_type(self) -> crate::api::Type {
        crate::api::Type::named(self.api_name())
    }
    pub(crate) fn text(self) -> &'static str {
        match self {
            Self::LangObject => "Lang.Object",
            Self::ApplicationAppBase => "Application.AppBase",
            Self::WatchUiView => "WatchUi.View",
            Self::WatchUiBehaviorDelegate => "WatchUi.BehaviorDelegate",
        }
    }
}

/// A supported numeric typedef, independent of nullability or storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericAlias {
    /// Number or Long.
    Integer,
    /// Float or Double.
    Decimal,
    /// Number, Float, Long, or Double.
    Numeric,
}

impl NumericAlias {
    /// Return the saved-IR tag for this non-null alias.
    fn tag(self) -> &'static str {
        self.value_tag(false)
    }

    /// Return the scalar result contract for this alias and declared nullability.
    pub(crate) fn result_type(self, nullable: bool) -> ReturnType {
        match (self, nullable) {
            (Self::Integer, false) => ReturnType::IntegerAlias,
            (Self::Integer, true) => ReturnType::NullableIntegerAlias,
            (Self::Decimal, false) => ReturnType::Decimal,
            (Self::Decimal, true) => ReturnType::NullableDecimal,
            (Self::Numeric, false) => ReturnType::Numeric,
            (Self::Numeric, true) => ReturnType::NullableNumeric,
        }
    }

    /// Return the canonical scalar union, optionally including Null.
    pub(crate) fn api_type(self, nullable: bool) -> crate::api::Type {
        self.result_type(nullable).api_type()
    }

    /// Return the scalar text tag, retaining declared nullability.
    pub(crate) fn value_tag(self, nullable: bool) -> &'static str {
        match (self, nullable) {
            (Self::Integer, false) => "integer-union",
            (Self::Integer, true) => "nullable-integer-union",
            (Self::Decimal, false) => "decimal",
            (Self::Decimal, true) => "nullable-decimal",
            (Self::Numeric, false) => "numeric",
            (Self::Numeric, true) => "nullable-numeric",
        }
    }

    /// Resolve an exact supported numeric union, including its Null alternative.
    pub(crate) fn from_api_type(ty: &crate::api::Type) -> Option<(Self, bool)> {
        for alias in [Self::Integer, Self::Decimal, Self::Numeric] {
            for nullable in [false, true] {
                if alias.api_type(nullable) == *ty {
                    return Some((alias, nullable));
                }
            }
        }
        None
    }
}

/// A concrete scalar contract supported by typed instance fields.
/// Numeric classes stay distinct; assignments do not perform conversions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalarFieldType {
    Number,
    Long,
    Float,
    Double,
    Boolean,
    Char,
    String,
}

impl ScalarFieldType {
    fn tag(self) -> &'static str {
        match self {
            Self::Number => "integer",
            Self::Long => "long",
            Self::Float => "float",
            Self::Double => "double",
            Self::Boolean => "boolean",
            Self::Char => "char",
            Self::String => "string",
        }
    }

    fn parse_tag(tag: &str) -> Option<Self> {
        [
            Self::Number,
            Self::Long,
            Self::Float,
            Self::Double,
            Self::Boolean,
            Self::Char,
            Self::String,
        ]
        .into_iter()
        .find(|ty| ty.tag() == tag)
    }

    fn result_type(self, nullable: bool) -> ReturnType {
        match (self, nullable) {
            (Self::Number, false) => ReturnType::Integer,
            (Self::Number, true) => ReturnType::NullableInteger,
            (Self::Long, false) => ReturnType::Long,
            (Self::Long, true) => ReturnType::NullableLong,
            (Self::Float, false) => ReturnType::Float,
            (Self::Float, true) => ReturnType::NullableFloat,
            (Self::Double, false) => ReturnType::Double,
            (Self::Double, true) => ReturnType::NullableDouble,
            (Self::Boolean, false) => ReturnType::Boolean,
            (Self::Boolean, true) => ReturnType::NullableBoolean,
            (Self::Char, false) => ReturnType::Char,
            (Self::Char, true) => ReturnType::NullableChar,
            (Self::String, false) => ReturnType::String,
            (Self::String, true) => ReturnType::NullableString,
        }
    }
}

/// A supported field's default and initialization requirement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldKind {
    /// A readonly binding populated once by the allocation initializer.
    /// Its referenced contents may remain mutable; metadata starts as Null.
    /// Requires an instance method named `__ciq_initialize_fields` with no
    /// parameters and a Void result; unit checks require exactly one assignment.
    Constant { value_type: ReturnType },
    /// A supported value contract with a Null metadata slot. Contracts admitting
    /// Null have that default; others require constructor initialization.
    Value { value_type: ReturnType },
    /// A Number copied from class metadata into each new instance.
    Number(i32),
    /// A Null-initialized concrete scalar slot. Non-null fields require source
    /// initialization before reads, self exposure, control flow, or return.
    Scalar { ty: ScalarFieldType, nullable: bool },
    /// A strong opaque Object slot. Non-null fields require source initialization;
    /// erasure does not preserve a concrete source or container contract.
    LangObject { nullable: bool },
    /// A non-null Array slot with an invariant element contract, filled in initialize.
    Array { element: crate::ArrayElement },
    /// A non-null Dictionary slot with invariant key/value contracts, filled in initialize.
    Dictionary {
        dictionary_type: crate::DictionaryType,
    },
    /// A Null-initialized numeric alias slot. Non-null fields must be filled
    /// by source initialize before reads, self exposure, control flow, or return.
    Numeric { alias: NumericAlias, nullable: bool },
    /// A Symbol slot with an optional literal metadata default. Nullable slots
    /// otherwise default to Null; non-null slots require source initialization.
    Symbol {
        initial: Option<String>,
        nullable: bool,
    },
    /// A Null-initialized bound Method slot. Non-null fields must be filled by
    /// source initialize before reads, self exposure, control flow, or return.
    Method { nullable: bool },
    /// A Null-initialized ordinary WeakReference slot. Non-null fields require
    /// source initialization; storing the reference does not own its target.
    WeakReference { nullable: bool },
    /// A Null-initialized strong ByteArray reference slot. Non-null fields
    /// require source initialization before reads, self exposure, or control flow.
    ByteArray { nullable: bool },
    /// A null reference slot that source initialize must fill with this exact
    /// non-null source class before reads, self calls, or return.
    SourceObject(crate::SourceClass),
    /// An exact source-class reference or Null, with a Null metadata default.
    /// Reads are nullable snapshots; source dispatch requires a non-null proof.
    NullableSourceObject(crate::SourceClass),
    /// A Null-initialized comparison-interface slot. Non-null fields must be
    /// filled by source initialize before reads, self calls, or return.
    Interface {
        interface: crate::InterfaceType,
        nullable: bool,
    },
}

impl FieldKind {
    /// Normalize a field value contract, excluding Void and graphics contexts.
    pub(crate) fn value_contract(ty: ReturnType) -> Option<ReturnType> {
        crate::recursive::validate(&ty.api_type()).ok()?;

        let ty = if let Some(contract) = ty.source_contract() {
            contract.result()
        } else {
            crate::unions::result(crate::unions::normalize(vec![ty.api_type()]))?
        };
        if matches!(ty, ReturnType::Void)
            || crate::unions::alternatives(&ty.api_type())
                .iter()
                .any(|part| *part == crate::api::Type::named("Toybox.Graphics.Dc"))
        {
            return None;
        }
        Some(ty)
    }

    /// Adapt a signature value contract to canonical field storage.
    /// Void and Graphics.Dc are not ordinary field value contracts.
    pub(crate) fn from_result(ty: ReturnType) -> Option<Self> {
        let ty = Self::value_contract(ty)?;
        for scalar in [
            ScalarFieldType::Number,
            ScalarFieldType::Long,
            ScalarFieldType::Float,
            ScalarFieldType::Double,
            ScalarFieldType::Boolean,
            ScalarFieldType::Char,
            ScalarFieldType::String,
        ] {
            for nullable in [false, true] {
                if scalar.result_type(nullable) == ty {
                    return Some(Self::Scalar {
                        ty: scalar,
                        nullable,
                    });
                }
            }
        }
        for alias in [
            NumericAlias::Integer,
            NumericAlias::Decimal,
            NumericAlias::Numeric,
        ] {
            for nullable in [false, true] {
                if alias.result_type(nullable) == ty {
                    return Some(Self::Numeric { alias, nullable });
                }
            }
        }
        Some(match ty {
            ReturnType::LangObject => Self::LangObject { nullable: false },
            ReturnType::NullableObject => Self::LangObject { nullable: true },
            ReturnType::Array(element) => Self::Array { element },
            ReturnType::TypedDictionary(dictionary_type) => Self::Dictionary { dictionary_type },
            ReturnType::Symbol => Self::Symbol {
                initial: None,
                nullable: false,
            },
            ReturnType::NullableSymbol => Self::Symbol {
                initial: None,
                nullable: true,
            },
            ReturnType::Method => Self::Method { nullable: false },
            ReturnType::NullableMethod => Self::Method { nullable: true },
            ReturnType::WeakReference => Self::WeakReference { nullable: false },
            ReturnType::NullableWeakReference => Self::WeakReference { nullable: true },
            ReturnType::ByteArray => Self::ByteArray { nullable: false },
            ReturnType::NullableByteArray => Self::ByteArray { nullable: true },
            ReturnType::Interface(interface) => Self::Interface {
                interface,
                nullable: false,
            },
            ReturnType::NullableInterface(interface) => Self::Interface {
                interface,
                nullable: true,
            },
            ReturnType::SourceObject(class) => Self::SourceObject(class),
            ReturnType::NullableSourceObject(class) => Self::NullableSourceObject(class),
            value_type => Self::Value { value_type },
        })
    }
    fn requires_initialization(&self) -> bool {
        if let Self::Value { value_type } = self {
            return !crate::native::accepts(&value_type.api_type(), &crate::api::Type::Null);
        }
        matches!(
            self,
            Self::SourceObject(_)
                | Self::LangObject { nullable: false }
                | Self::Array { .. }
                | Self::Dictionary { .. }
                | Self::Scalar {
                    nullable: false,
                    ..
                }
                | Self::Numeric {
                    nullable: false,
                    ..
                }
                | Self::Method { nullable: false }
                | Self::WeakReference { nullable: false }
                | Self::ByteArray { nullable: false }
                | Self::Symbol {
                    initial: None,
                    nullable: false
                }
                | Self::Interface {
                    nullable: false,
                    ..
                }
        )
    }

    /// Return the literal Symbol name that application binding must resolve.
    pub(crate) fn symbol_default(&self) -> Option<&str> {
        if let Self::Symbol { initial, .. } = self {
            initial.as_deref()
        } else {
            None
        }
    }
}

/// An instance field declaration with a checked initialization contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceField {
    /// Unqualified member name, unique within the class.
    pub name: String,
    /// Default storage and required value contract, where applicable.
    pub kind: FieldKind,
}
impl InstanceField {
    pub(crate) fn result_type(&self) -> ReturnType {
        match &self.kind {
            FieldKind::Constant { value_type } | FieldKind::Value { value_type } => {
                value_type.clone()
            }
            FieldKind::Number(_) => ReturnType::Integer,
            FieldKind::LangObject { nullable } => {
                if *nullable {
                    ReturnType::NullableObject
                } else {
                    ReturnType::LangObject
                }
            }
            FieldKind::Array { element } => ReturnType::Array(element.clone()),
            FieldKind::Dictionary { dictionary_type } => {
                ReturnType::TypedDictionary(dictionary_type.clone())
            }
            FieldKind::Scalar { ty, nullable } => ty.result_type(*nullable),
            FieldKind::Numeric { alias, nullable } => alias.result_type(*nullable),
            FieldKind::ByteArray { nullable } => {
                if *nullable {
                    ReturnType::NullableByteArray
                } else {
                    ReturnType::ByteArray
                }
            }
            FieldKind::WeakReference { nullable } => {
                if *nullable {
                    ReturnType::NullableWeakReference
                } else {
                    ReturnType::WeakReference
                }
            }
            FieldKind::Method { nullable } => {
                if *nullable {
                    ReturnType::NullableMethod
                } else {
                    ReturnType::Method
                }
            }
            FieldKind::Symbol { nullable, .. } => {
                if *nullable {
                    ReturnType::NullableSymbol
                } else {
                    ReturnType::Symbol
                }
            }
            FieldKind::SourceObject(class) => ReturnType::SourceObject(class.clone()),
            FieldKind::NullableSourceObject(class) => {
                ReturnType::NullableSourceObject(class.clone())
            }
            FieldKind::Interface {
                interface,
                nullable,
            } => {
                if *nullable {
                    ReturnType::NullableInterface(*interface)
                } else {
                    ReturnType::Interface(*interface)
                }
            }
        }
    }
}

/// A named native-derived class with checked instance fields and method bodies.
/// Source inheritance and native method bodies are not supported yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassDeclaration {
    /// Unqualified class identity, distinct from module/global names.
    pub name: String,
    /// Native superclass identity resolved by application generation.
    pub base: ClassBase,
    /// Instance fields in declaration order.
    pub fields: Vec<InstanceField>,
    /// Instance-method IR bodies, checked together during unit construction.
    pub methods: Vec<Function>,
}
impl ClassDeclaration {
    /// Return an owner-qualified CODE label for a method in this class.
    pub fn code_label(&self, method: &str) -> String {
        format!(
            "__ciq_fn_c_{}_{}_{}_{}",
            self.name.len(),
            self.name,
            method.len(),
            method
        )
    }
}

/// A declaration or cross-function validation failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitError {
    /// Zero-based body index: global/module functions first, then class methods.
    /// Absent for a unit-wide failure. Class errors use the first method index.
    pub declaration: Option<usize>,
    /// Zero-based operation index within that declaration, when applicable.
    pub operation: Option<usize>,
    /// Failed declaration or call requirement.
    pub message: String,
}
impl fmt::Display for UnitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}
impl std::error::Error for UnitError {}

/// Storage and owner activation required by a source body or lazy initializer.
#[derive(Clone, Default, PartialEq, Eq)]
struct SharedReads {
    fields: BTreeSet<(Option<String>, String)>,
    owners: BTreeSet<Option<String>>,
}
impl SharedReads {
    fn extend(&mut self, other: &Self) {
        self.fields.extend(other.fields.iter().cloned());
        self.owners.extend(other.owners.iter().cloned());
    }
}

/// A readonly slot shared by all accesses to a global or module declaration.
/// The matching reserved function initializes the binding once at owner startup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedConstant {
    /// None selects globals; Some selects a source module.
    pub module: Option<String>,
    /// Identifier within the owner; it must not collide with another value.
    pub name: String,
    /// Stored field contract, excluding Void and Graphics.Dc. Referenced contents
    /// may mutate while the binding remains readonly.
    pub value_type: ReturnType,
}
impl SharedConstant {
    /// Return the reserved ordinary function invoked by the owner's allocation hook.
    pub fn initializer(&self) -> String {
        format!("__ciq_initialize_shared_{}", self.name)
    }
}

/// Validated global/module functions and instance classes with checked calls.
///
/// Qualified user calls and source constructors resolve inside the unit.
/// Arguments and results must match the callee's supported value contracts.
/// Void calls are permitted only as statements. Self calls require a declared
/// method in the enclosing class; native callbacks cannot be called directly.
/// CODE lowering leaves symbol/DATA bindings and application metadata external.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompilationUnit {
    declarations: Vec<FunctionDeclaration>,
    classes: Vec<ClassDeclaration>,
    shared: Vec<SharedConstant>,
}

impl CompilationUnit {
    /// Check known native bases, field types, and unguarded API references.
    /// Errors retain the unit's flattened body indexes. The context is external
    /// to semantic IR; this does not validate a device or application profile.
    pub fn validate_api(&self, context: crate::api::ApiContext) -> Result<(), UnitError> {
        self.validate_declaration_api(context)?;
        self.validate_context(Some(context))?;
        for (index, body) in self
            .declarations
            .iter()
            .map(|d| &d.function)
            .chain(self.classes.iter().flat_map(|c| &c.methods))
            .enumerate()
        {
            body.validate_api(context).map_err(|error| UnitError {
                declaration: Some(index),
                operation: error.operation,
                message: error.message,
            })?;
        }
        Ok(())
    }

    /// Check class, callback, and field declarations without rechecking bodies.
    /// Source and IR entry points already validate bodies with their context.
    fn validate_declaration_api(&self, context: crate::api::ApiContext) -> Result<(), UnitError> {
        for class in &self.classes {
            context
                .check_class(crate::api::core(), class.base.api_name())
                .map_err(|message| UnitError {
                    declaration: None,
                    operation: None,
                    message,
                })?;
            for method in &class.methods {
                if crate::native::callback(class.base, method.name()).is_some() {
                    context
                        .method(crate::api::core(), class.base.api_name(), method.name())
                        .map_err(|message| UnitError {
                            declaration: None,
                            operation: None,
                            message,
                        })?;
                }
            }
            for field in &class.fields {
                context
                    .check_type(crate::api::core(), &field.result_type().api_type())
                    .map_err(|message| UnitError {
                        declaration: None,
                        operation: None,
                        message,
                    })?;
            }
        }
        Ok(())
    }

    /// Validate declarations and qualified calls against all function signatures.
    ///
    /// Accepts forward and recursive calls. Rejects duplicate identities,
    /// invalid/reserved module names, global/module name collisions, unresolved
    /// callees, wrong argument/result contracts, void calls used as values, and
    /// units outside 1..=256 functions.
    /// Each immutable Function has checked its internal types and control-flow
    /// invariants. This does not prove recursive termination.
    pub fn new(declarations: Vec<FunctionDeclaration>) -> Result<Self, UnitError> {
        Self::with_classes(declarations, Vec::new())
    }

    /// Validate global/module functions and instance classes together.
    ///
    /// Resolves self calls only against the enclosing class, and module calls
    /// only against module functions. Checks owner/name uniqueness, exact arity,
    /// receiver contracts, and call argument/result types. Requires 1..=256 bodies.
    /// Class errors use flattened body indexes after all global/module functions.
    pub fn with_classes(
        declarations: Vec<FunctionDeclaration>,
        classes: Vec<ClassDeclaration>,
    ) -> Result<Self, UnitError> {
        Self::with_shared(declarations, classes, Vec::new())
    }

    /// Validate shared constants with their owner initializer functions, classes,
    /// and ordinary declarations. Types, readonly writes, and initialization
    /// dependencies must agree in native construction and saved IR.
    pub fn with_shared(
        declarations: Vec<FunctionDeclaration>,
        mut classes: Vec<ClassDeclaration>,
        mut shared: Vec<SharedConstant>,
    ) -> Result<Self, UnitError> {
        for constant in &mut shared {
            constant.value_type = FieldKind::value_contract(constant.value_type.clone())
                .ok_or_else(|| UnitError {
                    declaration: None,
                    operation: None,
                    message: "unsupported shared constant contract".into(),
                })?;
        }
        for class in &mut classes {
            for field in &mut class.fields {
                if let FieldKind::Value { value_type } = &field.kind {
                    field.kind =
                        FieldKind::from_result(value_type.clone()).ok_or_else(|| UnitError {
                            declaration: None,
                            operation: None,
                            message: "unsupported field value contract".into(),
                        })?;
                } else if let FieldKind::Constant { value_type } = &field.kind {
                    field.kind = FieldKind::Constant {
                        value_type: FieldKind::value_contract(value_type.clone()).ok_or_else(
                            || UnitError {
                                declaration: None,
                                operation: None,
                                message: "unsupported constant field contract".into(),
                            },
                        )?,
                    };
                }
            }
        }
        let unit = Self {
            declarations,
            classes,
            shared,
        };
        unit.validate()?;
        Ok(unit)
    }

    fn validate(&self) -> Result<(), UnitError> {
        self.validate_context(None)
    }

    fn validate_context(&self, context: Option<crate::api::ApiContext>) -> Result<(), UnitError> {
        let fail = |declaration, operation, message| UnitError {
            declaration,
            operation,
            message,
        };
        let valid = |name: &str| {
            name.bytes()
                .next()
                .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
                && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
        };
        let reserved = |name: &str| {
            matches!(
                name,
                "System"
                    | "self"
                    | "function"
                    | "module"
                    | "class"
                    | "extends"
                    | "var"
                    | "return"
                    | "if"
                    | "else"
                    | "while"
                    | "break"
                    | "continue"
                    | "true"
                    | "false"
            )
        };
        let mut bodies = self
            .declarations
            .iter()
            .map(|d| {
                (
                    d.module.as_deref().map_or(Owner::Global, Owner::Module),
                    &d.function,
                )
            })
            .collect::<Vec<_>>();
        let mut names = std::collections::BTreeSet::new();
        for (index, d) in self.declarations.iter().enumerate() {
            if let Some(module) = &d.module {
                if !valid(module) || reserved(module) {
                    return Err(fail(
                        Some(index),
                        None,
                        "invalid or reserved module name".into(),
                    ));
                }
                names.insert(module.as_str());
            }
        }
        for class in &self.classes {
            let at = bodies.len();
            if !valid(&class.name)
                || reserved(&class.name)
                || matches!(
                    class.name.as_str(),
                    "Application"
                        | "AppBase"
                        | "globals"
                        | "statics"
                        | "Toybox"
                        | "initialize"
                        | "println"
                        | "View"
                        | "Object"
                        | "onStart"
                        | "getInitialView"
                )
            {
                return Err(fail(
                    Some(at),
                    None,
                    "invalid or reserved class name".into(),
                ));
            }
            if !names.insert(&class.name) {
                return Err(fail(
                    Some(at),
                    None,
                    "duplicate class or class/module name collision".into(),
                ));
            }
            if class.fields.len() > 256 {
                return Err(fail(
                    Some(at),
                    None,
                    "class exceeds 256 instance fields".into(),
                ));
            }
            // Dictionary dispatch can reach these methods after interface or
            // Object erasure, so validate overrides at the class declaration.
            let object = crate::api::core().class("Toybox.Lang.Object").unwrap();
            for required in object
                .methods
                .iter()
                .filter(|m| matches!(m.name.as_str(), "hashCode" | "equals"))
            {
                if let Some(implementation) =
                    class.methods.iter().find(|m| m.name() == required.name)
                {
                    if !signature_matches(implementation, required) {
                        return Err(fail(
                            Some(at),
                            None,
                            format!(
                                "{}.{} must match the native Object signature",
                                class.name, required.name
                            ),
                        ));
                    }
                }
            }
            if class.base == ClassBase::LangObject
                && class.methods.iter().any(|method| method.name() == "Object")
            {
                return Err(fail(
                    Some(at),
                    None,
                    "method conflicts with native Object base field".into(),
                ));
            }
            let mut field_names = BTreeSet::new();
            for field in &class.fields {
                if let FieldKind::Array { ref element } = field.kind {
                    if crate::ArrayElement::from_api_type(&element.api_type())
                        != Some(element.clone())
                    {
                        return Err(fail(
                            Some(at),
                            None,
                            "unsupported Array field element contract".into(),
                        ));
                    }
                }
                if field
                    .kind
                    .symbol_default()
                    .is_some_and(|name| !ciq_prg::symbols::source::valid_alias(name))
                {
                    return Err(fail(
                        Some(at),
                        None,
                        "Symbol field defaults require an identifier".into(),
                    ));
                }
                if !valid(&field.name)
                    || (class.base == ClassBase::LangObject && field.name == "Object")
                    || reserved(&field.name)
                    || matches!(
                        field.name.as_str(),
                        "View" | "AppBase" | "BehaviorDelegate" | "init"
                    )
                    || crate::api::core()
                        .method(class.base.api_name(), &field.name)
                        .is_some()
                    || class
                        .methods
                        .iter()
                        .any(|method| method.name() == field.name)
                    || !field_names.insert(&field.name)
                {
                    return Err(fail(
                        Some(at),
                        None,
                        "invalid, duplicate, or conflicting instance field".into(),
                    ));
                }
            }
            let mut references = Vec::new();
            for field in &class.fields {
                crate::structural::resolve_result(&mut field.result_type(), &mut |target| {
                    references.push(target.clone());
                    Ok::<_, std::convert::Infallible>(())
                })
                .unwrap();
            }
            for target in &references {
                if !self.classes.iter().any(|declaration| {
                    declaration.name == target.name && declaration.base == target.base
                }) {
                    return Err(fail(
                        Some(at),
                        None,
                        "reference field class and native base must match a declaration".into(),
                    ));
                }
            }
            if class
                .fields
                .iter()
                .any(|field| field.kind.requires_initialization())
                && !class
                    .methods
                    .iter()
                    .any(|method| method.name() == "initialize")
            {
                return Err(fail(
                    Some(at),
                    None,
                    "required fields require a source initializer".into(),
                ));
            }
            let has_constants = class
                .fields
                .iter()
                .any(|field| matches!(field.kind, FieldKind::Constant { .. }));
            if has_constants
                != class
                    .methods
                    .iter()
                    .any(|method| method.name() == FIELD_INITIALIZER)
            {
                return Err(fail(
                    Some(at),
                    None,
                    "constant fields require an allocation initializer".into(),
                ));
            }
            if class.methods.is_empty() {
                return Err(fail(
                    None,
                    None,
                    "class requires at least one method".into(),
                ));
            }
            bodies.extend(
                class
                    .methods
                    .iter()
                    .map(|f| (Owner::Class(class.name.as_str()), f)),
            );
        }
        if bodies.is_empty() || bodies.len() > MAX_FUNCTIONS {
            return Err(fail(
                None,
                None,
                "compilation unit requires 1..=256 functions".into(),
            ));
        }
        let mut signatures = BTreeMap::new();
        let exit_callers = exit_callers(&bodies);
        self.validate_shared(&bodies, context, &exit_callers)?;
        for (index, &(owner, function)) in bodies.iter().enumerate() {
            let mut source_classes = Vec::new();
            let mut collect = |class: &mut crate::SourceClass| {
                source_classes.push(class.clone());
                Ok::<_, std::convert::Infallible>(())
            };
            for mut parameter in function.parameter_types().iter().cloned() {
                crate::structural::resolve_parameter(&mut parameter, &mut collect).unwrap();
            }
            crate::structural::resolve_result(&mut function.return_type().clone(), &mut collect)
                .unwrap();
            for ty in function
                .value_types
                .values()
                .map(crate::ValueType::api_type)
                .chain(
                    function
                        .operations()
                        .iter()
                        .filter_map(|op| match &op.operation {
                            Operation::Cast { target, .. } => Some(target.clone()),
                            Operation::LocalDeclare { value_type, .. } => {
                                Some(value_type.api_type())
                            }
                            _ => None,
                        }),
                )
            {
                if let Some(mut result) = crate::unions::result(ty) {
                    crate::structural::resolve_result(&mut result, &mut collect).unwrap();
                }
            }
            for name in source_classes {
                if !self
                    .classes
                    .iter()
                    .any(|class| class.name == name.name && class.base == name.base)
                {
                    return Err(fail(
                        Some(index),
                        None,
                        format!(
                            "source signature class {name} must match a declaration and native base"
                        ),
                    ));
                }
            }
            let callback = matches!(owner, Owner::Class(_))
                .then(|| {
                    crate::native::callback(
                        self.classes
                            .iter()
                            .find(|c| owner == Owner::Class(c.name.as_str()))
                            .unwrap()
                            .base,
                        function.name(),
                    )
                })
                .flatten();
            if let Some(signature) = callback {
                if function.parameter_types() != signature.parameters
                    || !signature.accepts_result(&function.return_type())
                {
                    return Err(fail(
                        Some(index),
                        None,
                        format!(
                            "{} parameter/result types do not match its native callback contract",
                            function.name()
                        ),
                    ));
                }
            }
            let base_calls = function
                .operations()
                .iter()
                .filter(|op| matches!(op.operation, Operation::NativeBaseInitialize { .. }))
                .count();
            if function.name() == FIELD_INITIALIZER {
                if !matches!(owner, Owner::Class(_))
                    || function.parameter_count() != 0
                    || function.return_type() != ReturnType::Void
                    || base_calls != 0
                {
                    return Err(fail(
                        Some(index), None,
                        "allocation initializer requires an instance, no parameters, Void, and no native base initialization".into(),
                    ));
                }
                let declaration = self
                    .classes
                    .iter()
                    .find(|class| owner == Owner::Class(class.name.as_str()))
                    .unwrap();
                let mut pending = declaration
                    .fields
                    .iter()
                    .filter(|field| matches!(field.kind, FieldKind::Constant { .. }))
                    .map(|field| field.name.as_str())
                    .collect::<BTreeSet<_>>();
                let unreadable = declaration
                    .fields
                    .iter()
                    .filter(|field| field.kind.requires_initialization())
                    .map(|field| field.name.as_str())
                    .collect::<BTreeSet<_>>();
                for (operation, located) in function.operations().iter().enumerate() {
                    if (!pending.is_empty() || !unreadable.is_empty())
                        && (requests_exit(&located.operation)
                            || erased_callbacks(function, &located.operation)
                            || activation_callees(owner, &located.operation, &bodies)
                                .iter()
                                .any(|key| exit_callers.contains(key))
                            || comparator_callee(function, &located.operation, &bodies)
                                .is_some_and(|key| exit_callers.contains(&key)))
                    {
                        return Err(fail(
                            Some(index),
                            Some(operation),
                            "initialize constant fields before calls that may request shutdown"
                                .into(),
                        ));
                    }
                    let message = match &located.operation {
                        Operation::SelfFieldAssign { field, .. }
                            if !pending.remove(field.as_str()) =>
                        {
                            Some("allocation initializer must assign each constant exactly once")
                        }
                        Operation::SelfFieldRead { field, .. }
                            if pending.contains(field.as_str())
                                || unreadable.contains(field.as_str()) =>
                        {
                            Some("field read before allocation initialization")
                        }
                        Operation::SelfValue { .. }
                        | Operation::SelfCall { .. }
                        | Operation::SelfCallStatement { .. }
                            if !pending.is_empty() || !unreadable.is_empty() =>
                        {
                            Some(
                                "initialize required fields before exposing the allocation receiver",
                            )
                        }
                        Operation::Branch { .. } | Operation::Jump { .. }
                            if !pending.is_empty() =>
                        {
                            Some(
                                "constant allocation initialization requires a straight-line prefix",
                            )
                        }
                        Operation::ReturnVoid if !pending.is_empty() => {
                            Some("constant is not initialized on allocation")
                        }
                        _ => None,
                    };
                    if let Some(message) = message {
                        return Err(fail(Some(index), Some(operation), message.into()));
                    }
                }
            }
            if matches!(owner, Owner::Class(_)) && function.name() == "initialize" {
                let first = function
                    .operations()
                    .iter()
                    .find(|op| !matches!(op.operation, Operation::Block { .. }));
                let declaration = self
                    .classes
                    .iter()
                    .find(|class| owner == Owner::Class(class.name.as_str()))
                    .unwrap();
                if (declaration.base == ClassBase::ApplicationAppBase
                    && function.parameter_count() != 0)
                    || function.return_type() != crate::ReturnType::Void
                    || (if declaration.base == ClassBase::LangObject {
                        base_calls != 0
                    } else {
                        base_calls != 1 || !first.is_some_and(|op| matches!(op.operation, Operation::NativeBaseInitialize { base } if base == declaration.base))
                    })
                {
                    return Err(fail(Some(index), None,
                        "source initialize requires Void; Object has no native base call, other bases require one first; AppBase takes no parameters".into()));
                }
                let mut pending = declaration
                    .fields
                    .iter()
                    .filter_map(|field| {
                        if field.kind.requires_initialization() {
                            Some(field.name.as_str())
                        } else {
                            None
                        }
                    })
                    .collect::<BTreeSet<_>>();
                for (operation, located) in function.operations().iter().enumerate() {
                    if !pending.is_empty()
                        && (requests_exit(&located.operation)
                            || erased_callbacks(function, &located.operation)
                            || activation_callees(owner, &located.operation, &bodies)
                                .iter()
                                .any(|key| exit_callers.contains(key))
                            || comparator_callee(function, &located.operation, &bodies)
                                .is_some_and(|key| exit_callers.contains(&key)))
                    {
                        return Err(fail(
                            Some(index),
                            Some(operation),
                            "initialize required fields before calls that may request shutdown"
                                .into(),
                        ));
                    }
                    match &located.operation {
                        Operation::SelfValue { .. } if !pending.is_empty() => return Err(fail(Some(index), Some(operation), "initialize all required fields before exposing self".into())),
                        Operation::SelfFieldRead { field, .. } if pending.contains(field.as_str()) => return Err(fail(Some(index), Some(operation), "required field read before initialization".into())),
                        Operation::SelfFieldAssign { field, .. } => { pending.remove(field.as_str()); }
                        Operation::SelfCall { .. } | Operation::SelfCallStatement { .. } | Operation::Branch { .. } | Operation::Jump { .. } | Operation::ReturnVoid if !pending.is_empty() => return Err(fail(Some(index), Some(operation), "initialize all required fields before self calls, control flow, or return".into())),
                        _ => {}
                    }
                }
            } else if base_calls != 0 {
                return Err(fail(
                    Some(index),
                    None,
                    "native base initialization is supported only in a class initialize method"
                        .into(),
                ));
            }
            if function.is_instance_method() != matches!(owner, Owner::Class(_)) {
                return Err(fail(
                    Some(index),
                    None,
                    "function receiver contract does not match declaration owner".into(),
                ));
            }
            if owner == Owner::Global && names.contains(function.name()) {
                return Err(fail(
                    Some(index),
                    None,
                    "global function collides with a module or class name".into(),
                ));
            }
            if signatures
                .insert(
                    (owner, function.name()),
                    (function.parameter_types(), function.return_type()),
                )
                .is_some()
            {
                return Err(fail(
                    Some(index),
                    None,
                    "duplicate function declaration".into(),
                ));
            }
        }
        for (index, &(owner, function)) in bodies.iter().enumerate() {
            let array_builders = function
                .operations()
                .iter()
                .filter_map(|located| {
                    if let Operation::ArrayBegin {
                        builder,
                        ref element_type,
                        ..
                    } = located.operation
                    {
                        Some((builder, element_type.clone()))
                    } else {
                        None
                    }
                })
                .collect::<BTreeMap<_, _>>();
            let dictionary_builders = function
                .operations()
                .iter()
                .filter_map(|located| {
                    if let Operation::DictionaryBegin {
                        builder,
                        ref dictionary_type,
                        ..
                    } = located.operation
                    {
                        Some((builder, dictionary_type.clone()))
                    } else {
                        None
                    }
                })
                .collect::<BTreeMap<_, _>>();
            let interface_locals = function
                .operations()
                .iter()
                .filter_map(|located| {
                    if let Operation::LocalDeclare { local, value_type } = &located.operation {
                        let ty = crate::ValueType::from_api_type(&value_type.api_type())?;
                        if matches!(
                            ty,
                            crate::ValueType::Interface(_) | crate::ValueType::NullableInterface(_)
                        ) {
                            return Some((*local, ty));
                        }
                    }
                    if let Operation::LocalInitialize { local, value } = located.operation {
                        let ty = function.value_types[&value].clone();
                        if matches!(
                            ty,
                            crate::ValueType::Interface(_) | crate::ValueType::NullableInterface(_)
                        ) {
                            return Some((local, ty));
                        }
                    }
                    None
                })
                .collect::<BTreeMap<_, _>>();
            let origins = crate::source_objects::Origins::for_function(function)
                .map_err(|message| fail(Some(index), None, message.into()))?;
            let source_locals = function
                .operations()
                .iter()
                .filter_map(|located| match &located.operation {
                    Operation::LocalDeclare { local, value_type } => {
                        value_type.source_contract().map(|c| (*local, c))
                    }
                    Operation::LocalInitialize { local, value } => {
                        origins.contract(*value).cloned().map(|c| (*local, c))
                    }
                    _ => None,
                })
                .collect::<BTreeMap<_, _>>();
            let source_blocks = function
                .operations()
                .windows(2)
                .filter_map(|pair| {
                    if let (
                        Operation::Block { block },
                        Operation::BlockParameter { value_type, .. },
                    ) = (&pair[0].operation, &pair[1].operation)
                    {
                        value_type.source_contract().map(|c| (*block, c))
                    } else {
                        None
                    }
                })
                .collect::<BTreeMap<_, _>>();
            let structural_locals = function
                .operations()
                .iter()
                .filter_map(|located| {
                    let (local, ty) = match &located.operation {
                        Operation::LocalDeclare { local, value_type } => {
                            (*local, value_type.api_type())
                        }
                        Operation::LocalInitialize { local, value } => {
                            (*local, function.value_types[value].api_type())
                        }
                        _ => return None,
                    };
                    (crate::structural::contains(&ty) || crate::tuples::contains(&ty))
                        .then_some((local, ty))
                })
                .collect::<BTreeMap<_, _>>();
            let structural_blocks = function
                .operations()
                .windows(2)
                .filter_map(|pair| match (&pair[0].operation, &pair[1].operation) {
                    (Operation::Block { block }, Operation::BlockParameter { value_type, .. })
                        if crate::structural::contains(&value_type.api_type())
                            || crate::tuples::contains(&value_type.api_type()) =>
                    {
                        Some((*block, value_type.api_type()))
                    }
                    _ => None,
                })
                .collect::<BTreeMap<_, _>>();
            for (operation, located) in function.operations().iter().enumerate() {
                if let Operation::SelfFieldAssign { field, .. } = &located.operation {
                    if function.name() != FIELD_INITIALIZER
                        && self
                            .classes
                            .iter()
                            .filter(|class| owner == Owner::Class(class.name.as_str()))
                            .flat_map(|class| &class.fields)
                            .any(|decl| {
                                decl.name == *field
                                    && matches!(decl.kind, FieldKind::Constant { .. })
                            })
                    {
                        return Err(fail(
                            Some(index),
                            Some(operation),
                            "cannot rebind a constant field".into(),
                        ));
                    }
                }
                if let Operation::MemberWrite {
                    receiver, member, ..
                } = &located.operation
                {
                    if origins.contract(*receiver).is_some_and(|contract| {
                        contract.classes().iter().any(|target| {
                            self.classes
                                .iter()
                                .filter(|class| class.name == target.name)
                                .flat_map(|class| &class.fields)
                                .any(|field| {
                                    field.name == *member
                                        && matches!(field.kind, FieldKind::Constant { .. })
                                })
                        })
                    }) {
                        return Err(fail(
                            Some(index),
                            Some(operation),
                            "cannot rebind a constant field".into(),
                        ));
                    }
                }
                if let Operation::BindMethod { selector, .. } = &located.operation {
                    if origins.symbol(*selector).is_some_and(|name| {
                        name == FIELD_INITIALIZER || name.starts_with("__ciq_initialize_shared_")
                    }) {
                        return Err(fail(
                            Some(index),
                            Some(operation),
                            "allocation initializer is not a callable member".into(),
                        ));
                    }
                }
                if let Operation::SelfValue {
                    class: Some(claimed),
                    ..
                } = &located.operation
                {
                    let actual = match owner {
                        Owner::Class(name) => self.classes.iter().find(|class| class.name == name),
                        _ => None,
                    };
                    if !actual.is_some_and(|class| {
                        class.name == claimed.name && class.base == claimed.base
                    }) {
                        return Err(fail(
                            Some(index),
                            Some(operation),
                            "self class does not match its enclosing class".into(),
                        ));
                    }
                }
                if let Operation::BindMethod {
                    receiver,
                    selector,
                    signature,
                    ..
                } = &located.operation
                {
                    let owner = if let Owner::Class(name) = owner {
                        Some(name)
                    } else {
                        None
                    };
                    crate::callbacks::check_binding(
                        self, function, &origins, owner, *receiver, *selector, signature, context,
                    )
                    .map_err(|message| fail(Some(index), Some(operation), message))?;
                }
                if let Operation::MemberRead {
                    receiver,
                    member,
                    field_type: Some(claimed),
                    ..
                }
                | Operation::MemberWrite {
                    receiver,
                    member,
                    field_type: Some(claimed),
                    ..
                } = &located.operation
                {
                    let actual = crate::source_fields::contract(
                        crate::source_fields::receiver(origins.contract(*receiver))
                            .map_err(|message| fail(Some(index), Some(operation), message))?,
                        member,
                        |class, field| {
                            self.classes
                                .iter()
                                .find(|c| c.name == class.name && c.base == class.base)?
                                .fields
                                .iter()
                                .find(|f| f.name == field)
                                .map(InstanceField::result_type)
                        },
                    )
                    .map_err(|message| fail(Some(index), Some(operation), message))?;
                    if actual != *claimed {
                        return Err(fail(
                            Some(index),
                            Some(operation),
                            "source field metadata does not match its declaration".into(),
                        ));
                    }
                    if let Operation::MemberWrite { value, .. } = &located.operation {
                        self.check_value_contract(function, &origins, *value, &actual, context)
                            .map_err(|message| fail(Some(index), Some(operation), message))?;
                    }
                }
                if let Operation::MemberWrite {
                    receiver,
                    member,
                    value,
                    field_type: None,
                } = &located.operation
                {
                    for receiver_type in
                        crate::unions::alternatives(&function.value_types[receiver].api_type())
                    {
                        let expected = crate::structural::field_contract(receiver_type, member)
                            .map_err(|message| fail(Some(index), Some(operation), message))?;
                        self.check_value_contract(function, &origins, *value, &expected, context)
                            .map_err(|message| fail(Some(index), Some(operation), message))?;
                    }
                }
                let structural_boundary = match &located.operation {
                    Operation::Return { value } => {
                        Some((function.return_type().api_type(), *value))
                    }
                    Operation::ParameterAssign { index, value } => {
                        Some((function.parameter_types()[*index].api_type(), *value))
                    }
                    Operation::LocalAssign { local, value } => {
                        structural_locals.get(local).cloned().map(|ty| (ty, *value))
                    }
                    Operation::JumpValue { target, value } => structural_blocks
                        .get(target)
                        .cloned()
                        .map(|ty| (ty, *value)),
                    _ => None,
                };
                if let Some((expected, value)) = structural_boundary.filter(|(ty, value)| {
                    crate::structural::contains(ty)
                        || crate::tuples::contains(ty)
                        || crate::tuples::contains(&function.value_types[value].api_type())
                }) {
                    self.check_value_contract(
                        function,
                        &origins,
                        value,
                        &crate::unions::result(expected).unwrap(),
                        context,
                    )
                    .map_err(|message| fail(Some(index), Some(operation), message))?;
                }
                let source_boundary = match &located.operation {
                    Operation::Return { value } => function
                        .return_type()
                        .source_contract()
                        .map(|c| (c, *value)),
                    Operation::ParameterAssign { index, value } => function.parameter_types()
                        [*index]
                        .source_contract()
                        .map(|c| (c, *value)),
                    Operation::LocalAssign { local, value } => {
                        source_locals.get(local).cloned().map(|c| (c, *value))
                    }
                    Operation::JumpValue { target, value } => {
                        source_blocks.get(target).cloned().map(|c| (c, *value))
                    }
                    _ => None,
                };
                if let Some((expected, value)) = source_boundary {
                    self.check_source_value(function, &origins, value, &expected, context)
                        .map_err(|message| fail(Some(index), Some(operation), message))?;
                }
                let check_element = |element: crate::ArrayElement,
                                     value: crate::ValueId|
                 -> Result<(), UnitError> {
                    let expected = crate::unions::result(element.api_type()).unwrap();
                    self.check_value_contract(function, &origins, value, &expected, context)
                        .map_err(|message| fail(Some(index), Some(operation), message))
                };
                let check_record = |fields: &BTreeMap<String, crate::api::Type>,
                                    key: crate::ValueId,
                                    value: crate::ValueId| {
                    if let Some(ty) = origins.symbol(key).and_then(|key| fields.get(key)) {
                        self.check_api_value_contract(function, &origins, value, ty, context)
                            .map_err(|message| fail(Some(index), Some(operation), message))?;
                    }
                    Ok::<(), UnitError>(())
                };
                let check_dictionary_value = |dictionary: crate::DictionaryType, value| {
                    let crate::api::Type::Dictionary { value: ty, .. } = dictionary.api_type()
                    else {
                        unreachable!()
                    };
                    self.check_value_contract(
                        function,
                        &origins,
                        value,
                        &crate::unions::result(*ty).unwrap(),
                        context,
                    )
                    .map_err(|message| fail(Some(index), Some(operation), message))
                };
                let check_dictionary_key = |dictionary: crate::DictionaryType, value| {
                    let expected = crate::unions::result(dictionary.key.api_type()).unwrap();
                    self.check_value_contract(function, &origins, value, &expected, context)
                        .map_err(|message| fail(Some(index), Some(operation), message))
                };
                match &located.operation {
                    Operation::DictionaryFill {
                        builder, entries, ..
                    } => {
                        if let Some(fields) = function.operations().iter().find_map(|op| match &op
                            .operation
                        {
                            Operation::RecordBegin {
                                builder: owner,
                                fields,
                                ..
                            } if owner == builder => Some(fields),
                            _ => None,
                        }) {
                            for (key, value) in entries {
                                check_record(fields, *key, *value)?;
                            }
                        }
                        if let Some(dictionary) = dictionary_builders.get(builder) {
                            for (key, value) in entries {
                                check_dictionary_key(dictionary.clone(), *key)?;
                                check_dictionary_value(dictionary.clone(), *value)?;
                            }
                        }
                    }
                    Operation::DictionaryWrite {
                        dictionary,
                        key,
                        value,
                    } => {
                        if let crate::ValueType::TypedDictionary(ref dictionary) =
                            function.value_types[dictionary]
                        {
                            check_dictionary_key(dictionary.clone(), *key)?;
                            check_dictionary_value(dictionary.clone(), *value)?;
                        }
                    }
                    Operation::DictionaryRead {
                        dictionary, key, ..
                    } => {
                        if let crate::ValueType::TypedDictionary(ref dictionary) =
                            function.value_types[dictionary]
                        {
                            check_dictionary_key(dictionary.clone(), *key)?;
                        }
                    }
                    Operation::ArrayFill { builder, value, .. } => {
                        if let Some(element) = array_builders.get(builder) {
                            check_element(element.clone(), *value)?;
                        }
                    }
                    Operation::ArrayFillMany {
                        builder, values, ..
                    } => {
                        if let Some(element) = array_builders.get(builder) {
                            for value in values {
                                check_element(element.clone(), *value)?;
                            }
                        }
                    }
                    Operation::ArrayWrite {
                        array,
                        index: key,
                        value,
                    } => {
                        for part in
                            crate::unions::alternatives(&function.value_types[array].api_type())
                        {
                            if let crate::api::Type::Array(element) = part {
                                if let Some(element) = crate::ArrayElement::from_api_type(element) {
                                    check_element(element, *value)?;
                                }
                            } else if let Some(dictionary) =
                                crate::DictionaryType::from_api_type(part)
                            {
                                check_dictionary_key(dictionary.clone(), *key)?;
                                check_dictionary_value(dictionary, *value)?;
                            }
                        }
                    }
                    Operation::ArrayRead {
                        array, index: key, ..
                    } => {
                        for part in
                            crate::unions::alternatives(&function.value_types[array].api_type())
                        {
                            if let Some(dictionary) = crate::DictionaryType::from_api_type(part) {
                                check_dictionary_key(dictionary, *key)?;
                            }
                        }
                    }
                    Operation::NativeCall {
                        receiver,
                        member,
                        arguments,
                        ..
                    }
                    | Operation::NativeCallStatement {
                        receiver,
                        member,
                        arguments,
                    } => {
                        for alternative in
                            crate::unions::alternatives(&function.value_types[receiver].api_type())
                        {
                            if let Some(signature) =
                                crate::structural::method_contract(alternative, member)
                            {
                                for (argument, expected) in
                                    arguments.iter().zip(&signature.parameters)
                                {
                                    let expected = expected
                                        .source_contract()
                                        .map(crate::SourceUnion::result)
                                        .or_else(|| crate::unions::result(expected.api_type()))
                                        .unwrap();
                                    self.check_value_contract(
                                        function, &origins, *argument, &expected, context,
                                    )
                                    .map_err(|message| {
                                        fail(Some(index), Some(operation), message)
                                    })?;
                                }
                            }
                        }
                        for (argument, expected) in arguments.iter().zip(
                            crate::native::value_parameters(
                                &function.value_types[receiver].api_type(),
                                member,
                            )
                            .unwrap_or_default(),
                        ) {
                            if crate::structural::contains(&expected)
                                || crate::tuples::contains(&expected)
                                || crate::tuples::contains(
                                    &function.value_types[argument].api_type(),
                                )
                            {
                                self.check_api_value_contract(
                                    function, &origins, *argument, &expected, context,
                                )
                                .map_err(|message| fail(Some(index), Some(operation), message))?;
                            }
                        }
                        if matches!(
                            function.value_types[receiver],
                            crate::ValueType::Array(_) | crate::ValueType::TypedDictionary(_)
                        ) {
                            let parameters = crate::native::value_parameters(
                                &function.value_types[receiver].api_type(),
                                member,
                            )
                            .unwrap_or_default();
                            for (value, expected) in arguments.iter().zip(parameters) {
                                if let Some(element) = crate::ArrayElement::from_api_type(&expected)
                                {
                                    check_element(element, *value)?;
                                }
                            }
                        }
                    }
                    _ => {}
                }
                if let Operation::Return { value } = &located.operation {
                    if let ReturnType::Interface(interface)
                    | ReturnType::NullableInterface(interface) = function.return_type()
                    {
                        self.check_interface_value(
                            function,
                            &origins,
                            *value,
                            interface,
                            matches!(function.return_type(), ReturnType::NullableInterface(_)),
                        )
                        .map_err(|message| fail(Some(index), Some(operation), message))?;
                    }
                }
                if let Operation::ParameterAssign {
                    index: parameter,
                    value,
                } = located.operation
                {
                    if let crate::ParameterType::Interface(interface)
                    | crate::ParameterType::NullableInterface(interface) =
                        function.parameter_types()[parameter]
                    {
                        self.check_interface_value(
                            function,
                            &origins,
                            value,
                            interface,
                            matches!(
                                function.parameter_types()[parameter],
                                crate::ParameterType::NullableInterface(_)
                            ),
                        )
                        .map_err(|message| fail(Some(index), Some(operation), message))?;
                    }
                }
                if let Operation::LocalAssign { local, value } = located.operation {
                    if let Some(ty) = interface_locals.get(&local) {
                        let (crate::ValueType::Interface(interface)
                        | crate::ValueType::NullableInterface(interface)) = *ty
                        else {
                            unreachable!()
                        };
                        self.check_interface_value(
                            function,
                            &origins,
                            value,
                            interface,
                            matches!(ty, crate::ValueType::NullableInterface(_)),
                        )
                        .map_err(|message| fail(Some(index), Some(operation), message))?;
                    }
                }
                if let Operation::SelfFieldRead { field, .. }
                | Operation::SelfFieldAssign { field, .. } = &located.operation
                {
                    let declaration = self
                        .classes
                        .iter()
                        .find(|class| owner == Owner::Class(class.name.as_str()))
                        .and_then(|class| {
                            class
                                .fields
                                .iter()
                                .find(|declaration| declaration.name == *field)
                        })
                        .ok_or_else(|| {
                            fail(
                                Some(index),
                                Some(operation),
                                format!("unresolved instance field {field}"),
                            )
                        })?;
                    match &located.operation {
                        Operation::SelfFieldRead {
                            result_type,
                            declared_type,
                            ..
                        } if *declared_type.as_ref().unwrap_or(result_type)
                            != declaration.result_type() =>
                        {
                            return Err(fail(
                                Some(index),
                                Some(operation),
                                "field read type does not match its declaration".into(),
                            ));
                        }
                        Operation::SelfFieldAssign { value, .. } => {
                            self.check_value_contract(
                                function,
                                &origins,
                                *value,
                                &declaration.result_type(),
                                context,
                            )
                            .map_err(|message| fail(Some(index), Some(operation), message))?;
                        }
                        _ => {}
                    }
                }
                if let Operation::SourceCast { class, .. }
                | Operation::SourceInstanceOf { class, .. }
                | Operation::BlockParameter {
                    value_type: crate::JoinType::Source { class, .. },
                    ..
                }
                | Operation::LocalDeclare {
                    value_type: crate::JoinType::Source { class, .. },
                    ..
                }
                | Operation::VariableRead {
                    value_type: crate::JoinType::Source { class, .. },
                    ..
                } = &located.operation
                {
                    if !self.classes.iter().any(|declaration| {
                        declaration.name == class.name && declaration.base == class.base
                    }) {
                        return Err(fail(
                            Some(index),
                            Some(operation),
                            "source type target and native base must match a unit declaration"
                                .into(),
                        ));
                    }
                }
                if let Operation::BlockParameter { value_type, .. }
                | Operation::LocalDeclare { value_type, .. }
                | Operation::VariableRead { value_type, .. } = &located.operation
                {
                    if let Some(contract) = value_type.source_contract() {
                        for class in contract.classes() {
                            if !self
                                .classes
                                .iter()
                                .any(|d| d.name == class.name && d.base == class.base)
                            {
                                return Err(fail(Some(index), Some(operation), "source union class and native base must match a unit declaration".into()));
                            }
                        }
                    }
                }
                if let Operation::SourceNew { class, base, .. }
                | Operation::SourceBegin { class, base, .. } = &located.operation
                {
                    let declaration = self
                        .classes
                        .iter()
                        .find(|declaration| declaration.name == *class)
                        .ok_or_else(|| {
                            fail(
                                Some(index),
                                Some(operation),
                                format!("unresolved source construction class {class}"),
                            )
                        })?;
                    if declaration.base != *base {
                        return Err(fail(
                            Some(index),
                            Some(operation),
                            "source construction base does not match the class declaration".into(),
                        ));
                    }
                    if !declaration
                        .methods
                        .iter()
                        .any(|method| method.name() == "initialize")
                    {
                        return Err(fail(
                            Some(index),
                            Some(operation),
                            "source construction requires a declared initializer".into(),
                        ));
                    }
                }
                let (arguments, value_call) = match &located.operation {
                    Operation::ObjectCall { arguments, .. }
                    | Operation::GlobalCall { arguments, .. }
                    | Operation::ModuleCall { arguments, .. }
                    | Operation::SelfCall { arguments, .. } => (arguments, true),
                    Operation::ObjectCallStatement { arguments, .. }
                    | Operation::GlobalCallStatement { arguments, .. }
                    | Operation::ModuleCallStatement { arguments, .. }
                    | Operation::SelfCallStatement { arguments, .. }
                    | Operation::SourceNew { arguments, .. } => (arguments, false),
                    _ => continue,
                };
                let dispatch = if let Operation::ObjectCall {
                    receiver, member, ..
                }
                | Operation::ObjectCallStatement {
                    receiver, member, ..
                } = &located.operation
                {
                    let contract = origins
                        .contract(*receiver)
                        .expect("validated source receiver");
                    for class in contract.classes() {
                        let declaration = self
                            .classes
                            .iter()
                            .find(|c| c.name == class.name && c.base == class.base)
                            .ok_or_else(|| {
                                fail(
                                    Some(index),
                                    Some(operation),
                                    "source receiver class and base must match its declaration"
                                        .into(),
                                )
                            })?;
                        if crate::native::callback(declaration.base, member).is_some() {
                            return Err(fail(
                                Some(index),
                                Some(operation),
                                format!(
                                    "{member} is a native lifecycle callback, not a user-call target"
                                ),
                            ));
                        }
                    }
                    let targets = crate::source_calls::targets(contract, member, |class| {
                        signatures
                            .get(&(Owner::Class(class.name.as_str()), member.as_str()))
                            .map(|(parameters, result)| crate::StructuralMethod {
                                parameters: parameters.to_vec(),
                                result: result.clone(),
                            })
                    });
                    for native in &targets.native {
                        for (argument, expected) in arguments.iter().zip(
                            crate::native::value_parameters(native, member).unwrap_or_default(),
                        ) {
                            let expected = crate::unions::result(expected).ok_or_else(|| {
                                fail(
                                    Some(index),
                                    Some(operation),
                                    "unsupported native parameter contract".into(),
                                )
                            })?;
                            self.check_value_contract(
                                function, &origins, *argument, &expected, context,
                            )
                            .map_err(|message| fail(Some(index), Some(operation), message))?;
                        }
                    }
                    let result = targets
                        .result(
                            member,
                            |native| {
                                origins.native_arguments(
                                    native,
                                    member,
                                    arguments
                                        .iter()
                                        .map(|v| (*v, function.value_types[v].api_type())),
                                )
                            },
                            context,
                        )
                        .map_err(|message| fail(Some(index), Some(operation), message))?;
                    if value_call && result == ReturnType::Void {
                        return Err(fail(
                            Some(index),
                            Some(operation),
                            "source member has no common value result contract".into(),
                        ));
                    }
                    Some(result)
                } else {
                    None
                };
                let mut joined_result = None;
                for key in source_callees(owner, &located.operation) {
                    if key.1 == FIELD_INITIALIZER || key.1.starts_with("__ciq_initialize_shared_") {
                        return Err(fail(
                            Some(index),
                            Some(operation),
                            "allocation initializer is not a callable member".into(),
                        ));
                    }
                    if matches!(
                        located.operation,
                        Operation::SelfCall { .. } | Operation::SelfCallStatement { .. }
                    ) && !matches!(owner, Owner::Class(_))
                    {
                        return Err(fail(
                            Some(index),
                            Some(operation),
                            "self calls require a class method".into(),
                        ));
                    }
                    let Some((parameters, result)) = signatures.get(&key) else {
                        if dispatch.is_some() {
                            continue;
                        }
                        return Err(fail(
                            Some(index),
                            Some(operation),
                            format!("unresolved function {}.{}", key.0.name(), key.1),
                        ));
                    };
                    if matches!(key.0, Owner::Class(_))
                        && crate::native::callback(
                            self.classes
                                .iter()
                                .find(|c| key.0 == Owner::Class(c.name.as_str()))
                                .map(|c| c.base)
                                .unwrap_or(ClassBase::ApplicationAppBase),
                            key.1,
                        )
                        .is_some()
                    {
                        return Err(fail(
                            Some(index),
                            Some(operation),
                            format!(
                                "{} is a native lifecycle callback, not a user-call target",
                                key.1
                            ),
                        ));
                    }
                    if value_call && *result == ReturnType::Void {
                        return Err(fail(
                            Some(index),
                            Some(operation),
                            format!(
                                "void function {}.{} cannot be used as a value",
                                key.0.name(),
                                key.1
                            ),
                        ));
                    }
                    let count = parameters.len();
                    if count != arguments.len() {
                        return Err(fail(
                            Some(index),
                            Some(operation),
                            format!(
                                "{}.{} expects {count} arguments, got {}",
                                key.0.name(),
                                key.1,
                                arguments.len()
                            ),
                        ));
                    }
                    for (argument, parameter) in arguments.iter().zip(*parameters) {
                        if crate::structural::contains(&parameter.api_type())
                            || crate::tuples::contains(&parameter.api_type())
                            || crate::tuples::contains(&function.value_types[argument].api_type())
                        {
                            self.check_value_contract(
                                function,
                                &origins,
                                *argument,
                                &parameter
                                    .source_contract()
                                    .map(crate::SourceUnion::result)
                                    .or_else(|| crate::unions::result(parameter.api_type()))
                                    .unwrap(),
                                context,
                            )
                            .map_err(|message| fail(Some(index), Some(operation), message))?;
                            continue;
                        }
                        if let crate::ParameterType::Interface(interface)
                        | crate::ParameterType::NullableInterface(interface) = parameter
                        {
                            self.check_interface_value(
                                function,
                                &origins,
                                *argument,
                                *interface,
                                matches!(parameter, crate::ParameterType::NullableInterface(_)),
                            )
                            .map_err(|message| fail(Some(index), Some(operation), message))?;
                            continue;
                        }
                        if let Some(expected) = parameter.source_contract() {
                            if self
                                .check_source_value(
                                    function, &origins, *argument, &expected, context,
                                )
                                .is_err()
                            {
                                return Err(fail(
                                    Some(index),
                                    Some(operation),
                                    "argument does not preserve the declared source alternatives"
                                        .into(),
                                ));
                            }
                        }
                        if !crate::native::accepts_boundary(
                            &parameter.api_type(),
                            &function.value_types[argument].api_type(),
                        ) {
                            return Err(fail(
                                Some(index),
                                Some(operation),
                                format!(
                                    "{}.{} argument type does not match parameter contract",
                                    key.0.name(),
                                    key.1
                                ),
                            ));
                        }
                    }
                    if value_call && dispatch.is_none() {
                        joined_result = Some(match joined_result {
                            None => result.clone(),
                            Some(previous) => crate::joins::call_result(&previous, result)
                                .ok_or_else(|| {
                                    fail(
                                        Some(index),
                                        Some(operation),
                                        "source callees have no common value result contract"
                                            .into(),
                                    )
                                })?,
                        });
                    }
                }
                if value_call && dispatch.is_some() {
                    joined_result = dispatch;
                }
                let expected_result = match &located.operation {
                    Operation::GlobalCall { result_type, .. }
                    | Operation::ModuleCall { result_type, .. }
                    | Operation::ObjectCall { result_type, .. }
                    | Operation::SelfCall { result_type, .. } => Some(result_type),
                    _ => None,
                };
                if expected_result != joined_result.as_ref() {
                    return Err(fail(
                        Some(index),
                        Some(operation),
                        "call result type does not match resolved callee contracts".into(),
                    ));
                }
            }
        }
        Ok(())
    }

    /// Check a stored or passed value against its full contract. Exact class
    /// provenance and structural interface witnesses remain separate from the
    /// native projection, including when interfaces occur inside native unions.
    fn check_value_contract(
        &self,
        function: &Function,
        origins: &crate::source_objects::Origins,
        value: crate::ValueId,
        expected: &ReturnType,
        context: Option<crate::api::ApiContext>,
    ) -> Result<(), String> {
        if function.value_types[&value] == crate::ValueType::Any {
            return Ok(());
        }
        if let Some(contract) = expected
            .source_contract()
            .filter(|_| !crate::tuples::contains(&function.value_types[&value].api_type()))
        {
            return self.check_source_value(function, origins, value, &contract, context);
        }
        self.check_api_value_contract(
            function,
            origins,
            value,
            &crate::tuples::exact_type(expected),
            context,
        )
    }

    /// Check a runtime API argument, including erased container declarations
    /// that do not have an ordinary source/IR result tag.
    fn check_api_value_contract(
        &self,
        function: &Function,
        origins: &crate::source_objects::Origins,
        value: crate::ValueId,
        expected: &crate::api::Type,
        context: Option<crate::api::ApiContext>,
    ) -> Result<(), String> {
        let actual = origins
            .contract(value)
            .cloned()
            .map(|c| crate::api::Type::SourceValue(Box::new(c)))
            .unwrap_or_else(|| function.value_types[&value].api_type());
        if let crate::api::Type::SourceValue(contract) = expected {
            if origins.contract(value).is_some()
                && !crate::tuples::contains(&function.value_types[&value].api_type())
            {
                return self.check_source_value(function, origins, value, contract, context);
            }
        }
        // A partial native match carries its actual union through IR. Exact
        // source/structural witnesses still use the full proof below.
        if !matches!(expected, crate::api::Type::SourceValue(_))
            && !matches!(actual, crate::api::Type::SourceValue(_))
            && !crate::native::accepts(expected, &actual)
            && crate::native::accepts_boundary(expected, &actual)
        {
            return crate::native::structural_api(expected, &actual, context);
        }
        if crate::source_union::matches_type_with(
            expected,
            &actual,
            &|class, target| self.check_source_witness(class, target, context),
            &|target, actual| {
                crate::native::accepts(target, actual)
                    && crate::native::structural_api(target, actual, context).is_ok()
            },
        ) {
            return Ok(());
        }
        Err("field assignment type or interface witness does not match its declaration".into())
    }

    /// Resolve a source member witness using the ordinary declaration checks.
    /// Native inherited members retain their API requirements.
    pub(crate) fn check_source_witness(
        &self,
        class: &crate::SourceClass,
        target: &crate::api::Type,
        context: Option<crate::api::ApiContext>,
    ) -> bool {
        if let Some(body) = crate::recursive::unfold(target) {
            return self.check_source_witness(class, &body, context);
        }

        if let Some(crate::ValueType::Interface(interface)) =
            crate::ValueType::from_api_type(target)
        {
            if let Some(context) = context {
                self.check_interface(class.as_str(), interface.name(), context)
                    .is_ok()
            } else {
                self.check_interface_signature(class.as_str(), interface.name())
                    .is_ok()
            }
        } else if let crate::api::Type::Structural(shape) = target {
            self.check_structural_signature(class.as_str(), shape, context)
                .is_ok()
        } else {
            false
        }
    }

    /// Check a class-containing value boundary using retained identities and
    /// unit-resolved interface witnesses. Overlap permits uncertainty without
    /// turning native projections into exact class or interface proofs.
    fn check_source_value(
        &self,
        function: &Function,
        origins: &crate::source_objects::Origins,
        value: crate::ValueId,
        expected: &crate::SourceUnion,
        context: Option<crate::api::ApiContext>,
    ) -> Result<(), String> {
        if function.value_types[&value] == crate::ValueType::Any {
            return Ok(());
        }
        if crate::tuples::contains(&function.value_types[&value].api_type()) {
            return self.check_api_value_contract(
                function,
                origins,
                value,
                &crate::api::Type::SourceValue(Box::new(expected.clone())),
                context,
            );
        }
        let accepted = if let Some(actual) = origins.contract(value) {
            for ty in actual.native() {
                crate::native::structural_api(&expected.native_type(), ty, context)?;
            }
            expected.accepts_boundary_with(actual, |class, ty| {
                self.check_source_witness(class, ty, context)
            })
        } else {
            crate::native::structural_api(
                &expected.native_type(),
                &function.value_types[&value].api_type(),
                context,
            )?;
            expected.accepts_api(&function.value_types[&value].api_type())
        };
        if accepted {
            Ok(())
        } else {
            Err(
                "value does not satisfy the class-containing union or its interface witnesses"
                    .into(),
            )
        }
    }

    /// Require the same interface, a verified scalar, or an exact source witness.
    /// Native-base projection alone cannot establish structural members.
    fn check_interface_value(
        &self,
        function: &Function,
        origins: &crate::source_objects::Origins,
        value: crate::ValueId,
        interface: crate::InterfaceType,
        nullable: bool,
    ) -> Result<(), String> {
        if crate::native::accepts(
            &interface.api_type(nullable),
            &function.value_types[&value].api_type(),
        ) {
            return Ok(());
        }
        let contract = origins
            .contract(value)
            .ok_or("interface argument or return requires a checked source implementation")?;
        if contract.nullable() && !nullable {
            return Err("nullable source union requires a non-null proof".into());
        }
        for ty in contract.native() {
            if !crate::native::accepts(&interface.api_type(nullable), ty) {
                return Err("native union alternative does not satisfy the interface".into());
            }
        }
        for class in contract.classes() {
            self.check_interface_signature(class.as_str(), interface.name())?;
        }
        Ok(())
    }

    /// Check a source class against a named core API interface and API levels.
    ///
    /// Requires directly declared methods with exactly matching parameter and
    /// result contracts. Rejects unknown classes/interfaces and unavailable
    /// interface or signature declarations. This proves a structural signature,
    /// not method behavior or native inheritance.
    pub fn check_interface(
        &self,
        class: &str,
        interface: &str,
        context: crate::api::ApiContext,
    ) -> Result<(), String> {
        self.check_interface_signature(class, interface)?;
        let catalogue = crate::api::core();
        context.check_type(catalogue, &crate::api::Type::named(interface))?;
        let declaration = self.classes.iter().find(|c| c.name == class).unwrap();
        context.check_class(catalogue, declaration.base.api_name())?;
        for member in &catalogue.interface(interface).unwrap().methods {
            context.require(&format!("{interface}.{}", member.name), member.since)?;
            for ty in member
                .parameters
                .iter()
                .chain(std::iter::once(&member.result))
            {
                context.check_type(catalogue, ty)?;
            }
        }
        Ok(())
    }

    /// Check readonly shared slots, reserved initialization bodies, and the
    /// readiness of fields reached through source calls or lazy module startup.
    fn validate_shared(
        &self,
        bodies: &[(Owner<'_>, &Function)],
        context: Option<crate::api::ApiContext>,
        exits: &BTreeSet<(Owner<'_>, &str)>,
    ) -> Result<(), UnitError> {
        let fail = |declaration, operation, message: &str| UnitError {
            declaration,
            operation,
            message: message.into(),
        };
        if self.shared.is_empty() {
            for (index, (_, function)) in bodies.iter().enumerate() {
                if function.name().starts_with("__ciq_initialize_shared_")
                    || function.operations().iter().any(|op| {
                        matches!(
                            op.operation,
                            Operation::SharedRead { .. } | Operation::SharedAssign { .. }
                        )
                    })
                {
                    return Err(fail(
                        Some(index),
                        None,
                        "shared operations require declared shared bindings",
                    ));
                }
            }
            return Ok(());
        }
        let mut fields = BTreeMap::new();
        let mut initializers = BTreeMap::new();
        if self.shared.len() > 256 {
            return Err(fail(None, None, "unit exceeds 256 shared constants"));
        }
        for constant in &self.shared {
            crate::validate_call_names(None, &constant.name)
                .map_err(|message| fail(None, None, message))?;
            if let Some(module) = &constant.module {
                crate::validate_call_names(None, module)
                    .map_err(|message| fail(None, None, message))?;
            }
            if FieldKind::value_contract(constant.value_type.clone()).as_ref()
                != Some(&constant.value_type)
            {
                return Err(fail(
                    None,
                    None,
                    "unsupported or noncanonical shared constant contract",
                ));
            }
            if let Some(context) = context {
                context
                    .check_type(crate::api::core(), &constant.value_type.api_type())
                    .map_err(|message| fail(None, None, &message))?;
            }
            let mut retained = constant.value_type.clone();
            crate::structural::resolve_result(&mut retained, &mut |class| {
                if self
                    .classes
                    .iter()
                    .any(|c| c.name == class.name && c.base == class.base)
                {
                    Ok(())
                } else {
                    Err(fail(
                        None,
                        None,
                        "shared source class must match its declaration",
                    ))
                }
            })?;
            let owner = constant
                .module
                .as_deref()
                .map_or(Owner::Global, Owner::Module);
            let field = (owner, constant.name.as_str());
            if fields.insert(field, constant).is_some()
                || bodies
                    .iter()
                    .any(|(o, f)| *o == owner && f.name() == constant.name)
            {
                return Err(fail(None, None, "duplicate or conflicting shared constant"));
            }
            if constant.module.is_none()
                && (self.classes.iter().any(|c| c.name == constant.name)
                    || self
                        .declarations
                        .iter()
                        .any(|d| d.module.as_ref() == Some(&constant.name)))
            {
                return Err(fail(
                    None,
                    None,
                    "shared constant conflicts with a class or module",
                ));
            }
            let initializer = constant.initializer();
            let Some((index, (_, function))) = bodies
                .iter()
                .enumerate()
                .find(|(_, (o, f))| *o == owner && f.name() == initializer)
            else {
                return Err(fail(
                    None,
                    None,
                    "shared constant requires its reserved initializer",
                ));
            };
            if function.is_instance_method()
                || function.parameter_count() != 0
                || function.return_type() != ReturnType::Void
            {
                return Err(fail(
                    Some(index),
                    None,
                    "shared initializer requires no parameters, a global/module owner, and Void",
                ));
            }
            initializers.insert((owner, function.name()), field);
        }
        // Function summaries include ordinary callees plus lazy initialization of
        // other modules. Globals already exist before any module is activated.
        let callees = |owner, function: &Function, op: &Operation| {
            let mut targets = activation_callees(owner, op, bodies);
            if let Some(key) = comparator_callee(function, op, bodies) {
                targets.push(key);
            }
            targets
                .into_iter()
                .map(|(owner, name)| (owner.name().to_owned(), name.to_owned()))
                .collect::<Vec<_>>()
        };
        let mut reads = bodies
            .iter()
            .map(|(o, f)| {
                (
                    (o.name().to_owned(), f.name().to_owned()),
                    SharedReads::default(),
                )
            })
            .collect::<BTreeMap<_, SharedReads>>();
        loop {
            let previous = reads.clone();
            for (owner, function) in bodies {
                let summary = reads
                    .get_mut(&(owner.name().to_owned(), function.name().to_owned()))
                    .unwrap();
                if !matches!(owner, Owner::Class(_))
                    && !function.name().starts_with("__ciq_initialize_shared_")
                {
                    summary.owners.insert(match owner {
                        Owner::Global => None,
                        Owner::Module(module) => Some((*module).to_owned()),
                        _ => unreachable!(),
                    });
                }
                for op in function.operations() {
                    if let Operation::SharedRead {
                        module,
                        field,
                        result_type,
                        ..
                    } = &op.operation
                    {
                        let target = module.as_deref().map_or(Owner::Global, Owner::Module);
                        let snapshot = initializers.contains_key(&(*owner, function.name()))
                            && target == *owner
                            && *result_type == ReturnType::Null;
                        if !snapshot {
                            summary.fields.insert((module.clone(), field.clone()));
                        }
                        if module.as_deref().map_or(Owner::Global, Owner::Module) != *owner {
                            summary.owners.insert(module.clone());
                        }
                    }
                    for target in callees(*owner, function, &op.operation) {
                        if let Some(extra) = previous.get(&target) {
                            summary.extend(extra);
                        }
                    }
                }
            }
            if reads == previous {
                break;
            }
        }
        let mut ready = BTreeSet::new();
        for constant in &self.shared {
            let owner = constant
                .module
                .as_deref()
                .map_or(Owner::Global, Owner::Module);
            let name = constant.initializer();
            let (index, (_, function)) = bodies
                .iter()
                .enumerate()
                .find(|(_, (o, f))| *o == owner && f.name() == name)
                .unwrap();
            let mut written = false;
            for (operation, op) in function.operations().iter().enumerate() {
                let mut required = SharedReads::default();
                if let Operation::SharedRead {
                    module,
                    field,
                    result_type,
                    ..
                } = &op.operation
                {
                    let target = module.as_deref().map_or(Owner::Global, Owner::Module);
                    let declaration = fields.get(&(target, field.as_str())).ok_or_else(|| {
                        fail(
                            Some(index),
                            Some(operation),
                            "shared read requires a declared binding",
                        )
                    })?;
                    let snapshot =
                        target == owner && !ready.contains(&(module.clone(), field.clone()));
                    let expected = if snapshot {
                        &ReturnType::Null
                    } else {
                        &declaration.value_type
                    };
                    if result_type != expected {
                        return Err(fail(
                            Some(index),
                            Some(operation),
                            "shared initializer read must match its current slot state",
                        ));
                    }
                    if !snapshot {
                        required.fields.insert((module.clone(), field.clone()));
                    }
                    if target != owner {
                        required.owners.insert(module.clone());
                    }
                }
                for target in callees(owner, function, &op.operation) {
                    if let Some(extra) = reads.get(&target) {
                        required.extend(extra);
                    }
                }
                if required.owners.contains(&constant.module) {
                    return Err(fail(
                        Some(index),
                        Some(operation),
                        "shared initialization re-enters its initializing owner",
                    ));
                }
                if required.fields.iter().any(|(module, field)| {
                    *module == constant.module && !ready.contains(&(module.clone(), field.clone()))
                }) {
                    return Err(fail(
                        Some(index),
                        Some(operation),
                        "shared initializer reads an incomplete binding or initialization cycle",
                    ));
                }
                let pending = self.shared.iter().any(|c| {
                    c.module == constant.module
                        && !ready.contains(&(c.module.clone(), c.name.clone()))
                });
                if pending
                    && (requests_exit(&op.operation)
                        || erased_callbacks(function, &op.operation)
                        || callees(owner, function, &op.operation).iter().any(|key| {
                            exits
                                .iter()
                                .any(|(owner, name)| owner.name() == key.0 && *name == key.1)
                        }))
                {
                    return Err(fail(
                        Some(index),
                        Some(operation),
                        "initialize shared bindings before calls that may request shutdown",
                    ));
                }
                match &op.operation {
                    Operation::SharedAssign { module, field, .. } => {
                        if written || *module != constant.module || *field != constant.name {
                            return Err(fail(
                                Some(index),
                                Some(operation),
                                "shared initializer must assign its binding exactly once",
                            ));
                        }
                        written = true;
                        ready.insert((module.clone(), field.clone()));
                    }
                    Operation::Branch { .. }
                    | Operation::Jump { .. }
                    | Operation::JumpValue { .. }
                        if !written =>
                    {
                        return Err(fail(
                            Some(index),
                            Some(operation),
                            "shared initialization requires a straight-line assignment prefix",
                        ));
                    }
                    Operation::ReturnVoid if !written => {
                        return Err(fail(
                            Some(index),
                            Some(operation),
                            "shared binding is not initialized",
                        ));
                    }
                    _ => {}
                }
            }
        }
        for (index, (owner, function)) in bodies.iter().enumerate() {
            if function.name().starts_with("__ciq_initialize_shared_")
                && !initializers.contains_key(&(*owner, function.name()))
            {
                return Err(fail(
                    Some(index),
                    None,
                    "reserved shared initializer has no binding",
                ));
            }
            let origins =
                crate::source_objects::Origins::for_function(function).expect("checked function");
            for (operation, op) in function.operations().iter().enumerate() {
                match &op.operation {
                    Operation::SharedRead {
                        module,
                        field,
                        result_type,
                        ..
                    } => {
                        if initializers.contains_key(&(*owner, function.name())) {
                            continue;
                        }
                        let target = module.as_deref().map_or(Owner::Global, Owner::Module);
                        if fields
                            .get(&(target, field.as_str()))
                            .is_none_or(|c| c.value_type != *result_type)
                        {
                            return Err(fail(
                                Some(index),
                                Some(operation),
                                "shared read must match its declared field contract",
                            ));
                        }
                    }
                    Operation::SharedAssign {
                        module,
                        field,
                        value,
                    } => {
                        let target = module.as_deref().map_or(Owner::Global, Owner::Module);
                        if initializers.get(&(*owner, function.name()))
                            != Some(&(target, field.as_str()))
                        {
                            return Err(fail(
                                Some(index),
                                Some(operation),
                                "cannot rebind a shared constant",
                            ));
                        }
                        self.check_value_contract(
                            function,
                            &origins,
                            *value,
                            &fields[&(target, field.as_str())].value_type,
                            context,
                        )
                        .map_err(|message| fail(Some(index), Some(operation), &message))?;
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }

    fn check_structural_signature(
        &self,
        class: &str,
        shape: &crate::StructuralInterface,
        context: Option<crate::api::ApiContext>,
    ) -> Result<(), String> {
        let declaration = self
            .classes
            .iter()
            .find(|c| c.name == class)
            .ok_or("unknown structural implementation class")?;
        for (name, expected) in &shape.fields {
            let field = declaration
                .fields
                .iter()
                .find(|f| f.name == *name)
                .ok_or_else(|| format!("{class} does not implement field {name}"))?;
            if matches!(field.kind, FieldKind::Constant { .. }) {
                return Err(format!(
                    "{class}.{name} is readonly; interface fields require mutable bindings"
                ));
            }
            if !crate::structural::same_result(&field.result_type(), expected) {
                return Err(format!(
                    "{class}.{name} does not have the invariant interface field type"
                ));
            }
        }
        for (name, required) in &shape.methods {
            let actual = if let Some(method) = declaration.methods.iter().find(|m| m.name() == name)
            {
                crate::StructuralMethod {
                    parameters: method.parameter_types().to_vec(),
                    result: method.return_type().clone(),
                }
            } else {
                let method = crate::api::core()
                    .method_for_type(&crate::api::Type::named(declaration.base.api_name()), name)
                    .ok_or_else(|| format!("{class} does not implement method {name}"))?;
                if let Some(context) = context {
                    context.require(&format!("{class}.{name}"), method.declaration.required_api)?;
                }
                crate::StructuralMethod::from_native(method.parameters, method.result)
                    .ok_or("unsupported native structural signature")?
            };
            if !required.accepts(&actual) {
                return Err(format!(
                    "{class}.{name} does not satisfy its interface signature"
                ));
            }
        }
        Ok(())
    }

    /// Match directly declared source members against an interface signature.
    /// Availability is checked separately when API context is supplied.
    fn check_interface_signature(&self, class: &str, interface: &str) -> Result<(), String> {
        let declaration = self
            .classes
            .iter()
            .find(|c| c.name == class)
            .ok_or_else(|| format!("unresolved source interface implementation {class}"))?;
        let interface = crate::api::core()
            .interface(interface)
            .ok_or_else(|| format!("unknown structural interface {interface}"))?;
        for required in &interface.methods {
            let implementation = declaration
                .methods
                .iter()
                .find(|m| m.name() == required.name)
                .ok_or_else(|| {
                    format!(
                        "{class} does not implement {}.{}",
                        interface.name, required.name
                    )
                })?;
            if !signature_matches(implementation, required) {
                return Err(format!(
                    "{class}.{} does not satisfy the {} signature",
                    required.name, interface.name
                ));
            }
        }
        Ok(())
    }

    /// Return shared bindings in initialization order within each owner.
    pub fn shared_constants(&self) -> &[SharedConstant] {
        &self.shared
    }

    /// Return classes in input-file and class declaration order.
    pub fn classes(&self) -> &[ClassDeclaration] {
        &self.classes
    }

    /// Return declarations in source-file and declaration order.
    pub fn declarations(&self) -> &[FunctionDeclaration] {
        &self.declarations
    }

    /// Lower global/module functions, then class methods, into one assembly model.
    ///
    /// Uses unit-unique CODE/internal/string labels. Calls retain symbolic
    /// module/member or current-receiver references. Ordering within each group
    /// follows input files and declarations; layout is shared across all bodies.
    /// Encoding still requires explicit symbol and literal placements.
    pub fn lower(&self) -> Program {
        let items = self
            .declarations
            .iter()
            .flat_map(|d| {
                backend::lower_owned(&d.function, &d.code_label(), Some(d.module.as_deref()))
                    .items()
                    .to_vec()
            })
            .chain(self.classes.iter().flat_map(|c| {
                c.methods.iter().flat_map(|f| {
                    backend::lower_named(f, &c.code_label(f.name()))
                        .items()
                        .to_vec()
                })
            }))
            .collect();
        Program::new(items).expect("unit identities lower to disjoint assembly labels")
    }

    /// Serialize validated function/class ownership and bodies as unit IR text.
    ///
    /// Inner functions retain optional spans. Classes retain native
    /// base identity without a VM import index. Source filenames are not stored.
    pub fn to_ir_text(&self) -> String {
        let mut text = String::from("ciq-unit\n");
        for constant in &self.shared {
            text.push_str(&format!(
                ".shared-constant {} {} {}\n",
                constant.module.as_deref().unwrap_or("$"),
                constant.name,
                crate::type_text::result_tag(&constant.value_type)
            ));
        }
        for d in &self.declarations {
            match &d.module {
                None => text.push_str(".owner global\n"),
                Some(module) => text.push_str(&format!(".owner module {module}\n")),
            }
            text.push_str(&d.function.to_ir_text());
        }
        for class in &self.classes {
            text.push_str(&format!(".class {} {}\n", class.name, class.base.text()));
            for field in &class.fields {
                text.push_str(&match &field.kind {
                    FieldKind::Constant { value_type } => format!(
                        ".field-constant {} {}\n",
                        field.name,
                        crate::type_text::result_tag(value_type)
                    ),
                    FieldKind::Value { value_type } => format!(
                        ".field-value {} {}\n",
                        field.name,
                        crate::type_text::result_tag(value_type)
                    ),
                    FieldKind::Number(initial) => {
                        format!(".field-number {} {initial}\n", field.name)
                    }
                    FieldKind::LangObject { nullable } => format!(
                        ".field-object {} {}object\n",
                        field.name,
                        if *nullable { "nullable-" } else { "" }
                    ),
                    FieldKind::Array { element } => {
                        format!(".field-array {} {}\n", field.name, element.text())
                    }
                    FieldKind::Dictionary { dictionary_type } => format!(
                        ".field-dictionary {} {}\n",
                        field.name,
                        dictionary_type.text()
                    ),
                    FieldKind::Scalar { ty, nullable } => format!(
                        ".field-scalar {} {}{}\n",
                        field.name,
                        if *nullable { "nullable-" } else { "" },
                        ty.tag()
                    ),
                    FieldKind::Numeric { alias, nullable } => format!(
                        ".field-numeric {} {}{}\n",
                        field.name,
                        if *nullable { "nullable-" } else { "" },
                        alias.tag()
                    ),
                    FieldKind::ByteArray { nullable } => format!(
                        ".field-byte-array {} {}byte-array\n",
                        field.name,
                        if *nullable { "nullable-" } else { "" }
                    ),
                    FieldKind::WeakReference { nullable } => format!(
                        ".field-weak-reference {} {}weak-reference\n",
                        field.name,
                        if *nullable { "nullable-" } else { "" }
                    ),
                    FieldKind::Method { nullable } => format!(
                        ".field-method {} {}method\n",
                        field.name,
                        if *nullable { "nullable-" } else { "" }
                    ),
                    FieldKind::Symbol { initial, nullable } => format!(
                        ".field-symbol {} {}{}\n",
                        field.name,
                        if *nullable {
                            "nullable-symbol"
                        } else {
                            "symbol"
                        },
                        initial
                            .as_ref()
                            .map(|name| format!(" default=:{name}"))
                            .unwrap_or_default()
                    ),
                    FieldKind::SourceObject(_) | FieldKind::NullableSourceObject(_) => {
                        format!(
                            ".field-reference {} {}\n",
                            field.name,
                            crate::type_text::result_tag(&field.result_type())
                        )
                    }
                    FieldKind::Interface {
                        interface,
                        nullable,
                    } => {
                        format!(
                            ".field-interface {} {}{}\n",
                            field.name,
                            if *nullable { "nullable-" } else { "" },
                            interface.tag()
                        )
                    }
                });
            }
            for method in &class.methods {
                text.push_str(&method.to_ir_text());
            }
            text.push_str(".end-class\n");
        }
        text
    }
}

/// A source diagnostic identifying its zero-based input-file position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceDiagnostic {
    /// Zero-based index in the caller's source list; zero for an empty list.
    pub source: usize,
    /// Location and failure description within the selected source.
    pub diagnostic: Diagnostic,
}
impl fmt::Display for SourceDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "source {}:{}", self.source, self.diagnostic)
    }
}
impl std::error::Error for SourceDiagnostic {}

/// Compile one or more source files into a validated declaration unit.
///
/// Collects global/module functions and supported native-derived class methods
/// before resolving qualified calls, so declarations may follow their callers.
/// Modules may be reopened across files; class names and member identities
/// must remain unique. Calls on self resolve within their own class.
/// Resolves the supported native module aliases and imported superclass names;
/// native bindings are file-local and inherited by class/module scopes.
/// Collects scoped typedefs across files before parsing annotations, expanding
/// them in their declaration's native-name environment. Rejects duplicate,
/// unresolved, cyclic, or unsupported targets, including unused aliases.
/// Returned IR contains expanded contracts rather than alias declarations.
/// Supplies an empty zero-argument Void initializer for Object-rooted classes
/// without one, preserving it as an ordinary method in the returned unit.
/// All user callees must be present. Rejects empty units and more than 256
/// functions; existing syntax/type/control-flow bounds apply per function.
/// Diagnostics identify the source index and byte/line/column within that file.
/// Application lifecycle, metadata, signing, and project configuration are not
/// generated here.
pub fn compile_sources(sources: &[&str]) -> Result<CompilationUnit, SourceDiagnostic> {
    compile_sources_context(sources, None)
}

/// Compile a source unit with explicit target and deployment API levels.
/// Checks known unguarded references; device and permission checks are separate.
pub fn compile_sources_with_api(
    sources: &[&str],
    context: crate::api::ApiContext,
) -> Result<CompilationUnit, SourceDiagnostic> {
    compile_sources_context(sources, Some(context))
}

fn compile_sources_context(
    sources: &[&str],
    context: Option<crate::api::ApiContext>,
) -> Result<CompilationUnit, SourceDiagnostic> {
    compile_sources_mapped(sources, context, false).map(|(unit, _)| unit)
}

/// Preserve existing body-to-source origins for optional source diagnostics.
pub(crate) fn compile_sources_mapped(
    sources: &[&str],
    context: Option<crate::api::ApiContext>,
    unchecked: bool,
) -> Result<(CompilationUnit, Vec<(usize, usize)>), SourceDiagnostic> {
    let mut declarations = Vec::new();
    let mut classes = Vec::new();
    let mut origins = Vec::new();
    let mut class_origins = Vec::new();
    let aliases = parser::collect_declarations(sources)?;
    if let Some(context) = context {
        aliases.validate_api(context)?;
    }
    let mut parsed_sources = sources
        .iter()
        .enumerate()
        .map(|(source, text)| {
            parser::declarations(text, aliases.clone(), source)
                .map_err(|diagnostic| SourceDiagnostic { source, diagnostic })
        })
        .collect::<Result<Vec<_>, _>>()?;
    // Normalize Object's default initializer before collecting signatures, so
    // source calls and saved IR use the same ordinary method contract.
    for class in parsed_sources.iter_mut().flat_map(|p| &mut p.classes) {
        if class.base == ClassBase::LangObject
            && !class.methods.iter().any(|m| m.name == "initialize")
        {
            class.methods.push(parser::ParsedFunction {
                name: "initialize".into(),
                parameters: Vec::new(),
                parameter_types: Vec::new(),
                result_type: Some(ReturnType::Void),
                statements: Vec::new(),
                body_span: class.name_span,
            });
        }
    }
    // Constructors retain their implicit Void ABI independently of ordinary
    // unannotated methods, which expose Any to callers.
    for class in parsed_sources.iter_mut().flat_map(|p| &mut p.classes) {
        for method in &mut class.methods {
            if method.name == "initialize" && method.result_type.is_none() {
                method.result_type = Some(ReturnType::Void);
            }
        }
    }
    let bases = parsed_sources
        .iter()
        .flat_map(|p| p.classes.iter())
        .map(|class| (class.name.clone(), class.base))
        .collect::<BTreeMap<_, _>>();
    aliases.validate_constant_members(&bases)?;
    for (index, parsed) in parsed_sources.iter_mut().enumerate() {
        let mut resolve = |reference: &mut crate::SourceClass| -> Result<(), SourceDiagnostic> {
            reference.base =
                bases
                    .get(&reference.name)
                    .copied()
                    .ok_or_else(|| SourceDiagnostic {
                        source: index,
                        diagnostic: Diagnostic::at(
                            sources[index],
                            0,
                            format!(
                                "source reference {} must resolve to a declared class",
                                reference.name
                            ),
                        ),
                    })?;
            Ok(())
        };
        for ast in parsed.functions.iter_mut().map(|(_, ast)| ast).chain(
            parsed
                .classes
                .iter_mut()
                .flat_map(|class| class.methods.iter_mut()),
        ) {
            for parameter in ast.parameter_types.iter_mut().flatten() {
                crate::structural::resolve_parameter(parameter, &mut resolve)?;
            }
            if let Some(result) = &mut ast.result_type {
                crate::structural::resolve_result(result, &mut resolve)?;
            }
        }
        for field in parsed
            .classes
            .iter_mut()
            .flat_map(|class| class.fields.iter_mut())
        {
            let mut result = field.result_type();
            crate::structural::resolve_result(&mut result, &mut resolve)?;
            if result != field.result_type() {
                field.kind = FieldKind::from_result(result).expect("resolved field contract");
            }
        }
    }
    let mut calls = source_ir::CallContext::new();
    calls.constants = aliases.immediate_constants();
    calls.unchecked = unchecked;
    for parsed in &parsed_sources {
        for (module, ast) in &parsed.functions {
            calls.insert(
                (module.clone().unwrap_or_default(), ast.name.clone()),
                source_ir::declared_signature(ast),
            );
        }
        for class in &parsed.classes {
            calls.class_bases.insert(class.name.clone(), class.base);
            calls.source_fields.insert(
                class.name.clone(),
                class
                    .fields
                    .iter()
                    .map(|field| (field.name.clone(), field.result_type()))
                    .collect(),
            );
            for ast in &class.methods {
                calls.insert(
                    (class.name.clone(), ast.name.clone()),
                    source_ir::declared_signature(ast),
                );
            }
        }
    }
    let mut shared = Vec::new();
    infer_stored_constants(sources, &parsed_sources, context, &mut calls)?;
    for parsed in &parsed_sources {
        for (module, declaration) in &parsed.shared {
            shared.push(SharedConstant {
                module: module.clone(),
                name: declaration.name.clone(),
                value_type: calls.shared_fields[&(module.clone(), declaration.name.clone())]
                    .clone(),
            });
        }
    }
    // Literal defaults precede runtime initializers, as DATA defaults do in SDK
    // output. Preserve runtime expression order; do not topologically move calls.
    let shared_order = shared_initialization_order(&parsed_sources, &calls);
    shared.sort_by_key(|constant| {
        shared_order
            .iter()
            .position(|key| *key == (constant.module.clone(), constant.name.clone()))
            .unwrap()
    });
    for (source_index, parsed) in parsed_sources.iter().enumerate() {
        for (module, declaration) in &parsed.shared {
            let source = sources[source_index];
            let constant = shared
                .iter()
                .find(|c| c.module == *module && c.name == declaration.name)
                .unwrap();
            let ast = parser::ParsedFunction {
                name: constant.initializer(),
                parameters: Vec::new(),
                parameter_types: Vec::new(),
                result_type: Some(ReturnType::Void),
                statements: vec![
                    parser::Statement::FieldAssign {
                        name: declaration.name.clone(),
                        value: declaration.initializer.clone(),
                        span: declaration.span,
                    },
                    parser::Statement::Return {
                        value: None,
                        span: declaration.span,
                    },
                ],
                body_span: declaration.span,
            };
            let mut owner_calls = calls.clone();
            owner_calls.current_module = module.clone();
            owner_calls.shared_initializer = true;
            owner_calls.shared_snapshots =
                uninitialized_shared(&shared_order, module.as_deref(), &declaration.name);
            let function = source_ir::build_with_calls(source, ast, context, None, &owner_calls)
                .map_err(|diagnostic| SourceDiagnostic {
                    source: source_index,
                    diagnostic,
                })?;
            origins.push((source_index, declaration.span.start));
            declarations.push(FunctionDeclaration {
                module: module.clone(),
                function,
            });
        }
    }
    // Contracts are complete before lowering any initializer or consumer.
    // Runtime writes retain declaration order and ordinary readiness checks.
    for parsed in &mut parsed_sources {
        for class in &mut parsed.classes {
            if class.constants.is_empty() {
                continue;
            }
            let constants = class.constants.iter().collect::<Vec<_>>();
            let order =
                constant_initialization_order(Owner::Class(&class.name), &constants, &calls);
            let mut assignments = Vec::new();
            for declaration in &class.constants {
                let result = calls.source_fields[&class.name][&declaration.name].clone();
                class.fields.push(InstanceField {
                    name: declaration.name.clone(),
                    kind: FieldKind::Constant {
                        value_type: result.clone(),
                    },
                });
            }
            for index in order {
                let declaration = &class.constants[index];
                assignments.push(parser::Statement::FieldAssign {
                    name: declaration.name.clone(),
                    value: declaration.initializer.clone(),
                    span: declaration.span,
                });
            }
            assignments.push(parser::Statement::Return {
                value: None,
                span: class.name_span,
            });
            class.methods.push(parser::ParsedFunction {
                name: FIELD_INITIALIZER.into(),
                parameters: Vec::new(),
                parameter_types: Vec::new(),
                result_type: Some(ReturnType::Void),
                statements: assignments,
                body_span: class.name_span,
            });
        }
    }
    for (source_index, parsed) in parsed_sources.into_iter().enumerate() {
        let source = sources[source_index];
        let diagnostic = |diagnostic| SourceDiagnostic {
            source: source_index,
            diagnostic,
        };
        for (module, ast) in parsed.functions {
            if origins.len() + class_origins.len() >= MAX_FUNCTIONS {
                return Err(diagnostic(Diagnostic::at(
                    source,
                    ast.body_span.start,
                    "compilation unit exceeds 256-function limit",
                )));
            }
            origins.push((source_index, ast.body_span.start));
            let mut owner_calls = calls.clone();
            owner_calls.current_module = module.clone();
            declarations.push(FunctionDeclaration {
                module,
                function: source_ir::build_with_calls(source, ast, context, None, &owner_calls)
                    .map_err(diagnostic)?,
            });
        }
        for class in parsed.classes {
            if let Some(context) = context {
                context
                    .check_class(crate::api::core(), class.base.api_name())
                    .map_err(|message| diagnostic(Diagnostic::at(source, 0, message)))?;
            }
            let mut methods = Vec::new();
            let mut method_calls = calls.clone();
            method_calls.readonly_fields = class
                .constants
                .iter()
                .map(|value| value.name.clone())
                .collect();
            method_calls.current_class = Some(crate::SourceClass {
                name: class.name.clone(),
                base: class.base,
            });
            method_calls.field_types = class
                .fields
                .iter()
                .map(|field| (field.name.clone(), field.result_type()))
                .collect();
            for ast in &class.methods {
                method_calls.insert(
                    ("self".into(), ast.name.clone()),
                    source_ir::declared_signature(ast),
                );
            }
            for ast in class.methods {
                if origins.len() + class_origins.len() >= MAX_FUNCTIONS {
                    return Err(diagnostic(Diagnostic::at(
                        source,
                        ast.body_span.start,
                        "compilation unit exceeds 256-function limit",
                    )));
                }
                class_origins.push((source_index, ast.body_span.start));
                methods.push(
                    source_ir::build_with_calls(
                        source,
                        ast,
                        context,
                        Some(class.base),
                        &method_calls,
                    )
                    .map_err(diagnostic)?,
                );
            }
            classes.push(ClassDeclaration {
                name: class.name,
                base: class.base,
                fields: class.fields,
                methods,
            });
        }
    }
    origins.extend(class_origins);
    let unit = CompilationUnit {
        declarations,
        classes,
        shared,
    };
    unit.validate_context(context)
        .and_then(|()| context.map_or(Ok(()), |context| unit.validate_declaration_api(context)))
        .map_err(|error| {
            let bodies = unit
                .declarations
                .iter()
                .map(|d| &d.function)
                .chain(unit.classes.iter().flat_map(|c| &c.methods))
                .collect::<Vec<_>>();
            let (source_index, start) = error
                .declaration
                .and_then(|i| origins.get(i).copied())
                .unwrap_or((0, 0));
            let start = error
                .declaration
                .and_then(|i| bodies.get(i))
                .and_then(|f| error.operation.and_then(|op| f.operations().get(op)))
                .and_then(|op| op.span)
                .map_or(start, |span| span.start);
            SourceDiagnostic {
                source: source_index,
                diagnostic: Diagnostic::at(
                    sources.get(source_index).copied().unwrap_or(""),
                    start,
                    error.message,
                ),
            }
        })?;
    Ok((unit, origins))
}

/// Return explicit binding order for every shared owner, independent of types.
fn shared_initialization_order(
    parsed: &[parser::ParsedUnit],
    calls: &source_ir::CallContext,
) -> Vec<(Option<String>, String)> {
    let mut order = Vec::new();
    let owners = parsed
        .iter()
        .flat_map(|unit| unit.shared.iter().map(|(module, _)| module.as_deref()))
        .collect::<BTreeSet<_>>();
    for module in owners {
        let constants = parsed
            .iter()
            .flat_map(|unit| &unit.shared)
            .filter(|(owner, _)| owner.as_deref() == module)
            .map(|(_, value)| value)
            .collect::<Vec<_>>();
        let owner = module.map_or(Owner::Global, Owner::Module);
        for index in constant_initialization_order(owner, &constants, calls) {
            order.push((module.map(str::to_owned), constants[index].name.clone()));
        }
    }
    order
}

/// Identify same-owner slots still containing their Null DATA default at entry
/// to this binding's initializer, including the binding being initialized.
fn uninitialized_shared(
    order: &[(Option<String>, String)],
    module: Option<&str>,
    name: &str,
) -> BTreeSet<(Option<String>, String)> {
    order
        .iter()
        .skip_while(|(owner, field)| owner.as_deref() != module || field != name)
        .filter(|(owner, _)| owner.as_deref() == module)
        .cloned()
        .collect()
}

/// Place literal defaults before runtime expressions. Alias assignments, calls,
/// and other runtime initializers retain declaration order.
/// Class aggregate forward reads remain subject to the SDK's readiness rule.
fn constant_initialization_order(
    owner: Owner<'_>,
    constants: &[&parser::ParsedConstant],
    calls: &source_ir::CallContext,
) -> Vec<usize> {
    use parser::ExpressionKind as E;
    fn scalar(
        expression: &parser::Expression,
        owner: Owner<'_>,
        calls: &source_ir::CallContext,
        names: &BTreeSet<&str>,
    ) -> bool {
        if local_reference(expression, owner).is_some_and(|name| names.contains(name)) {
            return false;
        }
        match &expression.kind {
            E::String(_)
            | E::Long(_)
            | E::Double(_)
            | E::Integer(_)
            | E::Float(_)
            | E::Boolean(_)
            | E::Char(_)
            | E::Symbol(_)
            | E::Null
            | E::NativeConstant { .. } => true,
            _ => crate::constants::evaluate(expression, &mut |path, _| {
                let local = path
                    .strip_prefix("self.")
                    .or_else(|| path.strip_prefix(&format!("{}.", owner.name())))
                    .unwrap_or(path);
                if names.contains(local) {
                    return Err("stored binding requires an ordered read".into());
                }
                let path = path
                    .strip_prefix("self.")
                    .map(|field| format!("{}.{field}", owner.name()))
                    .unwrap_or_else(|| path.to_owned());
                calls
                    .constants
                    .get(&format!("{}.{path}", owner.name()))
                    .or_else(|| calls.constants.get(&path))
                    .cloned()
                    .ok_or_else(|| "not an immediate constant".into())
            })
            .is_ok(),
        }
    }
    fn local_reference<'a>(
        expression: &'a parser::Expression,
        owner: Owner<'_>,
    ) -> Option<&'a str> {
        match &expression.kind {
            E::Local(name) | E::SelfField(name) => Some(name),
            E::MemberRead { receiver, member } if matches!((&receiver.kind,owner),(E::Local(module),Owner::Module(current)) if module==current) => {
                Some(member)
            }
            _ => None,
        }
    }
    let names = constants
        .iter()
        .map(|c| c.name.as_str())
        .collect::<BTreeSet<_>>();
    let mut ready = BTreeSet::new();
    let mut order = Vec::new();
    for (index, constant) in constants.iter().enumerate() {
        let expression = &constant.initializer;
        let is_scalar = scalar(expression, owner, calls, &names);
        let aggregate = !matches!(owner, Owner::Class(_))
            && match &expression.kind {
                E::ArrayLiteral(elements) | E::ByteArrayLiteral(elements) => {
                    elements.iter().all(|e| scalar(e, owner, calls, &names))
                }
                _ => false,
            };
        if is_scalar || aggregate {
            ready.insert(index);
            order.push(index);
        }
    }
    order.extend((0..constants.len()).filter(|index| !ready.contains(index)));
    order
}

/// Resolve stored declaration contracts independently of evaluation order.
/// Only successful, fully checked expression types enter the shared tables;
/// unresolved dependencies are retried after other declarations make progress.
fn infer_stored_constants(
    sources: &[&str],
    parsed: &[parser::ParsedUnit],
    context: Option<crate::api::ApiContext>,
    calls: &mut source_ir::CallContext,
) -> Result<(), SourceDiagnostic> {
    let mut pending = Vec::new();
    let mut class_names = BTreeMap::new();
    for (source, unit) in parsed.iter().enumerate() {
        for (module, declaration) in &unit.shared {
            calls
                .shared_pending
                .insert((module.clone(), declaration.name.clone()));
            pending.push((
                source,
                module.as_deref().map_or(Owner::Global, Owner::Module),
                declaration,
            ));
        }
        for class in &unit.classes {
            class_names.insert(
                class.name.as_str(),
                class
                    .constants
                    .iter()
                    .map(|c| c.name.clone())
                    .collect::<BTreeSet<_>>(),
            );
            pending.extend(
                class
                    .constants
                    .iter()
                    .map(|declaration| (source, Owner::Class(class.name.as_str()), declaration)),
            );
        }
    }
    let shared_order = shared_initialization_order(parsed, calls);
    while !pending.is_empty() {
        let count = pending.len();
        let mut unresolved = Vec::new();
        let mut first_error = None;
        for (source, owner, declaration) in pending {
            let mut owner_calls = calls.clone();
            if !matches!(owner, Owner::Class(_)) {
                let module = match owner {
                    Owner::Module(module) => Some(module),
                    _ => None,
                };
                owner_calls.shared_snapshots =
                    uninitialized_shared(&shared_order, module, &declaration.name);
            }
            let base = match owner {
                Owner::Global => None,
                Owner::Module(module) => {
                    owner_calls.current_module = Some(module.into());
                    None
                }
                Owner::Class(class) => {
                    let base = calls.class_bases[class];
                    owner_calls.current_class = Some(crate::SourceClass {
                        name: class.into(),
                        base,
                    });
                    owner_calls.readonly_fields = class_names[class].clone();
                    owner_calls.field_types = calls.source_fields[class].clone();
                    Some(base)
                }
            };
            match infer_constant(sources[source], declaration, context, base, &owner_calls) {
                Ok(value_type) => match owner {
                    Owner::Class(class) => {
                        calls
                            .source_fields
                            .get_mut(class)
                            .unwrap()
                            .insert(declaration.name.clone(), value_type);
                    }
                    _ => {
                        let module = match owner {
                            Owner::Module(module) => Some(module.to_owned()),
                            _ => None,
                        };
                        calls
                            .shared_pending
                            .remove(&(module.clone(), declaration.name.clone()));
                        calls
                            .shared_fields
                            .insert((module, declaration.name.clone()), value_type);
                    }
                },
                Err(diagnostic) => {
                    first_error.get_or_insert(SourceDiagnostic { source, diagnostic });
                    unresolved.push((source, owner, declaration));
                }
            }
        }
        if unresolved.len() == count {
            return Err(first_error.unwrap());
        }
        pending = unresolved;
    }
    Ok(())
}

/// Infer the stored value with ordinary expression IR before compiling writes.
/// Array literals retain mutable element contracts rather than positional slots.
fn infer_constant(
    source: &str,
    declaration: &parser::ParsedConstant,
    context: Option<crate::api::ApiContext>,
    base: Option<ClassBase>,
    calls: &source_ir::CallContext,
) -> Result<ReturnType, Diagnostic> {
    let probe = parser::ParsedFunction {
        name: FIELD_INITIALIZER.into(),
        parameters: Vec::new(),
        parameter_types: Vec::new(),
        result_type: None,
        statements: vec![parser::Statement::Return {
            value: Some(declaration.initializer.clone()),
            span: declaration.span,
        }],
        body_span: declaration.span,
    };
    let probe = source_ir::build_with_calls(source, probe, context, base, calls)?;
    let value = probe
        .operations()
        .iter()
        .find_map(|op| {
            if let Operation::Return { value } = op.operation {
                Some(value)
            } else {
                None
            }
        })
        .unwrap();
    let origins = crate::source_objects::Origins::for_function(&probe)
        .expect("validated constant expression");
    let mut result = origins
        .contract(value)
        .cloned()
        .map(crate::SourceUnion::result)
        .or_else(|| crate::unions::result(probe.value_types[&value].api_type()))
        .unwrap();
    if matches!(result, ReturnType::Tuple(_)) {
        result = crate::unions::result(
            crate::tuples::array_contract(&[result.api_type()], &[])
                .map_err(|message| Diagnostic::at(source, declaration.span.start, message))?,
        )
        .unwrap();
    }
    FieldKind::value_contract(result).ok_or_else(|| {
        Diagnostic::at(
            source,
            declaration.span.start,
            "unsupported constant field contract",
        )
    })
}

/// Parse provisional unit text and validate owners and call signatures.
/// AppBase classes contain instance-method function IR. Diagnostics
/// use text positions; serialized source spans are provenance only.
pub fn parse_unit_ir(text: &str) -> Result<CompilationUnit, Diagnostic> {
    parse_unit_ir_context(text, None)
}

/// Parse unit IR while checking known unguarded API references against the
/// supplied context. Supply the context again when resuming a saved text build.
pub fn parse_unit_ir_with_api(
    text: &str,
    context: crate::api::ApiContext,
) -> Result<CompilationUnit, Diagnostic> {
    parse_unit_ir_context(text, Some(context))
}

fn parse_unit_ir_context(
    text: &str,
    context: Option<crate::api::ApiContext>,
) -> Result<CompilationUnit, Diagnostic> {
    let rows = text
        .split_inclusive('\n')
        .scan(0, |offset, line| {
            let at = *offset;
            *offset += line.len();
            Some((at, line))
        })
        .collect::<Vec<_>>();
    let fields = |line: &str| {
        line.split('#')
            .next()
            .unwrap()
            .split_whitespace()
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    let header = rows.first().map(|(_, l)| fields(l)).unwrap_or_default();
    if header != ["ciq-unit"] {
        return Err(Diagnostic::at(text, 0, "expected ciq-unit header"));
    }
    let mut i = 1;
    let mut declarations = Vec::new();
    let mut classes = Vec::new();
    let mut shared = Vec::new();
    let mut locations = Vec::new();
    let mut class_locations = Vec::new();
    fn function(
        text: &str,
        rows: &[(usize, &str)],
        i: &mut usize,
        context: Option<crate::api::ApiContext>,
    ) -> Result<Function, Diagnostic> {
        let start = rows.get(*i).map_or(text.len(), |r| r.0);
        while let Some((_, line)) = rows.get(*i) {
            *i += 1;
            if line.split('#').next().unwrap().trim() == ".end" {
                let end = rows.get(*i).map_or(text.len(), |r| r.0);
                return crate::ir_text::parse_function_ir_context(&text[start..end], context)
                    .map_err(|e| Diagnostic::at(text, start + e.offset, e.message));
            }
        }
        Err(Diagnostic::at(text, start, "expected function .end"))
    }
    while let Some(&(at, line)) = rows.get(i) {
        i += 1;
        let parts = fields(line);
        if parts.is_empty() {
            continue;
        }
        match parts
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .as_slice()
        {
            [".shared-constant", owner, name, tag] => {
                let value_type = crate::type_text::call_result(tag)
                    .ok()
                    .and_then(FieldKind::value_contract)
                    .ok_or_else(|| Diagnostic::at(text, at, "unsupported shared constant type"))?;
                shared.push(SharedConstant {
                    module: (*owner != "$").then(|| (*owner).to_owned()),
                    name: (*name).to_owned(),
                    value_type,
                });
            }
            [".owner", "global"] | [".owner", "module", _] => {
                let module = (parts.len() == 3).then(|| parts[2].clone());
                locations.push(at);
                declarations.push(FunctionDeclaration {
                    module,
                    function: function(text, &rows, &mut i, context)?,
                });
            }
            [
                ".class",
                name,
                base @ ("Application.AppBase"
                | "WatchUi.View"
                | "WatchUi.BehaviorDelegate"
                | "Lang.Object"),
            ] => {
                let base = if *base == "Lang.Object" {
                    ClassBase::LangObject
                } else if *base == "Application.AppBase" {
                    ClassBase::ApplicationAppBase
                } else if *base == "WatchUi.View" {
                    ClassBase::WatchUiView
                } else {
                    ClassBase::WatchUiBehaviorDelegate
                };
                if let Some(context) = context {
                    context
                        .check_class(crate::api::core(), base.api_name())
                        .map_err(|message| Diagnostic::at(text, at, message))?;
                }
                let name = (*name).to_owned();
                let mut methods = Vec::new();
                let mut instance_fields = Vec::new();
                let mut ended = false;
                while let Some(&(method_at, line)) = rows.get(i) {
                    let parts = fields(line);
                    if parts.is_empty() {
                        i += 1;
                        continue;
                    }
                    if parts == [".end-class"] {
                        i += 1;
                        ended = true;
                        break;
                    }
                    if parts.first().is_some_and(|part| {
                        matches!(
                            part.as_str(),
                            ".field-constant"
                                | ".field-value"
                                | ".field-number"
                                | ".field-object"
                                | ".field-array"
                                | ".field-dictionary"
                                | ".field-scalar"
                                | ".field-numeric"
                                | ".field-reference"
                                | ".field-interface"
                                | ".field-symbol"
                                | ".field-method"
                                | ".field-weak-reference"
                                | ".field-byte-array"
                        )
                    }) {
                        if (parts.len() != 3 && !(parts[0] == ".field-symbol" && parts.len() == 4))
                            || !methods.is_empty()
                            || instance_fields.len() >= 256
                        {
                            return Err(Diagnostic::at(
                                text,
                                method_at,
                                "expected a three-part field declaration before methods, at most 256 fields",
                            ));
                        }
                        let kind = if parts[0] == ".field-constant" {
                            let value_type = crate::type_text::call_result(&parts[2])
                                .ok()
                                .and_then(FieldKind::value_contract)
                                .ok_or_else(|| {
                                    Diagnostic::at(
                                        text,
                                        method_at,
                                        "unsupported constant field contract",
                                    )
                                })?;
                            FieldKind::Constant { value_type }
                        } else if parts[0] == ".field-value" {
                            crate::type_text::call_result(&parts[2])
                                .ok()
                                .and_then(FieldKind::from_result)
                                .ok_or_else(|| {
                                    Diagnostic::at(
                                        text,
                                        method_at,
                                        "unsupported field value contract",
                                    )
                                })?
                        } else if parts[0] == ".field-number" {
                            FieldKind::Number(parts[2].parse::<i32>().map_err(|_| {
                                Diagnostic::at(text, method_at, "invalid Number field initializer")
                            })?)
                        } else if parts[0] == ".field-object" {
                            FieldKind::LangObject {
                                nullable: match parts[2].as_str() {
                                    "object" => false,
                                    "nullable-object" => true,
                                    _ => {
                                        return Err(Diagnostic::at(
                                            text,
                                            method_at,
                                            "invalid Object field contract",
                                        ));
                                    }
                                },
                            }
                        } else if parts[0] == ".field-array" {
                            let Some(element) = crate::type_text::element(&parts[2]) else {
                                return Err(Diagnostic::at(
                                    text,
                                    method_at,
                                    "invalid Array field contract",
                                ));
                            };
                            FieldKind::Array { element }
                        } else if parts[0] == ".field-dictionary" {
                            let dictionary_type = crate::DictionaryType::parse(&parts[2])
                                .ok_or_else(|| {
                                    Diagnostic::at(
                                        text,
                                        method_at,
                                        "invalid Dictionary field contract",
                                    )
                                })?;
                            FieldKind::Dictionary { dictionary_type }
                        } else if parts[0] == ".field-scalar" {
                            let (tag, nullable) = parts[2]
                                .strip_prefix("nullable-")
                                .map(|tag| (tag, true))
                                .unwrap_or((&parts[2], false));
                            let ty = ScalarFieldType::parse_tag(tag).ok_or_else(|| {
                                Diagnostic::at(text, method_at, "invalid scalar field contract")
                            })?;
                            FieldKind::Scalar { ty, nullable }
                        } else if parts[0] == ".field-numeric" {
                            let (tag, nullable) = parts[2]
                                .strip_prefix("nullable-")
                                .map(|tag| (tag, true))
                                .unwrap_or((&parts[2], false));
                            let alias = match tag {
                                "integer-union" => NumericAlias::Integer,
                                "decimal" => NumericAlias::Decimal,
                                "numeric" => NumericAlias::Numeric,
                                _ => {
                                    return Err(Diagnostic::at(
                                        text,
                                        method_at,
                                        "invalid numeric alias field contract",
                                    ));
                                }
                            };
                            FieldKind::Numeric { alias, nullable }
                        } else if parts[0] == ".field-byte-array" {
                            FieldKind::ByteArray {
                                nullable: match parts[2].as_str() {
                                    "byte-array" => false,
                                    "nullable-byte-array" => true,
                                    _ => {
                                        return Err(Diagnostic::at(
                                            text,
                                            method_at,
                                            "invalid ByteArray field contract",
                                        ));
                                    }
                                },
                            }
                        } else if parts[0] == ".field-weak-reference" {
                            FieldKind::WeakReference {
                                nullable: match parts[2].as_str() {
                                    "weak-reference" => false,
                                    "nullable-weak-reference" => true,
                                    _ => {
                                        return Err(Diagnostic::at(
                                            text,
                                            method_at,
                                            "invalid WeakReference field contract",
                                        ));
                                    }
                                },
                            }
                        } else if parts[0] == ".field-method" {
                            FieldKind::Method {
                                nullable: match parts[2].as_str() {
                                    "method" => false,
                                    "nullable-method" => true,
                                    _ => {
                                        return Err(Diagnostic::at(
                                            text,
                                            method_at,
                                            "invalid Method field contract",
                                        ));
                                    }
                                },
                            }
                        } else if parts[0] == ".field-symbol" {
                            let nullable = match parts[2].as_str() {
                                "symbol" => false,
                                "nullable-symbol" => true,
                                _ => {
                                    return Err(Diagnostic::at(
                                        text,
                                        method_at,
                                        "invalid Symbol field contract",
                                    ));
                                }
                            };
                            let initial = parts
                                .get(3)
                                .map(|value| {
                                    value
                                        .strip_prefix("default=:")
                                        .filter(|name| ciq_prg::symbols::source::valid_alias(name))
                                        .map(str::to_owned)
                                        .ok_or_else(|| {
                                            Diagnostic::at(
                                                text,
                                                method_at,
                                                "Symbol field defaults require default=:name",
                                            )
                                        })
                                })
                                .transpose()?;
                            FieldKind::Symbol { initial, nullable }
                        } else if parts[0] == ".field-interface" {
                            let (tag, nullable) = parts[2]
                                .strip_prefix("nullable-")
                                .map_or((parts[2].as_str(), false), |tag| (tag, true));
                            let interface = match tag {
                                "comparable" => crate::InterfaceType::Comparable,
                                "comparator" => crate::InterfaceType::Comparator,
                                _ => {
                                    return Err(Diagnostic::at(
                                        text,
                                        method_at,
                                        "invalid interface field contract",
                                    ));
                                }
                            };
                            FieldKind::Interface {
                                interface,
                                nullable,
                            }
                        } else {
                            match crate::type_text::call_result(&parts[2]) {
                                Ok(ReturnType::SourceObject(class)) => {
                                    FieldKind::SourceObject(class)
                                }
                                Ok(ReturnType::NullableSourceObject(class)) => {
                                    FieldKind::NullableSourceObject(class)
                                }
                                _ => {
                                    return Err(Diagnostic::at(
                                        text,
                                        method_at,
                                        "invalid source reference field contract",
                                    ));
                                }
                            }
                        };
                        instance_fields.push(InstanceField {
                            name: parts[1].clone(),
                            kind,
                        });
                        i += 1;
                        continue;
                    }
                    if locations.len() + class_locations.len() >= MAX_FUNCTIONS {
                        return Err(Diagnostic::at(
                            text,
                            method_at,
                            "compilation unit exceeds 256-function limit",
                        ));
                    }
                    class_locations.push(method_at);
                    methods.push(function(text, &rows, &mut i, context)?);
                }
                if !ended {
                    return Err(Diagnostic::at(text, at, "expected .end-class"));
                }
                classes.push(ClassDeclaration {
                    name,
                    base,
                    fields: instance_fields,
                    methods,
                });
            }
            _ => {
                return Err(Diagnostic::at(
                    text,
                    at,
                    "expected owner record or AppBase class",
                ));
            }
        }
        if declarations.len() + classes.iter().map(|c| c.methods.len()).sum::<usize>()
            > MAX_FUNCTIONS
        {
            return Err(Diagnostic::at(
                text,
                at,
                "compilation unit exceeds 256-function limit",
            ));
        }
    }
    locations.extend(class_locations);
    CompilationUnit::with_shared(declarations, classes, shared)
        .and_then(|unit| {
            if let Some(context) = context {
                unit.validate_declaration_api(context)?;
                unit.validate_context(Some(context))?;
            }
            Ok(unit)
        })
        .map_err(|e| {
            Diagnostic::at(
                text,
                e.declaration
                    .and_then(|i| locations.get(i).copied())
                    .unwrap_or(0),
                e.message,
            )
        })
}
