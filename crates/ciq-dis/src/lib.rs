//! Text output for the supported ordinary bytecode subset.

use ciq_mc::{Error, decode};
use ciq_prg::{
    CODE, DATA, Prg, SYMBOLS,
    data::{Fragment, Source as DataSource, StringDeclaration},
    manifest::{Encoding, Entry, FILE_NAME, Manifest},
    symbols::{Symbols, source::Source},
};
use std::fmt::Write;
use std::{error::Error as StdError, fs, path::Path};

/// Export every section, falling back to raw bytes for unsupported code.
pub fn export_project(prg: &Prg, directory: &Path) -> Result<Manifest, Box<dyn StdError>> {
    let symbols = optional_symbols(prg);
    // Only use structured symbols when declarations can reproduce the complete payload.
    let structured = symbols.as_ref().and_then(|table| {
        let source = Source::from_symbols(table);
        let bytes = source.encode().ok()?;
        (bytes == prg.unique_section(SYMBOLS).ok()?.payload()).then_some(source)
    });
    let mut data = referenced_data(prg);
    if let Some(source) = data.as_mut() {
        source.name_classes(symbols.as_ref(), structured.is_some())?;
        let bindings = structured
            .as_ref()
            .map(Source::bindings)
            .transpose()?
            .unwrap_or_default();
        if source.encode_with_symbols(&bindings)?.0 != prg.unique_section(DATA)?.payload() {
            return Err("named DATA export changed bytes".into());
        }
    }
    let strings = data.as_ref().map(string_declarations).unwrap_or_default();
    let mut entries = Vec::new();
    let mut files = Vec::new();
    let mut comments = Vec::new();
    let mut offset = 0;
    for (index, section) in prg.sections().iter().enumerate() {
        let name = section.name().unwrap_or("UNKNOWN");
        let (encoding, payload) = if section.kind() == CODE {
            match disassemble_named(
                section.payload(),
                symbols.as_ref(),
                structured.is_some(),
                &strings,
            ) {
                Ok(text) => (Encoding::Assembly, text.into_bytes()),
                Err(error) => {
                    eprintln!("Section {index} CODE exported as raw binary: {error}");
                    (Encoding::Raw, section.payload().to_vec())
                }
            }
        } else {
            match (section.kind(), data.as_ref(), structured.as_ref()) {
                (DATA, Some(source), _) => (Encoding::Data, source.to_text().into_bytes()),
                (SYMBOLS, _, Some(source)) => (Encoding::Symbols, source.to_text().into_bytes()),
                _ => (Encoding::Raw, section.payload().to_vec()),
            }
        };
        let extension = match encoding {
            Encoding::Raw => "bin",
            Encoding::Assembly | Encoding::Symbols | Encoding::Data => "mcasm",
        };
        let file = format!(
            "{index:03}-{}-{:08x}.{extension}",
            name.to_ascii_lowercase(),
            section.kind()
        );
        comments.push(format!("# {name}, original file offset 0x{offset:08x}\n"));
        entries.push(Entry {
            kind: section.kind(),
            length: section.payload().len() as u32,
            encoding,
            file: file.clone(),
        });
        files.push((file, payload));
        if section.kind() == DATA {
            if let Ok(listing) = classes(prg) {
                files.push((format!("{index:03}-data.classes.txt"), listing.into_bytes()));
            }
        }
        if section.kind() == SYMBOLS {
            if let Ok(table) = Symbols::parse(section.payload()) {
                let mut listing = String::from(
                    "# Inspection only: reassembly consumes the manifest-selected symbol source.\n",
                );
                listing.push_str(&format_symbols(&table));
                files.push((
                    format!("{index:03}-symbols.symbols.txt"),
                    listing.into_bytes(),
                ));
            }
        }
        offset += 8 + section.payload().len();
    }
    let manifest = Manifest { entries };
    // Validate and prepare everything before creating the destination.
    let manifest_text = manifest.to_text();
    Manifest::parse(&manifest_text)?;
    let mut annotated = String::new();
    let mut comments = comments.into_iter();
    for line in manifest_text.lines() {
        if line.starts_with("section ") {
            annotated.push_str(&comments.next().unwrap());
        }
        writeln!(annotated, "{line}").unwrap();
    }
    // Refuse an existing directory, avoiding overwritten projects or stale files.
    fs::create_dir(directory)?;
    for (file, payload) in files {
        fs::write(directory.join(file), payload)?;
    }
    let temporary_manifest = directory.join(".sections.ciq.tmp");
    fs::write(&temporary_manifest, annotated)?;
    fs::rename(temporary_manifest, directory.join(FILE_NAME))?;
    Ok(manifest)
}

