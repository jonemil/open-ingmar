//! Initial assembly grammar; see docs/specs/bytecode-subset.md.

use ciq_mc::{Instruction, Opcode};
use ciq_prg::{
    Prg, Section,
    data::{Source as DataSource, StringDeclaration},
    manifest::{Encoding, FILE_NAME, Manifest},
    symbols::source::{Declaration, Source, tokens},
};
use std::fmt;
use std::{error::Error as StdError, fs, path::Path};

/// Reconstruct all sections without reading an original PRG template.
pub fn assemble_project(directory: &Path) -> Result<Prg, Box<dyn StdError>> {
    let directory = directory.canonicalize()?;
    let manifest_path = directory.join(FILE_NAME).canonicalize()?;
    if !manifest_path.starts_with(&directory) {
        return Err("manifest resolves outside the project directory".into());
    }
    let manifest = Manifest::parse(&fs::read_to_string(manifest_path)?)?;
    // Read all inputs first: code can refer to declarations in a later section.
    if manifest
        .entries
        .iter()
        .any(|e| e.encoding == Encoding::Symbols)
        && manifest
            .entries
            .iter()
            .filter(|e| e.kind == ciq_prg::SYMBOLS)
            .count()
            != 1
    {
        return Err("structured symbols require a single symbol section".into());
    }
    if manifest
        .entries
        .iter()
        .any(|e| e.encoding == Encoding::Data)
        && manifest
            .entries
            .iter()
            .filter(|e| e.kind == ciq_prg::DATA)
            .count()
            != 1
    {
        return Err("structured DATA requires a single DATA section".into());
    }
    let mut data_text = None;
    let mut inputs = Vec::new();
    let mut symbol_source = Source::default();
    for entry in &manifest.entries {
        let path = directory.join(&entry.file).canonicalize()?;
        if !path.starts_with(&directory) {
            return Err(format!("{} resolves outside the project directory", entry.file).into());
        }
        let bytes = fs::read(path)?;
        if entry.encoding == Encoding::Symbols {
            symbol_source = Source::parse(std::str::from_utf8(&bytes)?)?;
        }
        if entry.encoding == Encoding::Data {
            data_text = Some(std::str::from_utf8(&bytes)?.to_owned());
        }
        inputs.push(bytes);
    }
    let bindings = symbol_source.bindings()?;
    let mut labels = std::collections::BTreeMap::new();
    for (entry, bytes) in manifest.entries.iter().zip(&inputs) {
        if entry.encoding == Encoding::Assembly {
            for (alias, offset) in code_labels(std::str::from_utf8(bytes)?)? {
                if labels.insert(alias.clone(), offset).is_some() {
                    return Err(format!("duplicate project code label {alias}").into());
                }
            }
        }
    }
    if !labels.is_empty()
        && manifest
            .entries
            .iter()
            .filter(|e| e.kind == ciq_prg::CODE)
            .count()
            != 1
    {
        return Err("code labels require a unique CODE section".into());
    }
    let data_source = data_text
        .map(|text| DataSource::parse_with_references(&text, &bindings, &labels))
        .transpose()?
        .unwrap_or_default();
    let (data_payload, strings) = data_source.encode_with_references(&bindings, &labels)?;
    let mut sections = Vec::new();
    for (entry, bytes) in manifest.entries.iter().zip(inputs) {
        let payload = match entry.encoding {
            Encoding::Raw => bytes,
            Encoding::Assembly => {
                let text = std::str::from_utf8(&bytes)?;
                for line in text.lines() {
                    if tokens(line)?.first().is_some_and(|f| f == ".string") {
                        return Err(
                            "project string declarations belong in the authoritative DATA source"
                                .into(),
                        );
                    }
                }
                assemble_with_references(text, &bindings, &strings)?
            }
            Encoding::Symbols => symbol_source.encode()?,
            Encoding::Data => data_payload.clone(),
        };
        if payload.len() != entry.length as usize {
            return Err(format!(
                "{} produced {} bytes; manifest requires {} (relocation is unsupported)",
                entry.file,
                payload.len(),
                entry.length
            )
            .into());
        }
        sections.push(Section::new(entry.kind, payload)?);
    }
    let prg = Prg::from_sections(sections);
    if !labels.is_empty() {
        let length = prg.unique_section(ciq_prg::CODE)?.payload().len();
        for fragment in &data_source.fragments {
            if let ciq_prg::data::Fragment::Class(c) = fragment {
                for field in &c.fields {
                    if let ciq_prg::data::classes::source::Reference::Code(alias) = &field.value {
                        if labels[alias] as usize >= length {
                            return Err(format!(
                                "method code label {alias} points past the final instruction"
                            )
                            .into());
                        }
                    }
                }
            }
        }
    }
    Ok(prg)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "assembly line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for Error {}

/// Instruction-aligned CODE labels; labels must occupy their own source line.
pub fn code_labels(text: &str) -> Result<std::collections::BTreeMap<String, u32>, Error> {
    let mut labels = std::collections::BTreeMap::new();
    let mut offset = 0u32;
    for (index, line) in text.lines().enumerate() {
        let fail = |message: String| Error {
            line: index + 1,
            message,
        };
        let fields = tokens(line).map_err(|e| fail(e.to_string()))?;
        if fields.is_empty() || matches!(fields[0].as_str(), ".symbol" | ".string") {
            continue;
        }
        if let Some(alias) = fields[0].strip_suffix(':') {
            if fields.len() != 1 || !ciq_prg::symbols::source::valid_alias(alias) {
                return Err(fail("expected label: on its own line".into()));
            }
            if labels.insert(alias.into(), offset).is_some() {
                return Err(fail(format!("duplicate code label {alias}")));
            }
            continue;
        }
        let opcode = Opcode::ALL
            .iter()
            .find(|op| op.mnemonic() == fields[0])
            .ok_or_else(|| fail(format!("unsupported mnemonic {:?}", fields[0])))?;
        if fields.len() - 1 != opcode.operand_widths().len() {
            return Err(fail("incorrect instruction operand count".into()));
        }
        offset = offset
            .checked_add(1 + opcode.operand_widths().iter().sum::<usize>() as u32)
            .filter(|n| *n <= 0x1000000)
            .ok_or_else(|| fail("CODE exceeds 24-bit address space".into()))?;
    }
    Ok(labels)
}

pub fn assemble(text: &str) -> Result<Vec<u8>, Error> {
    assemble_with_bindings(text, &std::collections::BTreeMap::new())
}

pub fn assemble_with_bindings(
    text: &str,
    external: &std::collections::BTreeMap<String, u32>,
) -> Result<Vec<u8>, Error> {
    assemble_with_references(text, external, &std::collections::BTreeMap::new())
}

pub fn assemble_with_references(
    text: &str,
    external: &std::collections::BTreeMap<String, u32>,
    external_strings: &std::collections::BTreeMap<String, u32>,
) -> Result<Vec<u8>, Error> {
    code_labels(text)?;
    let fail = |line, message: String| Error { line, message };
    let mut lines = Vec::new();
    let mut declarations = Source::default();
    let mut strings = external_strings.clone();
    for (index, line) in text.lines().enumerate() {
        let fields = tokens(line).map_err(|e| fail(index + 1, e.to_string()))?;
        if fields.first().is_some_and(|f| f == ".symbol") {
            declarations
                .declarations
                .push(Declaration::parse(&fields).map_err(|e| fail(index + 1, e.to_string()))?);
        } else if fields.first().is_some_and(|f| f == ".string") {
            let declaration =
                StringDeclaration::parse(&fields).map_err(|e| fail(index + 1, e.to_string()))?;
            let at = declaration.at.ok_or_else(|| {
                fail(
                    index + 1,
                    "inline code strings require at=0x...; automatic layout belongs in DATA source"
                        .into(),
                )
            })?;
            if strings.insert(declaration.alias.clone(), at).is_some() {
                return Err(fail(
                    index + 1,
                    format!("duplicate string alias {}", declaration.alias),
                ));
            }
        } else if fields.len() == 1 && fields[0].ends_with(':') {
            // Validated and accounted for by code_labels; labels emit no bytes.
        } else if !fields.is_empty() {
            lines.push((index + 1, fields));
        }
    }
    let mut bindings = external.clone();
    for (alias, id) in declarations
        .bindings()
        .map_err(|e| fail(1, e.to_string()))?
    {
        if bindings.insert(alias.clone(), id).is_some() {
            return Err(fail(1, format!("duplicate symbol alias {alias}")));
        }
    }
    let mut code = Vec::new();
    for (line, fields) in lines {
        let mnemonic = &fields[0];
        let opcode = Opcode::ALL
            .iter()
            .copied()
            .find(|opcode| opcode.mnemonic() == mnemonic)
            .ok_or_else(|| fail(line, format!("unsupported mnemonic {mnemonic:?}")))?;
        let values = &fields[1..];
        if values.len() != opcode.operand_widths().len() {
            return Err(fail(
                line,
                format!(
                    "{} requires {} operand fields",
                    mnemonic,
                    opcode.operand_widths().len()
                ),
            ));
        }
        let mut operands = Vec::new();
        for (field, (value, &width)) in values.iter().zip(opcode.operand_widths()).enumerate() {
            let number = if let Some(alias) = value.strip_prefix('@') {
                if !opcode.symbol_operands().contains(&field) {
                    return Err(fail(
                        line,
                        "symbol reference used in a non-symbol operand".into(),
                    ));
                }
                *bindings
                    .get(alias)
                    .ok_or_else(|| fail(line, format!("undefined symbol alias {alias}")))?
                    as u64
            } else if let Some(alias) = value.strip_prefix('&') {
                if opcode != Opcode::News || field != 0 {
                    return Err(fail(
                        line,
                        "string reference used in a non-string operand".into(),
                    ));
                }
                *strings
                    .get(alias)
                    .ok_or_else(|| fail(line, format!("undefined string alias {alias}")))?
                    as u64
            } else {
                let hex = value.strip_prefix("0x").ok_or_else(|| {
                    fail(
                        line,
                        "operands must start with 0x or use a declared @alias or &alias".into(),
                    )
                })?;
                if hex.is_empty() || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                    return Err(fail(line, format!("invalid hexadecimal operand {value:?}")));
                }
                u64::from_str_radix(hex, 16)
                    .map_err(|_| fail(line, format!("operand {value:?} is too large")))?
            };
            if number >= (1u64 << (width * 8)) {
                return Err(fail(
                    line,
                    format!("operand {value:?} exceeds {width}-byte field"),
                ));
            }
            operands.extend_from_slice(&number.to_be_bytes()[8 - width..]);
        }
        Instruction::new(opcode, operands)
            .map_err(|e| fail(line, e.to_string()))?
            .encode_into(&mut code);
    }
    Ok(code)
}

