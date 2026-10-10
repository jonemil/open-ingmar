//! Ordered EXCEPTIONS records and symbolic section source.

use crate::symbols::source::{Error, tokens, valid_alias};
use std::{collections::BTreeMap, fmt::Write};

/// A raw CODE offset or a label to resolve after CODE layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Offset {
    Raw(u32),
    Code(String),
}

impl Offset {
    fn parse(text: &str) -> Result<Self, Error> {
        if let Some(alias) = text.strip_prefix("code:") {
            if !valid_alias(alias) {
                return Err(Error("invalid exception CODE label".into()));
            }
            return Ok(Self::Code(alias.into()));
        }
        let hex = text
            .strip_prefix("0x")
            .ok_or_else(|| Error("expected exception offset or code:label".into()))?;
        if hex.is_empty() || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(Error("invalid exception offset".into()));
        }
        let n = u32::from_str_radix(hex, 16)
            .map_err(|_| Error("exception offset exceeds u24".into()))?;
        if n > 0xffffff {
            return Err(Error("exception offset exceeds u24".into()));
        }
        Ok(Self::Raw(n))
    }
    fn resolve(&self, labels: &BTreeMap<String, u32>) -> Result<u32, Error> {
        let n = match self {
            Self::Raw(n) => *n,
            Self::Code(alias) => {
                if !valid_alias(alias) {
                    return Err(Error("invalid exception CODE label".into()));
                }
                *labels
                    .get(alias)
                    .ok_or_else(|| Error(format!("undefined exception CODE label {alias}")))?
            }
        };
        if n > 0xffffff {
            return Err(Error("exception offset exceeds u24".into()));
        }
        Ok(n)
    }
    fn text(&self) -> String {
        match self {
            Self::Raw(n) => format!("0x{n:06x}"),
            Self::Code(a) => format!("code:{a}"),
        }
    }
}

/// One ordered protected-range and handler record; endpoint semantics are unresolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub begin: Offset,
    pub end: Offset,
    pub handler: Offset,
}

/// Exception records in original order, including duplicates and overlapping ranges.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Source {
    pub entries: Vec<Entry>,
}

impl Source {
    /// Parse a u16 count followed by exactly that many big-endian u24 triples.
    ///
    /// Rejects truncation and trailing bytes. Retains record order and raw offsets
    /// without inferring interval ordering, endpoint semantics, or CODE validity.
    pub fn from_payload(payload: &[u8]) -> Result<Self, Error> {
        if payload.len() < 2 {
            return Err(Error("truncated exception count".into()));
        }
        let count = u16::from_be_bytes(payload[..2].try_into().unwrap()) as usize;
        if payload.len() != 2 + count * 9 {
            return Err(Error(
                "exception count does not match payload length".into(),
            ));
        }
        let entries = payload[2..]
            .chunks_exact(9)
            .map(|row| {
                let read =
                    |at| Offset::Raw(u32::from_be_bytes([0, row[at], row[at + 1], row[at + 2]]));
                Entry {
                    begin: read(0),
                    end: read(3),
                    handler: read(6),
                }
            })
            .collect();
        Ok(Self { entries })
    }

    /// Parse ordered `.exception begin=... end=... handler=...` declarations.
    ///
    /// Accepts u24 hexadecimal offsets and `code:label` references. Rejects
    /// malformed syntax and counts exceeding u16; labels resolve during encoding.
    pub fn parse(text: &str) -> Result<Self, Error> {
        let mut entries = Vec::new();
        for (index, line) in text.lines().enumerate() {
            let parse = || -> Result<Option<Entry>, Error> {
                let fields = tokens(line)?;
                if fields.is_empty() {
                    return Ok(None);
                }
                if fields.len() != 4 || fields[0] != ".exception" {
                    return Err(Error(
                        "expected .exception begin=... end=... handler=...".into(),
                    ));
                }
                let attribute =
                    |i: usize, prefix: &str| -> Result<Offset, Error> {
                        Offset::parse(fields[i].strip_prefix(prefix).ok_or_else(|| {
                            Error(format!("expected exception attribute {prefix}"))
                        })?)
                    };
                Ok(Some(Entry {
                    begin: attribute(1, "begin=")?,
                    end: attribute(2, "end=")?,
                    handler: attribute(3, "handler=")?,
                }))
            };
            if let Some(entry) =
                parse().map_err(|e| Error(format!("exception source {}: {e}", index + 1)))?
            {
                entries.push(entry);
            }
        }
        if entries.len() > u16::MAX as usize {
            return Err(Error("exception count exceeds u16".into()));
        }
        Ok(Self { entries })
    }

