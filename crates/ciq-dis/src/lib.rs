//! Text output for the supported ordinary bytecode subset.

use ciq_mc::{Error, decode};
use ciq_prg::{
    CODE, Prg, SYMBOLS,
    manifest::{Encoding, Entry, FILE_NAME, Manifest},
    symbols::Symbols,
};
use std::fmt::Write;
use std::{error::Error as StdError, fs, path::Path};

/// Export every section, falling back to raw bytes for unsupported code.
pub fn export_project(prg: &Prg, directory: &Path) -> Result<Manifest, Box<dyn StdError>> {
    let symbols = optional_symbols(prg);
    let mut entries = Vec::new();
    let mut files = Vec::new();
    let mut comments = Vec::new();
    let mut offset = 0;
    for (index, section) in prg.sections().iter().enumerate() {
        let name = section.name().unwrap_or("UNKNOWN");
        let (encoding, payload) = if section.kind() == CODE {
            match disassemble_with_symbols(section.payload(), symbols.as_ref()) {
                Ok(text) => (Encoding::Assembly, text.into_bytes()),
                Err(error) => {
                    eprintln!("Section {index} CODE exported as raw binary: {error}");
                    (Encoding::Raw, section.payload().to_vec())
                }
            }
        } else {
            (Encoding::Raw, section.payload().to_vec())
        };
        let extension = match encoding {
            Encoding::Raw => "bin",
            Encoding::Assembly => "mcasm",
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
        if section.kind() == SYMBOLS {
            if let Ok(table) = Symbols::parse(section.payload()) {
                let mut listing = String::from(
                    "# Inspection only: reassembly consumes the corresponding raw .bin payload.\n",
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
    let mut text = String::new();
    // Validate the full stream before producing any output.
    for located in decode(code)? {
        let instruction = located.instruction;
        text.push_str(instruction.opcode().mnemonic());
        let mut start = 0;
        for &width in instruction.opcode().operand_widths() {
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
