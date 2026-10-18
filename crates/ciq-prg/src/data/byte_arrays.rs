//! Compact ByteArray constant records and editable DATA declarations.
use super::StringDeclaration;
use crate::symbols::source::{Error, valid_alias};
use std::fmt::Write;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declaration {
    pub alias: String,
    pub at: Option<u32>,
    /// Preserve an optional local pool, even though bytes do not reference it.
    pub pool: Option<Vec<StringDeclaration>>,
    pub values: Vec<u8>,
}

impl Declaration {
    /// Parse a ByteArray header and its optional pinned DATA offset.
    pub fn parse_header(fields: &[String]) -> Result<Self, Error> {
        if !(2..=3).contains(&fields.len()) || fields[0] != ".bytearray" || !valid_alias(&fields[1])
        {
            return Err(Error("expected .bytearray <alias> [at=0x...]".into()));
        }
        let at = fields
            .get(2)
            .map(|s| {
                super::address(
                    s.strip_prefix("at=")
                        .ok_or_else(|| Error("expected ByteArray placement".into()))?,
                )
            })
            .transpose()?;
        Ok(Self {
            alias: fields[1].clone(),
            at,
            pool: None,
            values: vec![],
        })
    }

    /// Append exact hexadecimal byte pairs or preserve a record-local pool.
    /// Byte values are already unsigned; this syntax performs no numeric casts.
    pub fn push_field(&mut self, fields: &[String]) -> Result<(), Error> {
        if fields == [".pool"] {
            if self.pool.is_some() {
                return Err(Error("duplicate ByteArray pool".into()));
            }
            self.pool = Some(vec![]);
        } else if fields.first().is_some_and(|s| s == ".string") {
            self.pool
                .get_or_insert_with(Vec::new)
                .push(StringDeclaration::parse(fields)?);
        } else if fields.len() == 2 && fields[0] == ".bytes" {
            let hex = &fields[1];
            if hex.is_empty() || hex.len() % 2 != 0 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err(Error("ByteArray bytes require hexadecimal pairs".into()));
            }
            self.values.extend(
                (0..hex.len())
                    .step_by(2)
                    .map(|at| u8::from_str_radix(&hex[at..at + 2], 16).unwrap()),
            );
        } else {
            return Err(Error("expected .bytes <hex>, .string, or .pool".into()));
        }
        Ok(())
    }

    /// Encode a compact byte payload with checked u32 lengths and local pools.
    /// Preserve pool layout and byte order; reject inconsistent pool placements.
    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        super::constants::encode_bytes(self.pool.as_deref(), &self.values)
    }

    /// Return the encoded record length after validating its pool and counts.
    pub fn encoded_length(&self) -> Result<usize, Error> {
        Ok(self.encode()?.len())
    }

    /// Format an editable ByteArray, wrapping raw payloads at 32 bytes per line.
    pub fn to_text(&self) -> String {
        let mut text = format!(".bytearray {}", self.alias);
        if let Some(at) = self.at {
            write!(text, " at=0x{at:08x}").unwrap();
        }
        text.push('\n');
        if let Some(pool) = &self.pool {
            if pool.is_empty() {
                text.push_str(".pool\n");
            }
            for s in pool {
                text.push_str(&s.to_text());
            }
        }
        for chunk in self.values.chunks(32) {
            text.push_str(".bytes ");
            for byte in chunk {
                write!(text, "{byte:02x}").unwrap();
            }
            text.push('\n');
        }
        text.push_str(".endbytearray\n");
        text
    }
}

/// Decode a ByteArray at a known DATA boundary, returning its exclusive end.
/// Check outer/body lengths, byte count, optional pool, and zero terminator.
/// Reject other roots and trailing bytes rather than scanning for another record.
pub fn byte_array_at(payload: &[u8], start: u32) -> Result<(Declaration, usize), Error> {
    let record = super::constants::decode(payload, start)?;
    if record.root != 0x14 {
        return Err(Error("expected ByteArray root tag".into()));
    }
    let end = record.end;
    Ok((from_record(start, record), end))
}

