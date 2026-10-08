//! Section project manifest, documented in docs/specs/section-project.md.

use crate::{CODE, SYMBOLS};
use std::{fmt, fmt::Write};

pub const FILE_NAME: &str = "sections.ciq";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    Raw,
    Assembly,
    Symbols,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub kind: u32,
    pub length: u32,
    pub encoding: Encoding,
    pub file: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    pub entries: Vec<Entry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub line: usize,
    pub message: &'static str,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "manifest line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for Error {}

fn valid_file(file: &str) -> bool {
    !file.is_empty()
        && file != "."
        && file != ".."
        && file
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
}

impl Manifest {
    pub fn parse(text: &str) -> Result<Self, Error> {
        let mut entries = Vec::new();
        let mut version_seen = false;
        let mut version = "1";
        for (index, line) in text.lines().enumerate() {
            let fail = |message| Error {
                line: index + 1,
                message,
            };
            let fields: Vec<_> = line.split('#').next().unwrap().split_whitespace().collect();
            if fields.is_empty() {
                continue;
            }
            if !version_seen {
                if fields != ["ciq-project", "1"] && fields != ["ciq-project", "2"] {
                    return Err(fail("expected ciq-project 1 or 2 header"));
                }
                version = fields[1];
                version_seen = true;
                continue;
            }
            if fields.len() != 5 || fields[0] != "section" {
                return Err(fail(
                    "expected section <hex-id> <length> <raw|assembly|symbols> <file>",
                ));
            }
            let hex = fields[1]
                .strip_prefix("0x")
                .ok_or_else(|| fail("identifier must start with 0x"))?;
            if hex.is_empty() || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                return Err(fail("invalid section identifier"));
            }
            let kind =
                u32::from_str_radix(hex, 16).map_err(|_| fail("section identifier exceeds u32"))?;
            if kind == 0 {
                return Err(fail(
                    "terminator is implicit; section identifier cannot be zero",
                ));
            }
            if fields[2].is_empty() || !fields[2].bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(fail("length must be decimal u32"));
            }
            let length = fields[2]
                .parse::<u32>()
                .map_err(|_| fail("length exceeds u32"))?;
            let encoding = match fields[3] {
                "raw" => Encoding::Raw,
                "assembly" if kind == CODE => Encoding::Assembly,
                "symbols" if kind == SYMBOLS && version == "2" => Encoding::Symbols,
                _ => {
                    return Err(fail(
                        "unsupported encoding for this version or section kind",
                    ));
                }
            };
            if !valid_file(fields[4]) {
                return Err(fail("section file must be a simple local filename"));
            }
            entries.push(Entry {
                kind,
                length,
                encoding,
                file: fields[4].into(),
            });
        }
        if !version_seen {
            return Err(Error {
                line: 1,
                message: "missing ciq-project 1 or 2 header",
            });
        }
        Ok(Self { entries })
    }

    pub fn to_text(&self) -> String {
        let version = if self.entries.iter().any(|e| e.encoding == Encoding::Symbols) {
            2
        } else {
            1
        };
        let mut text = format!(
            "ciq-project {version}\n# Ordered sections: identifier, original payload length, encoding, file\n"
        );
        for entry in &self.entries {
            let encoding = match entry.encoding {
                Encoding::Raw => "raw",
                Encoding::Assembly => "assembly",
                Encoding::Symbols => "symbols",
            };
            writeln!(
                text,
                "section 0x{:08x} {} {} {}",
                entry.kind, entry.length, encoding, entry.file
            )
            .unwrap();
        }
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comments_blank_lines_and_duplicate_identifiers_are_preserved_in_order() {
        let manifest = Manifest::parse("# project\n\nciq-project 1\nsection 0x12345678 3 raw first.bin\nsection 0x12345678 0 raw empty.bin # duplicate\n").unwrap();
        assert_eq!(manifest.entries.len(), 2);
        assert_eq!(manifest.entries[0].kind, manifest.entries[1].kind);
        assert_eq!(Manifest::parse(&manifest.to_text()).unwrap(), manifest);
    }

    #[test]
    fn rejects_unsafe_paths_invalid_encodings_and_invalid_sizes() {
        for entry in [
            "section 0x12345678 3 raw ../outside.bin",
            "section 0x12345678 3 raw /outside.bin",
            "section 0x12345678 3 raw .",
            "section 0x12345678 3 raw ..",
            "section 0x12345678 3 raw C:\\outside.bin",
            "section 0x12345678 3 assembly data.mcasm",
            "section 0x00000000 0 raw terminator.bin",
            "section 0x12345678 -1 raw file.bin",
            "section 0x12345678 4294967296 raw file.bin",
            "section 0x123456789 3 raw file.bin",
            "section 0x+1234567 3 raw file.bin",
            "section 0x12345678 3 unknown file.bin",
        ] {
            assert_eq!(
                Manifest::parse(&format!("ciq-project 1\n{entry}"))
                    .unwrap_err()
                    .line,
                2,
                "{entry}"
            );
        }
    }

    #[test]
    fn rejects_missing_or_unsupported_version() {
        for text in [
            "",
            "# comment",
            "ciq-project 3",
            "section 0x12345678 0 raw empty.bin",
        ] {
            assert!(Manifest::parse(text).is_err());
        }
    }
}

#[cfg(test)]
mod symbol_manifest_tests {
    use super::*;
    #[test]
    fn symbols_require_version_two_and_the_symbol_section_identifier() {
        let text = "ciq-project 2\nsection 0x5717b015 2 symbols symbols.mcasm\n";
        let manifest = Manifest::parse(text).unwrap();
        assert_eq!(manifest.entries[0].encoding, Encoding::Symbols);
        assert_eq!(Manifest::parse(&manifest.to_text()).unwrap(), manifest);
        assert!(Manifest::parse(&text.replace("project 2", "project 1")).is_err());
        assert!(Manifest::parse(&text.replace("5717b015", "c0debabe")).is_err());
    }
}