    /// Encode ordered records, resolving labels to CODE offsets without a region prefix.
    ///
    /// Rejects counts exceeding u16, offsets exceeding u24, and undefined labels.
    /// Does not validate CODE boundaries or interval ordering. Callers must check
    /// named starts/handlers denote instructions and ends denote CODE boundaries.
    pub fn encode(&self, labels: &BTreeMap<String, u32>) -> Result<Vec<u8>, Error> {
        let count = u16::try_from(self.entries.len())
            .map_err(|_| Error("exception count exceeds u16".into()))?;
        let mut bytes = count.to_be_bytes().to_vec();
        for entry in &self.entries {
            for offset in [&entry.begin, &entry.end, &entry.handler] {
                bytes.extend_from_slice(&offset.resolve(labels)?.to_be_bytes()[1..]);
            }
        }
        Ok(bytes)
    }

    /// Replace mapped raw offsets with labels, allowing end-of-CODE only for ends.
    ///
    /// The supplied map must contain verified CODE boundaries. Unmapped offsets
    /// remain numeric, preserving unfamiliar or invalid input.
    pub fn name_references(&mut self, labels: &BTreeMap<u32, String>, code_length: u32) {
        for entry in &mut self.entries {
            for (offset, allow_end) in [
                (&mut entry.begin, false),
                (&mut entry.end, true),
                (&mut entry.handler, false),
            ] {
                if let Offset::Raw(n) = offset {
                    if *n < code_length || allow_end && *n == code_length {
                        if let Some(alias) = labels.get(n) {
                            *offset = Offset::Code(alias.clone());
                        }
                    }
                }
            }
        }
    }

    /// Print canonical exception declarations without sorting or deduplicating.
    pub fn to_text(&self) -> String {
        let mut text = String::new();
        for entry in &self.entries {
            writeln!(
                text,
                ".exception begin={} end={} handler={}",
                entry.begin.text(),
                entry.end.text(),
                entry.handler.text()
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
    fn ordered_duplicate_and_unusual_ranges_roundtrip_without_normalization() {
        let text = ".exception begin=0x000003 end=0x000001 handler=0xffffff\n.exception begin=0x000003 end=0x000001 handler=0xffffff\n.exception begin=0x000000 end=0x000004 handler=0x000002\n";
        let source = Source::parse(text).unwrap();
        let bytes = source.encode(&BTreeMap::new()).unwrap();
        assert_eq!(bytes.len(), 29);
        assert_eq!(&bytes[..2], &[0, 3]);
        assert_eq!(Source::from_payload(&bytes).unwrap(), source);
        assert_eq!(Source::parse(&source.to_text()).unwrap(), source);
        assert_eq!(Source::from_payload(&[0, 0]).unwrap(), Source::default());
    }

    #[test]
    fn malformed_framing_syntax_and_field_overflow_fail() {
        for bytes in [&[][..], &[0][..], &[0, 1][..], &[0, 0, 0][..]] {
            assert!(Source::from_payload(bytes).is_err());
        }
        for text in [
            ".exception begin=0x1000000 end=0x01 handler=0x00",
            ".exception begin=code:bad.name end=0x01 handler=0x00",
            ".exception end=0x00 begin=0x01 handler=0x00",
            ".exception begin=0x00 end=0x01",
        ] {
            assert!(Source::parse(text).is_err());
        }
        let entry = Entry {
            begin: Offset::Raw(0),
            end: Offset::Raw(1),
            handler: Offset::Raw(0),
        };
        assert!(
            Source {
                entries: vec![entry.clone(); 65536]
            }
            .encode(&BTreeMap::new())
            .is_err()
        );
        let bad = Source {
            entries: vec![Entry {
                begin: Offset::Raw(0x1000000),
                ..entry
            }],
        };
        assert!(bad.encode(&BTreeMap::new()).is_err());
    }

    #[test]
    fn named_references_resolve_and_only_end_roles_name_end_of_code() {
        let mut source = Source::parse(".exception begin=0x02 end=0x02 handler=0x02\n.exception begin=0x00 end=0x02 handler=0x00").unwrap();
        let names = BTreeMap::from([(0, "start".into()), (2, "end".into())]);
        source.name_references(&names, 2);
        assert_eq!(source.entries[0].begin, Offset::Raw(2));
        assert_eq!(source.entries[0].end, Offset::Code("end".into()));
        assert_eq!(source.entries[0].handler, Offset::Raw(2));
        assert!(source.encode(&BTreeMap::new()).is_err());
        let labels = BTreeMap::from([("start".into(), 0), ("end".into(), 2)]);
        let bytes = source.encode(&labels).unwrap();
        assert_eq!(
            Source::from_payload(&bytes).unwrap().entries[1].end,
            Offset::Raw(2)
        );
        assert_eq!(Source::parse(&source.to_text()).unwrap(), source);
    }
}
