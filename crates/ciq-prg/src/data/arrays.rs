//! Flat scalar constant-array records and their editable DATA declarations.

use super::StringDeclaration;
use crate::symbols::source::{Error, valid_alias};
use std::fmt::Write;

pub use super::constants::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declaration {
    pub alias: String,
    pub at: Option<u32>,
    /// None omits the pool block; Some retains it even if empty or unused.
    pub pool: Option<Vec<StringDeclaration>>,
    pub values: Vec<Value>,
}

impl Declaration {
    /// Parse an array declaration header, retaining its optional pinned offset.
    pub fn parse_header(fields: &[String]) -> Result<Self, Error> {
        if !(2..=3).contains(&fields.len()) || fields[0] != ".array" || !valid_alias(&fields[1]) {
            return Err(Error("expected .array <alias> [at=0x...]".into()));
        }
        let at = fields
            .get(2)
            .map(|s| {
                super::address(
                    s.strip_prefix("at=")
                        .ok_or_else(|| Error("expected array placement".into()))?,
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

    /// Append a typed scalar element, local .string, or explicit empty .pool.
    /// String references name this record's pool, independently of DATA strings.
    pub fn push_element(&mut self, fields: &[String]) -> Result<(), Error> {
        if fields == [".pool"] {
            if self.pool.is_some() {
                return Err(Error("duplicate array pool".into()));
            }
            self.pool = Some(vec![]);
        } else if fields.first().is_some_and(|s| s == ".string") {
            self.pool
                .get_or_insert_with(Vec::new)
                .push(StringDeclaration::parse(fields)?);
        } else if fields.len() == 2 && fields[0] == ".element" {
            self.values.push(Value::parse(&fields[1])?);
        } else {
            return Err(Error("expected .string, .pool, or .element <value>".into()));
        }
        Ok(())
    }

    /// Calculate the record size, validating scalar values and pool references.
    /// Lengths/counts must fit u32; each local String plus terminator must fit u16.
    pub fn encoded_length(&self) -> Result<usize, Error> {
        Ok(self.encode()?.len())
    }

    /// Encode a flat scalar array in declared order, preserving local strings.
    /// Derive lengths/counts; reject missing aliases and inconsistent placements.
    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        super::constants::encode(self.pool.as_deref(), 5, self.values.len(), &self.values)
    }

    /// Format an editable record with distinct DATA and pool-relative placements.
    pub fn to_text(&self) -> String {
        let mut text = format!(".array {}", self.alias);
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
        for value in &self.values {
            writeln!(text, ".element {}", value.text()).unwrap();
        }
        text.push_str(".endarray\n");
        text
    }
}

/// Decode a flat scalar Array at a known DATA boundary and return its end.
/// Check lengths, scalar tags, UTF-8 pool boundaries/references, element count,
/// and the trailing terminator. Reject unsupported nested containers.
pub fn array_at(payload: &[u8], start: u32) -> Result<(Declaration, usize), Error> {
    let record = super::constants::decode(payload, start)?;
    if record.root != 5 {
        return Err(Error("expected Array root tag".into()));
    }
    let end = record.end;
    Ok((from_record(start, record), end))
}

pub(super) fn from_record(start: u32, record: super::constants::Record) -> Declaration {
    Declaration {
        alias: format!("array_{start:08x}"),
        at: Some(start),
        pool: record.pool,
        values: record.values,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{Fragment, Source, classes};

    #[test]
    fn observed_number_record_lengths_exclude_the_terminator() {
        let bytes = [
            3, 0, 0, 0, 28, 0xda, 0x7a, 0xda, 0x7a, 0, 0, 0, 20, 5, 0, 0, 0, 3, 1, 0, 0, 0, 10, 1,
            0, 0, 0, 20, 1, 0, 0, 0, 30, 0,
        ];
        let (array, end) = array_at(&bytes, 0).unwrap();
        assert_eq!(end, bytes.len());
        assert_eq!(
            array.values,
            [Value::Number(10), Value::Number(20), Value::Number(30)]
        );
        assert_eq!(array.encode().unwrap(), bytes);
        for length in 0..bytes.len() {
            assert!(array_at(&bytes[..length], 0).is_err());
        }
        for index in [0, 4, 5, 12, 13, 17, 18, 33] {
            let mut bad = bytes;
            bad[index] = 0xff;
            assert!(array_at(&bad, 0).is_err(), "accepted corrupt byte {index}");
        }
    }

    #[test]
    fn scalar_text_roundtrip_preserves_types_bits_and_following_record_boundaries() {
        let text = ".array values\n.element null\n.element number=0xffffffff\n.element boolean=true\n.element boolean=false\n.element char=0x00000061\n.endarray\n.string after \"tail\"\n";
        let source = Source::parse(text).unwrap();
        let (bytes, strings) = source.encode().unwrap();
        assert_eq!(source.array_bindings().unwrap()["values"], 0);
        assert_eq!(strings["after"], 34);
        let records = classes::records(&bytes).unwrap();
        assert!(
            matches!(&records[0], classes::Record::Array(a) if a.values[1] == Value::Number(u32::MAX))
        );
        assert!(matches!(
            &records[1],
            classes::Record::String {
                offset: 34,
                value: "tail"
            }
        ));
        let restored = Source::from_records(&bytes).unwrap();
        assert_eq!(
            Source::parse(&restored.to_text())
                .unwrap()
                .encode()
                .unwrap()
                .0,
            bytes
        );
        let Fragment::Array(a) = &source.fragments[0] else {
            panic!()
        };
        let mut bad = a.encode().unwrap();
        bad[25] = 2;
        assert!(array_at(&bad, 0).is_err());
        for bad in [
            ".array a\n",
            ".array a\n.class x\n.endclass\n.endarray\n",
            ".array a\n.element string=0x01\n.endarray\n",
            ".array a\n.element boolean=2\n.endarray\n",
            ".array a\n.element number=0x100000000\n.endarray\n",
            ".array a\n.endarray\n.array a\n.endarray\n",
            ".array a at=0x01\n.endarray\n",
        ] {
            assert!(Source::parse(bad).is_err(), "accepted {bad}");
        }
    }
}
