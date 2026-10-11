//! Declaration collection, cross-function checks, and unit text serialization.
use crate::{Diagnostic, Function, Operation, ReturnType, backend, parser, source_ir};
use ciq_assembly::Program;
use std::{collections::BTreeMap, fmt};

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

/// A global function or a member of a flat source module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionDeclaration {
    /// None for a global function, otherwise its unqualified module name.
    pub module: Option<String>,
    /// Validated function body with integer parameters and integer/void result.
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassBase {
    ApplicationAppBase,
}

/// A named instance class and its declared method bodies in source order.
/// Fields, source inheritance, and native method bodies are not supported yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassDeclaration {
    /// Unqualified class identity, distinct from module/global names.
    pub name: String,
    /// Native superclass identity resolved by application generation.
    pub base: ClassBase,
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
/// All qualified user calls resolve inside the unit. Parameters and arguments
/// remain integers; results may be integers or void. Void calls are permitted
/// only as statements. Self calls require a declared method in the enclosing
/// class. System.println is the separate intrinsic.
/// CODE lowering leaves symbol/DATA bindings and application metadata external.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompilationUnit {
    declarations: Vec<FunctionDeclaration>,
    classes: Vec<ClassDeclaration>,
}

impl CompilationUnit {
    /// Check known native bases and unguarded API references in all bodies.
    /// Errors retain the unit's flattened body indexes. The context is external
    /// to semantic IR; this does not validate a device or application profile.
    pub fn validate_api(&self, context: crate::api::ApiContext) -> Result<(), UnitError> {
        for class in &self.classes {
            match class.base {
                ClassBase::ApplicationAppBase => {
                    context.check_class(crate::api::core(), "Toybox.Application.AppBase")
                }
            }
            .map_err(|message| UnitError {
                declaration: None,
                operation: None,
                message,
            })?;
        }
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

    /// Validate declarations and qualified calls against all function signatures.
    ///
    /// Accepts forward and recursive calls. Rejects duplicate identities,
    /// invalid/reserved module names, global/module name collisions, unresolved
    /// callees, wrong argument counts, void calls used as values, and units outside
    /// 1..=256 functions.
    /// Each immutable Function has already checked argument/result types and
    /// local control-flow invariants. This does not prove recursive termination.
    pub fn new(declarations: Vec<FunctionDeclaration>) -> Result<Self, UnitError> {
        Self::with_classes(declarations, Vec::new())
    }

    /// Validate global/module functions and instance classes together.
    ///
    /// Resolves self calls only against the enclosing class, and module calls
    /// only against module functions. Checks owner/name uniqueness, exact arity,
    /// receiver contracts, and integer/void results. Requires 1..=256 bodies.
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
        for (index, &(owner, function)) in bodies.iter().enumerate() {
            let callback = matches!(owner, Owner::Class(_))
                .then(|| crate::native::callback(ClassBase::ApplicationAppBase, function.name()))
                .flatten();
            if let Some(signature) = callback {
                if function.parameter_types() != signature.parameters
                    || function.return_type() != signature.result
                {
                    return Err(fail(
                        Some(index),
                        None,
                        format!(
                            "{} requires one nullable-dictionary parameter and a void result",
                            function.name()
                        ),
                    ));
                }
            } else if function
                .parameter_types()
                .iter()
                .any(|t| *t != crate::ParameterType::Integer)
            {
                return Err(fail(Some(index), None,
                    "noninteger parameter contracts are currently supported only for native lifecycle callbacks".into()));
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
                if function.parameter_count() != 0
                    || function.return_type() != crate::ReturnType::Void
                    || base_calls != 1
                    || !first.is_some_and(|op| {
                        matches!(op.operation, Operation::NativeBaseInitialize { .. })
                    })
                {
                    return Err(fail(Some(index), None,
                        "source initialize requires zero parameters, a void result, and one native base call as its first statement".into()));
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
                    (function.parameter_count(), function.return_type()),
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
            for (operation, located) in function.operations().iter().enumerate() {
                let (key, arguments, value_call) = match &located.operation {
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
                let Some((count, result)) = signatures.get(&key) else {
                    return Err(fail(
                        Some(index),
                        Some(operation),
                        format!("unresolved function {}.{}", key.0.name(), key.1),
                    ));
                };
                if matches!(key.0, Owner::Class(_))
                    && crate::native::callback(ClassBase::ApplicationAppBase, key.1).is_some()
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
                if *count != arguments.len() {
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
            text.push_str(&format!(".class {} Application.AppBase\n", class.name));
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
    for (source_index, source) in sources.iter().enumerate() {
        let diagnostic = |diagnostic| SourceDiagnostic {
            source: source_index,
            diagnostic,
        };
        let parsed = parser::declarations(source).map_err(diagnostic)?;
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
                function: source_ir::build(source, ast, context).map_err(diagnostic)?,
            });
        }
        for class in parsed.classes {
            if let Some(context) = context {
                context
                    .check_class(crate::api::core(), "Toybox.Application.AppBase")
                    .map_err(|message| diagnostic(Diagnostic::at(source, 0, message)))?;
            }
            let mut methods = Vec::new();
            for ast in class.methods {
                if origins.len() + class_origins.len() >= MAX_FUNCTIONS {
                    return Err(diagnostic(Diagnostic::at(
                        source,
                        ast.body_span.start,
                        "compilation unit exceeds 256-function limit",
                    )));
                }
                class_origins.push((source_index, ast.body_span.start));
                methods.push(source_ir::build_method(source, ast, context).map_err(diagnostic)?);
            }
            classes.push(ClassDeclaration {
                name: class.name,
                base: ClassBase::ApplicationAppBase,
                methods,
            });
        }
    }
    origins.extend(class_origins);
    let unit = CompilationUnit {
        declarations,
        classes,
    };
    unit.validate().map_err(|error| {
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
            [".class", name, "Application.AppBase"] => {
                if let Some(context) = context {
                    context
                        .check_class(crate::api::core(), "Toybox.Application.AppBase")
                        .map_err(|message| Diagnostic::at(text, at, message))?;
                }
                let name = (*name).to_owned();
                let mut methods = Vec::new();
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
                    base: ClassBase::ApplicationAppBase,
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
    CompilationUnit::with_classes(declarations, classes).map_err(|e| {
        Diagnostic::at(
            text,
            e.declaration
                .and_then(|i| locations.get(i).copied())
                .unwrap_or(0),
            e.message,
        )
    })
}
