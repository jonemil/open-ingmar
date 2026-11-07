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
        Operation::ClassCall { class, member, .. }
        | Operation::ClassCallStatement { class, member, .. } => {
            Some((Owner::Class(class), member))
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

/// Resolve statically selected declarations for access and signature checks.
fn source_bindings<'a>(
    unit: &'a CompilationUnit,
    owner: Owner<'a>,
    operation: &'a Operation,
) -> Vec<(Owner<'a>, &'a str)> {
    let raw = match operation {
        Operation::ObjectCall {
            classes, member, ..
        }
        | Operation::ObjectCallStatement {
            classes, member, ..
        } => classes
            .iter()
            .map(|c| (Owner::Class(c.as_str()), member.as_str()))
            .collect(),
        _ => source_callee(owner, operation)
            .into_iter()
            .collect::<Vec<_>>(),
    };
    raw.into_iter()
        .map(|(owner, member)| match owner {
            Owner::Class(class) => unit
                .method(class, member)
                .map_or((owner, member), |(c, f)| {
                    (Owner::Class(c.name.as_str()), f.name())
                }),
            _ => (owner, member),
        })
        .collect()
}

/// Include every possible instance override in effect summaries. Declaration
/// signatures/access remain tied to the statically selected member.
fn source_callees<'a>(
    unit: &'a CompilationUnit,
    owner: Owner<'a>,
    operation: &'a Operation,
) -> Vec<(Owner<'a>, &'a str)> {
    let bindings = source_bindings(unit, owner, operation);
    let bounds = match operation {
        Operation::ObjectCall { classes, .. } | Operation::ObjectCallStatement { classes, .. } => {
            classes.iter().map(String::as_str).collect::<Vec<_>>()
        }
        Operation::SelfCall { .. } | Operation::SelfCallStatement { .. } => {
            if let Owner::Class(c) = owner {
                vec![c]
            } else {
                Vec::new()
            }
        }
        _ => return bindings,
    };
    let mut targets = bindings.iter().copied().collect::<BTreeSet<_>>();
    for bound in bounds {
        let member = match operation {
            Operation::ObjectCall { member, .. }
            | Operation::ObjectCallStatement { member, .. }
            | Operation::SelfCall { member, .. }
            | Operation::SelfCallStatement { member, .. } => member,
            _ => unreachable!(),
        };
        targets.extend(
            unit.runtime_methods(bound, member)
                .into_iter()
                .map(|(class, method)| (Owner::Class(class.name.as_str()), method.name())),
        );
    }
    targets.into_iter().collect()
}

