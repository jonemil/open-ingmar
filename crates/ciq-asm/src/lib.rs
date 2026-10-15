//! Initial assembly grammar.

use ciq_prg::{
    Prg, Section,
    data::{Source as DataSource, StringDeclaration},
    manifest::{Encoding, FILE_NAME, Manifest},
    symbols::source::{Source, tokens},
};
use std::{error::Error as StdError, fs, path::Path};

/// Reconstruct all sections without reading an original PRG template.
pub fn assemble_project(directory: &Path) -> Result<Prg, Box<dyn StdError>> {
    let directory = directory.canonicalize()?;
    let manifest_path = directory.join(FILE_NAME).canonicalize()?;
    if !manifest_path.starts_with(&directory) {
        return Err("manifest resolves outside the project directory".into());
    }
    let manifest = Manifest::parse(&fs::read_to_string(manifest_path)?)?;
    for (encoding, kind, role) in [
        (Encoding::Debug, 0xd0000d1e, "DEBUG"),
        (Encoding::Lines, 0xc0de7ab1, "LINE_NUMBERS"),
        (Encoding::Exceptions, ciq_prg::EXCEPTIONS, "EXCEPTIONS"),
    ] {
        if manifest.entries.iter().any(|e| e.encoding == encoding)
            && manifest.entries.iter().filter(|e| e.kind == kind).count() != 1
        {
            return Err(format!("structured {role} requires a unique section").into());
        }
    }
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
    let mut debug_source = ciq_prg::debug::DebugSource::default();
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
        if entry.encoding == Encoding::Debug {
            debug_source = ciq_prg::debug::DebugSource::parse(std::str::from_utf8(&bytes)?)?;
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
    let arrays = data_source.array_bindings()?;
    let dictionaries = data_source.dictionary_bindings()?;
    let (debug_payload, debug_strings) = debug_source.encode()?;
    let mut line_targets = Vec::new();
    let mut exception_targets = Vec::new();
    let mut sections = Vec::new();
    for (entry, bytes) in manifest.entries.iter().zip(inputs) {
        let payload = match entry.encoding {
            Encoding::Raw => bytes,
            Encoding::Assembly => {
                let text = std::str::from_utf8(&bytes)?;
                for line in text.lines() {
                    if tokens(line)?
                        .first()
                        .is_some_and(|f| f == ".string" || f == ".array" || f == ".dictionary")
                    {
                        return Err(
                            "project literal declarations belong in the authoritative DATA source"
                                .into(),
                        );
                    }
                }
                Program::parse(text)?.encode_with_data_references(
                    &bindings,
                    &strings,
                    &arrays,
                    &dictionaries,
                )?
            }
            Encoding::Symbols => symbol_source.encode()?,
            Encoding::Data => data_payload.clone(),
            Encoding::Debug => debug_payload.clone(),
            Encoding::Lines => {
                let source = ciq_prg::debug::LineSource::parse(std::str::from_utf8(&bytes)?)?;
                for line in &source.entries {
                    if let ciq_prg::debug::Pc::Code(alias) = &line.pc {
                        line_targets.push(alias.clone());
                    }
                }
                source.encode(&labels, &debug_strings)?
            }
            Encoding::Exceptions => {
                let source = ciq_prg::exceptions::Source::parse(std::str::from_utf8(&bytes)?)?;
                for record in &source.entries {
                    for (offset, allow_end) in [
                        (&record.begin, false),
                        (&record.end, true),
                        (&record.handler, false),
                    ] {
                        if let ciq_prg::exceptions::Offset::Code(alias) = offset {
                            exception_targets.push((alias.clone(), allow_end));
                        }
                    }
                }
                source.encode(&labels)?
            }
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
    if !exception_targets.is_empty() {
        let code = prg.unique_section(ciq_prg::CODE)?.payload();
        for (alias, allow_end) in exception_targets {
            let target = labels[&alias] as usize;
            // Assembly labels already denote instruction boundaries or end-of-CODE.
            if target > code.len() || !allow_end && target == code.len() {
                return Err(format!(
                    "exception CODE label {alias} is not a valid boundary for its role"
                )
                .into());
            }
        }
    }
    if !labels.is_empty() {
        let length = prg.unique_section(ciq_prg::CODE)?.payload().len();
        for alias in line_targets {
            if labels[&alias] as usize >= length {
                return Err(format!("line code label {alias} is not an instruction").into());
            }
        }
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

pub use ciq_assembly::{
    Error, Program, assemble, assemble_with_bindings, assemble_with_references, code_labels,
};

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

#[cfg(test)]
mod label_tests {
    use super::*;
    #[test]
    fn labels_are_zero_size_and_track_instruction_boundaries() {
        let text = "first:\nargc 0x01\nprobe:\nipush1 0x2a\nreturn\nend:\n";
        assert_eq!(
            code_labels(text).unwrap(),
            std::collections::BTreeMap::from([
                ("first".into(), 0),
                ("probe".into(), 2),
                ("end".into(), 5)
            ])
        );
        assert_eq!(
            assemble(text).unwrap(),
            assemble("argc 0x01\nipush1 0x2a\nreturn").unwrap()
        );
        for text in [
            "a:\na:\nreturn",
            "1bad:\nreturn",
            "a: return",
            "a:\ninvalid",
            "a:\nargc",
        ] {
            assert!(assemble(text).is_err(), "{text}");
        }
    }
}
