//! Restricted Monkey C frontend. See docs/specs/compiler-subset.md.

mod ir_text;
mod parser;
pub use ir_text::parse_ir;

use ciq_mc::Opcode;
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValueId(pub u32);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operation {
    Integer { result: ValueId, value: i32 },
    Return { value: ValueId },
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
        for (index, operation) in operations.iter().enumerate() {
            if operation.span.is_some_and(|span| span.start > span.end) {
                return Err(IrError {
                    operation: Some(index),
                    message: "source span ends before it starts".into(),
                });
            }
        }
        let [constant, returned] = operations.as_slice() else {
            return Err(IrError {
                operation: None,
                message: "supported IR requires one integer definition followed by one return"
                    .into(),
            });
        };
        let Operation::Integer { result, value } = constant.operation else {
            return Err(IrError {
                operation: Some(0),
                message: "first operation must define an integer".into(),
            });
        };
        if !(0..=127).contains(&value) {
            return Err(IrError {
                operation: Some(0),
                message: "supported integer range is 0..=127".into(),
            });
        }
        if returned.operation != (Operation::Return { value: result }) {
            return Err(IrError {
                operation: Some(1),
                message: "return must use the defined integer value".into(),
            });
        }
        Ok(Self { name, operations })
    }

    /// Lower directly to the shared typed assembly model, without printing/parsing.
    pub fn lower(&self) -> ciq_assembly::Program {
        use ciq_assembly::{Item, Operand, Program};
        let mut items = vec![
            Item::Label(self.name.clone()),
            Item::Instruction {
                opcode: Opcode::Argc,
                operands: vec![Operand::Bits(1)],
            },
        ];
        for operation in &self.operations {
            let (opcode, operands) = match operation.operation {
                Operation::Integer { value: 0, .. } => (Opcode::Ipushz, vec![]),
                Operation::Integer { value, .. } => {
                    (Opcode::Ipush1, vec![Operand::Bits(value as u64)])
                }
                Operation::Return { .. } => (Opcode::Return, vec![]),
            };
            items.push(Item::Instruction { opcode, operands });
        }
        Program::new(items).expect("validated constant-return IR lowers to valid assembly")
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
    let value = ValueId(0);
    Function::new(
        ast.name,
        vec![
            LocatedOperation {
                operation: Operation::Integer {
                    result: value,
                    value: ast.integer,
                },
                span: Some(ast.integer_span),
            },
            LocatedOperation {
                operation: Operation::Return { value },
                span: Some(ast.return_span),
            },
        ],
    )
    .map_err(|error| Diagnostic::at(source, 0, error.message))
}