/// Include allocation-time bodies activated by a module lookup, independently
/// of the selected member. Same-owner access must not restart initialization.
fn activation_callees<'a>(
    unit: &'a CompilationUnit,
    owner: Owner<'a>,
    operation: &'a Operation,
    function: &'a Function,
    bodies: &[(Owner<'a>, &'a Function)],
) -> Vec<(Owner<'a>, &'a str)> {
    let mut calls = effect_callees(unit, owner, operation);
    if let Some((class, _)) = class_store(operation, function) {
        if owner != Owner::Class(class) {
            calls.push((Owner::Class(class), STATIC_INITIALIZER));
        }
    }
    let operation = if let Operation::CallBegin { builder } = operation {
        &function
            .operations()
            .iter()
            .find(|op| op.operation.call_capture() == Some(*builder))
            .expect("validated capture completion")
            .operation
    } else if operation.call_capture().is_some() {
        return calls;
    } else {
        operation
    };

    if let Operation::ClassCall { class, member, .. }
    | Operation::ClassCallStatement { class, member, .. } = operation
    {
        calls.extend(
            unit.static_lookup(class, member, true)
                .into_iter()
                .filter(|c| !(owner == Owner::Class(c) && function.is_static_method()))
                .map(|c| (Owner::Class(c), STATIC_INITIALIZER)),
        );
    }
    let mut static_targets = Vec::new();
    match operation {
        Operation::SelfFieldRead { field, .. } | Operation::SelfFieldAssign { field, .. } => {
            if let Owner::Class(class) = owner {
                if !function.is_static_method()
                    && unit
                        .field(class, field)
                        .is_some_and(|(_, f)| f.kind.class_shared())
                {
                    static_targets.extend(unit.static_lookup(class, field, false));
                }
            }
        }
        Operation::ObjectCall {
            classes, member, ..
        }
        | Operation::ObjectCallStatement {
            classes, member, ..
        } => {
            for class in classes {
                if unit
                    .method(class, member)
                    .is_some_and(|(_, f)| f.is_static_method())
                {
                    static_targets.extend(unit.static_lookup(class, member, true));
                }
            }
        }
        Operation::MemberRead {
            receiver, member, ..
        }
        | Operation::MemberWrite {
            receiver, member, ..
        } if unit.classes.iter().any(|c| {
            c.fields
                .iter()
                .any(|f| f.name == *member && f.kind.class_shared())
        }) =>
        {
            if let Ok(origins) = crate::source_objects::Origins::for_function(function) {
                if let Some(contract) = origins.contract(*receiver) {
                    for target in contract.classes() {
                        if let Some(class) = unit.classes.iter().find(|c| c.name == target.name) {
                            if unit
                                .field(&class.name, member)
                                .is_some_and(|(_, f)| f.kind.class_shared())
                            {
                                static_targets.extend(unit.static_lookup(
                                    &class.name,
                                    member,
                                    false,
                                ));
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    }
    calls.extend(
        static_targets
            .into_iter()
            .filter(|c| !(owner == Owner::Class(c) && function.is_static_method()))
            .map(|c| (Owner::Class(c), STATIC_INITIALIZER)),
    );
    let module = match operation {
        Operation::ModuleValue {
            module: Some(module),
            ..
        }
        | Operation::SharedRead {
            module: Some(module),
            ..
        }
        | Operation::SharedBegin {
            module: Some(module),
            lexical: false,
            ..
        }
        | Operation::ModuleCall { module, .. }
        | Operation::ModuleCallStatement { module, .. } => Some(module.as_str()),
        Operation::ClassValue { class, .. }
        | Operation::SourceBegin { class, .. }
        | Operation::ClassCall { class, .. }
        | Operation::ClassCallStatement { class, .. }
        | Operation::ClassFieldRead { class, .. }
        | Operation::ClassFieldBegin { class, .. } => {
            class.rsplit_once('.').map(|(owner, _)| owner)
        }
        Operation::SourceInstanceOf { class, .. } => {
            class.name.rsplit_once('.').map(|(owner, _)| owner)
        }
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
    if let (Owner::Module(current) | Owner::Class(current), Some(target)) =
        (owner, lexical_target(operation, function))
    {
        if matches!(operation, Operation::SharedBegin { .. }) {
            return calls;
        }
        for scope in crate::names::scopes(current).skip(1) {
            calls.extend(
                bodies
                    .iter()
                    .filter(|(owner, f)| {
                        *owner == Owner::Module(scope)
                            && f.name().starts_with("__ciq_initialize_shared_")
                    })
                    .map(|(owner, f)| (*owner, f.name())),
            );
            if target == Some(scope) {
                break;
            }
        }
    }
    if let Some(module) = module {
        for module in receiver_modules(owner, module, operation.receiver_path()) {
            calls.extend(
                bodies
                    .iter()
                    .filter(|(owner, f)| {
                        *owner == Owner::Module(&module)
                            && f.name().starts_with("__ciq_initialize_shared_")
                    })
                    .map(|(owner, f)| (*owner, f.name())),
            );
        }
    }
    calls
}

/// List initializers reached while searching and traversing a relative namespace receiver.
fn receiver_modules(owner: Owner<'_>, module: &str, path: &[String]) -> Vec<String> {
    if path.is_empty() {
        return Vec::new();
    }
    let parts = module.split('.').collect::<Vec<_>>();
    let base = parts.len().saturating_sub(path.len());
    let parent = parts[..base].join(".");
    let mut result = Vec::new();
    if let Owner::Module(current) | Owner::Class(current) = owner {
        for scope in crate::names::scopes(current) {
            if scope != current {
                result.push(scope.into());
            }
            if scope == parent {
                break;
            }
        }
    }
    for end in base + 1..=parts.len() {
        result.push(parts[..end].join("."));
    }
    result
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

fn effect_callees<'a>(
    unit: &'a CompilationUnit,
    owner: Owner<'a>,
    operation: &'a Operation,
) -> Vec<(Owner<'a>, &'a str)> {
    let mut calls = source_callees(unit, owner, operation);
    if let Operation::SourceBegin { class, .. } | Operation::SourceNew { class, .. } = operation {
        calls.push((Owner::Class(class), FIELD_INITIALIZER));
        if let Some(identity) = unit.member_index.classes.get(class) {
            calls.extend(
                identity
                    .ancestors
                    .iter()
                    .map(|parent| (Owner::Class(parent.as_str()), FIELD_INITIALIZER)),
            );
        }
        calls.push((Owner::Class(class), STATIC_INITIALIZER));
    }
    if let Operation::ClassFieldRead { class, field, .. } = operation {
        calls.extend(
            unit.static_lookup(class, field, false)
                .into_iter()
                .filter(|c| owner != Owner::Class(c))
                .map(|c| (Owner::Class(c), STATIC_INITIALIZER)),
        );
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
fn comparator_callees<'a>(
    unit: &'a CompilationUnit,
    function: &Function,
    operation: &Operation,
) -> Vec<(Owner<'a>, &'a str)> {
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
        _ => return Vec::new(),
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
        return Vec::new();
    }
    let [argument] = arguments.as_slice() else {
        return Vec::new();
    };
    let Some(class) = crate::source_objects::class(
        function.operations(),
        *argument,
        function.parameter_types().to_vec(),
    ) else {
        return Vec::new();
    };
    unit.runtime_methods(&class, "compare")
        .into_iter()
        .map(|(c, f)| (Owner::Class(c.name.as_str()), f.name()))
        .collect()
}

/// Find bodies that can request shutdown, including through recursive source
/// calls. Conservatively include conditional calls; this does not prove safety
/// against runtime failures or unknown native callback effects.
fn exit_callers<'a>(
    unit: &'a CompilationUnit,
    bodies: &[(Owner<'a>, &'a Function)],
) -> BTreeSet<(Owner<'a>, &'a str)> {
    let mut exits = BTreeSet::new();
    loop {
        let mut changed = false;
        for &(owner, function) in bodies {
            if function.operations().iter().any(|located| {
                requests_exit(&located.operation)
                    || erased_callbacks(function, &located.operation)
                    || activation_callees(unit, owner, &located.operation, function, bodies)
                        .iter()
                        .any(|key| exits.contains(key))
                    || comparator_callees(unit, function, &located.operation)
                        .iter()
                        .any(|key| exits.contains(key))
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
    /// None for a global function, otherwise its canonical module path.
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
                crate::names::module_key(module),
                name.len(),
                name
            ),
        }
    }
}

/// Allocation-only field initialization body, excluded from callable members.
pub(crate) const FIELD_INITIALIZER: &str = "__ciq_initialize_fields";
pub(crate) const STATIC_INITIALIZER: &str = "__ciq_initialize_statics";
pub(crate) fn field_initializer(name: &str) -> bool {
    matches!(name, FIELD_INITIALIZER | STATIC_INITIALIZER)
}

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
    /// A mutable class-shared slot with a checked declared or dynamic contract.
    /// Its default and statics initializer run independently of instance allocation.
    StaticVariable {
        value_type: ReturnType,
        initial: crate::ClassFieldInitialization,
    },
    /// A readonly class slot initialized once on first static-member access.
    /// Number defaults live in metadata; other values require the statics initializer.
    /// Unit validation rejects the allocation initialization mode for this kind.
    StaticConstant {
        value_type: ReturnType,
        initial: crate::ClassFieldInitialization,
    },
    /// A readonly class enum slot shared by all instances. Initialization selects
    /// a metadata Number, a statics default, or a write on every allocation.
    Enum {
        value_type: ReturnType,
        initial: crate::ClassFieldInitialization,
    },
    /// A readonly binding populated once by the allocation initializer.
    /// Its referenced contents may remain mutable; metadata starts as Null.
    /// Requires an instance method named `__ciq_initialize_fields` with no
    /// parameters and a Void result; unit checks require exactly one assignment.
    Constant { value_type: ReturnType },
    /// A mutable binding populated by the allocation initializer before initialize.
    /// Metadata starts as Null; the reserved body must assign it exactly once.
    Initialized { value_type: ReturnType },
    /// A supported value contract with a Null metadata slot. Contracts admitting
    /// Null have that default; others require constructor initialization.
    Value { value_type: ReturnType },
    /// A Number copied from class metadata into each new instance.
    Number(i32),
    /// A Number metadata default for a mutable Any slot; later writes may change type.
    DynamicNumber(i32),
    /// A Symbol metadata default for a mutable Any slot; the name must be an identifier.
    DynamicSymbol(String),
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
    /// Whether this member is stored in the class statics record rather than instances.
    pub(crate) fn class_shared(&self) -> bool {
        matches!(
            self,
            Self::Enum { .. } | Self::StaticConstant { .. } | Self::StaticVariable { .. }
        )
    }
    fn static_write(&self) -> bool {
        matches!(
            self,
            Self::Enum {
                initial: crate::ClassFieldInitialization::Static,
                ..
            } | Self::StaticConstant {
                initial: crate::ClassFieldInitialization::Static,
                ..
            } | Self::StaticVariable {
                initial: crate::ClassFieldInitialization::Static,
                ..
            }
        )
    }
    pub(crate) fn readonly(&self) -> bool {
        matches!(
            self,
            Self::Constant { .. } | Self::StaticConstant { .. } | Self::Enum { .. }
        )
    }
    fn allocation_write(&self) -> bool {
        matches!(
            self,
            Self::Constant { .. }
                | Self::Initialized { .. }
                | Self::Enum {
                    initial: crate::ClassFieldInitialization::Allocation,
                    ..
                }
        )
    }
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
        match self {
            Self::Symbol { initial, .. } => initial.as_deref(),
            Self::DynamicSymbol(name) => Some(name),
            _ => None,
        }
    }
}

/// A class member storage declaration with a checked initialization contract.
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
            FieldKind::StaticVariable { value_type, .. }
            | FieldKind::StaticConstant { value_type, .. }
            | FieldKind::Enum { value_type, .. }
            | FieldKind::Constant { value_type }
            | FieldKind::Initialized { value_type }
            | FieldKind::Value { value_type } => value_type.clone(),
            FieldKind::Number(_) => ReturnType::Integer,
            FieldKind::DynamicNumber(_) | FieldKind::DynamicSymbol(_) => ReturnType::Any,
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

/// A named class with a terminal native projection and optional source parent.
/// The unit checks ancestry and the supported member/construction profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassDeclaration {
    /// Direct source superclass; base retains the terminal native projection.
    pub superclass: Option<String>,
    /// Canonical class identity, including the containing module path.
    pub name: String,
    /// Terminal native superclass identity used for API checking and imports.
    pub base: ClassBase,
    /// Member storage in declaration order, including instance and class-shared slots.
    pub fields: Vec<InstanceField>,
    /// Instance/static method IR bodies, checked together during unit construction.
    pub methods: Vec<Function>,
}
impl ClassDeclaration {
    /// Return an owner-qualified CODE label for a method in this class.
    pub fn code_label(&self, method: &str) -> String {
        format!(
            "__ciq_fn_c_{}_{}_{}_{}",
            self.name.len(),
            crate::names::module_key(&self.name),
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

/// Summarize class-shared accesses across source calls and lazy owner activation.
/// Statics initializers use these requirements to reject reentrant reads of
/// still-unwritten slots, including accesses hidden behind another initializer.
fn class_shared_requirements<'a>(
    unit: &'a CompilationUnit,
    bodies: &[(Owner<'a>, &'a Function)],
    classes: &[ClassDeclaration],
) -> BTreeMap<(Owner<'a>, &'a str), BTreeSet<(String, String)>> {
    if !classes
        .iter()
        .any(|c| c.fields.iter().any(|f| f.kind.static_write()))
    {
        return BTreeMap::new();
    }
    let mut reads = bodies
        .iter()
        .map(|(owner, body)| {
            let direct = body
                .operations()
                .iter()
                .filter_map(|op| match &op.operation {
                    Operation::ClassFieldRead { class, field, .. } => {
                        Some(unit.static_field_key(class, field))
                    }
                    Operation::SharedWrite { .. } => {
                        class_store(&op.operation, body).map(|(c, f)| (c.into(), f.into()))
                    }
                    Operation::SelfFieldAssign { field, .. }
                        if body.name() != STATIC_INITIALIZER =>
                    {
                        let Owner::Class(class) = owner else {
                            return None;
                        };
                        classes
                            .iter()
                            .find(|c| c.name == *class)
                            .and_then(|c| c.fields.iter().find(|f| f.name == *field))
                            .filter(|f| matches!(f.kind, FieldKind::StaticVariable { .. }))
                            .map(|_| ((*class).to_owned(), field.clone()))
                    }
                    Operation::SelfFieldRead { field, .. } => {
                        let Owner::Class(class) = owner else {
                            return None;
                        };
                        unit.field(class, field)
                            .filter(|(_, f)| f.kind.class_shared())
                            .map(|_| unit.static_field_key(class, field))
                    }
                    _ => None,
                })
                .collect::<BTreeSet<_>>();
            ((*owner, body.name()), direct)
        })
        .collect::<BTreeMap<_, _>>();
    loop {
        let previous = reads.clone();
        for (owner, body) in bodies {
            let summary = reads.get_mut(&(*owner, body.name())).unwrap();
            for op in body.operations() {
                for callee in effect_callees(unit, *owner, &op.operation)
                    .into_iter()
                    .chain(activation_callees(
                        unit,
                        *owner,
                        &op.operation,
                        body,
                        bodies,
                    ))
                    .chain(comparator_callees(unit, body, &op.operation))
                {
                    if body.name() == STATIC_INITIALIZER && callee == (*owner, STATIC_INITIALIZER) {
                        continue;
                    }
                    if let Some(required) = previous.get(&callee) {
                        summary.extend(required.iter().cloned());
                    }
                }
            }
        }
        if previous == reads {
            return reads;
        }
    }
}
impl SharedReads {
    fn extend(&mut self, other: &Self) {
        self.fields.extend(other.fields.iter().cloned());
        self.owners.extend(other.owners.iter().cloned());
    }
}

/// A slot shared by all accesses to a global or module declaration.
/// The matching reserved function initializes the binding once at owner startup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedBinding {
    /// None selects globals; Some selects a source module.
    pub module: Option<String>,
    /// Identifier within the owner; it must not collide with another value.
    pub name: String,
    /// Stored field contract, excluding Void and Graphics.Dc. Untyped variables
    /// use Any; mutability is independent of the referenced contents.
    pub value_type: ReturnType,
    /// Permit ordinary stores after owner initialization; false for const/enums.
    pub mutable: bool,
}
impl SharedBinding {
    /// Return the reserved ordinary function invoked by the owner's initialization hook.
    pub fn initializer(&self) -> String {
        format!("__ciq_initialize_shared_{}", self.name)
    }
}

/// Validated global/module functions and instance classes with checked calls.
///
/// Qualified user calls and source constructors resolve inside the unit.
/// Arguments and results must match the callee's supported value contracts.
/// Void calls are permitted only as statements. Self calls require a declared
/// method in the enclosing class hierarchy; native callbacks cannot be called directly.
/// CODE lowering leaves symbol/DATA bindings and application metadata external.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompilationUnit {
    member_index: crate::inheritance::MemberIndex,
    visibility: crate::visibility::Members,
    modules: BTreeSet<String>,
    declarations: Vec<FunctionDeclaration>,
    classes: Vec<ClassDeclaration>,
    shared: Vec<SharedBinding>,
    enums: Vec<crate::EnumDeclaration>,
}

impl CompilationUnit {
    /// Resolve an effective field to its declaring class and storage contract.
    pub(crate) fn field(
        &self,
        class: &str,
        member: &str,
    ) -> Option<(&ClassDeclaration, &InstanceField)> {
        let (owner, field) = *self
            .member_index
            .fields
            .get(&(class.into(), member.into()))?;
        Some((&self.classes[owner], &self.classes[owner].fields[field]))
    }
    pub(crate) fn method(
        &self,
        class: &str,
        member: &str,
    ) -> Option<(&ClassDeclaration, &Function)> {
        let (owner, method) = *self
            .member_index
            .methods
            .get(&(class.into(), member.into()))?;
        Some((&self.classes[owner], &self.classes[owner].methods[method]))
    }
    /// Return classes visited by class-qualified lookup, through the declaring
    /// owner. Each visited statics record may initialize independently.
    fn static_lookup<'a>(&'a self, class: &'a str, member: &str, method: bool) -> Vec<&'a str> {
        let owner = if method {
            self.method(class, member).map(|(c, _)| c.name.as_str())
        } else {
            self.field(class, member).map(|(c, _)| c.name.as_str())
        };
        let mut path = vec![class];
        if owner != Some(class) {
            if let Some(identity) = self.member_index.classes.get(class) {
                for parent in &identity.ancestors {
                    path.push(parent.as_str());
                    if Some(parent.as_str()) == owner {
                        break;
                    }
                }
            }
        }
        path
    }
    /// Report whether this class needs a distinct static receiver, including
    /// inherited static members. Its record contains only its own storage.
    pub(crate) fn has_statics(&self, class: &str) -> bool {
        self.classes
            .iter()
            .filter(|c| self.is_subclass(class, &c.name))
            .any(|c| {
                c.fields.iter().any(|f| f.kind.class_shared())
                    || c.methods.iter().any(Function::is_static_method)
            })
    }
    fn static_field_key(&self, class: &str, field: &str) -> (String, String) {
        (
            self.field(class, field)
                .map_or(class, |(c, _)| c.name.as_str())
                .into(),
            field.into(),
        )
    }
    /// Check readonly selectors reachable by a direct self store. Private fields
    /// have separate selectors; explicit receiver stores keep their static bound.
    fn self_field_readonly(&self, class: &str, member: &str) -> bool {
        let Some((owner, field)) = self.field(class, member) else {
            return false;
        };
        if field.kind.readonly() {
            return true;
        }
        if field.kind.class_shared()
            || self.member_visibility(&owner.name, member) == crate::Visibility::Private
        {
            return false;
        }
        self.classes.iter().any(|candidate| {
            self.is_subclass(&candidate.name, class)
                && candidate
                    .fields
                    .iter()
                    .any(|f| f.name == member && f.kind.readonly() && !f.kind.class_shared())
                && self.member_visibility(&candidate.name, member) != crate::Visibility::Private
        })
    }
    fn runtime_methods(&self, bound: &str, member: &str) -> Vec<(&ClassDeclaration, &Function)> {
        let Some((class, method)) = self.method(bound, member) else {
            return Vec::new();
        };
        if method.is_static_method()
            || self.member_visibility(&class.name, member) == crate::Visibility::Private
        {
            return vec![(class, method)];
        }
        let mut targets = BTreeSet::new();
        for class in &self.classes {
            if self.is_subclass(&class.name, bound) {
                if let Some(index) = self
                    .member_index
                    .methods
                    .get(&(class.name.clone(), member.into()))
                {
                    targets.insert(*index);
                }
            }
        }
        targets
            .into_iter()
            .map(|(owner, method)| (&self.classes[owner], &self.classes[owner].methods[method]))
            .collect()
    }
    fn method_owner<'a>(&'a self, class: &'a str, member: &str) -> &'a str {
        self.method(class, member)
            .map(|(c, _)| c.name.as_str())
            .or_else(|| self.field(class, member).map(|(c, _)| c.name.as_str()))
            .unwrap_or(class)
    }
    fn is_subclass(&self, class: &str, parent: &str) -> bool {
        self.member_index
            .classes
            .get(class)
            .is_some_and(|c| c.name == parent || c.ancestors.iter().any(|a| a == parent))
    }
    /// Apply class-member access declarations and recheck all uses. Duplicate
    /// entries and names without a declared field/method are rejected.
    pub fn with_visibility(
        mut self,
        members: Vec<(String, String, crate::Visibility)>,
    ) -> Result<Self, UnitError> {
        if members.is_empty() {
            return Ok(self);
        }
        for (class, member, access) in members {
            let key = (class.clone(), member.clone());
            if self.visibility.insert(key, access).is_some() {
                return Err(UnitError {
                    declaration: None,
                    operation: None,
                    message: "duplicate member visibility".into(),
                });
            }
        }
        self.member_index = crate::inheritance::member_index(&self.classes, &self.visibility)
            .map_err(|message| UnitError {
                declaration: None,
                operation: None,
                message,
            })?;
        self.validate()?;
        self.visibility
            .retain(|_, access| *access != crate::Visibility::Public);
        Ok(self)
    }
    /// Return a member's effective access, resolving its declaring owner.
    /// Omitted access is public.
    pub fn member_visibility(&self, class: &str, member: &str) -> crate::Visibility {
        let owner = self.method_owner(class, member);
        self.visibility
            .get(&(owner.into(), member.into()))
            .copied()
            .unwrap_or_default()
    }
    pub(crate) fn has_private_members(&self, class: &str) -> bool {
        self.visibility
            .iter()
            .any(|((owner, _), access)| owner == class && *access == crate::Visibility::Private)
    }
    pub(crate) fn member_symbol(&self, class: &str, member: &str) -> String {
        if member == FIELD_INITIALIZER {
            return crate::visibility::private_symbol(class, FIELD_INITIALIZER);
        }
        if self.member_visibility(class, member) == crate::Visibility::Private {
            crate::visibility::private_symbol(self.method_owner(class, member), member)
        } else {
            member.into()
        }
    }
    fn check_member_access(
        &self,
        owner: Owner<'_>,
        class: &str,
        member: &str,
    ) -> Result<(), String> {
        let declaring = self.method_owner(class, member);
        let access = self.member_visibility(class, member);
        let allowed = match access {
            crate::Visibility::Public => true,
            crate::Visibility::Private => owner == Owner::Class(declaring),
            crate::Visibility::Protected => {
                matches!(owner,Owner::Class(c) if self.is_subclass(c,declaring))
            }
        };
        if !allowed {
            return Err(format!(
                "{class}.{member} is not accessible from this declaration"
            ));
        }
        Ok(())
    }
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
                if method.is_instance_method()
                    && crate::native::callback(class.base, method.name()).is_some()
                {
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
    /// Resolves self calls through the enclosing class and its ancestors, and module calls
    /// only against module functions. Checks owner/name uniqueness, exact arity,
    /// receiver contracts, and call argument/result types. Requires 1..=256 bodies.
    /// Class errors use flattened body indexes after all global/module functions.
    pub fn with_classes(
        declarations: Vec<FunctionDeclaration>,
        classes: Vec<ClassDeclaration>,
    ) -> Result<Self, UnitError> {
        Self::with_shared(declarations, classes, Vec::new())
    }

    /// Validate shared bindings with their owner initializer functions, classes,
    /// and ordinary declarations. Types, readonly writes, and initialization
    /// dependencies must agree in native construction and saved IR.
    pub fn with_shared(
        declarations: Vec<FunctionDeclaration>,
        classes: Vec<ClassDeclaration>,
        shared: Vec<SharedBinding>,
    ) -> Result<Self, UnitError> {
        Self::with_enums(declarations, classes, shared, Vec::new())
    }

    /// Validate nominal source enum declarations together with functions and storage.
    /// Every enum type used by a body or contract must have exactly one declaration.
    pub fn with_enums(
        declarations: Vec<FunctionDeclaration>,
        classes: Vec<ClassDeclaration>,
        shared: Vec<SharedBinding>,
        enums: Vec<crate::EnumDeclaration>,
    ) -> Result<Self, UnitError> {
        Self::with_modules(declarations, classes, shared, enums, Vec::new())
    }

    /// Validate explicit modules, including empty scopes, with all owned declarations.
    /// Owners imply their containing modules; paths and member collisions are checked.
    pub fn with_modules(
        declarations: Vec<FunctionDeclaration>,
        classes: Vec<ClassDeclaration>,
        shared: Vec<SharedBinding>,
        enums: Vec<crate::EnumDeclaration>,
        modules: Vec<String>,
    ) -> Result<Self, UnitError> {
        Self::with_modules_and_visibility(declarations, classes, shared, enums, modules, Vec::new())
    }

    /// Validate declarations and member access together. Use this constructor
    /// when private inherited names distinguish otherwise unrelated methods.
    /// Duplicate or undeclared access records are rejected before canonicalization.
    pub fn with_modules_and_visibility(
        declarations: Vec<FunctionDeclaration>,
        mut classes: Vec<ClassDeclaration>,
        mut shared: Vec<SharedBinding>,
        enums: Vec<crate::EnumDeclaration>,
        mut modules: Vec<String>,
        access: Vec<(String, String, crate::Visibility)>,
    ) -> Result<Self, UnitError> {
        let mut visibility = crate::visibility::Members::new();
        for (class, member, access) in access {
            if visibility.insert((class, member), access).is_some() {
                return Err(UnitError {
                    declaration: None,
                    operation: None,
                    message: "duplicate member visibility".into(),
                });
            }
        }
        modules.extend(
            classes
                .iter()
                .filter_map(|c| c.name.rsplit_once('.').map(|(owner, _)| owner.to_owned())),
        );
        let modules = module_names(modules, &declarations, &shared);
        if modules.len() > 256 {
            return Err(UnitError {
                declaration: None,
                operation: None,
                message: "unit exceeds 256 module scopes".into(),
            });
        }
        for constant in &mut shared {
            constant.value_type = FieldKind::value_contract(constant.value_type.clone())
                .ok_or_else(|| UnitError {
                    declaration: None,
                    operation: None,
                    message: "unsupported shared binding contract".into(),
                })?;
        }
        for class in &mut classes {
            for field in &mut class.fields {
                if let FieldKind::Enum {
                    value_type,
                    initial,
                }
                | FieldKind::StaticConstant {
                    value_type,
                    initial,
                }
                | FieldKind::StaticVariable {
                    value_type,
                    initial,
                } = &field.kind
                {
                    if matches!(
                        field.kind,
                        FieldKind::StaticConstant {
                            initial: crate::ClassFieldInitialization::Allocation,
                            ..
                        } | FieldKind::StaticVariable {
                            initial: crate::ClassFieldInitialization::Allocation,
                            ..
                        }
                    ) {
                        return Err(UnitError {
                            declaration: None,
                            operation: None,
                            message: "static fields cannot use allocation initialization".into(),
                        });
                    }
                    if matches!(initial, crate::ClassFieldInitialization::Number(_))
                        && !crate::native::accepts(
                            crate::enums::project(&value_type.api_type()),
                            &crate::api::Type::named("Toybox.Lang.Number"),
                        )
                    {
                        return Err(UnitError {
                            declaration: None,
                            operation: None,
                            message: "class Number default does not satisfy its field contract"
                                .into(),
                        });
                    }
                    if FieldKind::value_contract(value_type.clone()).is_none() {
                        return Err(UnitError {
                            declaration: None,
                            operation: None,
                            message: "unsupported class-shared field contract".into(),
                        });
                    }
                }
                if let FieldKind::Value { value_type } = &field.kind {
                    field.kind =
                        FieldKind::from_result(value_type.clone()).ok_or_else(|| UnitError {
                            declaration: None,
                            operation: None,
                            message: "unsupported field value contract".into(),
                        })?;
                } else if let FieldKind::Constant { value_type }
                | FieldKind::Initialized { value_type } = &field.kind
                {
                    let value_type =
                        FieldKind::value_contract(value_type.clone()).ok_or_else(|| UnitError {
                            declaration: None,
                            operation: None,
                            message: "unsupported allocation field contract".into(),
                        })?;
                    field.kind = if field.kind.readonly() {
                        FieldKind::Constant { value_type }
                    } else {
                        FieldKind::Initialized { value_type }
                    };
                }
            }
        }
        let member_index =
            crate::inheritance::member_index(&classes, &visibility).map_err(|message| {
                UnitError {
                    declaration: None,
                    operation: None,
                    message,
                }
            })?;
        let mut unit = Self {
            member_index,
            visibility,
            modules,
            declarations,
            classes,
            shared,
            enums,
        };
        unit.validate()?;
        unit.visibility
            .retain(|_, access| *access != crate::Visibility::Public);
        Ok(unit)
    }

    fn validate(&self) -> Result<(), UnitError> {
        self.validate_context(None)
    }

    fn validate_context(&self, context: Option<crate::api::ApiContext>) -> Result<(), UnitError> {
        crate::enums::validate_unit(self)?;
        let hierarchy = &self.member_index.classes;
        if self.classes.iter().any(|c| {
            c.fields
                .iter()
                .any(|f| f.name.starts_with("__ciq_private_"))
                || c.methods
                    .iter()
                    .any(|f| f.name().starts_with("__ciq_private_"))
        }) {
            return Err(UnitError {
                declaration: None,
                operation: None,
                message: "class members use a reserved private-selector prefix".into(),
            });
        }
        for (class, member) in self.visibility.keys() {
            if !self.classes.iter().any(|c| {
                c.name == *class
                    && (c.fields.iter().any(|f| f.name == *member)
                        || c.methods.iter().any(|f| f.name() == member))
            }) {
                return Err(UnitError {
                    declaration: None,
                    operation: None,
                    message: format!("visibility target {class}.{member} is not declared"),
                });
            }
        }

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
                if !crate::names::module_path(module) || module.split('.').any(reserved) {
                    return Err(fail(
                        Some(index),
                        None,
                        "invalid or reserved module name".into(),
                    ));
                }
                names.insert(module.as_str());
            }
        }
        for module in &self.modules {
            if !crate::names::module_path(module) || module.split('.').any(reserved) {
                return Err(fail(None, None, "invalid or reserved module path".into()));
            }
            names.insert(module.as_str());
            let (parent, member) = module
                .rsplit_once('.')
                .map_or((None, module.as_str()), |(parent, member)| {
                    (Some(parent), member)
                });
            if self
                .declarations
                .iter()
                .any(|d| d.module.as_deref() == parent && d.function.name() == member)
                || self
                    .shared
                    .iter()
                    .any(|binding| binding.module.as_deref() == parent && binding.name == member)
            {
                return Err(fail(
                    None,
                    None,
                    "module conflicts with an owned value or function".into(),
                ));
            }
        }
        for class in &self.classes {
            let at = bodies.len();
            if !crate::names::module_path(&class.name)
                || class.name.split('.').any(reserved)
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
            let (parent, member) = class
                .name
                .rsplit_once('.')
                .map_or((None, class.name.as_str()), |(parent, member)| {
                    (Some(parent), member)
                });
            if self
                .declarations
                .iter()
                .any(|d| d.module.as_deref() == parent && d.function.name() == member)
                || self
                    .shared
                    .iter()
                    .any(|b| b.module.as_deref() == parent && b.name == member)
            {
                return Err(fail(
                    Some(at),
                    None,
                    "class conflicts with an owned value or function".into(),
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
                self.check_static_origins(crate::tuples::exact_type(&field.result_type()))
                    .map_err(|message| fail(Some(at), None, message))?;
                crate::structural::resolve_result(&mut field.result_type(), &mut |target| {
                    references.push(target.clone());
                    Ok::<_, std::convert::Infallible>(())
                })
                .unwrap();
            }
            for target in &references {
                if hierarchy.get(&target.name) != Some(target) {
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
                .any(|field| field.kind.allocation_write());
            if has_constants
                != class
                    .methods
                    .iter()
                    .any(|method| method.name() == FIELD_INITIALIZER)
            {
                return Err(fail(
                    Some(at),
                    None,
                    "allocation fields require their reserved initializer".into(),
                ));
            }
            if class.fields.iter().any(|field| field.kind.static_write())
                != class.methods.iter().any(|f| f.name() == STATIC_INITIALIZER)
            {
                return Err(fail(
                    Some(at),
                    None,
                    "class static values require their initializer".into(),
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
        for class in &self.classes {
            for ancestor in &hierarchy[&class.name].ancestors {
                let parent = self.classes.iter().find(|c| c.name == *ancestor).unwrap();
                for member in class
                    .fields
                    .iter()
                    .map(|f| f.name.as_str())
                    .chain(class.methods.iter().map(Function::name))
                {
                    if member == "initialize" || field_initializer(member) {
                        continue;
                    }
                    let inherited = parent.fields.iter().find(|f| f.name == member);
                    let declared = class.fields.iter().find(|f| f.name == member);
                    if let Some(inherited) = inherited.filter(|_| {
                        self.member_visibility(&parent.name, member) != crate::Visibility::Private
                    }) {
                        let Some(declared) = declared else {
                            return Err(fail(
                                None,
                                None,
                                "field/method shadowing is not implemented".into(),
                            ));
                        };
                        if inherited.kind.class_shared() || declared.kind.class_shared() {
                            return Err(fail(
                                None,
                                None,
                                "inherited static field shadowing is not implemented".into(),
                            ));
                        }
                        if self.member_visibility(&parent.name, member)
                            == crate::Visibility::Protected
                        {
                            return Err(fail(
                                None,
                                None,
                                format!("protected inherited field {member} cannot be redeclared"),
                            ));
                        }
                    }
                    if declared.is_some() && parent.methods.iter().any(|f| f.name() == member) {
                        return Err(fail(
                            None,
                            None,
                            "field/method shadowing is not implemented".into(),
                        ));
                    }
                }
            }
        }
        for class in &self.classes {
            let Some(parent) = class.superclass.as_deref() else {
                continue;
            };
            for method in &class.methods {
                if method.name() == "initialize" || field_initializer(method.name()) {
                    continue;
                }
                if let Some((owner, inherited)) = self.method(parent, method.name()) {
                    let access = self.member_visibility(&owner.name, inherited.name());
                    if access == crate::Visibility::Private {
                        continue;
                    }
                    if inherited.is_static_method()
                        || self.member_visibility(&class.name, method.name()) != access
                        || method.is_static_method() != inherited.is_static_method()
                    {
                        return Err(fail(
                            None,
                            None,
                            "inherited access/receiver-kind shadowing is not implemented".into(),
                        ));
                    }
                    if method.parameter_types().len() != inherited.parameter_types().len()
                        || !method
                            .parameter_types()
                            .iter()
                            .zip(inherited.parameter_types())
                            .all(|(a, b)| crate::inheritance::override_parameter(a, b))
                        || !crate::inheritance::override_result(
                            &method.return_type(),
                            &inherited.return_type(),
                        )
                    {
                        return Err(fail(
                            None,
                            None,
                            format!(
                                "override {}.{} differs from its inherited signature",
                                class.name,
                                method.name()
                            ),
                        ));
                    }
                }
            }
        }
        let mut signatures = BTreeMap::new();
        let exit_callers = exit_callers(self, &bodies);
        let class_requirements = class_shared_requirements(self, &bodies, &self.classes);
        self.validate_shared(&bodies, context, &exit_callers, hierarchy)?;
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
                self.check_static_origins(ty.clone())
                    .map_err(|message| fail(Some(index), None, message))?;
                if let Some(mut result) = crate::unions::result(ty) {
                    crate::structural::resolve_result(&mut result, &mut collect).unwrap();
                }
            }
            for ty in function
                .parameter_types()
                .iter()
                .map(crate::ParameterType::exact_type)
                .chain(std::iter::once(crate::tuples::exact_type(
                    &function.return_type(),
                )))
            {
                self.check_static_origins(ty)
                    .map_err(|message| fail(Some(index), None, message))?;
            }
            for name in source_classes {
                if hierarchy.get(&name.name) != Some(&name) {
                    return Err(fail(
                        Some(index),
                        None,
                        format!(
                            "source signature class {name} must match a declaration and native base"
                        ),
                    ));
                }
            }
            if function.is_static_method() {
                let declaration = self
                    .classes
                    .iter()
                    .find(|c| owner == Owner::Class(c.name.as_str()))
                    .ok_or_else(|| {
                        fail(
                            Some(index),
                            None,
                            "static methods require a class owner".into(),
                        )
                    })?;
                if function.name() == "initialize"
                    || crate::native::callback(declaration.base, function.name()).is_some()
                {
                    return Err(fail(
                        Some(index),
                        None,
                        "initializers and native callbacks require instance receivers".into(),
                    ));
                }
                for (at, op) in function.operations().iter().enumerate() {
                    let invalid = match &op.operation {
                        Operation::StaticSelf { class, .. } => class != &declaration.name,
                        Operation::SelfFieldRead { field, .. }
                        | Operation::SelfFieldAssign { field, .. } => !self
                            .field(&declaration.name, field)
                            .is_some_and(|(_, f)| f.kind.class_shared()),
                        Operation::SelfCall { member, .. }
                        | Operation::SelfCallStatement { member, .. } => !self
                            .method(&declaration.name, member)
                            .is_some_and(|(_, f)| f.is_static_method()),
                        _ => false,
                    };
                    if invalid {
                        return Err(fail(
                            Some(index),
                            Some(at),
                            "static method receiver does not declare that static field or method"
                                .into(),
                        ));
                    }
                }
            }
            let callback = (matches!(owner, Owner::Class(_)) && function.is_instance_method())
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
            for (at, located) in function.operations().iter().enumerate() {
                let path = located.operation.receiver_path();
                if path.is_empty() {
                    continue;
                }
                let claimed = match &located.operation {
                    Operation::ModuleCall { module, .. }
                    | Operation::ModuleCallStatement { module, .. } => module,
                    _ => unreachable!(),
                };
                let current = match owner {
                    Owner::Module(current) | Owner::Class(current) => Some(current),
                    Owner::Global => None,
                };
                let mut resolved = None;
                for scope in current
                    .into_iter()
                    .flat_map(crate::names::scopes)
                    .map(Some)
                    .chain(std::iter::once(None))
                {
                    let root = scope
                        .map_or_else(|| path[0].clone(), |scope| format!("{scope}.{}", path[0]));
                    let shadow =
                        self.shared
                            .iter()
                            .any(|b| b.module.as_deref() == scope && b.name == path[0])
                            || self.declarations.iter().any(|d| {
                                d.module.as_deref() == scope && d.function.name() == path[0]
                            })
                            || self.classes.iter().any(|c| {
                                Some(c.name.as_str()) == scope
                                    && (self.field(&c.name, &path[0]).is_some()
                                        || self.method(&c.name, &path[0]).is_some()
                                        || (!function.is_static_method()
                                            && (c.base.api_name().rsplit('.').next()
                                                == Some(path[0].as_str())
                                                || crate::api::core()
                                                    .method(c.base.api_name(), &path[0])
                                                    .is_some())))
                            });
                    if shadow {
                        break;
                    }
                    if self.modules.contains(&root) {
                        resolved = Some(root);
                        break;
                    }
                }
                if let Some(mut module) = resolved {
                    let mut valid = true;
                    for part in path.iter().skip(1) {
                        module.push('.');
                        module.push_str(part);
                        valid &= self.modules.contains(&module);
                    }
                    if valid && module == *claimed {
                        continue;
                    }
                }
                return Err(fail(
                    Some(index),
                    Some(at),
                    "receiver namespace path does not resolve to the claimed module".into(),
                ));
            }
            for (at, located) in function.operations().iter().enumerate() {
                let Some(target) = lexical_target(&located.operation, function) else {
                    continue;
                };
                let (Owner::Module(current) | Owner::Class(current)) = owner else {
                    return Err(fail(
                        Some(index),
                        Some(at),
                        "lexical lookup requires a module or class body".into(),
                    ));
                };
                if target.is_some_and(|target| {
                    target == current || !current.starts_with(&format!("{target}."))
                }) {
                    return Err(fail(
                        Some(index),
                        Some(at),
                        "lexical target must be an enclosing module or globals".into(),
                    ));
                }
                let member = match &located.operation {
                    Operation::SharedRead { field, .. } | Operation::SharedBegin { field, .. } => {
                        Some(field.as_str())
                    }
                    Operation::ModuleCall { member, .. }
                    | Operation::ModuleCallStatement { member, .. }
                    | Operation::GlobalCall { member, .. }
                    | Operation::GlobalCallStatement { member, .. } => Some(member.as_str()),
                    _ => None,
                };
                if let Some(member) = member {
                    for scope in
                        crate::names::scopes(current).take_while(|scope| Some(*scope) != target)
                    {
                        if self.classes.iter().any(|class| {
                            class.name == scope
                                && (self.field(&class.name, member).is_some()
                                    || self.method(&class.name, member).is_some()
                                    || (!function.is_static_method()
                                        && (class.base.api_name().rsplit('.').next()
                                            == Some(member)
                                            || crate::api::core()
                                                .method(class.base.api_name(), member)
                                                .is_some())))
                        }) || self.shared.iter().any(|binding| {
                            binding.module.as_deref() == Some(scope) && binding.name == member
                        }) || self.declarations.iter().any(|decl| {
                            decl.module.as_deref() == Some(scope) && decl.function.name() == member
                        }) || self.modules.contains(&format!("{scope}.{member}"))
                        {
                            return Err(fail(
                                Some(index),
                                Some(at),
                                "lexical target is shadowed by a nearer declaration".into(),
                            ));
                        }
                    }
                }
            }
            let base_calls = function
                .operations()
                .iter()
                .filter(|op| matches!(op.operation, Operation::NativeBaseInitialize { .. }))
                .count();
            if field_initializer(function.name()) {
                if !matches!(owner, Owner::Class(_))
                    || function.is_static_method() != (function.name() == STATIC_INITIALIZER)
                    || function.parameter_count() != 0
                    || function.return_type() != ReturnType::Void
                    || base_calls != 0
                {
                    return Err(fail(
                        Some(index), None,
                        "field initializer requires its declared instance/static receiver, no parameters, Void, and no native base initialization".into(),
                    ));
                }
                let declaration = self
                    .classes
                    .iter()
                    .find(|class| owner == Owner::Class(class.name.as_str()))
                    .unwrap();
                let pending = declaration
                    .fields
                    .iter()
                    .filter(|field| {
                        if function.name() == STATIC_INITIALIZER {
                            field.kind.static_write()
                        } else {
                            field.kind.allocation_write()
                        }
                    })
                    .map(|field| field.name.as_str())
                    .collect::<BTreeSet<_>>();
                let unreadable = declaration
                    .fields
                    .iter()
                    .filter(|field| {
                        field.kind.requires_initialization()
                            || (function.name() == STATIC_INITIALIZER
                                && field.kind.allocation_write())
                    })
                    .map(|field| field.name.as_str())
                    .collect::<BTreeSet<_>>();
                let states = pending
                    .iter()
                    .map(|field| {
                        crate::control_flow::binding_write_states(
                            function.operations(),
                            |operation| {
                                matches!(operation, Operation::SelfFieldAssign { field: target, .. } if target == field)
                            },
                        )
                        .map(|states| (*field, states))
                        .map_err(|error| fail(Some(index), error.operation, error.message))
                    })
                    .collect::<Result<BTreeMap<_, _>, _>>()?;
                for (operation, located) in function.operations().iter().enumerate() {
                    if function.name() == STATIC_INITIALIZER {
                        let mut required = BTreeSet::new();
                        if let Some((class, field)) = class_store(&located.operation, function) {
                            required.insert((class.to_owned(), field.to_owned()));
                        }
                        if let Operation::ClassFieldRead { class, field, .. } = &located.operation {
                            required.insert(self.static_field_key(class, field));
                        }
                        for callee in effect_callees(self, owner, &located.operation)
                            .into_iter()
                            .chain(activation_callees(
                                self,
                                owner,
                                &located.operation,
                                function,
                                &bodies,
                            ))
                            .chain(comparator_callees(self, function, &located.operation))
                        {
                            if callee == (owner, STATIC_INITIALIZER) {
                                continue;
                            }
                            if let Some(reads) = class_requirements.get(&callee) {
                                required.extend(reads.iter().cloned());
                            }
                        }
                        if required.iter().any(|(class, field)| {
                            class == &declaration.name
                                && states
                                    .get(field.as_str())
                                    .is_some_and(|s| s[operation].unwritten)
                        }) {
                            return Err(fail(
                                Some(index),
                                Some(operation),
                                "static constant read before required initialization".into(),
                            ));
                        }
                    }
                    let incomplete = states.values().any(|states| states[operation].unwritten);
                    if (incomplete || !unreadable.is_empty())
                        && (requests_exit(&located.operation)
                            || erased_callbacks(function, &located.operation)
                            || activation_callees(
                                self,
                                owner,
                                &located.operation,
                                function,
                                &bodies,
                            )
                            .iter()
                            .any(|key| exit_callers.contains(key))
                            || comparator_callees(self, function, &located.operation)
                                .into_iter()
                                .any(|key| exit_callers.contains(&key)))
                    {
                        return Err(fail(
                            Some(index),
                            Some(operation),
                            "initialize allocation fields before calls that may request shutdown"
                                .into(),
                        ));
                    }
                    let message = match &located.operation {
                        Operation::SelfFieldAssign { field, .. }
                            if !pending.contains(field.as_str()) =>
                        {
                            Some(
                                "field initializer must assign each designated binding exactly once",
                            )
                        }
                        Operation::SelfFieldRead { field, .. }
                            if states
                                .get(field.as_str())
                                .is_some_and(|states| states[operation].unwritten)
                                || unreadable.contains(field.as_str()) =>
                        {
                            Some("field read before required initialization")
                        }
                        Operation::SelfValue { .. }
                        | Operation::SelfCall { .. }
                        | Operation::SelfCallStatement { .. }
                            if function.name() != STATIC_INITIALIZER
                                && (incomplete || !unreadable.is_empty()) =>
                        {
                            Some(
                                "initialize required fields before exposing the initializer receiver",
                            )
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
                            || activation_callees(
                                self,
                                owner,
                                &located.operation,
                                function,
                                &bodies,
                            )
                            .iter()
                            .any(|key| exit_callers.contains(key))
                            || comparator_callees(self, function, &located.operation)
                                .into_iter()
                                .any(|key| exit_callers.contains(&key)))
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
            if (function.is_instance_method() || function.is_static_method())
                != matches!(owner, Owner::Class(_))
            {
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
            for (operation, located) in function.operations().iter().enumerate() {
                let op = &located.operation;
                for (target, member) in source_bindings(self, owner, op) {
                    if let Owner::Class(class) = target {
                        self.check_member_access(owner, class, member)
                            .map_err(|m| fail(Some(index), Some(operation), m))?;
                    }
                }
                let receiver_classes = match op {
                    Operation::ObjectCall { classes, .. }
                    | Operation::ObjectCallStatement { classes, .. } => classes.clone(),
                    Operation::MemberRead { receiver, .. }
                    | Operation::MemberWrite { receiver, .. } => origins
                        .contract(*receiver)
                        .map(|c| c.classes().iter().map(|c| c.name.clone()).collect())
                        .unwrap_or_default(),
                    _ => Vec::new(),
                };
                let member = match op {
                    Operation::ObjectCall { member, .. }
                    | Operation::ObjectCallStatement { member, .. }
                    | Operation::MemberRead { member, .. }
                    | Operation::MemberWrite { member, .. } => Some(member),
                    _ => None,
                };
                if let Some(member) = member {
                    let selectors = receiver_classes
                        .iter()
                        .map(|c| self.member_symbol(c, member))
                        .collect::<BTreeSet<_>>();
                    if selectors.len() > 1 {
                        return Err(fail(Some(index),Some(operation),"receiver alternatives require different private member selectors; narrow the receiver first".into()));
                    }
                }
                match op {
                    Operation::ClassFieldRead { class, field, .. }
                    | Operation::ClassFieldBegin { class, field, .. } => {
                        self.check_member_access(owner, class, field)
                            .map_err(|m| fail(Some(index), Some(operation), m))?;
                    }
                    Operation::MemberRead {
                        receiver, member, ..
                    }
                    | Operation::MemberWrite {
                        receiver, member, ..
                    } => {
                        if let Some(contract) = origins.contract(*receiver) {
                            for class in contract.classes() {
                                self.check_member_access(owner, &class.name, member)
                                    .map_err(|m| fail(Some(index), Some(operation), m))?;
                            }
                        }
                    }
                    Operation::BindMethod {
                        receiver, selector, ..
                    } => {
                        if let (Some(contract), Some(member)) =
                            (origins.contract(*receiver), origins.symbol(*selector))
                        {
                            for class in contract.classes() {
                                if self.member_visibility(&class.name, member)
                                    != crate::Visibility::Public
                                {
                                    return Err(fail(Some(index),Some(operation),"non-public methods cannot supply a checked runtime binding".into()));
                                }
                                self.check_member_access(owner, &class.name, member)
                                    .map_err(|m| fail(Some(index), Some(operation), m))?;
                            }
                        }
                    }
                    _ => {}
                }
            }
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
                if let Operation::EnumValue { name, operand, .. } = &located.operation {
                    self.check_api_value_contract(
                        function,
                        &origins,
                        *operand,
                        name.require_backing()
                            .map_err(|e| fail(Some(index), Some(operation), e.into()))?,
                        context,
                    )
                    .map_err(|e| fail(Some(index), Some(operation), e))?;
                }
                if let Operation::ClassFieldBegin { class, field, .. } = &located.operation {
                    if !self
                        .field(class, field)
                        .map(|(_, f)| f)
                        .is_some_and(|f| matches!(f.kind, FieldKind::StaticVariable { .. }))
                    {
                        return Err(fail(
                            Some(index),
                            Some(operation),
                            "class stores require a declared mutable static field".into(),
                        ));
                    }
                }
                if let (Operation::SharedWrite { value, .. }, Some((class, field))) = (
                    &located.operation,
                    class_store(&located.operation, function),
                ) {
                    let declaration = self
                        .field(class, field)
                        .map(|(_, f)| f)
                        .filter(|f| matches!(f.kind, FieldKind::StaticVariable { .. }))
                        .ok_or_else(|| {
                            fail(
                                Some(index),
                                Some(operation),
                                "class stores require a declared mutable static field".into(),
                            )
                        })?;
                    self.check_value_contract(
                        function,
                        &origins,
                        *value,
                        &declaration.result_type(),
                        context,
                    )
                    .map_err(|message| fail(Some(index), Some(operation), message))?;
                }
                if let Operation::ClassFieldRead {
                    class,
                    field,
                    result_type,
                    ..
                } = &located.operation
                {
                    let actual = self
                        .field(class, field)
                        .map(|(_, f)| f)
                        .filter(|f| f.kind.class_shared());
                    if !actual.is_some_and(|f| f.result_type() == *result_type) {
                        return Err(fail(
                            Some(index),
                            Some(operation),
                            "class-shared read must match its declaration".into(),
                        ));
                    }
                }
                if let Operation::SelfFieldAssign { field, .. } = &located.operation {
                    if !field_initializer(function.name())
                        && matches!(owner, Owner::Class(class) if self.self_field_readonly(class, field))
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
                            self.field(&target.name, member)
                                .is_some_and(|(_, f)| f.kind.readonly())
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
                        field_initializer(name) || name.starts_with("__ciq_initialize_shared_")
                    }) {
                        return Err(fail(
                            Some(index),
                            Some(operation),
                            "allocation initializer is not a callable member".into(),
                        ));
                    }
                }
                match &located.operation {
                    Operation::ModuleSelf { .. } if matches!(owner, Owner::Class(_)) => {
                        return Err(fail(
                            Some(index),
                            Some(operation),
                            "module-self requires a global/module body".into(),
                        ));
                    }
                    Operation::ModuleValue {
                        module: Some(module),
                        ..
                    } if !self.modules.contains(module)
                        && crate::api::core().module(module).is_none() =>
                    {
                        return Err(fail(
                            Some(index),
                            Some(operation),
                            format!("unknown module value {module}"),
                        ));
                    }
                    Operation::ClassValue { class, .. } => {
                        let supported_native = crate::native_bindings::class_alias(class).is_some();
                        if !self.classes.iter().any(|c| c.name == *class) && !supported_native {
                            return Err(fail(
                                Some(index),
                                Some(operation),
                                format!("unknown class value {class}"),
                            ));
                        }
                    }
                    _ => {}
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
                        class.name == claimed.name && hierarchy.get(&claimed.name) == Some(claimed)
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
                        |class, field| self.field(&class.name, field).map(|(_, f)| f.result_type()),
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
                    let declaration = match owner {
                        Owner::Class(class) => self.field(class, field).map(|(_, f)| f),
                        _ => None,
                    }
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
                    if hierarchy.get(&class.name) != Some(class) {
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
                            if hierarchy.get(&class.name) != Some(class) {
                                return Err(fail(Some(index), Some(operation), "source union class and native base must match a unit declaration".into()));
                            }
                        }
                    }
                }
                if let Operation::SourceNew {
                    class,
                    ancestors,
                    base,
                    ..
                } = &located.operation
                {
                    if hierarchy.get(class)
                        != Some(&crate::SourceClass {
                            name: class.clone(),
                            ancestors: ancestors.clone(),
                            base: *base,
                        })
                    {
                        return Err(fail(
                            Some(index),
                            Some(operation),
                            "allocation ancestry disagrees with class declarations".into(),
                        ));
                    }
                }
                if let Operation::SourceBegin { class, .. } | Operation::SourceNew { class, .. } =
                    &located.operation
                {
                    if let Some(identity) = hierarchy.get(class).filter(|c| !c.ancestors.is_empty())
                    {
                        if identity.base != ClassBase::LangObject {
                            return Err(fail(Some(index),Some(operation),"source subclass construction currently requires an Object-rooted chain".into()));
                        }
                        for ancestor in &identity.ancestors {
                            let parent = self.classes.iter().find(|c| c.name == *ancestor).unwrap();
                            if self.member_visibility(&parent.name, "initialize")
                                != crate::Visibility::Public
                                || parent.fields.iter().any(|f| {
                                    !f.kind.class_shared() && f.kind.requires_initialization()
                                })
                                || parent
                                    .methods
                                    .iter()
                                    .filter(|f| f.name() == FIELD_INITIALIZER)
                                    .any(|f| {
                                        f.operations().iter().any(|op| {
                                            matches!(
                                                op.operation,
                                                Operation::SelfValue { .. }
                                                    | Operation::SelfCall { .. }
                                                    | Operation::SelfCallStatement { .. }
                                            )
                                        })
                                    })
                                || !parent
                                    .methods
                                    .iter()
                                    .find(|f| f.name() == "initialize")
                                    .is_some_and(|f| {
                                        f.parameter_count() == 0
                                            && f.operations().iter().all(|op| {
                                                matches!(op.operation, Operation::ReturnVoid)
                                            })
                                    })
                            {
                                return Err(fail(Some(index),Some(operation),"source ancestors require initialized instance fields, non-escaping allocation hooks, and empty public zero-argument initializers".into()));
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
                    | Operation::ClassCall { arguments, .. }
                    | Operation::SelfCall { arguments, .. } => (arguments, true),
                    Operation::ObjectCallStatement { arguments, .. }
                    | Operation::GlobalCallStatement { arguments, .. }
                    | Operation::ModuleCallStatement { arguments, .. }
                    | Operation::ClassCallStatement { arguments, .. }
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
                        self.method(&class.name, member)
                            .and_then(|(owner, _)| {
                                signatures
                                    .get(&(Owner::Class(owner.name.as_str()), member.as_str()))
                            })
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
                if let Operation::ClassCall { class, member, .. }
                | Operation::ClassCallStatement { class, member, .. } = &located.operation
                {
                    if !self
                        .method(class, member)
                        .map(|(_, f)| f)
                        .is_some_and(|f| f.is_static_method())
                    {
                        return Err(fail(
                            Some(index),
                            Some(operation),
                            "class calls require a declared static method".into(),
                        ));
                    }
                }
                for key in source_bindings(self, owner, &located.operation) {
                    if field_initializer(key.1) || key.1.starts_with("__ciq_initialize_shared_") {
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
                    | Operation::ClassCall { result_type, .. }
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

    /// Require every opaque receiver origin in a stored contract to name a
    /// declared class, including origins nested in containers and signatures.
    fn check_static_origins(&self, ty: crate::api::Type) -> Result<(), String> {
        use crate::api::Type;
        let mut pending = vec![ty];
        while let Some(ty) = pending.pop() {
            match ty {
                Type::StaticReceiver(class) => {
                    if !self.classes.iter().any(|c| c.name == class) {
                        return Err(format!(
                            "static receiver origin {class} must name a declared class"
                        ));
                    }
                }
                Type::Recursive(body) | Type::Array(body) => pending.push(*body),
                Type::Enum(identity) => pending.extend(identity.backing().cloned()),
                Type::SourceValue(contract) => pending.extend(contract.native().iter().cloned()),
                Type::Union(parts) | Type::Tuple(parts) => pending.extend(parts),
                Type::Dictionary { key, value } => pending.extend([*key, *value]),
                Type::Options(fields) => pending.extend(fields.into_values()),
                Type::TupleValue(tuple) => {
                    pending.extend(tuple.slots().iter().map(crate::tuples::exact_type))
                }
                Type::Callback(signature) => {
                    pending.extend(
                        signature
                            .parameters
                            .iter()
                            .map(crate::ParameterType::exact_type),
                    );
                    pending.push(crate::tuples::exact_type(&signature.result));
                }
                Type::Structural(shape) => pending.extend(shape.api_types()),
                _ => {}
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
        let actual = if matches!(function.value_types[&value], crate::ValueType::Enum(_)) {
            function.value_types[&value].api_type()
        } else {
            origins
                .contract(value)
                .cloned()
                .map(|c| crate::api::Type::SourceValue(Box::new(c)))
                .unwrap_or_else(|| function.value_types[&value].api_type())
        };
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

    /// Check shared slot contracts, reserved initialization bodies, and the
    /// readiness of fields reached through source calls or lazy module startup.
    fn validate_shared(
        &self,
        bodies: &[(Owner<'_>, &Function)],
        context: Option<crate::api::ApiContext>,
        exits: &BTreeSet<(Owner<'_>, &str)>,
        hierarchy: &BTreeMap<String, crate::SourceClass>,
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
                            Operation::SharedRead { .. }
                                | Operation::SharedAssign { .. }
                                | Operation::SharedBegin { .. }
                        )
                    })
                    || function.operations().iter().any(|op| {
                        matches!(op.operation, Operation::SharedWrite { .. })
                            && class_store(&op.operation, function).is_none()
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
            return Err(fail(None, None, "unit exceeds 256 shared bindings"));
        }
        for constant in &self.shared {
            crate::validate_call_names(None, &constant.name)
                .map_err(|message| fail(None, None, message))?;
            if let Some(module) = &constant.module {
                crate::validate_call_names(Some(module), "module")
                    .map_err(|message| fail(None, None, message))?;
            }
            if FieldKind::value_contract(constant.value_type.clone()).as_ref()
                != Some(&constant.value_type)
            {
                return Err(fail(
                    None,
                    None,
                    "unsupported or noncanonical shared binding contract",
                ));
            }
            if let Some(context) = context {
                context
                    .check_type(crate::api::core(), &constant.value_type.api_type())
                    .map_err(|message| fail(None, None, &message))?;
            }
            let mut retained = constant.value_type.clone();
            crate::structural::resolve_result(&mut retained, &mut |class| {
                if hierarchy.get(&class.name) == Some(class) {
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
                return Err(fail(None, None, "duplicate or conflicting shared binding"));
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
                    "shared binding conflicts with a class or module",
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
                    "shared binding requires its reserved initializer",
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
            let mut targets = activation_callees(self, owner, op, function, bodies);
            targets.extend(comparator_callees(self, function, op));
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
                    if let Some((module, field)) = lexical_store(&op.operation, function) {
                        summary.fields.insert((module.cloned(), field.to_owned()));
                        if module
                            .map(String::as_str)
                            .map_or(Owner::Global, Owner::Module)
                            != *owner
                        {
                            summary.owners.insert(module.cloned());
                        }
                    }
                    if let Operation::SharedBegin {
                        module,
                        field,
                        lexical: false,
                        ..
                    } = &op.operation
                    {
                        summary.fields.insert((module.clone(), field.clone()));
                        if module.as_deref().map_or(Owner::Global, Owner::Module) != *owner {
                            summary.owners.insert(module.clone());
                        }
                    }
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
            let states = crate::control_flow::shared_write_states(function.operations())
                .map_err(|error| fail(Some(index), error.operation, &error.message))?;
            for (operation, op) in function.operations().iter().enumerate() {
                let state = states[operation];
                let binding_ready = |module: &Option<String>, field: &String| {
                    ready.contains(&(module.clone(), field.clone()))
                        || (*module == constant.module
                            && *field == constant.name
                            && state.written
                            && !state.unwritten)
                };
                let mut required = SharedReads::default();
                if let Some((module, field)) = lexical_store(&op.operation, function) {
                    required.fields.insert((module.cloned(), field.to_owned()));
                    if module
                        .map(String::as_str)
                        .map_or(Owner::Global, Owner::Module)
                        != owner
                    {
                        required.owners.insert(module.cloned());
                    }
                }
                if let Operation::SharedBegin {
                    module,
                    field,
                    lexical: false,
                    ..
                } = &op.operation
                {
                    required.fields.insert((module.clone(), field.clone()));
                    if module.as_deref().map_or(Owner::Global, Owner::Module) != owner {
                        required.owners.insert(module.clone());
                    }
                }
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
                    if *module == constant.module
                        && *field == constant.name
                        && state.written
                        && state.unwritten
                        && declaration.value_type != ReturnType::Null
                    {
                        return Err(fail(
                            Some(index),
                            Some(operation),
                            "shared read requires a definite initialized or Null slot view",
                        ));
                    }
                    let snapshot = target == owner && !binding_ready(module, field);
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
                    *module == constant.module && !binding_ready(module, field)
                }) {
                    return Err(fail(
                        Some(index),
                        Some(operation),
                        "shared initializer reads an incomplete binding or initialization cycle",
                    ));
                }
                let pending = self
                    .shared
                    .iter()
                    .any(|c| c.module == constant.module && !binding_ready(&c.module, &c.name));
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
                if let Operation::SharedAssign { module, field, .. } = &op.operation {
                    if *module != constant.module || *field != constant.name {
                        return Err(fail(
                            Some(index),
                            Some(operation),
                            "shared initializer may assign only its declared binding",
                        ));
                    }
                }
            }
            ready.insert((constant.module.clone(), constant.name.clone()));
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
            let stores = function
                .operations()
                .iter()
                .filter_map(|o| match &o.operation {
                    Operation::SharedBegin {
                        builder,
                        module,
                        field,
                        ..
                    } => Some((*builder, (module, field))),
                    _ => None,
                })
                .collect::<BTreeMap<_, _>>();
            for (operation, op) in function.operations().iter().enumerate() {
                match &op.operation {
                    Operation::SharedBegin { module, field, .. } => {
                        let target = module.as_deref().map_or(Owner::Global, Owner::Module);
                        if !fields
                            .get(&(target, field.as_str()))
                            .is_some_and(|binding| binding.mutable)
                        {
                            return Err(fail(
                                Some(index),
                                Some(operation),
                                "shared stores require a declared mutable binding",
                            ));
                        }
                    }
                    Operation::SharedWrite { builder, value } => {
                        let Some((module, field)) = stores.get(builder).copied() else {
                            continue;
                        };
                        let target = module.as_deref().map_or(Owner::Global, Owner::Module);
                        let binding = fields.get(&(target, field.as_str())).ok_or_else(|| {
                            fail(
                                Some(index),
                                Some(operation),
                                "shared write requires a declared binding",
                            )
                        })?;
                        self.check_value_contract(
                            function,
                            &origins,
                            *value,
                            &binding.value_type,
                            context,
                        )
                        .map_err(|message| fail(Some(index), Some(operation), &message))?;
                    }
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
                                "shared assignment requires its reserved initializer",
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
            if self.member_visibility(class, name) != crate::Visibility::Public {
                return Err(format!("{class}.{name} is not a public interface field"));
            }
            let field = self
                .field(class, name)
                .map(|(_, f)| f)
                .ok_or_else(|| format!("{class} does not implement field {name}"))?;
            if field.kind.readonly() {
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
            if self.member_visibility(class, name) != crate::Visibility::Public {
                return Err(format!("{class}.{name} is not a public interface method"));
            }
            let actual = if let Some((_, method)) = self.method(class, name) {
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
        if !self.classes.iter().any(|c| c.name == class) {
            return Err(format!(
                "unresolved source interface implementation {class}"
            ));
        }
        let interface = crate::api::core()
            .interface(interface)
            .ok_or_else(|| format!("unknown structural interface {interface}"))?;
        for required in &interface.methods {
            if self.member_visibility(class, &required.name) != crate::Visibility::Public {
                return Err(format!(
                    "{class}.{} is not a public interface method",
                    required.name
                ));
            }
            let implementation = self
                .method(class, &required.name)
                .map(|(_, method)| method)
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
    pub fn shared_bindings(&self) -> &[SharedBinding] {
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
        let initializers = self
            .classes
            .iter()
            .filter(|c| self.member_visibility(&c.name, "initialize") == crate::Visibility::Private)
            .map(|c| (c.name.clone(), self.member_symbol(&c.name, "initialize")))
            .collect();
        let items = self
            .declarations
            .iter()
            .flat_map(|d| {
                backend::lower_bound(
                    &crate::visibility::bind_body(self, None, &d.function),
                    &d.code_label(),
                    Some(d.module.as_deref()),
                    &initializers,
                )
                .items()
                .to_vec()
            })
            .chain(self.classes.iter().flat_map(|c| {
                c.methods.iter().flat_map(|f| {
                    backend::lower_bound(
                        &crate::visibility::bind_body(self, Some(&c.name), f),
                        &c.code_label(f.name()),
                        None,
                        &initializers,
                    )
                    .items()
                    .to_vec()
                })
            }))
            .collect();
        Program::new(items).expect("unit identities lower to disjoint assembly labels")
    }

    /// Return canonical module paths in parent-before-child order, including empty scopes.
    pub fn modules(&self) -> &BTreeSet<String> {
        &self.modules
    }

    /// Return nominal source enums in their saved declaration order.
    pub fn enums(&self) -> &[crate::EnumDeclaration] {
        &self.enums
    }

    /// Serialize validated ownership, enum types, storage, and function bodies.
    ///
    /// Inner functions retain optional spans. Classes retain native
    /// base identity without a VM import index. Source filenames are not stored.
    pub fn to_ir_text(&self) -> String {
        let mut text = String::from("ciq-unit\n");
        for module in &self.modules {
            text.push_str(&format!(".module {module}\n"));
        }
        for definition in &self.enums {
            text.push_str(&format!(
                ".enum {} {} {}\n",
                definition.module.as_deref().unwrap_or("$"),
                definition.name,
                crate::type_text::result_tag(
                    &crate::unions::result(definition.backing.clone()).expect("validated backing")
                )
            ));
        }
        for constant in &self.shared {
            text.push_str(&format!(
                "{} {} {} {}\n",
                if constant.mutable {
                    ".shared-variable"
                } else {
                    ".shared-constant"
                },
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
            text.push_str(&format!(
                ".class {} {}{}\n",
                class.name,
                class.base.text(),
                class
                    .superclass
                    .as_ref()
                    .map_or(String::new(), |p| format!(" parent={p}"))
            ));
            for ((owner, member), access) in &self.visibility {
                if owner == &class.name {
                    text.push_str(&format!(".visibility {member} {}\n", access.text()));
                }
            }
            for field in &class.fields {
                text.push_str(&match &field.kind {
                    FieldKind::StaticVariable {
                        value_type,
                        initial,
                    } => format!(
                        ".field-static-variable {} {} {}\n",
                        field.name,
                        crate::type_text::result_tag(value_type),
                        initial.text()
                    ),
                    FieldKind::StaticConstant {
                        value_type,
                        initial,
                    } => format!(
                        ".field-static-constant {} {} {}\n",
                        field.name,
                        crate::type_text::result_tag(value_type),
                        initial.text()
                    ),
                    FieldKind::Enum {
                        value_type,
                        initial,
                    } => format!(
                        ".field-enum {} {} {}\n",
                        field.name,
                        crate::type_text::result_tag(value_type),
                        initial.text()
                    ),
                    FieldKind::Initialized { value_type } => format!(
                        ".field-initialized {} {}\n",
                        field.name,
                        crate::type_text::result_tag(value_type)
                    ),
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
                    FieldKind::DynamicNumber(initial) => {
                        format!(".field-dynamic-number {} {initial}\n", field.name)
                    }
                    FieldKind::DynamicSymbol(initial) => {
                        format!(".field-dynamic-symbol {} :{initial}\n", field.name)
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
    let mut attempts = 0;
    let (mut parsed_sources, mut calls) = loop {
        aliases.refresh_aliases()?;
        let (parsed, mut calls) = prepare_sources(sources, &aliases, unchecked)?;
        if !aliases.has_enums() {
            break (parsed, calls);
        }
        let (backings, error) =
            infer_stored_constants(sources, &parsed, context, &mut calls, true)?;
        if backings == aliases.enum_backings() {
            if let Some(error) = error {
                return Err(error);
            }
            break (parsed, calls);
        }
        aliases.set_enum_backings(backings);
        attempts += 1;
        if attempts > 256 {
            return Err(SourceDiagnostic {
                source: 0,
                diagnostic: Diagnostic::at(
                    sources[0],
                    0,
                    "enum backing contracts do not reach a finite fixed point",
                ),
            });
        }
    };
    if let Some(context) = context {
        aliases.validate_api(context)?;
    }
    let mut shared = Vec::new();
    infer_stored_constants(sources, &parsed_sources, context, &mut calls, false)?;
    for parsed in &parsed_sources {
        for (module, declaration) in &parsed.shared {
            shared.push(SharedBinding {
                module: module.clone(),
                name: declaration.name.clone(),
                mutable: declaration.mutable,
                value_type: calls.shared_fields[&(module.clone(), declaration.name.clone())]
                    .clone(),
            });
        }
    }
    // Literal defaults precede runtime initializers, as DATA defaults do in SDK
    // output. Preserve runtime expression order; do not topologically move calls.
    let shared_order = shared_initialization_order(&parsed_sources);
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
                is_static: false,
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
            if class.bindings.is_empty() {
                continue;
            }
            let constants = class.bindings.iter().collect::<Vec<_>>();
            let order = constant_initialization_order(&constants);
            let mut assignments = Vec::new();
            let mut defaults = Vec::new();
            for declaration in &class.bindings {
                let result = calls.source_fields[&class.name][&declaration.name].clone();
                class.fields.push(InstanceField {
                    name: declaration.name.clone(),
                    kind: if declaration.static_binding && declaration.mutable {
                        FieldKind::StaticVariable {
                            value_type: result,
                            initial: match crate::constants::literal(&declaration.initializer) {
                                Some(crate::constants::Immediate::Number(n)) => {
                                    crate::ClassFieldInitialization::Number(n)
                                }
                                _ => crate::ClassFieldInitialization::Static,
                            },
                        }
                    } else if declaration.static_binding {
                        FieldKind::StaticConstant {
                            value_type: result,
                            initial: match crate::constants::literal(&declaration.initializer) {
                                Some(crate::constants::Immediate::Number(n)) => {
                                    crate::ClassFieldInitialization::Number(n)
                                }
                                _ => crate::ClassFieldInitialization::Static,
                            },
                        }
                    } else if declaration.enumeration {
                        let initial = if !declaration.literal_default {
                            crate::ClassFieldInitialization::Allocation
                        } else if let Some(crate::constants::Immediate::Number(n)) =
                            crate::constants::literal(&declaration.initializer)
                        {
                            crate::ClassFieldInitialization::Number(n)
                        } else {
                            crate::ClassFieldInitialization::Static
                        };
                        FieldKind::Enum {
                            value_type: result,
                            initial,
                        }
                    } else if declaration.mutable {
                        FieldKind::Initialized { value_type: result }
                    } else {
                        FieldKind::Constant { value_type: result }
                    },
                });
            }
            for index in order {
                let declaration = &class.bindings[index];
                let target = if declaration.static_binding
                    || (declaration.enumeration && declaration.literal_default)
                {
                    if matches!(
                        crate::constants::literal(&declaration.initializer),
                        Some(crate::constants::Immediate::Number(_))
                    ) {
                        continue;
                    }
                    &mut defaults
                } else {
                    &mut assignments
                };
                target.push(parser::Statement::FieldAssign {
                    name: declaration.name.clone(),
                    value: declaration.initializer.clone(),
                    span: declaration.span,
                });
            }
            for (name, mut statements) in [
                (STATIC_INITIALIZER, defaults),
                (FIELD_INITIALIZER, assignments),
            ] {
                if statements.is_empty() {
                    continue;
                }
                statements.push(parser::Statement::Return {
                    value: None,
                    span: class.name_span,
                });
                class.methods.push(parser::ParsedFunction {
                    is_static: name == STATIC_INITIALIZER,
                    name: name.into(),
                    parameters: Vec::new(),
                    parameter_types: Vec::new(),
                    result_type: Some(ReturnType::Void),
                    statements,
                    body_span: class.name_span,
                });
            }
        }
    }
    let modules = parsed_sources
        .iter()
        .flat_map(|parsed| parsed.modules.iter().cloned())
        .collect::<Vec<_>>();
    let visibility = parsed_sources
        .iter()
        .flat_map(|p| &p.classes)
        .flat_map(|c| {
            c.visibility
                .iter()
                .filter(|(_, v)| *v != crate::Visibility::Public)
                .map(|(m, v)| ((c.name.clone(), m.clone()), *v))
        })
        .collect();
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
                .bindings
                .iter()
                .filter(|binding| !binding.mutable)
                .map(|value| value.name.clone())
                .collect();
            method_calls.current_module = class
                .name
                .rsplit_once('.')
                .map(|(owner, _)| owner.to_owned());
            method_calls.current_class = Some(crate::SourceClass {
                name: class.name.clone(),
                ancestors: calls
                    .class_ancestors
                    .get(&class.name)
                    .cloned()
                    .unwrap_or_default(),
                base: class.base,
            });
            method_calls.field_names = calls.source_fields[&class.name].keys().cloned().collect();
            method_calls.field_types = calls.source_fields[&class.name].clone();
            for (member, signature) in calls.class_signatures(&class.name) {
                method_calls.insert(("self".into(), member), signature);
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
                let receiver_base = (!ast.is_static).then_some(class.base);
                methods.push(
                    source_ir::build_with_calls(source, ast, context, receiver_base, &method_calls)
                        .map_err(diagnostic)?,
                );
            }
            classes.push(ClassDeclaration {
                superclass: class.superclass,
                name: class.name,
                base: class.base,
                fields: class.fields,
                methods,
            });
        }
    }
    origins.extend(class_origins);
    let modules = module_names(modules, &declarations, &shared);
    let member_index =
        crate::inheritance::member_index(&classes, &visibility).map_err(|message| {
            SourceDiagnostic {
                source: 0,
                diagnostic: Diagnostic::at(sources[0], 0, message),
            }
        })?;
    let unit = CompilationUnit {
        member_index,
        visibility,
        modules,
        declarations,
        classes,
        shared,
        enums: aliases.enum_declarations(),
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
fn shared_initialization_order(parsed: &[parser::ParsedUnit]) -> Vec<(Option<String>, String)> {
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
        for index in constant_initialization_order(&constants) {
            order.push((module.map(str::to_owned), constants[index].name.clone()));
        }
    }
    order
}

fn prepare_sources<'a>(
    sources: &[&'a str],
    aliases: &std::rc::Rc<parser::Declarations<'a>>,
    unchecked: bool,
) -> Result<(Vec<parser::ParsedUnit>, source_ir::CallContext), SourceDiagnostic> {
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
                is_static: false,
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
            reference.ancestors = aliases
                .class_types
                .get(&reference.name)
                .map(|c| c.ancestors.clone())
                .unwrap_or_default();
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
        for constant in parsed
            .shared
            .iter_mut()
            .map(|(_, constant)| constant)
            .chain(
                parsed
                    .classes
                    .iter_mut()
                    .flat_map(|class| &mut class.bindings),
            )
        {
            if let Some(annotation) = &mut constant.annotation {
                crate::structural::resolve_result(annotation, &mut resolve)?;
            }
        }
    }
    crate::inheritance::prepare_overrides(sources, &mut parsed_sources, &aliases.class_types)?;
    let mut calls = source_ir::CallContext::new();
    calls.class_ancestors = aliases
        .class_types
        .iter()
        .map(|(n, c)| (n.clone(), c.ancestors.clone()))
        .collect();
    calls.modules = parsed_sources
        .iter()
        .flat_map(|unit| unit.modules.iter().cloned())
        .collect();
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
    for class in parsed_sources.iter().flat_map(|p| &p.classes) {
        for ancestor in &aliases.class_types[&class.name].ancestors {
            let parent = parsed_sources
                .iter()
                .flat_map(|p| &p.classes)
                .find(|p| p.name == *ancestor)
                .unwrap();
            for (name, shared) in parent
                .fields
                .iter()
                .map(|f| (&f.name, f.kind.class_shared()))
                .chain(
                    parent
                        .bindings
                        .iter()
                        .map(|b| (&b.name, b.static_binding || b.enumeration)),
                )
            {
                let own = class.fields.iter().any(|f| f.name == *name)
                    || class.bindings.iter().any(|b| b.name == *name);
                let private = parent
                    .visibility
                    .iter()
                    .any(|(n, v)| n == name && *v == crate::Visibility::Private);
                if !own && !private {
                    if shared {
                        calls
                            .class_shared
                            .insert((class.name.clone(), name.clone()));
                    }
                    calls
                        .inherited_fields
                        .entry((class.name.clone(), name.clone()))
                        .or_insert_with(|| ancestor.clone());
                }
            }
            for method in &parent.methods {
                if method.name == "initialize" || field_initializer(&method.name) {
                    continue;
                }
                if parent
                    .visibility
                    .iter()
                    .any(|(name, v)| name == &method.name && *v == crate::Visibility::Private)
                {
                    continue;
                }
                calls.inherit(
                    (class.name.clone(), method.name.clone()),
                    source_ir::declared_signature(method),
                );
            }
        }
    }
    calls.refresh_inherited_fields();
    for parsed in &parsed_sources {
        for (owner, declaration) in &parsed.shared {
            if declaration.mutable {
                calls
                    .shared_mutable
                    .insert((owner.clone(), declaration.name.clone()));
            }
            if let Some(name) = &declaration.enum_type {
                calls
                    .enum_members
                    .insert((owner.clone(), declaration.name.clone()), name.clone());
            }
        }
    }
    for parsed in &parsed_sources {
        for class in &parsed.classes {
            for declaration in &class.bindings {
                if declaration.enumeration || declaration.static_binding {
                    calls
                        .class_shared
                        .insert((class.name.clone(), declaration.name.clone()));
                }
                if let Some(name) = &declaration.enum_type {
                    calls.enum_members.insert(
                        (Some(class.name.clone()), declaration.name.clone()),
                        name.clone(),
                    );
                }
            }
        }
    }
    Ok((parsed_sources, calls))
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

/// Preserve each phase's declaration order using default eligibility recorded
/// by the parser. Enum syntax can distinguish grouped and ungrouped literals.
fn constant_initialization_order(constants: &[&parser::ParsedBinding]) -> Vec<usize> {
    let (defaults, runtime): (Vec<_>, Vec<_>) =
        (0..constants.len()).partition(|index| constants[*index].literal_default);
    defaults.into_iter().chain(runtime).collect()
}

/// Resolve stored declaration contracts independently of evaluation order.
/// Only successful, fully checked expression types enter the shared tables;
/// unresolved dependencies are retried after other declarations make progress.
fn infer_stored_constants(
    sources: &[&str],
    parsed: &[parser::ParsedUnit],
    context: Option<crate::api::ApiContext>,
    calls: &mut source_ir::CallContext,
    discover: bool,
) -> Result<(BTreeMap<String, crate::api::Type>, Option<SourceDiagnostic>), SourceDiagnostic> {
    let mut backings: BTreeMap<String, crate::api::Type> = BTreeMap::new();
    let mut pending = Vec::new();
    let mut class_names = BTreeMap::new();
    let mut field_names = BTreeMap::new();
    for (source, unit) in parsed.iter().enumerate() {
        for (module, declaration) in &unit.shared {
            if declaration.mutable {
                let contract = declaration
                    .annotation
                    .clone()
                    .expect("mutable declarations have explicit or dynamic contracts");
                calls
                    .shared_fields
                    .insert((module.clone(), declaration.name.clone()), contract);
            }
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
            field_names.insert(
                class.name.as_str(),
                class
                    .fields
                    .iter()
                    .map(|field| field.name.clone())
                    .chain(class.bindings.iter().map(|field| field.name.clone()))
                    .collect::<BTreeSet<_>>(),
            );
            class_names.insert(
                class.name.as_str(),
                class
                    .bindings
                    .iter()
                    .filter(|binding| !binding.mutable)
                    .map(|c| c.name.clone())
                    .collect::<BTreeSet<_>>(),
            );
            pending.extend(
                class
                    .bindings
                    .iter()
                    .map(|declaration| (source, Owner::Class(class.name.as_str()), declaration)),
            );
        }
    }
    for (class, field) in calls.inherited_fields.keys() {
        field_names
            .get_mut(class.as_str())
            .unwrap()
            .insert(field.clone());
    }
    let shared_order = shared_initialization_order(parsed);
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
                    owner_calls.current_module =
                        class.rsplit_once('.').map(|(owner, _)| owner.to_owned());
                    owner_calls.current_class = Some(crate::SourceClass {
                        name: class.into(),
                        ancestors: calls
                            .class_ancestors
                            .get(class)
                            .cloned()
                            .unwrap_or_default(),
                        base,
                    });
                    if declaration.static_binding {
                        for (member, signature) in calls.class_signatures(class) {
                            owner_calls.insert(("self".into(), member), signature);
                        }
                    }
                    owner_calls.field_names = field_names[class].clone();
                    owner_calls.readonly_fields = class_names[class].clone();
                    owner_calls.field_types = calls.source_fields[class].clone();
                    Some(base)
                }
            };
            let mut probe_declaration = declaration.clone();
            if discover && probe_declaration.enum_type.is_some() {
                probe_declaration.annotation = None;
                probe_declaration.enum_type = None;
            }
            match infer_constant(
                sources[source],
                &probe_declaration,
                context,
                base,
                &owner_calls,
            ) {
                Ok(mut value_type) => {
                    if discover {
                        if let Some(identity) = &declaration.enum_type {
                            let backing =
                                crate::enums::project(&crate::tuples::exact_type(&value_type))
                                    .clone();
                            let next = backings.get(identity.name()).map_or_else(
                                || backing.clone(),
                                |previous| {
                                    crate::unions::normalize(vec![
                                        previous.clone(),
                                        backing.clone(),
                                    ])
                                },
                            );
                            backings.insert(identity.name().into(), next);
                            // Only publish resolved nominal views. Unresolved groups supply
                            // evidence for the next pass, not executable placeholder types.
                            if identity.backing().is_none() {
                                continue;
                            }
                            value_type = ReturnType::Enum(identity.clone());
                        }
                    }
                    match owner {
                        Owner::Class(class) => {
                            calls
                                .source_fields
                                .get_mut(class)
                                .unwrap()
                                .insert(declaration.name.clone(), value_type);
                            calls.refresh_inherited_fields();
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
                    }
                }
                Err(diagnostic) => {
                    first_error.get_or_insert(SourceDiagnostic { source, diagnostic });
                    unresolved.push((source, owner, declaration));
                }
            }
        }
        if unresolved.len() == count {
            if discover {
                return Ok((backings, first_error));
            }
            return Err(first_error.unwrap());
        }
        pending = unresolved;
    }
    Ok((backings, None))
}

/// Check an initializer with ordinary expression IR and return its stored contract.
/// An annotation supplies contextual construction and remains the binding's type;
/// inferred Array literals use mutable element contracts rather than positional slots.
fn infer_constant(
    source: &str,
    declaration: &parser::ParsedBinding,
    context: Option<crate::api::ApiContext>,
    base: Option<ClassBase>,
    calls: &source_ir::CallContext,
) -> Result<ReturnType, Diagnostic> {
    let probe = parser::ParsedFunction {
        is_static: declaration.static_binding,
        name: FIELD_INITIALIZER.into(),
        parameters: Vec::new(),
        parameter_types: Vec::new(),
        result_type: if declaration.enum_type.is_some() {
            None
        } else {
            declaration.annotation.clone()
        },
        statements: vec![parser::Statement::Return {
            value: Some(declaration.initializer.clone()),
            span: declaration.span,
        }],
        body_span: declaration.span,
    };
    let probe = source_ir::build_with_calls(source, probe, context, base, calls)?;
    if let Some(annotation) = &declaration.annotation {
        return FieldKind::value_contract(annotation.clone()).ok_or_else(|| {
            Diagnostic::at(
                source,
                declaration.span.start,
                "unsupported constant field contract",
            )
        });
    }
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
    let mut visibility = Vec::new();
    let mut i = 1;
    let mut declarations = Vec::new();
    let mut classes = Vec::new();
    let mut shared = Vec::new();
    let mut enums = Vec::new();
    let mut modules = Vec::new();
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
            [".module", path] => modules.push((*path).to_owned()),
            [".enum", owner, name, backing] => enums.push(crate::EnumDeclaration {
                module: (*owner != "$").then(|| (*owner).to_owned()),
                name: (*name).to_owned(),
                backing: crate::tuples::exact_type(
                    &crate::type_text::call_result(backing)
                        .map_err(|e| Diagnostic::at(text, at, e))?,
                ),
            }),
            [
                kind @ (".shared-constant" | ".shared-variable"),
                owner,
                name,
                tag,
            ] => {
                let value_type = crate::type_text::call_result(tag)
                    .ok()
                    .and_then(FieldKind::value_contract)
                    .ok_or_else(|| Diagnostic::at(text, at, "unsupported shared binding type"))?;
                shared.push(SharedBinding {
                    module: (*owner != "$").then(|| (*owner).to_owned()),
                    name: (*name).to_owned(),
                    mutable: *kind == ".shared-variable",
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
                tail @ ..,
            ] if tail.len() <= 1 => {
                let superclass = tail
                    .first()
                    .map(|p| {
                        p.strip_prefix("parent=")
                            .filter(|n| crate::names::module_path(n))
                            .map(str::to_owned)
                            .ok_or_else(|| Diagnostic::at(text, at, "invalid source parent"))
                    })
                    .transpose()?;
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
                    if parts.first().is_some_and(|p| p == ".visibility") {
                        if !methods.is_empty() || !instance_fields.is_empty() || parts.len() != 3 {
                            return Err(Diagnostic::at(
                                text,
                                method_at,
                                "visibility records must precede fields and methods",
                            ));
                        }
                        let access = crate::Visibility::parse(&parts[2])
                            .ok_or_else(|| Diagnostic::at(text, method_at, "invalid visibility"))?;
                        visibility.push((name.clone(), parts[1].clone(), access));
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
                            ".field-enum"
                                | ".field-constant"
                                | ".field-initialized"
                                | ".field-value"
                                | ".field-number"
                                | ".field-dynamic-number"
                                | ".field-dynamic-symbol"
                                | ".field-object"
                                | ".field-array"
                                | ".field-dictionary"
                                | ".field-scalar"
                                | ".field-numeric"
                                | ".field-reference"
                                | ".field-interface"
                                | ".field-static-constant"
                                | ".field-static-variable"
                                | ".field-symbol"
                                | ".field-method"
                                | ".field-weak-reference"
                                | ".field-byte-array"
                        )
                    }) {
                        if (parts.len() != 3
                            && !(matches!(
                                parts[0].as_str(),
                                ".field-symbol"
                                    | ".field-enum"
                                    | ".field-static-constant"
                                    | ".field-static-variable"
                            ) && parts.len() == 4))
                            || !methods.is_empty()
                            || instance_fields.len() >= 256
                        {
                            return Err(Diagnostic::at(
                                text,
                                method_at,
                                "expected a three-part field declaration before methods, at most 256 fields",
                            ));
                        }
                        let kind = if matches!(
                            parts[0].as_str(),
                            ".field-enum" | ".field-static-constant" | ".field-static-variable"
                        ) {
                            if parts.len() != 4 {
                                return Err(Diagnostic::at(
                                    text,
                                    method_at,
                                    "expected enum contract and Number/null default",
                                ));
                            }
                            let value_type = crate::type_text::call_result(&parts[2])
                                .map_err(|e| Diagnostic::at(text, method_at, e))?;
                            let initial = crate::ClassFieldInitialization::parse(&parts[3])
                                .ok_or_else(|| {
                                    Diagnostic::at(
                                        text,
                                        method_at,
                                        "invalid enum initialization mode",
                                    )
                                })?;
                            if parts[0] == ".field-static-variable" {
                                FieldKind::StaticVariable {
                                    value_type,
                                    initial,
                                }
                            } else if parts[0] == ".field-static-constant" {
                                FieldKind::StaticConstant {
                                    value_type,
                                    initial,
                                }
                            } else {
                                FieldKind::Enum {
                                    value_type,
                                    initial,
                                }
                            }
                        } else if matches!(
                            parts[0].as_str(),
                            ".field-constant" | ".field-initialized"
                        ) {
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
                            if parts[0] == ".field-constant" {
                                FieldKind::Constant { value_type }
                            } else {
                                FieldKind::Initialized { value_type }
                            }
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
                        } else if matches!(
                            parts[0].as_str(),
                            ".field-number" | ".field-dynamic-number"
                        ) {
                            let initial = parts[2].parse::<i32>().map_err(|_| {
                                Diagnostic::at(text, method_at, "invalid Number field initializer")
                            })?;
                            if parts[0] == ".field-number" {
                                FieldKind::Number(initial)
                            } else {
                                FieldKind::DynamicNumber(initial)
                            }
                        } else if parts[0] == ".field-dynamic-symbol" {
                            FieldKind::DynamicSymbol(
                                parts[2]
                                    .strip_prefix(':')
                                    .ok_or_else(|| {
                                        Diagnostic::at(
                                            text,
                                            method_at,
                                            "dynamic Symbol defaults use :identifier",
                                        )
                                    })?
                                    .to_owned(),
                            )
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
                    superclass,
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
    CompilationUnit::with_modules_and_visibility(
        declarations,
        classes,
        shared,
        enums,
        modules,
        visibility,
    )
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

/// Include explicit modules and every containing scope implied by owned members.
fn module_names(
    modules: Vec<String>,
    declarations: &[FunctionDeclaration],
    shared: &[SharedBinding],
) -> BTreeSet<String> {
    modules
        .iter()
        .map(String::as_str)
        .chain(declarations.iter().filter_map(|d| d.module.as_deref()))
        .chain(shared.iter().filter_map(|d| d.module.as_deref()))
        .flat_map(crate::names::scopes)
        .map(str::to_owned)
        .collect()
}

/// Resolve the class and member captured by a generic shared-write completion.
/// Function validation has already checked the capture's lifetime and identity.
pub(crate) fn class_store<'a>(
    operation: &Operation,
    function: &'a Function,
) -> Option<(&'a str, &'a str)> {
    let Operation::SharedWrite { builder, .. } = operation else {
        return None;
    };
    function
        .operations()
        .iter()
        .find_map(|op| match &op.operation {
            Operation::ClassFieldBegin {
                builder: begin,
                class,
                field,
            } if begin == builder => Some((class.as_str(), field.as_str())),
            _ => None,
        })
}

/// Find a lexical store's declared binding when its completion performs lookup.
fn lexical_store<'a>(
    operation: &'a Operation,
    function: &'a Function,
) -> Option<(Option<&'a String>, &'a str)> {
    let Operation::SharedWrite { builder, .. } = operation else {
        return None;
    };
    function
        .operations()
        .iter()
        .find_map(|op| match &op.operation {
            Operation::SharedBegin {
                builder: begin,
                module,
                field,
                lexical: true,
            } if begin == builder => Some((module.as_ref(), field.as_str())),
            _ => None,
        })
}

/// Return the declared target for lexical resolution; None within Some means globals.
fn lexical_target<'a>(operation: &'a Operation, function: &'a Function) -> Option<Option<&'a str>> {
    match operation {
        Operation::SharedRead {
            module,
            lexical: true,
            ..
        }
        | Operation::SharedBegin {
            module,
            lexical: true,
            ..
        } => Some(module.as_deref()),
        Operation::ModuleCall {
            module,
            lexical: true,
            ..
        }
        | Operation::ModuleCallStatement {
            module,
            lexical: true,
            ..
        } => Some(Some(module.as_str())),
        Operation::GlobalCall { lexical: true, .. }
        | Operation::GlobalCallStatement { lexical: true, .. } => Some(None),
        Operation::SharedWrite { .. } => {
            lexical_store(operation, function).map(|(module, _)| module.map(String::as_str))
        }
        _ => None,
    }
}
