//! Restricted Monkey C frontend. See docs/specs/compiler-subset.md.

mod parser;

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
    pub span: Span,
}

/// Validated subset IR. Construction is restricted to the frontend for now.
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

    /// Lower semantic operations to the shared readable assembly language.
    pub fn assembly(&self) -> String {
        let mut text = format!("{}:\n{} 0x01\n", self.name, Opcode::Argc.mnemonic());
        // The initial frontend creates one value, immediately consumed by return.
        // General scheduling, local allocation and control-flow joins are deferred.
        for operation in &self.operations {
            match operation.operation {
                Operation::Integer { value: 0, .. } => {
                    text.push_str(Opcode::Ipushz.mnemonic());
                    text.push('\n');
                }
                Operation::Integer { value, .. } => {
                    text.push_str(&format!("{} 0x{value:02x}\n", Opcode::Ipush1.mnemonic()));
                }
                Operation::Return { .. } => {
                    text.push_str(Opcode::Return.mnemonic());
                    text.push('\n');
                }
            }
        }
        text
    }
}

pub fn compile(source: &str) -> Result<Function, Diagnostic> {
    let ast = parser::parse(source)?;
    let value = ValueId(0);
    Ok(Function {
        name: ast.name,
        operations: vec![
            LocatedOperation {
                operation: Operation::Integer {
                    result: value,
                    value: ast.integer,
                },
                span: ast.integer_span,
            },
            LocatedOperation {
                operation: Operation::Return { value },
                span: ast.return_span,
            },
        ],
    })
}
