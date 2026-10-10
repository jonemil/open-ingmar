//! Declaration collection, cross-function checks, and unit text serialization.
use crate::{Diagnostic, Function, Operation, ReturnType, backend, parser, source_ir};
use ciq_assembly::Program;
use std::{collections::BTreeMap, fmt};

pub(crate) const MAX_FUNCTIONS: usize = 256;

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

/// A declaration or cross-function validation failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitError {
    /// Zero-based declaration index, absent for a unit-wide failure.
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

/// Validated global and module functions in deterministic input order.
///
/// All qualified user calls resolve inside the unit. Parameters and arguments
/// remain integers; results may be integers or void. Void calls are permitted
/// only as statements. System.println is the separate intrinsic.
/// CODE lowering leaves symbol/DATA bindings and application metadata external.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompilationUnit {
    declarations: Vec<FunctionDeclaration>,
}

impl CompilationUnit {
    /// Validate declarations and qualified calls against all function signatures.
    ///
    /// Accepts forward and recursive calls. Rejects duplicate identities,
    /// invalid/reserved module names, global/module name collisions, unresolved
    /// callees, wrong argument counts, void calls used as values, and units outside
    /// 1..=256 functions.
    /// Each immutable Function has already checked argument/result types and
    /// local control-flow invariants. This does not prove recursive termination.
    pub fn new(declarations: Vec<FunctionDeclaration>) -> Result<Self, UnitError> {
        Self::validate(&declarations)?;
        Ok(Self { declarations })
    }

