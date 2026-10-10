//! Shared ordinary CODE assembly model.
//!
//! Parses, prints, and encodes ordinary CODE assembly through [`Program`].
//! Text parsing and typed construction use the same validation. Encoding
//! resolves symbol/string aliases and relative CODE labels into instruction bytes; PRG
//! reconstruction and signing belong to the assembler tool.
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
    /// A local CODE label resolved to a signed relative instruction operand.
    Code(String),
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

/// A validated CODE assembly program containing instructions and declarations.
///
/// Symbol/string aliases and CODE labels resolve during encoding. Equality compares
/// program content and ignores source line numbers.
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
    /// Construct and validate a CODE assembly program from typed items.
    ///
    /// Applies the same operand and layout checks as [`Self::parse`]. Symbol,
    /// string, and CODE references may remain unresolved until encoding. Errors use line
    /// zero because the items have no source line numbers.
    pub fn new(items: Vec<Item>) -> Result<Self, Error> {
        let program = Self {
            lines: vec![0; items.len()],
            items,
        };
        program.validate()?;
        Ok(program)
    }
    /// Return instructions and declarations in source order.
    pub fn items(&self) -> &[Item] {
        &self.items
    }
    fn error(&self, index: usize, message: impl Into<String>) -> Error {
        Error {
            line: self.lines[index],
            message: message.into(),
        }
    }
    /// Parse CODE `.mcasm` text into a validated assembly program.
    ///
    /// Rejects unsupported syntax and invalid operands or layout. Symbol, string,
    /// and CODE references are retained for binding. Errors identify the input
    /// line; comments and whitespace are not retained in the program.
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
                        if let Some(alias) = value.strip_prefix("code:") {
                            return Ok(Operand::Code(alias.into()));
                        }
                        let hex = value.strip_prefix("0x").ok_or_else(|| {
                            fail("operands must use 0x, @alias, &alias, or code:label".into())
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
        let mut string_aliases = std::collections::BTreeSet::new();
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
                    if !string_aliases.insert(&declaration.alias) {
                        return Err(fail("duplicate string alias"));
                    }
                    if !valid_alias(&declaration.alias)
                        || declaration.value.len() > u16::MAX as usize
                    {
                        return Err(fail("invalid string declaration"));
                    }
                    match declaration.at {
                        None => (),
                        Some(at) if at < 0x1000000 => (),
                        _ => {
                            return Err(fail("inline code string placement exceeds 24 bits"));
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
                            Operand::Code(alias)
                                if !opcode.has_relative_target()
                                    || field != 0
                                    || !valid_alias(alias) =>
                            {
                                return Err(fail("invalid CODE reference or non-relative operand"));
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
    /// Calculate each label's byte offset in the encoded CODE payload.
    ///
    /// Rejects duplicate labels and layouts exceeding the 24-bit address space.
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
    /// Serialize the program as canonical CODE `.mcasm` text.
    ///
    /// Retains instructions, declarations, aliases, and placement constraints.
    /// Original comments and whitespace are not reproduced.
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
                            Operand::Code(alias) => text.push_str(&format!("code:{alias}")),
                        }
                    }
                    text.push('\n');
                }
            }
        }
        text
    }
    /// Encode the program into raw CODE bytes using its inline declarations.
    ///
    /// Unresolved aliases are errors. Use [`Self::encode_with_references`] to
    /// supply external bindings. Output contains no SYMBOLS/DATA sections or
    /// container signatures.
    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        self.encode_with_references(&BTreeMap::new(), &BTreeMap::new())
    }
    /// Encode raw CODE bytes using inline declarations and external alias maps.
    ///
    /// The maps bind symbol aliases to IDs and string aliases to DATA offsets.
    /// Missing or duplicate aliases and values exceeding operand widths are
    /// errors. Output contains instructions only, not symbol or string records.
    /// Unplaced literal declarations require an external binding when referenced;
    /// their content must be materialized separately in DATA by the caller.
    /// CODE labels resolve locally to instruction boundaries. Missing labels,
    /// end-of-CODE targets, and signed 16-bit displacement overflow are errors;
    /// numeric relative operands preserve their bits without target validation.
    pub fn encode_with_references(
        &self,
        external: &BTreeMap<String, u32>,
        external_strings: &BTreeMap<String, u32>,
    ) -> Result<Vec<u8>, Error> {
        let labels = self.labels()?;
        let code_length: usize = self
            .items
            .iter()
            .map(|item| match item {
                Item::Instruction { opcode, .. } => {
                    1 + opcode.operand_widths().iter().sum::<usize>()
                }
                _ => 0,
            })
            .sum();
        let mut declarations = Source::default();
        let mut strings = external_strings.clone();
        for (index, item) in self.items.iter().enumerate() {
            match item {
                Item::Symbol(d) => declarations.declarations.push(d.clone()),
                Item::String(d) => {
                    if let Some(at) = d.at {
                        if strings.insert(d.alias.clone(), at).is_some() {
                            return Err(
                                self.error(index, format!("duplicate string alias {}", d.alias))
                            );
                        }
                    }
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
                    Operand::Code(alias) => {
                        let target = *labels.get(alias).ok_or_else(|| {
                            self.error(index, format!("undefined code label {alias}"))
                        })?;
                        if target as usize >= code_length {
                            return Err(self.error(index, "CODE target must denote an instruction"));
                        }
                        let displacement = i64::from(target) - code.len() as i64 - 3;
                        let displacement = i16::try_from(displacement).map_err(|_| {
                            self.error(index, "relative CODE displacement exceeds signed 16 bits")
                        })?;
                        displacement as u16 as u64
                    }
                    Operand::Symbol(alias) => *bindings.get(alias).ok_or_else(|| {
                        self.error(index, format!("undefined symbol alias {alias}"))
                    })? as u64,
                    Operand::String(alias) => {
                        let at = *strings.get(alias).ok_or_else(|| {
                            self.error(index, format!("undefined string alias {alias}"))
                        })?;
                        if at >= 0x1000000 {
                            return Err(
                                self.error(index, "resolved string placement exceeds 24 bits")
                            );
                        }
                        at as u64
                    }
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
