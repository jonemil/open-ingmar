//! Restricted Monkey C frontend.
//!
//! [`compile`] parses Monkey C source and [`parse_ir`] parses saved IR text.
//! Both produce a validated [`Function`], which [`backend::lower`] converts
//! into typed VM assembly. Typed IR construction uses the same validation.

pub mod backend;
mod bindings;
pub use bindings::{string_bindings, symbol_bindings};
mod ir_text;
mod parser;
pub use ir_text::parse_ir;

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

pub(crate) const MAX_PARAMETERS: usize = 254;
pub(crate) const MAX_LOCALS: usize = 255;
pub(crate) const MAX_OPERATIONS: usize = 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Subtract,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operation {
    Integer {
        result: ValueId,
        value: i32,
    },
    String {
        result: ValueId,
        value: String,
    },
    Println {
        value: ValueId,
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
        module: String,
        member: String,
        arguments: Vec<ValueId>,
    },
    LocalInitialize {
        local: LocalId,
        value: ValueId,
    },
    LocalRead {
        result: ValueId,
        local: LocalId,
    },
    Return {
        value: ValueId,
    },
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
/// Every expression value has one use and refers only to earlier definitions.
/// Locals must be initialized before reads. Functions containing calls require
/// operations in expression evaluation order. Operations cannot be mutated
/// after validation; VM frame slots are assigned during lowering.
/// String types propagate through local reads; arithmetic, user-call arguments,
/// parameters, and the return value remain integer-only. Print statements accept
/// either integer or string values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    name: String,
    parameter_count: usize,
    operations: Vec<LocatedOperation>,
}

impl Function {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn parameter_count(&self) -> usize {
        self.parameter_count
    }