pub(super) fn from_record(start: u32, record: super::constants::Record) -> Declaration {
    Declaration {
        alias: format!("bytes_{start:08x}"),
        at: Some(start),
        pool: record.pool,
        values: record.bytes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{Source, classes};

    const RECORD: &[u8] = &[
        3, 0, 0, 0, 20, 0xda, 0x7a, 0xda, 0x7a, 0, 0, 0, 12, 0x14, 0, 0, 0, 7, 0, 1, 127, 128, 255,
        255, 128, 0,
    ];

    #[test]
    fn compact_records_validate_exact_counts_lengths_and_boundaries() {
        let (d, end) = byte_array_at(RECORD, 0).unwrap();
        assert_eq!(d.values, [0, 1, 127, 128, 255, 255, 128]);
        assert_eq!(end, RECORD.len());
        assert_eq!(d.encode().unwrap(), RECORD);
        assert_eq!(d.encoded_length().unwrap(), end);
        assert!(
            matches!(&classes::records(RECORD).unwrap()[0], classes::Record::ByteArray(a) if a == &d)
        );
        for len in 0..RECORD.len() {
            assert!(byte_array_at(&RECORD[..len], 0).is_err());
        }
        for at in [0, 4, 5, 12, 13, 17, 25] {
            let mut corrupt = RECORD.to_vec();
            corrupt[at] ^= 0xff;
            assert!(byte_array_at(&corrupt, 0).is_err(), "{at}");
        }
        assert!(byte_array_at(RECORD, 1).is_err());
        let mut trailing = RECORD.to_vec();
        trailing.insert(25, 42);
        trailing[4] += 1;
        trailing[12] += 1;
        assert!(byte_array_at(&trailing, 0).is_err());
        let source = Source::from_records(RECORD).unwrap();
        assert_eq!(
            Source::parse(&source.to_text())
                .unwrap()
                .encode()
                .unwrap()
                .0,
            RECORD
        );
    }

    #[test]
    fn data_layout_keeps_literal_namespaces_separate_and_moves_unpinned_records() {
        let source = Source::parse(".bytearray same\n.bytes 0001ff\n.endbytearray\n.array same\n.element number=0x2a\n.endarray\n.dictionary same\n.enddictionary\n.string same \"x\"\n").unwrap();
        let bytes = source.encode().unwrap().0;
        assert_eq!(source.byte_array_bindings().unwrap()["same"], 0);
        assert_eq!(source.array_bindings().unwrap()["same"], 22);
        assert_eq!(source.dictionary_bindings().unwrap()["same"], 46);
        assert_eq!(source.string_bindings().unwrap()["same"], 65);
        assert_eq!(
            Source::from_records(&bytes).unwrap().encode().unwrap().0,
            bytes
        );
        let empty = Source::parse(".bytearray empty\n.endbytearray\n")
            .unwrap()
            .encode()
            .unwrap()
            .0;
        assert_eq!(byte_array_at(&empty, 0).unwrap().0.values, []);
        for bad in [
            ".bytearray x\n.bytes f\n.endbytearray",
            ".bytearray x\n.bytes gg\n.endbytearray",
            ".bytearray x\n.bytes 100\n.endbytearray",
            ".bytearray x\n.element number=0x1\n.endbytearray",
            ".bytearray x\n.bytes 00",
            ".bytearray x at=0x1\n.endbytearray",
            ".bytearray x at=0x1000000\n.endbytearray",
            ".bytearray x\n.bytearray y\n.endbytearray",
            ".bytearray x\n.endbytearray\n.bytearray x\n.endbytearray",
        ] {
            assert!(Source::parse(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn optional_unused_pools_and_long_payloads_roundtrip_without_reinterpretation() {
        let source = Source::parse(".bytearray bytes\n.string unused \"text\"\n.bytes 00ff\n.endbytearray\n.bytearray empty\n.pool\n.endbytearray").unwrap();
        let bytes = source.encode().unwrap().0;
        let decoded = Source::from_records(&bytes).unwrap();
        assert_eq!(decoded.encode().unwrap().0, bytes);
        assert_eq!(
            Source::parse(&decoded.to_text())
                .unwrap()
                .encode()
                .unwrap()
                .0,
            bytes
        );
        let d = Declaration {
            alias: "all".into(),
            at: None,
            pool: None,
            values: (0..=255).collect(),
        };
        assert_eq!(
            d.to_text()
                .lines()
                .filter(|l| l.starts_with(".bytes "))
                .count(),
            8
        );
        assert_eq!(
            Source::parse(&d.to_text()).unwrap().encode().unwrap().0,
            d.encode().unwrap()
        );
        assert!(Source::parse(".bytearray x\n.pool\n.pool\n.endbytearray").is_err());
    }
}
