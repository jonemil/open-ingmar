//! Shared ordinary CODE assembly model. See docs/specs/assembly-model.md.
use ciq_mc::{Instruction, Opcode};
use ciq_prg::{
    data::StringDeclaration,
    symbols::source::{Declaration, Source, tokens, valid_alias},
};
use std::{collections::BTreeMap, fmt};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    /// One-based input line, or zero for a constructed in-memory model.
    pub line: usize,
    pub message: String,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "assembly line {}: {}", self.line, self.message)
    }
}
impl std::error::Error for Error {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operand {
    Bits(u64),
    Symbol(String),
    String(String),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    Label(String),
    Instruction {
        opcode: Opcode,
        operands: Vec<Operand>,
    },
    Symbol(Declaration),
    String(StringDeclaration),
}

/// Validated symbolic CODE source; bindings and encoding are later steps.
#[derive(Debug, Clone)]
pub struct Program {
    items: Vec<Item>,
    lines: Vec<usize>,
}
impl PartialEq for Program {
    fn eq(&self, other: &Self) -> bool {
        self.items == other.items
    }
}
impl Eq for Program {}

impl Program {
    pub fn new(items: Vec<Item>) -> Result<Self, Error> {
        let program = Self {
            lines: vec![0; items.len()],
            items,
        };
        program.validate()?;
        Ok(program)
    }
    pub fn items(&self) -> &[Item] {
        &self.items
    }
    fn error(&self, index: usize, message: impl Into<String>) -> Error {
        Error {
            line: self.lines[index],
            message: message.into(),
        }
    }
    pub fn parse(text: &str) -> Result<Self, Error> {
        let mut program = Self {
            items: Vec::new(),
            lines: Vec::new(),
        };
        for (index, line) in text.lines().enumerate() {
            let fail = |message: String| Error {
                line: index + 1,
                message,
            };
            let fields = tokens(line).map_err(|e| fail(e.to_string()))?;
            let Some(first) = fields.first() else {
                continue;
            };
            let item = if first == ".symbol" {
                Item::Symbol(Declaration::parse(&fields).map_err(|e| fail(e.to_string()))?)
            } else if first == ".string" {
                Item::String(StringDeclaration::parse(&fields).map_err(|e| fail(e.to_string()))?)
            } else if let Some(alias) = first.strip_suffix(':') {
                if fields.len() != 1 {
                    return Err(fail("expected label: on its own line".into()));
                }
                Item::Label(alias.into())
            } else {
                let opcode = Opcode::ALL
                    .iter()
                    .copied()
                    .find(|op| op.mnemonic() == first)
                    .ok_or_else(|| fail(format!("unsupported mnemonic {first:?}")))?;
                let operands = fields[1..]
                    .iter()
                    .map(|value| {
                        if let Some(alias) = value.strip_prefix('@') {
                            return Ok(Operand::Symbol(alias.into()));
                        }
                        if let Some(alias) = value.strip_prefix('&') {
                            return Ok(Operand::String(alias.into()));
                        }
                        let hex = value.strip_prefix("0x").ok_or_else(|| {
                            fail(
                                "operands must start with 0x or use a declared @alias or &alias"
                                    .into(),
                            )
                        })?;
                        if hex.is_empty() || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
                            return Err(fail(format!("invalid hexadecimal operand {value:?}")));
                        }
                        u64::from_str_radix(hex, 16)
                            .map(Operand::Bits)
                            .map_err(|_| fail(format!("operand {value:?} is too large")))
                    })
                    .collect::<Result<_, _>>()?;
                Item::Instruction { opcode, operands }
            };
            program.items.push(item);
            program.lines.push(index + 1);
        }
        program.validate()?;
        Ok(program)
    }
    fn validate(&self) -> Result<(), Error> {
        for (index, item) in self.items.iter().enumerate() {
            let fail = |message| self.error(index, message);
            match item {
                Item::Label(alias) if !valid_alias(alias) => {
                    return Err(fail("invalid code label"));
                }
                Item::Symbol(declaration) => {
                    if !valid_alias(&declaration.alias) {
                        return Err(fail("invalid symbol alias"));
                    }
                }
                Item::String(declaration) => {
                    if !valid_alias(&declaration.alias)
                        || declaration.value.len() > u16::MAX as usize
                    {
                        return Err(fail("invalid string declaration"));
                    }
                    match declaration.at {
                        Some(at) if at < 0x1000000 => (),
                        _ => {
                            return Err(fail(
                                "inline code strings require a 24-bit at=0x... placement",
                            ));
                        }
                    }
                }
                Item::Instruction { opcode, operands } => {
                    if operands.len() != opcode.operand_widths().len() {
                        return Err(fail("incorrect instruction operand count"));
                    }
                    for (field, (operand, width)) in
                        operands.iter().zip(opcode.operand_widths()).enumerate()
                    {
                        match operand {
                            Operand::Bits(value) if *value >= (1u64 << (width * 8)) => {
                                return Err(fail("operand exceeds field width"));
                            }
                            Operand::Symbol(alias)
                                if !opcode.symbol_operands().contains(&field)
                                    || !valid_alias(alias) =>
                            {
                                return Err(fail("invalid symbol reference or non-symbol operand"));
                            }
                            Operand::String(alias)
                                if *opcode != Opcode::News || field != 0 || !valid_alias(alias) =>
                            {
                                return Err(fail("invalid string reference or non-string operand"));
                            }
                            _ => (),
                        }
                    }
                }
                _ => (),
            }
        }
        self.labels()?;
        Ok(())
    }
    pub fn labels(&self) -> Result<BTreeMap<String, u32>, Error> {
        let mut labels = BTreeMap::new();
        let mut offset = 0u32;
        for (index, item) in self.items.iter().enumerate() {
            match item {
                Item::Label(alias) => {
                    if labels.insert(alias.clone(), offset).is_some() {
                        return Err(self.error(index, format!("duplicate code label {alias}")));
                    }
                }
                Item::Instruction { opcode, .. } => {
                    offset = offset
                        .checked_add(1 + opcode.operand_widths().iter().sum::<usize>() as u32)
                        .filter(|n| *n <= 0x1000000)
                        .ok_or_else(|| self.error(index, "CODE exceeds 24-bit address space"))?;
                }
                _ => (),
            }
        }
        Ok(labels)
    }
    pub fn to_text(&self) -> String {
        let mut text = String::new();
        for item in &self.items {
            match item {
                Item::Label(alias) => text.push_str(&format!("{alias}:\n")),
                Item::Symbol(d) => text.push_str(
                    &Source {
                        declarations: vec![d.clone()],
                    }
                    .to_text(),
                ),
                Item::String(d) => text.push_str(&d.to_text()),
                Item::Instruction { opcode, operands } => {
                    text.push_str(opcode.mnemonic());
                    for (operand, width) in operands.iter().zip(opcode.operand_widths()) {
                        text.push(' ');
                        match operand {
                            Operand::Bits(value) => {
                                text.push_str(&format!("0x{value:0digits$x}", digits = width * 2))
                            }
                            Operand::Symbol(alias) => text.push_str(&format!("@{alias}")),
                            Operand::String(alias) => text.push_str(&format!("&{alias}")),
                        }
                    }
                    text.push('\n');
                }
            }
        }
        text
    }
    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        self.encode_with_references(&BTreeMap::new(), &BTreeMap::new())
    }
    pub fn encode_with_references(
        &self,
        external: &BTreeMap<String, u32>,
        external_strings: &BTreeMap<String, u32>,
    ) -> Result<Vec<u8>, Error> {
        let mut declarations = Source::default();
        let mut strings = external_strings.clone();
        for (index, item) in self.items.iter().enumerate() {
            match item {
                Item::Symbol(d) => declarations.declarations.push(d.clone()),
                Item::String(d) if strings.insert(d.alias.clone(), d.at.unwrap()).is_some() => {
                    return Err(self.error(index, format!("duplicate string alias {}", d.alias)));
                }
                _ => (),
            }
        }
        let mut bindings = external.clone();
        for (alias, id) in declarations.bindings().map_err(|e| Error {
            line: 1,
            message: e.to_string(),
        })? {
            if bindings.insert(alias.clone(), id).is_some() {
                return Err(Error {
                    line: 1,
                    message: format!("duplicate symbol alias {alias}"),
                });
            }
        }
        let mut code = Vec::new();
        for (index, item) in self.items.iter().enumerate() {
            let Item::Instruction { opcode, operands } = item else {
                continue;
            };
            let mut bytes = Vec::new();
            for (operand, width) in operands.iter().zip(opcode.operand_widths()) {
                let value = match operand {
                    Operand::Bits(value) => *value,
                    Operand::Symbol(alias) => *bindings.get(alias).ok_or_else(|| {
                        self.error(index, format!("undefined symbol alias {alias}"))
                    })? as u64,
                    Operand::String(alias) => *strings.get(alias).ok_or_else(|| {
                        self.error(index, format!("undefined string alias {alias}"))
                    })? as u64,
                };
                if value >= (1u64 << (width * 8)) {
                    return Err(self.error(index, "resolved operand exceeds field width"));
                }
                bytes.extend_from_slice(&value.to_be_bytes()[8 - width..]);
            }
            Instruction::new(*opcode, bytes)
                .map_err(|e| self.error(index, e.to_string()))?
                .encode_into(&mut code);
        }
        Ok(code)
    }
}

pub fn code_labels(text: &str) -> Result<BTreeMap<String, u32>, Error> {
    Program::parse(text)?.labels()
}
pub fn assemble(text: &str) -> Result<Vec<u8>, Error> {
    Program::parse(text)?.encode()
}
pub fn assemble_with_bindings(
    text: &str,
    external: &BTreeMap<String, u32>,
) -> Result<Vec<u8>, Error> {
    assemble_with_references(text, external, &BTreeMap::new())
}
pub fn assemble_with_references(
    text: &str,
    external: &BTreeMap<String, u32>,
    external_strings: &BTreeMap<String, u32>,
) -> Result<Vec<u8>, Error> {
    Program::parse(text)?.encode_with_references(external, external_strings)
}