fn string_declarations(source: &DataSource) -> std::collections::BTreeMap<u32, StringDeclaration> {
    source
        .fragments
        .iter()
        .filter_map(|f| match f {
            Fragment::String(s) => Some((s.at.unwrap(), s.clone())),
            _ => None,
        })
        .collect()
}

fn referenced_data(prg: &Prg) -> Option<DataSource> {
    let payload = prg.unique_section(DATA).ok()?.payload();
    if let Ok(source) = DataSource::from_records(payload) {
        return Some(source);
    }
    let mut addresses: std::collections::BTreeSet<u32> = match ciq_prg::data::record_string_offsets(
        payload,
    ) {
        Ok(offsets) => offsets.into_iter().collect(),
        Err(error) => {
            eprintln!(
                "Complete DATA record walk unavailable; using supported news references: {error}"
            );
            std::collections::BTreeSet::new()
        }
    };
    if let Ok(section) = prg.unique_section(CODE) {
        if let Ok(code) = decode(section.payload()) {
            for located in code {
                if located.instruction.opcode() == ciq_mc::Opcode::News {
                    let address =
                        u32::from_be_bytes(located.instruction.operands().try_into().unwrap());
                    match ciq_prg::data::string_at(payload, address) {
                        Ok(_) => {
                            addresses.insert(address);
                        }
                        Err(error) => eprintln!(
                            "news at CODE+0x{:06x} retained numerically: {error}",
                            located.offset
                        ),
                    }
                }
            }
        }
    }
    if addresses.is_empty() {
        return None;
    }
    match DataSource::from_references(payload, addresses) {
        Ok(source) => Some(source),
        Err(error) => {
            eprintln!("DATA retained as raw binary: {error}");
            None
        }
    }
}

pub fn data_source(prg: &Prg) -> Result<String, Box<dyn StdError>> {
    let mut source = referenced_data(prg).ok_or("no supported, unambiguous DATA source")?;
    // Standalone DATA retains numeric symbol IDs; project source shares SYMBOLS bindings.
    source.name_classes(optional_symbols(prg).as_ref(), false)?;
    Ok(source.to_text())
}

/// Inspect class/module records; the listing never becomes assembly input.
pub fn classes(prg: &Prg) -> Result<String, Box<dyn StdError>> {
    use ciq_prg::data::classes::{Record, records};
    use std::collections::{BTreeMap, BTreeSet};
    let payload = prg.unique_section(DATA)?.payload();
    let records = records(payload)?;
    let symbols = optional_symbols(prg);
    let strings: BTreeMap<_, _> = records
        .iter()
        .filter_map(|record| match record {
            Record::String { offset, value } => Some((*offset, *value)),
            _ => None,
        })
        .collect();
    let class_offsets: BTreeSet<_> = records
        .iter()
        .filter_map(|record| match record {
            Record::Class(c) => Some(c.offset),
            _ => None,
        })
        .collect();
    let boundaries = prg
        .unique_section(CODE)
        .ok()
        .and_then(|section| decode(section.payload()).ok())
        .map(|code| {
            code.into_iter()
                .map(|instruction| instruction.offset)
                .collect::<BTreeSet<_>>()
        });
    let mut text = String::from(
        "# Inspection only: edit the manifest-selected DATA assembly to change records.\n",
    );
    for record in &records {
        let Record::Class(c) = record else { continue };
        writeln!(
            text,
            "DATA+0x{:08x} class-v{} bytes={} presence={} app_flags=0x{:04x} fields={}",
            c.offset,
            c.version,
            c.raw.len(),
            c.presence
                .map(|f| format!("0x{f:02x}"))
                .unwrap_or_else(|| "fixed".into()),
            c.app_flags,
            c.fields.len()
        )?;
        for (label, value) in [
            ("extends", c.extends),
            ("statics", c.statics),
            ("parent", c.parent),
            ("module", c.module),
        ] {
            if let Some(value) = value {
                write!(text, "  {label}=0x{value:08x}")?;
                // Only these two header roles have symbol-ID evidence in our specimens.
                if matches!(label, "parent" | "module") {
                    if let Some(symbol) = symbols.as_ref().and_then(|table| table.get(value)) {
                        write!(text, " {:?}", symbol.name)?;
                    }
                }
                text.push('\n');
            }
        }
        for field in &c.fields {
            write!(
                text,
                "  DATA+0x{:08x} key=0x{:08x} symbol=0x{:08x}",
                field.offset,
                field.key,
                field.symbol_id()
            )?;
            if let Some(symbol) = symbols
                .as_ref()
                .and_then(|table| table.get(field.symbol_id()))
            {
                write!(text, " {:?}", symbol.name)?;
            }
            write!(
                text,
                " key_bits=0x{:02x} type=0x{:02x} value=0x{:08x}",
                field.key & 0xff,
                field.kind,
                field.value
            )?;
            if let Some(offset) = field.code_offset() {
                write!(text, " -> CODE+0x{offset:08x}")?;
                match boundaries.as_ref() {
                    Some(starts) if !starts.contains(&(offset as usize)) => {
                        text.push_str(" [not an instruction boundary]")
                    }
                    None => text.push_str(" [boundary unchecked]"),
                    _ => {}
                }
            } else if field.kind == 7 && class_offsets.contains(&(field.value as usize)) {
                write!(text, " -> DATA+0x{:08x} class", field.value)?;
            } else if field.kind == 3 {
                if let Some(value) = strings.get(&(field.value as usize)) {
                    write!(text, " -> DATA+0x{:08x} {:?}", field.value, value)?;
                }
            }
            text.push('\n');
        }
    }
    Ok(text)
}