    pub fn operations(&self) -> &[LocatedOperation] {
        &self.operations
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
    /// Rejects invalid names, spans, value/local dependencies, call ordering,
    /// incompatible value types, and unsupported operation or parameter/local
    /// counts. External callee arity and types are not checked here.
    pub fn with_parameters(
        name: String,
        parameter_count: usize,
        operations: Vec<LocatedOperation>,
    ) -> Result<Self, IrError> {
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
        if operations.len() < 2 || operations.len() > MAX_OPERATIONS {
            return Err(IrError {
                operation: None,
                message: "IR requires 2..=1024 operations with a final return".into(),
            });
        }
        let mut definitions = std::collections::BTreeMap::new();
        let mut locals = std::collections::BTreeSet::new();
        let mut uses = std::collections::BTreeMap::new();
        let mut string_values = std::collections::BTreeSet::new();
        let mut string_locals = std::collections::BTreeSet::new();
        for (index, located) in operations.iter().enumerate() {
            let fail = |message: &str| IrError {
                operation: Some(index),
                message: message.into(),
            };
            if located.span.is_some_and(|span| span.start > span.end) {
                return Err(fail("source span ends before it starts"));
            }
            let (result, operands) = match located.operation {
                Operation::Integer { result, value } => {
                    if !(0..=127).contains(&value) {
                        return Err(fail("supported integer literal range is 0..=127"));
                    }
                    (Some(result), vec![])
                }
                Operation::String { result, ref value } => {
                    if value.len() > u16::MAX as usize {
                        return Err(fail("string UTF-8 byte length exceeds u16"));
                    }
                    string_values.insert(result);
                    (Some(result), vec![])
                }
                Operation::Println { value } => (None, vec![value]),
                Operation::Binary {
                    result,
                    left,
                    right,
                    ..
                } => {
                    if string_values.contains(&left) || string_values.contains(&right) {
                        return Err(fail("arithmetic requires integer values"));
                    }
                    (Some(result), vec![left, right])
                }
                Operation::ParameterRead { result, index } => {
                    if index >= parameter_count {
                        return Err(fail("parameter index is outside the function signature"));
                    }
                    (Some(result), vec![])
                }
                Operation::ModuleCall {
                    result,
                    ref module,
                    ref member,
                    ref arguments,
                } => {
                    let valid = |name: &str| {
                        name.bytes()
                            .next()
                            .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
                            && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                    };
                    if !valid(module) || !valid(member) {
                        return Err(fail("invalid module or member name"));
                    }
                    if module == "System" {
                        return Err(fail(
                            "System calls are supported only as println statements",
                        ));
                    }
                    if arguments.iter().any(|value| string_values.contains(value)) {
                        return Err(fail(
                            "user-module calls currently require integer arguments",
                        ));
                    }
                    if arguments.len() > MAX_PARAMETERS {
                        return Err(fail("call exceeds 254-argument limit"));
                    }
                    (Some(result), arguments.clone())
                }
                Operation::LocalInitialize { local, value } => {
                    if string_values.contains(&value) {
                        string_locals.insert(local);
                    }
                    if !locals.insert(local) {
                        return Err(fail("duplicate local initialization"));
                    }
                    if locals.len() > MAX_LOCALS {
                        return Err(fail("function exceeds 255-local limit"));
                    }
                    if parameter_count + locals.len() > MAX_LOCALS {
                        return Err(fail("parameters plus locals exceed 255-slot limit"));
                    }
                    (None, vec![value])
                }
                Operation::LocalRead { result, local } => {
                    if !locals.contains(&local) {
                        return Err(fail(
                            "local read must refer to an earlier initialized local",
                        ));
                    }
                    if string_locals.contains(&local) {
                        string_values.insert(result);
                    }
                    (Some(result), vec![])
                }
                Operation::Return { value } => {
                    if string_values.contains(&value) {
                        return Err(fail("return requires an integer value"));
                    }
                    if index + 1 != operations.len() {
                        return Err(fail("return must be the final operation"));
                    }
                    (None, vec![value])
                }
            };
            for operand in operands {
                if !definitions.contains_key(&operand) {
                    return Err(fail("operand must refer to an earlier defined value"));
                }
                *uses.entry(operand).or_insert(0usize) += 1;
            }
            if let Some(result) = result {
                if definitions.insert(result, index).is_some() {
                    return Err(fail("duplicate value definition"));
                }
            }
        }
        if !matches!(
            operations.last().unwrap().operation,
            Operation::Return { .. }
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
        if operations.iter().any(|located| {
            matches!(
                located.operation,
                Operation::ModuleCall { .. } | Operation::Println { .. }
            )
        }) {
            // Calls make operation order observable. Until a general scheduler
            // exists, accept only the expression order the backend can preserve.
            enum Task {
                Value(ValueId),
                Emit(usize),
            }
            let mut schedule = Vec::new();
            for (index, located) in operations.iter().enumerate() {
                let root = match located.operation {
                    Operation::LocalInitialize { value, .. }
                    | Operation::Return { value }
                    | Operation::Println { value } => value,
                    _ => continue,
                };
                let mut tasks = vec![Task::Value(root)];
                while let Some(task) = tasks.pop() {
                    match task {
                        Task::Emit(at) => schedule.push(at),
                        Task::Value(value) => {
                            let at = definitions[&value];
                            tasks.push(Task::Emit(at));
                            match &operations[at].operation {
                                Operation::Binary { left, right, .. } => {
                                    tasks.extend([Task::Value(*right), Task::Value(*left)])
                                }
                                Operation::ModuleCall { arguments, .. } => tasks.extend(
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
                        "functions with calls require expression evaluation order in IR (expected operation {expected})"
                    ),
                });
            }
        }
        Ok(Self {
            name,
            parameter_count,
            operations,
        })
    }

    /// Lower this IR function into a typed VM assembly program.
    ///
    /// [`backend::lower`] selects instructions and assigns frame slots. Module
    /// and member aliases remain unresolved for later binding. Deduplicated
    /// literal declarations carry content without DATA placement; bind them with
    /// [`string_bindings`] before encoding CODE. Output is not a complete PRG.
    pub fn lower(&self) -> ciq_assembly::Program {
        backend::lower(self)
    }

    /// Lower this function and serialize its VM assembly as `.mcasm` text.
    pub fn assembly(&self) -> String {
        self.lower().to_text()
    }

    /// Serialize this function as provisional `ciq-ir 1` text.
    ///
    /// Retains semantic IDs, operation order, and optional source spans. The
    /// output can be read by [`parse_ir`]; it is not Garmin MIR.
    pub fn to_ir_text(&self) -> String {
        let parameters = if self.parameter_count == 0 {
            String::new()
        } else {
            format!(" parameters={}", self.parameter_count)
        };
        let mut text = format!("ciq-ir 1\n.function {}{parameters}\n", self.name);
        for operation in &self.operations {
            text.push_str(&match operation.operation {
                Operation::Integer { result, value } => format!(".integer %{} {value}", result.0),
                Operation::String { result, ref value } => {
                    format!(".string %{} {value:?}", result.0)
                }
                Operation::Println { value } => format!(".println %{}", value.0),
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
                    },
                    result.0,
                    left.0,
                    right.0
                ),
                Operation::ModuleCall {
                    result,
                    ref module,
                    ref member,
                    ref arguments,
                } => {
                    let arguments = arguments
                        .iter()
                        .map(|value| format!(" %{}", value.0))
                        .collect::<String>();
                    format!(".call %{} {module} {member}{arguments}", result.0)
                }
                Operation::ParameterRead { result, index } => {
                    format!(".parameter %{} {index}", result.0)
                }
                Operation::LocalInitialize { local, value } => {
                    format!(".local ${} %{}", local.0, value.0)
                }
                Operation::LocalRead { result, local } => {
                    format!(".read %{} ${}", result.0, local.0)
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
/// Resolves parameter and local names and retains source spans. Unsupported
/// syntax, unresolved names, and invalid IR produce source diagnostics rather
/// than partial output. VM instruction selection and symbol binding occur later.
pub fn compile(source: &str) -> Result<Function, Diagnostic> {
    let ast = parser::parse(source)?;
    fn emit(
        source: &str,
        expression: parser::Expression,
        locals: &std::collections::BTreeMap<String, LocalId>,
        parameters: &std::collections::BTreeMap<String, usize>,
        operations: &mut Vec<LocatedOperation>,
    ) -> Result<ValueId, Diagnostic> {
        let operation = match expression.kind {
            parser::ExpressionKind::String(value) => Operation::String {
                result: ValueId(operations.len() as u32),
                value,
            },
            parser::ExpressionKind::Integer(value) => Operation::Integer {
                result: ValueId(operations.len() as u32),
                value,
            },
            parser::ExpressionKind::Local(name) => {
                let result = ValueId(operations.len() as u32);
                if let Some(local) = locals.get(&name) {
                    Operation::LocalRead {
                        result,
                        local: *local,
                    }
                } else if let Some(index) = parameters.get(&name) {
                    Operation::ParameterRead {
                        result,
                        index: *index,
                    }
                } else {
                    return Err(Diagnostic::at(
                        source,
                        expression.span.start,
                        format!("unknown or not yet initialized local {name:?}"),
                    ));
                }
            }
            parser::ExpressionKind::ModuleCall {
                module,
                member,
                arguments,
            } => {
                if locals.contains_key(&module) || parameters.contains_key(&module) {
                    return Err(Diagnostic::at(
                        source,
                        expression.span.start,
                        "calls on local receivers are unsupported",
                    ));
                }
                let arguments = arguments
                    .into_iter()
                    .map(|argument| emit(source, argument, locals, parameters, operations))
                    .collect::<Result<Vec<_>, _>>()?;
                Operation::ModuleCall {
                    result: ValueId(operations.len() as u32),
                    module,
                    member,
                    arguments,
                }
            }
            parser::ExpressionKind::Binary { kind, left, right } => {
                let left = emit(source, *left, locals, parameters, operations)?;
                let right = emit(source, *right, locals, parameters, operations)?;
                Operation::Binary {
                    result: ValueId(operations.len() as u32),
                    kind,
                    left,
                    right,
                }
            }
        };
        let result = ValueId(operations.len() as u32);
        operations.push(LocatedOperation {
            operation,
            span: Some(expression.span),
        });
        Ok(result)
    }
    let parameter_count = ast.parameters.len();
    let parameters = ast
        .parameters
        .into_iter()
        .enumerate()
        .map(|(index, name)| (name, index))
        .collect();
    let mut operations = Vec::new();
    let mut locals = std::collections::BTreeMap::new();
    for statement in ast.statements {
        match statement {
            parser::Statement::Local(declaration) => {
                let value = emit(
                    source,
                    declaration.initializer,
                    &locals,
                    &parameters,
                    &mut operations,
                )?;
                let local = LocalId(locals.len() as u32);
                operations.push(LocatedOperation {
                    operation: Operation::LocalInitialize { local, value },
                    span: Some(declaration.span),
                });
                locals.insert(declaration.name, local);
            }
            parser::Statement::Println { argument, span } => {
                if locals.contains_key("System") || parameters.contains_key("System") {
                    return Err(Diagnostic::at(
                        source,
                        span.start,
                        "System receiver is shadowed",
                    ));
                }
                let value = emit(source, argument, &locals, &parameters, &mut operations)?;
                operations.push(LocatedOperation {
                    operation: Operation::Println { value },
                    span: Some(span),
                });
            }
        }
    }
    let value = emit(
        source,
        ast.expression,
        &locals,
        &parameters,
        &mut operations,
    )?;
    operations.push(LocatedOperation {
        operation: Operation::Return { value },
        span: Some(ast.return_span),
    });
    let spans: Vec<_> = operations.iter().map(|op| op.span).collect();
    Function::with_parameters(ast.name, parameter_count, operations).map_err(|error| {
        let offset = error
            .operation
            .and_then(|index| spans.get(index).copied().flatten())
            .map_or(0, |span: Span| span.start);
        Diagnostic::at(source, offset, error.message)
    })
}
