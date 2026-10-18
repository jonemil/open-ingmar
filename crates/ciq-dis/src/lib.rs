//! Text output for the supported ordinary bytecode subset.
//!
//! Exports supported code and metadata as assembly and declarations. Unknown
//! payloads remain raw bytes so exported projects can preserve them.

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
    let debug_source = prg
        .unique_section(0xd0000d1e)
        .ok()
        .and_then(|s| ciq_prg::debug::DebugSource::from_payload(s.payload()).ok());
    let mut line_source = prg
        .unique_section(0xc0de7ab1)
        .ok()
        .and_then(|s| ciq_prg::debug::LineSource::from_payload(s.payload()).ok());
    let mut code_labels = project_code_labels(prg, data.as_ref());
    let mut exception_source = prg
        .unique_section(ciq_prg::EXCEPTIONS)
        .ok()
        .and_then(|s| ciq_prg::exceptions::Source::from_payload(s.payload()).ok());
    if let (Some(lines), Some(code)) = (
        &line_source,
        prg.unique_section(CODE)
            .ok()
            .and_then(|s| decode(s.payload()).ok()),
    ) {
        let boundaries: std::collections::BTreeSet<_> =
            code.iter().map(|i| i.offset as u32).collect();
        for line in &lines.entries {
            if let ciq_prg::debug::Pc::Raw(pc) = line.pc {
                let offset = pc & 0xffffff;
                if pc >> 24 == 0x10 && boundaries.contains(&offset) {
                    code_labels
                        .entry(offset)
                        .or_insert_with(|| format!("code_{offset:08x}"));
                }
            }
        }
    }
    if let Ok(section) = prg.unique_section(CODE) {
        name_branch_targets(section.payload(), &mut code_labels);
        if let (Some(source), Ok(instructions)) = (&mut exception_source, decode(section.payload()))
        {
            let boundaries: std::collections::BTreeSet<_> =
                instructions.iter().map(|i| i.offset).collect();
            let length = section.payload().len();
            for entry in &source.entries {
                for (offset, allow_end) in [
                    (&entry.begin, false),
                    (&entry.end, true),
                    (&entry.handler, false),
                ] {
                    if let ciq_prg::exceptions::Offset::Raw(n) = offset {
                        if boundaries.contains(&(*n as usize)) || allow_end && *n as usize == length
                        {
                            code_labels
                                .entry(*n)
                                .or_insert_with(|| format!("code_{n:08x}"));
                        }
                    }
                }
            }
            source.name_references(&code_labels, length as u32);
        }
    }
    let label_bindings = code_labels
        .iter()
        .map(|(offset, alias)| (alias.clone(), *offset))
        .collect();
    if let Some(source) = &exception_source {
        if source.encode(&label_bindings)? != prg.unique_section(ciq_prg::EXCEPTIONS)?.payload() {
            return Err("structured exception export changed bytes".into());
        }
    }
    let debug_bindings = debug_source
        .as_ref()
        .map(|s| s.encode().map(|(_, bindings)| bindings))
        .transpose()?
        .unwrap_or_default();
    let debug_offsets = debug_bindings
        .iter()
        .map(|(alias, offset)| (*offset, alias.clone()))
        .collect();
    if let Some(source) = line_source.as_mut() {
        source.name_references(&code_labels, &debug_offsets);
        if source.encode(&label_bindings, &debug_bindings)?
            != prg.unique_section(0xc0de7ab1)?.payload()
        {
            return Err("structured line export changed bytes".into());
        }
    }
    if let Some(source) = data.as_mut() {
        use ciq_prg::data::classes::source::Reference;
        for fragment in &mut source.fragments {
            let Fragment::Class(c) = fragment else {
                continue;
            };
            for field in &mut c.fields {
                if let (6, Reference::Raw(value)) = (field.kind, &field.value) {
                    if value >> 24 == 0x10 {
                        if let Some(alias) = code_labels.get(&(value & 0xffffff)) {
                            field.value = Reference::Code(alias.clone());
                        }
                    }
                }
            }
        }
        let bindings = structured
            .as_ref()
            .map(Source::bindings)
            .transpose()?
            .unwrap_or_default();
        if source.encode_with_references(&bindings, &label_bindings)?.0
            != prg.unique_section(DATA)?.payload()
        {
            return Err("code-labeled DATA export changed bytes".into());
        }
    }
    let strings = data.as_ref().map(string_declarations).unwrap_or_default();
    let arrays = data.as_ref().map(array_declarations).unwrap_or_default();
    let dictionaries = data
        .as_ref()
        .map(dictionary_declarations)
        .unwrap_or_default();
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
                &arrays,
                &dictionaries,
                &code_labels,
            ) {
                Ok(text) => (Encoding::Assembly, text.into_bytes()),
                Err(error) => {
                    eprintln!("Section {index} CODE exported as raw binary: {error}");
                    (Encoding::Raw, section.payload().to_vec())
                }
            }
        } else {
            match (
                section.kind(),
                data.as_ref(),
                structured.as_ref(),
                debug_source.as_ref(),
                line_source.as_ref(),
                exception_source.as_ref(),
            ) {
                (ciq_prg::EXCEPTIONS, _, _, _, _, Some(source)) => {
                    (Encoding::Exceptions, source.to_text().into_bytes())
                }
                (0xd0000d1e, _, _, Some(source), _, _) => {
                    (Encoding::Debug, source.to_text().into_bytes())
                }
                (0xc0de7ab1, _, _, _, Some(source), _) => {
                    (Encoding::Lines, source.to_text().into_bytes())
                }
                (DATA, Some(source), _, _, _, _) => (Encoding::Data, source.to_text().into_bytes()),
                (SYMBOLS, _, Some(source), _, _, _) => {
                    (Encoding::Symbols, source.to_text().into_bytes())
                }
                _ => (Encoding::Raw, section.payload().to_vec()),
            }
        };
        let extension = match encoding {
            Encoding::Raw => "bin",
            Encoding::Assembly
            | Encoding::Symbols
            | Encoding::Data
            | Encoding::Debug
            | Encoding::Lines
            | Encoding::Exceptions => "mcasm",
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

fn project_code_labels(
    prg: &Prg,
    data: Option<&DataSource>,
) -> std::collections::BTreeMap<u32, String> {
    use ciq_prg::data::classes::source::{Key, Reference};
    use std::collections::{BTreeMap, BTreeSet};
    let Some(data) = data else {
        return BTreeMap::new();
    };
    let Some(boundaries) = prg
        .unique_section(CODE)
        .ok()
        .and_then(|s| decode(s.payload()).ok())
    else {
        return BTreeMap::new();
    };
    let starts: BTreeSet<_> = boundaries.iter().map(|i| i.offset as u32).collect();
    // Retain every binding: shared targets must not inherit the first field's name.
    type MethodBinding = (Option<String>, Option<String>); // DATA owner, member symbol alias
    let mut candidates: BTreeMap<u32, BTreeSet<MethodBinding>> = BTreeMap::new();
    for fragment in &data.fragments {
        let Fragment::Class(c) = fragment else {
            continue;
        };
        for field in &c.fields {
            if let (6, Reference::Raw(value)) = (field.kind, &field.value) {
                if value >> 24 != 0x10 || !starts.contains(&(value & 0xffffff)) {
                    continue;
                }
                let alias = match &field.key {
                    Key::Symbol { alias, .. } => Some(alias.clone()),
                    _ => None,
                };
                candidates
                    .entry(value & 0xffffff)
                    .or_default()
                    .insert((c.alias.clone(), alias));
            }
        }
    }
    let mut targets: BTreeMap<String, BTreeSet<u32>> = BTreeMap::new();
    for (offset, bindings) in &candidates {
        for (_, name) in bindings {
            if let Some(name) = name {
                targets.entry(name.clone()).or_default().insert(*offset);
            }
        }
    }
    let proposed: BTreeMap<_, _> = candidates
        .into_iter()
        .map(|(offset, bindings)| {
            let name = if bindings.len() == 1 {
                let (owner, member) = bindings.into_iter().next().unwrap();
                member.and_then(|member| {
                    if ["code_", "sym_", "method_"]
                        .iter()
                        .any(|prefix| member.starts_with(prefix))
                    {
                        None
                    } else if targets[&member].len() == 1 {
                        Some(member)
                    } else {
                        owner.map(|owner| format!("method_{owner}__{member}"))
                    }
                })
            } else {
                None
            };
            (offset, name)
        })
        .collect();
    // Concatenated owner/member spellings can collide; neither target wins.
    let mut counts = BTreeMap::new();
    for name in proposed.values().flatten() {
        *counts.entry(name.clone()).or_insert(0usize) += 1;
    }
    proposed
        .into_iter()
        .map(|(offset, name)| {
            let alias = name
                .filter(|name| counts[name] == 1)
                .unwrap_or_else(|| format!("code_{offset:08x}"));
            (offset, alias)
        })
        .collect()
}

fn name_branch_targets(code: &[u8], labels: &mut std::collections::BTreeMap<u32, String>) {
    let Ok(instructions) = decode(code) else {
        return;
    };
    let boundaries: std::collections::BTreeSet<_> = instructions.iter().map(|i| i.offset).collect();
    for instruction in &instructions {
        if let Some(target) = instruction
            .relative_target()
            .and_then(|n| usize::try_from(n).ok())
            .filter(|n| boundaries.contains(n))
            .and_then(|n| u32::try_from(n).ok())
        {
            labels
                .entry(target)
                .or_insert_with(|| format!("code_{target:08x}"));
        }
    }
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

fn array_declarations(source: &DataSource) -> std::collections::BTreeMap<u32, String> {
    source
        .fragments
        .iter()
        .filter_map(|f| match f {
            Fragment::Array(a) => a.at.map(|at| (at, a.alias.clone())),
            _ => None,
        })
        .collect()
}

fn dictionary_declarations(source: &DataSource) -> std::collections::BTreeMap<u32, String> {
    source
        .fragments
        .iter()
        .filter_map(|f| match f {
            Fragment::Dictionary(d) => d.at.map(|at| (at, d.alias.clone())),
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
            } else if field.kind == 8 {
                if let Some(symbol) = symbols
                    .as_ref()
                    .and_then(|symbols| symbols.get(field.value))
                {
                    write!(text, " -> Symbol {:?}", symbol.name)?;
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
    let mut labels = std::collections::BTreeMap::new();
    name_branch_targets(code, &mut labels);
    text.push_str(&disassemble_named(
        code,
        symbols.as_ref(),
        true,
        &strings,
        &std::collections::BTreeMap::new(),
        &std::collections::BTreeMap::new(),
        &labels,
    )?);
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
    disassemble_named(
        code,
        symbols,
        false,
        &std::collections::BTreeMap::new(),
        &std::collections::BTreeMap::new(),
        &std::collections::BTreeMap::new(),
        &std::collections::BTreeMap::new(),
    )
}

fn disassemble_named(
    code: &[u8],
    symbols: Option<&Symbols<'_>>,
    named: bool,
    strings: &std::collections::BTreeMap<u32, StringDeclaration>,
    arrays: &std::collections::BTreeMap<u32, String>,
    dictionaries: &std::collections::BTreeMap<u32, String>,
    labels: &std::collections::BTreeMap<u32, String>,
) -> Result<String, Error> {
    let mut text = String::new();
    // Validate the full stream before producing any output.
    for located in decode(code)? {
        if let Some(alias) = u32::try_from(located.offset)
            .ok()
            .and_then(|n| labels.get(&n))
        {
            writeln!(text, "{alias}:").unwrap();
        }
        let target = located
            .relative_target()
            .and_then(|n| u32::try_from(n).ok());
        let instruction = located.instruction;
        text.push_str(instruction.opcode().mnemonic());
        let mut start = 0;
        for (field, &width) in instruction.opcode().operand_widths().iter().enumerate() {
            if field == 0 && instruction.opcode().has_relative_target() {
                if let Some(alias) = target.and_then(|n| labels.get(&n)) {
                    write!(text, " code:{alias}").unwrap();
                    start += width;
                    continue;
                }
            }
            if instruction.opcode() == ciq_mc::Opcode::News {
                let at = u32::from_be_bytes(instruction.operands().try_into().unwrap());
                if let Some(string) = strings.get(&at) {
                    write!(text, " &{}", string.alias).unwrap();
                    start += width;
                    continue;
                }
            }
            if instruction.opcode() == ciq_mc::Opcode::Hpush {
                let at = u32::from_be_bytes(instruction.operands().try_into().unwrap());
                if let Some(alias) = dictionaries.get(&at) {
                    write!(text, " &{alias}").unwrap();
                    start += width;
                    continue;
                }
            }
            if instruction.opcode() == ciq_mc::Opcode::Apush {
                let at = u32::from_be_bytes(instruction.operands().try_into().unwrap());
                if let Some(alias) = arrays.get(&at) {
                    write!(text, " &{alias}").unwrap();
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
    if let Some(alias) = u32::try_from(code.len()).ok().and_then(|n| labels.get(&n)) {
        writeln!(text, "{alias}:").unwrap();
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn method_labels(bindings: &[(&str, &str, u32)]) -> std::collections::BTreeMap<u32, String> {
        let mut symbols = std::collections::BTreeMap::new();
        let mut owners: std::collections::BTreeMap<&str, String> =
            std::collections::BTreeMap::new();
        for (owner, member, offset) in bindings {
            let next = symbols.len() as u32 + 1;
            symbols.entry(member.to_string()).or_insert(next);
            owners.entry(owner).or_default().push_str(&format!(
                ".field symbol=@{member} bits=0x10 type=0x06 value=0x{:08x}\n",
                0x10000000 | offset
            ));
        }
        let text: String = owners.into_iter().map(|(owner, fields)| format!(
            ".class {owner} version=0x02 extends=none statics=none parent=none module=none app_flags=0x0\n{fields}.endclass\n"
        )).collect();
        let data = DataSource::parse_with_symbols(&text, &symbols).unwrap();
        let prg = Prg::from_sections(vec![ciq_prg::Section::new(CODE, vec![0x16; 8]).unwrap()]);
        project_code_labels(&prg, Some(&data))
    }

    #[test]
    fn duplicate_members_use_owner_labels_and_unique_members_stay_short() {
        let labels = method_labels(&[
            ("Left", "answer", 0),
            ("Right", "answer", 1),
            ("App", "probe", 2),
        ]);
        assert_eq!(labels[&0], "method_Left__answer");
        assert_eq!(labels[&1], "method_Right__answer");
        assert_eq!(labels[&2], "probe");
    }

    #[test]
    fn shared_targets_are_numeric_independent_of_binding_order() {
        let bindings = [
            ("Left", "answer", 0),
            ("Right", "probe", 0),
            ("Other", "answer", 1),
        ];
        let labels = method_labels(&bindings);
        assert_eq!(labels[&0], "code_00000000");
        assert_eq!(labels[&1], "method_Other__answer");
        let reversed: Vec<_> = bindings.into_iter().rev().collect();
        assert_eq!(labels, method_labels(&reversed));
    }

    #[test]
    fn concatenation_collisions_and_reserved_names_fall_back() {
        let labels = method_labels(&[
            ("a__b", "c", 0),
            ("a", "b__c", 1),
            ("other", "c", 2),
            ("other", "b__c", 3),
            ("App", "method_other__c", 4),
            ("App", "code_00000002", 5),
        ]);
        assert_eq!(labels[&0], "code_00000000");
        assert_eq!(labels[&1], "code_00000001");
        assert_eq!(labels[&2], "method_other__c");
        assert_eq!(labels[&3], "method_other__b__c");
        assert_eq!(labels[&4], "code_00000004");
        assert_eq!(labels[&5], "code_00000005");
    }

    #[test]
    fn offsets_follow_variable_width_instructions() {
        assert_eq!(
            disassemble(&[0x46, 2, 0, 0x80, 0, 5, 0x16]).unwrap(),
            "getlocalv 0x02 0x00800005 # CODE+0x000000\nreturn # CODE+0x000006\n"
        );
    }

    #[test]
    fn class_listings_resolve_symbol_defaults_without_guessing_other_types() {
        let data = ciq_prg::data::Source::parse(".class root version=0x02 extends=none statics=none parent=none module=none app_flags=0x0\n.field key=0x100 type=0x08 value=0x81\n.field key=0x200 type=0x01 value=0x81\n.endclass").unwrap().encode().unwrap().0;
        let symbols = ciq_prg::symbols::source::Source::parse(".symbol alpha \"alpha\" id=0x81")
            .unwrap()
            .encode()
            .unwrap();
        let prg = Prg::from_sections(vec![
            ciq_prg::Section::new(ciq_prg::DATA, data).unwrap(),
            ciq_prg::Section::new(ciq_prg::SYMBOLS, symbols).unwrap(),
        ]);
        let text = classes(&prg).unwrap();
        assert_eq!(text.matches("-> Symbol \"alpha\"").count(), 1);
    }
}