pub fn resource_strings(prg: &Prg) -> Result<String, Box<dyn StdError>> {
    let payload = prg.unique_section(ciq_prg::RESOURCES)?.payload();
    let mut text = String::new();
    for string in ciq_prg::resources::strings(payload)? {
        writeln!(
            text,
            "language=0x{:08x} resource=0x{:08x} @0x{:08x} {:?}",
            string.language, string.id, string.offset, string.value
        )
        .unwrap();
    }
    Ok(text)
}

pub fn strings(prg: &Prg) -> Result<String, Box<dyn StdError>> {
    let source = referenced_data(prg).ok_or("no supported, unambiguous referenced DATA strings")?;
    let mut text = String::new();
    for (at, s) in string_declarations(&source) {
        writeln!(text, "0x{at:08x} {} {:?}", s.value.len(), s.value).unwrap();
    }
    Ok(text)
}

pub fn disassemble(code: &[u8]) -> Result<String, Error> {
    disassemble_with_symbols(code, None)
}

fn optional_symbols(prg: &Prg) -> Option<Symbols<'_>> {
    if !prg
        .sections()
        .iter()
        .any(|section| section.kind() == SYMBOLS)
    {
        return None;
    }
    match prg
        .unique_section(SYMBOLS)
        .and_then(|section| Symbols::parse(section.payload()))
    {
        Ok(symbols) => Some(symbols),
        Err(error) => {
            eprintln!("Symbol annotations unavailable; numeric operands retained: {error}");
            None
        }
    }
}

pub fn disassemble_prg(prg: &Prg) -> Result<String, Box<dyn StdError>> {
    let symbols = optional_symbols(prg);
    Ok(disassemble_with_symbols(
        prg.unique_section(CODE)?.payload(),
        symbols.as_ref(),
    )?)
}

/// Print authoritative symbol source only when it preserves every record.
pub fn symbol_source(prg: &Prg) -> Result<String, Box<dyn StdError>> {
    let payload = prg.unique_section(SYMBOLS)?.payload();
    let table = Symbols::parse(payload)?;
    let source = Source::from_symbols(&table);
    if source.encode()? != payload {
        return Err(
            "symbol source cannot preserve unreferenced name records; use raw export".into(),
        );
    }
    Ok(source.to_text())
}

