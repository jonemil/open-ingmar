//! Flat scalar Dictionary constants with record-local string pools.
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
    pub entries: Vec<(Value, Value)>,
}

impl Declaration {
    /// Parse a Dictionary header and optional pinned DATA address.
    pub fn parse_header(fields: &[String]) -> Result<Self, Error> {
        if !(2..=3).contains(&fields.len())
            || fields[0] != ".dictionary"
            || !valid_alias(&fields[1])
        {
            return Err(Error("expected .dictionary <alias> [at=0x...]".into()));
        }
        let at = fields
            .get(2)
            .map(|s| {
                super::address(
                    s.strip_prefix("at=")
                        .ok_or_else(|| Error("expected Dictionary placement".into()))?,
                )
            })
            .transpose()?;
        Ok(Self {
            alias: fields[1].clone(),
            at,
            pool: None,
            entries: vec![],
        })
    }

    /// Append a local .string, an explicit empty .pool, or a typed entry.
    pub fn push_field(&mut self, fields: &[String]) -> Result<(), Error> {
        if fields == [".pool"] {
            if self.pool.is_some() {
                return Err(Error("duplicate Dictionary pool".into()));
            }
            self.pool = Some(vec![]);
        } else if fields.first().is_some_and(|s| s == ".string") {
            self.pool
                .get_or_insert_with(Vec::new)
                .push(StringDeclaration::parse(fields)?);
        } else if fields.len() == 3 && fields[0] == ".entry" {
            self.entries
                .push((Value::parse(&fields[1])?, Value::parse(&fields[2])?));
        } else {
            return Err(Error(
                "expected .string, .pool, or .entry <key> <value>".into(),
            ));
        }
        Ok(())
    }

    /// Encode entries in their declared order, preserving local string identity.
    /// Derive lengths/counts; reject missing aliases and inconsistent placements.
    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        super::constants::encode(
            self.pool.as_deref(),
            0x0b,
            self.entries.len(),
            self.entries.iter().flat_map(|(k, v)| [k, v]),
        )
    }

    /// Return the serialized size after validating entries and pool references.
    pub fn encoded_length(&self) -> Result<usize, Error> {
        Ok(self.encode()?.len())
    }

    /// Format an editable record with distinct DATA and pool-relative placements.
    pub fn to_text(&self) -> String {
        let mut out = format!(".dictionary {}", self.alias);
        if let Some(at) = self.at {
            write!(out, " at=0x{at:08x}").unwrap();
        }
        out.push('\n');
        if let Some(pool) = &self.pool {
            if pool.is_empty() {
                out.push_str(".pool\n");
            }
            for s in pool {
                out.push_str(&s.to_text());
            }
        }
        for (key, value) in &self.entries {
            writeln!(out, ".entry {} {}", key.text(), value.text()).unwrap();
        }
        out.push_str(".enddictionary\n");
        out
    }
}

/// Decode a Dictionary at a known DATA boundary. Check lengths, scalar tags,
/// UTF-8 string boundaries/references, entry count, and the trailing terminator.
/// Never scan ahead or reinterpret unsupported nested values as scalar data.
pub fn dictionary_at(payload: &[u8], start: u32) -> Result<(Declaration, usize), Error> {
    let record = super::constants::decode(payload, start)?;
    if record.root != 0x0b {
        return Err(Error("expected Dictionary root tag".into()));
    }
    let end = record.end;
    Ok((from_record(start, record), end))
}

pub(super) fn from_record(start: u32, record: super::constants::Record) -> Declaration {
    let mut values = record.values.into_iter();
    let mut entries = Vec::new();
    while let Some(key) = values.next() {
        entries.push((
            key,
            values
                .next()
                .expect("Dictionary decoder checked pair count"),
        ));
    }
    Declaration {
        alias: format!("dict_{start:08x}"),
        at: Some(start),
        pool: record.pool,
        entries,
    }
}