    fn validate(declarations: &[FunctionDeclaration]) -> Result<(), UnitError> {
        let fail = |declaration, operation, message| UnitError {
            declaration,
            operation,
            message,
        };
        if declarations.is_empty() || declarations.len() > MAX_FUNCTIONS {
            return Err(fail(
                None,
                None,
                "compilation unit requires 1..=256 functions".into(),
            ));
        }
        let mut signatures = BTreeMap::new();
        for (index, declaration) in declarations.iter().enumerate() {
            if let Some(module) = &declaration.module {
                let valid = module
                    .bytes()
                    .next()
                    .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
                    && module
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'_');
                if !valid
                    || matches!(
                        module.as_str(),
                        "System"
                            | "function"
                            | "module"
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
                {
                    return Err(fail(
                        Some(index),
                        None,
                        "invalid or reserved module name".into(),
                    ));
                }
            }
            let key = (
                declaration.module.clone(),
                declaration.function.name().to_owned(),
            );
            if signatures
                .insert(
                    key,
                    (
                        declaration.function.parameter_count(),
                        declaration.function.return_type(),
                    ),
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
        for (index, declaration) in declarations.iter().enumerate() {
            if declaration.module.is_none()
                && declarations
                    .iter()
                    .any(|d| d.module.as_deref() == Some(declaration.function.name()))
            {
                return Err(fail(
                    Some(index),
                    None,
                    "global function collides with a module name".into(),
                ));
            }
            for (operation, located) in declaration.function.operations().iter().enumerate() {
                if let Operation::ModuleCall {
                    module,
                    member,
                    arguments,
                    ..
                }
                | Operation::ModuleCallStatement {
                    module,
                    member,
                    arguments,
                } = &located.operation
                {
                    let key = (Some(module.clone()), member.clone());
                    let Some((count, result)) = signatures.get(&key) else {
                        return Err(fail(
                            Some(index),
                            Some(operation),
                            format!("unresolved function {module}.{member}"),
                        ));
                    };
                    if *result == ReturnType::Void
                        && matches!(located.operation, Operation::ModuleCall { .. })
                    {
                        return Err(fail(
                            Some(index),
                            Some(operation),
                            format!("void function {module}.{member} cannot be used as a value"),
                        ));
                    }
                    if *count != arguments.len() {
                        return Err(fail(
                            Some(index),
                            Some(operation),
                            format!(
                                "{module}.{member} expects {count} arguments, got {}",
                                arguments.len()
                            ),
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    /// Return declarations in source-file and declaration order.
    pub fn declarations(&self) -> &[FunctionDeclaration] {
        &self.declarations
    }

    /// Lower all functions to one assembly model in declaration order.
    ///
    /// Uses unit-unique CODE/internal/string labels. Calls retain symbolic
    /// module/member references; layout is shared across all generated bodies.
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
            .collect();
        Program::new(items).expect("unit identities lower to disjoint assembly labels")
    }

    /// Serialize ownership and validated function IR as version-1 unit text.
    ///
    /// Inner functions use the supported version-1/version-2/version-3 IR format and
    /// optional spans. Source filenames are not serialized in this profile.
    pub fn to_ir_text(&self) -> String {
        let mut text = String::from("ciq-unit 1\n");
        for d in &self.declarations {
            match &d.module {
                None => text.push_str(".owner global\n"),
                Some(module) => text.push_str(&format!(".owner module {module}\n")),
            }
            text.push_str(&d.function.to_ir_text());
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
/// Collects global functions and flat module members in file/declaration order
/// before resolving qualified calls, so declarations may follow their callers.
/// Modules may be reopened across files; member identities must remain unique.
/// All user callees must be present. Rejects empty units and more than 256
/// functions; existing syntax/type/control-flow bounds apply per function.
/// Diagnostics identify the source index and byte/line/column within that file.
/// Application lifecycle, metadata, signing, and project configuration are not
/// generated here.
pub fn compile_sources(sources: &[&str]) -> Result<CompilationUnit, SourceDiagnostic> {
    let mut declarations = Vec::new();
    let mut origins = Vec::new();
    for (source_index, source) in sources.iter().enumerate() {
        let diagnostic = |diagnostic| SourceDiagnostic {
            source: source_index,
            diagnostic,
        };
        for (module, ast) in parser::declarations(source).map_err(diagnostic)? {
            let start = ast.body_span.start;
            if declarations.len() >= MAX_FUNCTIONS {
                return Err(diagnostic(Diagnostic::at(
                    source,
                    start,
                    "compilation unit exceeds 256-function limit",
                )));
            }
            origins.push((source_index, start));
            declarations.push(FunctionDeclaration {
                module,
                function: source_ir::build(source, ast).map_err(diagnostic)?,
            });
        }
    }
    CompilationUnit::validate(&declarations).map_err(|error| {
        let (source_index, start) = error.declaration.map_or((0, 0), |i| {
            let (source, start) = origins[i];
            let start = error
                .operation
                .and_then(|op| declarations[i].function.operations()[op].span)
                .map_or(start, |span| span.start);
            (source, start)
        });
        SourceDiagnostic {
            source: source_index,
            diagnostic: Diagnostic::at(
                sources.get(source_index).copied().unwrap_or(""),
                start,
                error.message,
            ),
        }
    })?;
    Ok(CompilationUnit { declarations })
}

/// Parse unit IR text and revalidate ownership and every qualified call.
///
/// Requires a `ciq-unit 1` header followed by owner records and complete
/// function IR documents. Reports text locations for malformed inner functions
/// and cross-function errors; source spans are retained as provenance only.
pub fn parse_unit_ir(text: &str) -> Result<CompilationUnit, Diagnostic> {
    let mut lines = text.split_inclusive('\n');
    let header = lines.next().unwrap_or("");
    if header
        .split('#')
        .next()
        .unwrap()
        .split_whitespace()
        .collect::<Vec<_>>()
        != ["ciq-unit", "1"]
    {
        return Err(Diagnostic::at(text, 0, "expected ciq-unit 1 header"));
    }
    let mut offset = header.len();
    let mut declarations = Vec::new();
    let mut locations = Vec::new();
    while let Some(line) = lines.next() {
        let owner_start = offset;
        offset += line.len();
        let parts: Vec<_> = line.split('#').next().unwrap().split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }
        let module = match parts.as_slice() {
            [".owner", "global"] => None,
            [".owner", "module", name] => Some((*name).to_owned()),
            _ => {
                return Err(Diagnostic::at(
                    text,
                    owner_start,
                    "expected .owner global or .owner module <name>",
                ));
            }
        };
        let function_start = offset;
        let mut function_text = String::new();
        let mut ended = false;
        for line in lines.by_ref() {
            function_text.push_str(line);
            offset += line.len();
            if line.split('#').next().unwrap().trim() == ".end" {
                ended = true;
                break;
            }
        }
        if !ended {
            return Err(Diagnostic::at(
                text,
                function_start,
                "expected function .end",
            ));
        }
        let function = crate::parse_ir(&function_text)
            .map_err(|error| Diagnostic::at(text, function_start + error.offset, error.message))?;
        locations.push(owner_start);
        declarations.push(FunctionDeclaration { module, function });
        if declarations.len() > MAX_FUNCTIONS {
            return Err(Diagnostic::at(
                text,
                owner_start,
                "compilation unit exceeds 256-function limit",
            ));
        }
    }
    CompilationUnit::new(declarations).map_err(|error| {
        Diagnostic::at(
            text,
            error.declaration.map_or(0, |i| locations[i]),
            error.message,
        )
    })
}