/// Apply same-length code and inline string edits to an existing template.
pub fn assemble_template(mut prg: Prg, text: &str) -> Result<Prg, Box<dyn StdError>> {
    let code = assemble(text)?;
    let mut patches = Vec::new();
    for line in text.lines() {
        let fields = tokens(line)?;
        if fields.first().is_some_and(|f| f == ".string") {
            patches.push(StringDeclaration::parse(&fields)?);
        }
    }
    if !patches.is_empty() {
        let mut payload = prg.unique_section(ciq_prg::DATA)?.payload().to_vec();
        let mut ranges = Vec::new();
        for declaration in patches {
            let at = declaration.at.ok_or("inline string lacks placement")?;
            let (old, end) = ciq_prg::data::string_at(&payload, at)?;
            if old.len() != declaration.value.len() {
                return Err(
                    "inline string edit changes record length; relocation is unsupported".into(),
                );
            }
            if ranges
                .iter()
                .any(|&(start, previous_end)| (at as usize) < previous_end && end > start)
            {
                return Err("overlapping inline string declarations".into());
            }
            ranges.push((at as usize, end));
            payload[at as usize + 3..end - 1].copy_from_slice(declaration.value.as_bytes());
        }
        prg.replace_same_length(ciq_prg::DATA, &payload)?;
    }
    prg.replace_same_length(ciq_prg::CODE, &code)?;
    Ok(prg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comments_and_hex_fields_preserve_bit_patterns() {
        let code = assemble("# hello\nargc 0x1\ngetmv 0x00800005 0x00800067 # System.println\ninvokem 0x02\nreturn\n").unwrap();
        assert_eq!(
            code,
            [
                0x35, 1, 0x45, 0, 0x80, 0, 5, 0, 0x80, 0, 0x67, 0x0f, 2, 0x16
            ]
        );
    }

    #[test]
    fn errors_identify_line_and_reject_invalid_operands() {
        for source in [
            "bogus",
            "return 0x1",
            "argc",
            "argc 0x100",
            "argc -1",
            "argc 1",
            "argc 0x",
            "argc 0x+1",
            "getmv 0x1",
        ] {
            let error = assemble(&format!("# comment\n{source}")).unwrap_err();
            assert_eq!(error.line, 2, "{source}");
        }
    }
}

#[cfg(test)]
mod symbolic_tests {
    use super::*;

    #[test]
    fn forward_symbol_references_and_optional_commas_resolve_to_ids() {
        let source = "getmv @system, @println\ngetlocalv 0x01 @println\n.symbol system \"Toybox_System\" id=0x00800005\n.symbol println \"println\" id=0x00800067\n";
        assert_eq!(
            assemble(source).unwrap(),
            assemble("getmv 0x00800005 0x00800067\ngetlocalv 0x01 0x00800067").unwrap()
        );
    }

    #[test]
    fn references_are_typed_and_undefined_or_duplicate_aliases_fail() {
        for text in [
            "news @s",
            "argc @s",
            "getlocalv @s 0x1",
            "getselfv @missing",
        ] {
            let error = assemble(&format!(".symbol s \"s\" id=0x1\n{text}")).unwrap_err();
            assert_eq!(error.line, 2);
        }
        assert!(assemble(".symbol s \"a\" id=0x1\n.symbol s \"b\" id=0x2").is_err());
    }
}
