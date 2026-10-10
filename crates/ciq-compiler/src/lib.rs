//! Restricted Monkey C frontend. See docs/specs/compiler-subset.md.

pub mod backend;
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
    Binary {
        result: ValueId,
        kind: BinaryOp,
        left: ValueId,
        right: ValueId,
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

/// Validated subset IR, constructible from source, text, or typed operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    name: String,
    operations: Vec<LocatedOperation>,
}

impl Function {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn operations(&self) -> &[LocatedOperation] {
        &self.operations
    }

    /// Construct IR through the same validation used by its text reader.
    pub fn new(name: String, operations: Vec<LocatedOperation>) -> Result<Self, IrError> {
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
                Operation::Binary {
                    result,
                    left,
                    right,
                    ..
                } => (Some(result), vec![left, right]),
                Operation::LocalInitialize { local, value } => {
                    if !locals.insert(local) {
                        return Err(fail("duplicate local initialization"));
                    }
                    if locals.len() > MAX_LOCALS {
                        return Err(fail("function exceeds 255-local limit"));
                    }
                    (None, vec![value])
                }
                Operation::LocalRead { result, local } => {
                    if !locals.contains(&local) {
                        return Err(fail(
                            "local read must refer to an earlier initialized local",
                        ));
                    }
                    (Some(result), vec![])
                }
                Operation::Return { value } => {
                    if index + 1 != operations.len() {
                        return Err(fail("return must be the final operation"));
                    }
                    (None, vec![value])
                }
            };
            for operand in operands {
                if !definitions.contains_key(&operand) {
                    return Err(fail(
                        "operand must refer to an earlier defined integer value",
                    ));
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
        for (value, index) in definitions {
            if uses.get(&value) != Some(&1) {
                return Err(IrError {
                    operation: Some(index),
                    message: "supported expression IR requires exactly one use per defined value"
                        .into(),
                });
            }
        }
        Ok(Self { name, operations })
    }

    /// Lower directly to the shared typed assembly model, without printing/parsing.
    pub fn lower(&self) -> ciq_assembly::Program {
        backend::lower(self)
    }

    pub fn assembly(&self) -> String {
        self.lower().to_text()
    }

    /// Provisional, versioned text representation of the semantic IR.
    pub fn to_ir_text(&self) -> String {
        let mut text = format!("ciq-ir 1\n.function {}\n", self.name);
        for operation in &self.operations {
            text.push_str(&match operation.operation {
                Operation::Integer { result, value } => format!(".integer %{} {value}", result.0),
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

pub fn compile(source: &str) -> Result<Function, Diagnostic> {
    let ast = parser::parse(source)?;
    fn emit(
        source: &str,
        expression: parser::Expression,
        locals: &std::collections::BTreeMap<String, LocalId>,
        operations: &mut Vec<LocatedOperation>,
    ) -> Result<ValueId, Diagnostic> {
        let operation = match expression.kind {
            parser::ExpressionKind::Integer(value) => Operation::Integer {
                result: ValueId(operations.len() as u32),
                value,
            },
            parser::ExpressionKind::Local(name) => Operation::LocalRead {
                result: ValueId(operations.len() as u32),
                local: *locals.get(&name).ok_or_else(|| {
                    Diagnostic::at(
                        source,
                        expression.span.start,
                        format!("unknown or not yet initialized local {name:?}"),
                    )
                })?,
            },
            parser::ExpressionKind::Binary { kind, left, right } => {
                let left = emit(source, *left, locals, operations)?;
                let right = emit(source, *right, locals, operations)?;
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
    let mut operations = Vec::new();
    let mut locals = std::collections::BTreeMap::new();
    for declaration in ast.locals {
        let value = emit(source, declaration.initializer, &locals, &mut operations)?;
        let local = LocalId(locals.len() as u32);
        operations.push(LocatedOperation {
            operation: Operation::LocalInitialize { local, value },
            span: Some(declaration.span),
        });
        locals.insert(declaration.name, local);
    }
    let value = emit(source, ast.expression, &locals, &mut operations)?;
    operations.push(LocatedOperation {
        operation: Operation::Return { value },
        span: Some(ast.return_span),
    });
    Function::new(ast.name, operations).map_err(|error| Diagnostic::at(source, 0, error.message))
}
