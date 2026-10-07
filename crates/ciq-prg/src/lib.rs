//! Ordered, opaque PRG sections. See docs/specs/prg-container.md.

use std::fmt;

pub const CODE: u32 = 0xc0de_babe;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub offset: usize,
    pub message: &'static str,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PRG offset 0x{:x}: {}", self.offset, self.message)
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    kind: u32,
    payload: Vec<u8>,
}

impl Section {
    pub fn kind(&self) -> u32 {
        self.kind
    }

    /// Research-derived names; recognizing a name does not decode its payload.
    /// See the section catalogue in docs/investigations/rust-bootstrap.md.
    pub fn name(&self) -> Option<&'static str> {
        match self.kind {
            0xd000_d00d => Some("HEADER_VERSIONED"),
            0x6060_c0de => Some("ENTRY_POINTS"),
            0xda7a_babe => Some("DATA"),
            CODE => Some("CODE"),
            0xd000_0d1e => Some("DEBUG"),
            0xc0de_7ab1 => Some("LINE_NUMBERS"),
            0xc1a5_57b1 => Some("IMPORTS"),
            0xf00d_600d => Some("RESOURCES"),
            0x6000_db01 => Some("PERMISSIONS"),
            0x0ece_7105 => Some("EXCEPTIONS"),
            0x5717_b015 => Some("SYMBOLS"),
            0x5e77_1465 => Some("SETTINGS"),
            0xe1c0_de12 => Some("DEVELOPER_SIGNATURE"),
            _ => None,
        }
    }

    pub fn payload(&self) -> &[u8] {
        &self.payload
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prg {
    sections: Vec<Section>,
}

impl Prg {
    pub fn parse(bytes: &[u8]) -> Result<Self, Error> {
        let mut offset = 0;
        let mut sections = Vec::new();
        loop {
            let remaining = bytes.len() - offset;
            if remaining < 8 {
                return Err(Error {
                    offset,
                    message: "missing terminator or truncated section header",
                });
            }
            let kind = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap());
            let length =
                u32::from_be_bytes(bytes[offset + 4..offset + 8].try_into().unwrap()) as usize;
            if kind == 0 {
                if length != 0 || remaining != 8 {
                    return Err(Error {
                        offset,
                        message: "invalid terminator or trailing bytes",
                    });
                }
                return Ok(Self { sections });
            }
            if length > remaining - 8 {
                return Err(Error {
                    offset,
                    message: "section payload exceeds file length",
                });
            }
            let end = offset + 8 + length;
            sections.push(Section {
                kind,
                payload: bytes[offset + 8..end].to_vec(),
            });
            offset = end;
        }
    }

    pub fn sections(&self) -> &[Section] {
        &self.sections
    }

    pub fn unique_section(&self, kind: u32) -> Result<&Section, Error> {
        let mut matching = self.sections.iter().filter(|section| section.kind == kind);
        let section = matching.next().ok_or(Error {
            offset: 0,
            message: "required section is missing",
        })?;
        if matching.next().is_some() {
            return Err(Error {
                offset: 0,
                message: "required section occurs more than once",
            });
        }
        Ok(section)
    }

    /// No relocation or signature recomputation is performed.
    pub fn replace_same_length(&mut self, kind: u32, payload: &[u8]) -> Result<(), Error> {
        if self.unique_section(kind)?.payload.len() != payload.len() {
            return Err(Error {
                offset: 0,
                message: "replacement changes section length; relocation is unsupported",
            });
        }
        let section = self
            .sections
            .iter_mut()
            .find(|section| section.kind == kind)
            .unwrap();
        section.payload.copy_from_slice(payload);
        Ok(())
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        for section in &self.sections {
            bytes.extend_from_slice(&section.kind.to_be_bytes());
            // All sections originate from bounded u32 lengths in parse().
            bytes.extend_from_slice(&(section.payload.len() as u32).to_be_bytes());
            bytes.extend_from_slice(&section.payload);
        }
        bytes.extend_from_slice(&[0; 8]);
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn specimen(sections: &[(u32, &[u8])]) -> Vec<u8> {
        let mut bytes = Vec::new();
        for (kind, payload) in sections {
            bytes.extend_from_slice(&kind.to_be_bytes());
            bytes.extend_from_slice(&(payload.len() as u32).to_be_bytes());
            bytes.extend_from_slice(payload);
        }
        bytes.extend_from_slice(&[0; 8]);
        bytes
    }

    #[test]
    fn preserves_unknown_duplicate_and_empty_sections() {
        let bytes = specimen(&[(0x12345678, &[1, 2, 3]), (CODE, &[0x16]), (0x12345678, &[])]);
        let prg = Prg::parse(&bytes).unwrap();
        assert_eq!(prg.sections().len(), 3);
        assert_eq!(prg.to_bytes(), bytes);
    }

    #[test]
    fn rejects_every_truncation_of_a_valid_file() {
        let bytes = specimen(&[(CODE, &[0x35, 1, 0x16])]);
        for end in 0..bytes.len() {
            assert!(
                Prg::parse(&bytes[..end]).is_err(),
                "accepted prefix of length {end}"
            );
        }
    }

    #[test]
    fn rejects_oversized_payload_and_trailing_bytes() {
        let mut bytes = specimen(&[(CODE, &[0x16])]);
        bytes[4..8].copy_from_slice(&u32::MAX.to_be_bytes());
        assert!(Prg::parse(&bytes).is_err());
        let mut bytes = specimen(&[]);
        bytes.push(1);
        assert!(Prg::parse(&bytes).is_err());
        bytes[7] = 1;
        assert!(Prg::parse(&bytes).is_err());
    }

    #[test]
    fn replacement_preserves_other_sections_and_rejects_ambiguous_code() {
        let mut prg = Prg::parse(&specimen(&[(CODE, &[0x38, 41, 0x16]), (7, &[8, 9])])).unwrap();
        assert!(prg.replace_same_length(CODE, &[0x16]).is_err());
        prg.replace_same_length(CODE, &[0x38, 42, 0x16]).unwrap();
        assert_eq!(
            prg.to_bytes(),
            specimen(&[(CODE, &[0x38, 42, 0x16]), (7, &[8, 9])])
        );
        let mut duplicate = Prg::parse(&specimen(&[(CODE, &[0x16]), (CODE, &[0x16])])).unwrap();
        assert!(duplicate.replace_same_length(CODE, &[0x16]).is_err());
        assert!(
            Prg::parse(&specimen(&[]))
                .unwrap()
                .unique_section(CODE)
                .is_err()
        );
    }
}