/// Self-contained symbolic code for the template workflow; declarations resolve IDs only.
pub fn disassemble_symbolic(prg: &Prg) -> Result<String, Box<dyn StdError>> {
    let symbols = optional_symbols(prg);
    let code = prg.unique_section(CODE)?.payload();
    let mut ids = std::collections::BTreeSet::new();
    let mut string_refs = std::collections::BTreeSet::new();
    for located in decode(code)? {
        let ins = located.instruction;
        if ins.opcode() == ciq_mc::Opcode::News {
            string_refs.insert(u32::from_be_bytes(ins.operands().try_into().unwrap()));
        }
        let mut start = 0;
        for (field, &width) in ins.opcode().operand_widths().iter().enumerate() {
            if ins.opcode().symbol_operands().contains(&field) {
                ids.insert(u32::from_be_bytes(
                    ins.operands()[start..start + width].try_into().unwrap(),
                ));
            }
            start += width;
        }
    }
    let mut source = symbols
        .as_ref()
        .map(Source::from_symbols)
        .unwrap_or_default();
    source.declarations.retain(|d| ids.contains(&d.id));
    for d in &mut source.declarations {
        d.name_at = None;
        d.entry_index = None;
    }
    let mut text = source.to_text();
    text.push('\n');
    let data = referenced_data(prg);
    let mut strings = data.as_ref().map(string_declarations).unwrap_or_default();
    strings.retain(|at, _| string_refs.contains(at));
    for declaration in strings.values() {
        text.push_str(&declaration.to_text());
    }
    text.push_str(&disassemble_named(code, symbols.as_ref(), true, &strings)?);
    Ok(text)
}

pub fn format_symbols(symbols: &Symbols<'_>) -> String {
    let mut text = String::new();
    for symbol in symbols.iter() {
        writeln!(
            text,
            "0x{:08x} @0x{:08x} {:?}",
            symbol.id, symbol.name_offset, symbol.name
        )
        .unwrap();
    }
    text
}

pub fn disassemble_with_symbols(
    code: &[u8],
    symbols: Option<&Symbols<'_>>,
) -> Result<String, Error> {
    disassemble_named(code, symbols, false, &std::collections::BTreeMap::new())
}

fn disassemble_named(
    code: &[u8],
    symbols: Option<&Symbols<'_>>,
    named: bool,
    strings: &std::collections::BTreeMap<u32, StringDeclaration>,
) -> Result<String, Error> {
    let mut text = String::new();
    // Validate the full stream before producing any output.
    for located in decode(code)? {
        let instruction = located.instruction;
        text.push_str(instruction.opcode().mnemonic());
        let mut start = 0;
        for (field, &width) in instruction.opcode().operand_widths().iter().enumerate() {
            if instruction.opcode() == ciq_mc::Opcode::News {
                let at = u32::from_be_bytes(instruction.operands().try_into().unwrap());
                if let Some(string) = strings.get(&at) {
                    write!(text, " &{}", string.alias).unwrap();
                    start += width;
                    continue;
                }
            }
            if named && instruction.opcode().symbol_operands().contains(&field) {
                let id = u32::from_be_bytes(
                    instruction.operands()[start..start + width]
                        .try_into()
                        .unwrap(),
                );
                if symbols.and_then(|s| s.get(id)).is_some() {
                    write!(text, " @{}", Source::alias(symbols.unwrap(), id)).unwrap();
                    start += width;
                    continue;
                }
            }
            text.push_str(" 0x");
            for byte in &instruction.operands()[start..start + width] {
                write!(text, "{byte:02x}").unwrap();
            }
            start += width;
        }
        write!(text, " # CODE+0x{:06x}", located.offset).unwrap();
        if let Some(symbols) = symbols {
            let mut start = 0;
            for (field, &width) in instruction.opcode().operand_widths().iter().enumerate() {
                if instruction.opcode().symbol_operands().contains(&field) {
                    let id = u32::from_be_bytes(
                        instruction.operands()[start..start + width]
                            .try_into()
                            .unwrap(),
                    );
                    if let Some(symbol) = symbols.get(id) {
                        use ciq_mc::Opcode;
                        let role = match (instruction.opcode(), field) {
                            (Opcode::Getmv, 0) => "module",
                            (Opcode::Getmv, 1) | (Opcode::Getlocalv, 1) => "member",
                            _ => "symbol",
                        };
                        write!(text, "; {role}={:?}", symbol.name).unwrap();
                    }
                }
                start += width;
            }
        }
        if instruction.opcode() == ciq_mc::Opcode::News {
            let at = u32::from_be_bytes(instruction.operands().try_into().unwrap());
            if let Some(string) = strings.get(&at) {
                write!(text, "; string={:?}", string.value).unwrap();
            }
        }
        text.push('\n');
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offsets_follow_variable_width_instructions() {
        assert_eq!(
            disassemble(&[0x46, 2, 0, 0x80, 0, 5, 0x16]).unwrap(),
            "getlocalv 0x02 0x00800005 # CODE+0x000000\nreturn # CODE+0x000006\n"
        );
    }
}
