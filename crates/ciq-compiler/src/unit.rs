//! Declaration collection, cross-function checks, and unit text serialization.
use crate::{Diagnostic, Function, Operation, ReturnType, backend, parser, source_ir};
use ciq_assembly::Program;
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

pub(crate) const MAX_FUNCTIONS: usize = 256;

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
        Operation::ModuleCall { module, member, .. }
        | Operation::ModuleCallStatement { module, member, .. } => {
            Some((Owner::Module(module), member))
        }
        Operation::SelfCall { member, .. } | Operation::SelfCallStatement { member, .. } => {
            Some((owner, member))
        }
        Operation::ObjectCall { class, member, .. }
        | Operation::ObjectCallStatement { class, member, .. } => {
            Some((Owner::Class(class), member))
        }
        Operation::SourceNew { class, .. } => Some((Owner::Class(class), "initialize")),
        _ => None,
    }
}

fn requests_exit(operation: &Operation) -> bool {
    matches!(operation, Operation::NativeModuleCallStatement { module, member, .. } if module == "Toybox.System" && member == "exit")
}

/// Treat erased comparison/element callbacks as possible shutdown requests.
/// No exact body remains available for the reference-initialization proof.
fn erased_callbacks(function: &Function, operation: &Operation) -> bool {
    match operation {
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
            matches!(
                function.value_types[receiver],
                crate::ValueType::Interface(_) | crate::ValueType::NullableInterface(_)
            ) || (member == "sort"
                && arguments.iter().any(|a| {
                    matches!(
                        function.value_types[a],
                        crate::ValueType::Interface(_) | crate::ValueType::NullableInterface(_)
                    )
                }))
                || matches!(function.value_types[receiver], crate::ValueType::Array(
                    crate::ArrayElement::Interface(_) | crate::ArrayElement::NullableInterface(_))
                    if matches!(member.as_str(), "toString" | "indexOf" | "remove" | "removeAll" | "sort"))
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
    if !matches!(function.value_types[&receiver], crate::ValueType::Array(_)) {
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
                    || source_callee(owner, &located.operation)
                        .is_some_and(|key| exits.contains(&key))
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
        (self != Self::ApplicationAppBase).then(|| crate::api::Type::named(self.api_name()))
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

/// A supported field's default and initialization requirement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldKind {
    /// A Number copied from class metadata into each new instance.
    Number(i32),
    /// A null reference slot that source initialize must fill with this exact
    /// non-null source class before reads, self calls, or return.
    SourceObject(crate::SourceClass),
    /// A Null-initialized comparison-interface slot. Non-null fields must be
    /// filled by source initialize before reads, self calls, or return.
    Interface {
        interface: crate::InterfaceType,
        nullable: bool,
    },
}

impl FieldKind {
    fn requires_initialization(&self) -> bool {
        matches!(
            self,
            Self::SourceObject(_)
                | Self::Interface {
                    nullable: false,
                    ..
                }
        )
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
            FieldKind::Number(_) => ReturnType::Integer,
            FieldKind::SourceObject(class) => ReturnType::SourceObject(class.clone()),
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
    /// Number/reference fields in declaration order.
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
}

impl CompilationUnit {
    /// Check known native bases, field types, and unguarded API references.
    /// Errors retain the unit's flattened body indexes. The context is external
    /// to semantic IR; this does not validate a device or application profile.
    pub fn validate_api(&self, context: crate::api::ApiContext) -> Result<(), UnitError> {
        self.validate_declaration_api(context)?;
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
        let unit = Self {
            declarations,
            classes,
        };
        unit.validate()?;
        Ok(unit)
    }

    fn validate(&self) -> Result<(), UnitError> {
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
            let references = class
                .fields
                .iter()
                .filter_map(|field| {
                    if let FieldKind::SourceObject(target) = &field.kind {
                        Some(target)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            for target in &references {
                if !self.classes.iter().any(|declaration| {
                    declaration.name == target.name
                        && declaration.base == target.base
                        && target.base.constructed_type().is_some()
                }) {
                    return Err(fail(
                        Some(at),
                        None,
                        "reference field class and native base must match a constructible declaration".into(),
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
                    "reference fields require a source initializer".into(),
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
        for (index, &(owner, function)) in bodies.iter().enumerate() {
            let source_result = match function.return_type() {
                ReturnType::SourceObject(class) => Some(class),
                _ => None,
            };
            let source_classes = function
                .parameter_types()
                .iter()
                .filter_map(|ty| {
                    if let crate::ParameterType::SourceObject(class) = ty {
                        Some(class)
                    } else {
                        None
                    }
                })
                .chain(source_result.as_ref());
            for name in source_classes {
                if !self.classes.iter().any(|class| {
                    class.name == name.name
                        && class.base == name.base
                        && name.base.constructed_type().is_some()
                }) {
                    return Err(fail(
                        Some(index),
                        None,
                        format!(
                            "source signature class {name} must match a constructible declaration and native base"
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
            } else if function
                .parameter_types()
                .contains(&crate::ParameterType::NullableDictionary)
            {
                return Err(fail(Some(index), None,
                    "nullable-dictionary parameters are supported only for native lifecycle callbacks".into()));
            }
            let base_calls = function
                .operations()
                .iter()
                .filter(|op| matches!(op.operation, Operation::NativeBaseInitialize { .. }))
                .count();
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
                            || source_callee(owner, &located.operation)
                                .is_some_and(|key| exit_callers.contains(&key))
                            || comparator_callee(function, &located.operation, &bodies)
                                .is_some_and(|key| exit_callers.contains(&key)))
                    {
                        return Err(fail(
                            Some(index),
                            Some(operation),
                            "initialize reference fields before calls that may request shutdown"
                                .into(),
                        ));
                    }
                    match &located.operation {
                        Operation::SelfValue { .. } if !pending.is_empty() => return Err(fail(Some(index), Some(operation), "initialize all reference fields before exposing self".into())),
                        Operation::SelfFieldRead { field, .. } if pending.contains(field.as_str()) => return Err(fail(Some(index), Some(operation), "reference field read before initialization".into())),
                        Operation::SelfFieldAssign { field, .. } => { pending.remove(field.as_str()); }
                        Operation::SelfCall { .. } | Operation::SelfCallStatement { .. } | Operation::Branch { .. } | Operation::Jump { .. } | Operation::ReturnVoid if !pending.is_empty() => return Err(fail(Some(index), Some(operation), "initialize all reference fields before self calls, control flow, or return".into())),
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
            let mut origins =
                crate::source_objects::Origins::new(function.parameter_types().to_vec());
            let array_builders = function
                .operations()
                .iter()
                .filter_map(|located| {
                    if let Operation::ArrayBegin {
                        builder,
                        element_type,
                        ..
                    } = located.operation
                    {
                        Some((builder, element_type))
                    } else {
                        None
                    }
                })
                .collect::<BTreeMap<_, _>>();
            let interface_locals = function
                .operations()
                .iter()
                .filter_map(|located| {
                    if let Operation::LocalInitialize { local, value } = located.operation {
                        let ty = function.value_types[&value];
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
            let inputs = crate::type_flow::prefix_inputs(function.operations());
            let mut facts = Default::default();
            for located in function.operations() {
                crate::type_flow::update(&mut facts, &inputs, &located.operation);
                origins
                    .observe(&located.operation, &facts)
                    .map_err(|message| fail(Some(index), None, message.into()))?;
            }
            for (operation, located) in function.operations().iter().enumerate() {
                let check_element = |element: crate::ArrayElement,
                                     value: crate::ValueId|
                 -> Result<(), UnitError> {
                    if let crate::ArrayElement::Interface(interface)
                    | crate::ArrayElement::NullableInterface(interface) = element
                    {
                        self.check_interface_value(
                            function,
                            &origins,
                            value,
                            interface,
                            matches!(element, crate::ArrayElement::NullableInterface(_)),
                        )
                        .map_err(|message| fail(Some(index), Some(operation), message))?;
                    }
                    Ok(())
                };
                match &located.operation {
                    Operation::ArrayFill { builder, value, .. } => {
                        check_element(array_builders[builder], *value)?
                    }
                    Operation::ArrayFillMany {
                        builder, values, ..
                    } => {
                        for value in values {
                            check_element(array_builders[builder], *value)?;
                        }
                    }
                    Operation::ArrayWrite { array, value, .. } => {
                        if let crate::ValueType::Array(element) = function.value_types[array] {
                            check_element(element, *value)?;
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
                        if matches!(function.value_types[receiver], crate::ValueType::Array(_)) {
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
                        Operation::SelfFieldRead { result_type, .. }
                            if *result_type != declaration.result_type() =>
                        {
                            return Err(fail(
                                Some(index),
                                Some(operation),
                                "field read type does not match its declaration".into(),
                            ));
                        }
                        Operation::SelfFieldAssign { value, .. } => {
                            if let FieldKind::SourceObject(class) = &declaration.kind {
                                if origins.class(*value) != Some(class.as_str()) {
                                    return Err(fail(
                                        Some(index),
                                        Some(operation),
                                        "field assignment must preserve the declared source class"
                                            .into(),
                                    ));
                                }
                            }
                            if let FieldKind::Interface {
                                interface,
                                nullable,
                            } = declaration.kind
                            {
                                self.check_interface_value(
                                    function, &origins, *value, interface, nullable,
                                )
                                .map_err(|message| fail(Some(index), Some(operation), message))?;
                            } else if !crate::native::accepts(
                                &declaration.result_type().api_type(),
                                &function.value_types[value].api_type(),
                            ) {
                                return Err(fail(
                                    Some(index),
                                    Some(operation),
                                    "field assignment type does not match its declaration".into(),
                                ));
                            }
                        }
                        _ => {}
                    }
                }
                if let Operation::SourceCast { class, .. }
                | Operation::SourceInstanceOf { class, .. } = &located.operation
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
                if let Operation::SourceNew { class, base, .. } = &located.operation {
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
                let (key, arguments, value_call) = match &located.operation {
                    Operation::SourceNew {
                        class, arguments, ..
                    } => (
                        (Owner::Class(class.as_str()), "initialize"),
                        arguments,
                        false,
                    ),
                    Operation::ObjectCall {
                        class,
                        member,
                        arguments,
                        ..
                    } => (
                        (Owner::Class(class.as_str()), member.as_str()),
                        arguments,
                        true,
                    ),
                    Operation::ObjectCallStatement {
                        class,
                        member,
                        arguments,
                        ..
                    } => (
                        (Owner::Class(class.as_str()), member.as_str()),
                        arguments,
                        false,
                    ),
                    Operation::ModuleCall {
                        module,
                        member,
                        arguments,
                        ..
                    } => (
                        (Owner::Module(module.as_str()), member.as_str()),
                        arguments,
                        true,
                    ),
                    Operation::ModuleCallStatement {
                        module,
                        member,
                        arguments,
                    } => (
                        (Owner::Module(module.as_str()), member.as_str()),
                        arguments,
                        false,
                    ),
                    Operation::SelfCall {
                        member, arguments, ..
                    } => ((owner, member.as_str()), arguments, true),
                    Operation::SelfCallStatement { member, arguments } => {
                        ((owner, member.as_str()), arguments, false)
                    }
                    _ => continue,
                };
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
                    if let crate::ParameterType::SourceObject(class) = parameter {
                        if origins.class(*argument) != Some(class.as_str()) {
                            return Err(fail(
                                Some(index),
                                Some(operation),
                                "argument does not preserve the declared source class".into(),
                            ));
                        }
                    }
                    if !crate::native::accepts(
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
                let expected_result = match &located.operation {
                    Operation::ModuleCall { result_type, .. }
                    | Operation::ObjectCall { result_type, .. }
                    | Operation::SelfCall { result_type, .. } => Some(result_type),
                    _ => None,
                };
                if expected_result.is_some_and(|expected| expected != result) {
                    return Err(fail(
                        Some(index),
                        Some(operation),
                        format!(
                            "{}.{} call result type does not match callee contract",
                            key.0.name(),
                            key.1
                        ),
                    ));
                }
            }
        }
        Ok(())
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
        let class = origins
            .class(value)
            .ok_or("interface argument or return requires a checked source implementation")?;
        self.check_interface_signature(class, interface.name())
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
            if required.variadic.is_some()
                || implementation.parameter_types().len() != required.parameters.len()
                || !implementation
                    .parameter_types()
                    .iter()
                    .zip(&required.parameters)
                    .all(|(actual, expected)| {
                        !matches!(actual, crate::ParameterType::SourceObject(_))
                            && &actual.api_type() == expected
                    })
                || required.result != implementation.return_type().api_type()
            {
                return Err(format!(
                    "{class}.{} does not satisfy the {} signature",
                    required.name, interface.name
                ));
            }
        }
        Ok(())
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
                backend::lower_named(&d.function, &d.code_label())
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
                    FieldKind::Number(initial) => {
                        format!(".field-number {} {initial}\n", field.name)
                    }
                    FieldKind::SourceObject(class) => {
                        format!(".field-reference {} {class}\n", field.name)
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
/// Collects global/module functions and AppBase-derived class methods
/// before resolving qualified calls, so declarations may follow their callers.
/// Modules may be reopened across files; class names and member identities
/// must remain unique. Calls on self resolve within their own class.
/// Resolves the supported native module aliases and imported superclass names;
/// native bindings are file-local and inherited by class/module scopes.
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
    let mut declarations = Vec::new();
    let mut classes = Vec::new();
    let mut origins = Vec::new();
    let mut class_origins = Vec::new();
    let mut parsed_sources = sources
        .iter()
        .enumerate()
        .map(|(source, text)| {
            parser::declarations(text).map_err(|diagnostic| SourceDiagnostic { source, diagnostic })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let bases = parsed_sources
        .iter()
        .flat_map(|p| p.classes.iter())
        .map(|class| (class.name.clone(), class.base))
        .collect::<BTreeMap<_, _>>();
    for (index, parsed) in parsed_sources.iter_mut().enumerate() {
        let resolve = |reference: &mut crate::SourceClass| -> Result<(), SourceDiagnostic> {
            reference.base = bases
                .get(&reference.name)
                .copied()
                .filter(|base| base.constructed_type().is_some())
                .ok_or_else(|| SourceDiagnostic {
                    source: index,
                    diagnostic: Diagnostic::at(
                        sources[index],
                        0,
                        format!(
                            "source reference {} must resolve to a constructible class",
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
                if let crate::ParameterType::SourceObject(class) = parameter {
                    resolve(class)?;
                }
            }
            if let Some(ReturnType::SourceObject(class)) = &mut ast.result_type {
                resolve(class)?;
            }
        }
        for field in parsed
            .classes
            .iter_mut()
            .flat_map(|class| class.fields.iter_mut())
        {
            if let FieldKind::SourceObject(class) = &mut field.kind {
                resolve(class)?;
            }
        }
    }
    let mut calls = source_ir::CallContext::new();
    for parsed in &parsed_sources {
        for (module, ast) in &parsed.functions {
            calls.insert(
                (module.clone().unwrap_or_default(), ast.name.clone()),
                source_ir::declared_signature(ast),
            );
        }
        for class in &parsed.classes {
            calls.class_bases.insert(class.name.clone(), class.base);
            for ast in &class.methods {
                calls.insert(
                    (class.name.clone(), ast.name.clone()),
                    source_ir::declared_signature(ast),
                );
            }
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
            declarations.push(FunctionDeclaration {
                module,
                function: source_ir::build_with_calls(source, ast, context, None, &calls)
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
    };
    unit.validate()
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
    Ok(unit)
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
                return crate::ir_text::parse_ir_context(&text[start..end], context)
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
                            ".field-number" | ".field-reference" | ".field-interface"
                        )
                    }) {
                        if parts.len() != 3 || !methods.is_empty() || instance_fields.len() >= 256 {
                            return Err(Diagnostic::at(
                                text,
                                method_at,
                                "expected a three-part field declaration before methods, at most 256 fields",
                            ));
                        }
                        let kind = if parts[0] == ".field-number" {
                            FieldKind::Number(parts[2].parse::<i32>().map_err(|_| {
                                Diagnostic::at(text, method_at, "invalid Number field initializer")
                            })?)
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
                            FieldKind::SourceObject(
                                crate::SourceClass::parse(&parts[2]).ok_or_else(|| {
                                    Diagnostic::at(
                                        text,
                                        method_at,
                                        "invalid source reference contract",
                                    )
                                })?,
                            )
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
    CompilationUnit::with_classes(declarations, classes)
        .and_then(|unit| {
            if let Some(context) = context {
                unit.validate_declaration_api(context)?;
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
