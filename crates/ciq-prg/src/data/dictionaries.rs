//! Flat scalar Dictionary constants with record-local string pools.
use super::StringDeclaration;
use crate::symbols::source::{Error, valid_alias};
use std::collections::BTreeMap;
use std::fmt::Write;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Null,
    Number(u32),
    Char(u32),
    Boolean(bool),
    /// Alias of a string in this record's local pool, not a DATA string alias.
    String(String),
}

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

    fn pool_bytes(&self) -> Result<(Vec<u8>, BTreeMap<String, u32>), Error> {
        let mut bytes = Vec::new();
        let mut offsets = BTreeMap::new();
        for s in self.pool.iter().flatten() {
            if !valid_alias(&s.alias)
                || offsets
                    .insert(s.alias.clone(), bytes.len() as u32)
                    .is_some()
            {
                return Err(Error("invalid or duplicate local string alias".into()));
            }
            if s.at.is_some_and(|at| at as usize != bytes.len()) {
                return Err(Error(
                    "local string placement must match next pool offset".into(),
                ));
            }
            let length = s
                .value
                .len()
                .checked_add(1)
                .and_then(|n| u16::try_from(n).ok())
                .ok_or_else(|| Error("Dictionary string plus terminator exceeds u16".into()))?;
            bytes.extend(length.to_be_bytes());
            bytes.extend(s.value.as_bytes());
            bytes.push(0);
            u32::try_from(bytes.len())
                .map_err(|_| Error("Dictionary string pool exceeds u32".into()))?;
        }
        Ok((bytes, offsets))
    }

    /// Encode entries in their declared order, preserving local string identity.
    /// Derive lengths/counts; reject missing aliases and inconsistent placements.
    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        let (pool, offsets) = self.pool_bytes()?;
        let mut body = Vec::new();
        body.push(0x0b);
        body.extend(
            u32::try_from(self.entries.len())
                .map_err(|_| Error("Dictionary entry count exceeds u32".into()))?
                .to_be_bytes(),
        );
        for (key, value) in &self.entries {
            key.encode(&mut body, &offsets)?;
            value.encode(&mut body, &offsets)?;
        }
        let mut data = Vec::new();
        if self.pool.is_some() {
            data.extend(0xabcd_abcdu32.to_be_bytes());
            data.extend(
                u32::try_from(pool.len())
                    .map_err(|_| Error("Dictionary pool exceeds u32".into()))?
                    .to_be_bytes(),
            );
            data.extend(pool);
        }
        data.extend(0xda7a_da7au32.to_be_bytes());
        data.extend(
            u32::try_from(body.len())
                .map_err(|_| Error("Dictionary body exceeds u32".into()))?
                .to_be_bytes(),
        );
        data.extend(body);
        let mut out = vec![3];
        out.extend(
            u32::try_from(data.len())
                .map_err(|_| Error("Dictionary record exceeds u32".into()))?
                .to_be_bytes(),
        );
        out.extend(data);
        out.push(0);
        Ok(out)
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

impl Value {
    fn parse(s: &str) -> Result<Self, Error> {
        if s == "null" {
            return Ok(Self::Null);
        }
        if let Some(alias) = s.strip_prefix("string=&") {
            if !valid_alias(alias) {
                return Err(Error("invalid local string reference".into()));
            }
            return Ok(Self::String(alias.into()));
        }
        if s == "boolean=true" {
            return Ok(Self::Boolean(true));
        }
        if s == "boolean=false" {
            return Ok(Self::Boolean(false));
        }
        let (kind, bits) = s
            .split_once("=0x")
            .ok_or_else(|| Error("invalid Dictionary scalar".into()))?;
        if bits.is_empty() || !bits.bytes().all(|c| c.is_ascii_hexdigit()) {
            return Err(Error("invalid Dictionary bit pattern".into()));
        }
        let bits = u32::from_str_radix(bits, 16)
            .map_err(|_| Error("Dictionary scalar exceeds u32".into()))?;
        match kind {
            "number" => Ok(Self::Number(bits)),
            "char" => Ok(Self::Char(bits)),
            _ => Err(Error("unsupported Dictionary scalar".into())),
        }
    }
    fn text(&self) -> String {
        match self {
            Self::Null => "null".into(),
            Self::Boolean(b) => format!("boolean={b}"),
            Self::Number(n) => format!("number=0x{n:08x}"),
            Self::Char(n) => format!("char=0x{n:08x}"),
            Self::String(s) => format!("string=&{s}"),
        }
    }
    fn encode(&self, out: &mut Vec<u8>, strings: &BTreeMap<String, u32>) -> Result<(), Error> {
        match self {
            Self::Null => out.push(0),
            Self::Boolean(b) => out.extend([9, u8::from(*b)]),
            Self::Number(n) | Self::Char(n) => {
                out.push(if matches!(self, Self::Number(_)) {
                    1
                } else {
                    0x13
                });
                out.extend(n.to_be_bytes());
            }
            Self::String(alias) => {
                out.push(3);
                out.extend(
                    strings
                        .get(alias)
                        .ok_or_else(|| Error(format!("undefined local string alias {alias}")))?
                        .to_be_bytes(),
                );
            }
        }
        Ok(())
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl<'a> Reader<'a> {
    fn take(&mut self, length: usize) -> Result<&'a [u8], Error> {
        let end = self
            .at
            .checked_add(length)
            .ok_or_else(|| Error("Dictionary length overflow".into()))?;
        let result = self
            .bytes
            .get(self.at..end)
            .ok_or_else(|| Error("truncated Dictionary record".into()))?;
        self.at = end;
        Ok(result)
    }
    fn word(&mut self) -> Result<u32, Error> {
        Ok(u32::from_be_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn value(&mut self, pool: &BTreeMap<u32, String>) -> Result<Value, Error> {
        Ok(match self.take(1)?[0] {
            0 => Value::Null,
            1 => Value::Number(self.word()?),
            0x13 => Value::Char(self.word()?),
            9 => match self.take(1)?[0] {
                0 => Value::Boolean(false),
                1 => Value::Boolean(true),
                _ => return Err(Error("invalid Dictionary Boolean".into())),
            },
            3 => Value::String(
                pool.get(&self.word()?)
                    .ok_or_else(|| {
                        Error("Dictionary string reference is not a pool boundary".into())
                    })?
                    .clone(),
            ),
            _ => return Err(Error("unsupported Dictionary value tag".into())),
        })
    }
}

/// Decode a Dictionary at a known DATA boundary. Check lengths, scalar tags,
/// UTF-8 string boundaries/references, entry count, and the trailing terminator.
/// Never scan ahead or reinterpret unsupported nested values as scalar data.
pub fn dictionary_at(payload: &[u8], start: u32) -> Result<(Declaration, usize), Error> {
    let mut reader = Reader {
        bytes: payload
            .get(start as usize..)
            .filter(|_| (start as usize) < super::ADDRESS_LIMIT)
            .ok_or_else(|| Error("unsupported Dictionary DATA address".into()))?,
        at: 0,
    };
    if reader.take(1)? != [3] {
        return Err(Error("unsupported Dictionary outer tag".into()));
    }
    let length = reader.word()? as usize;
    let content = reader.take(length)?;
    if reader.take(1)? != [0] {
        return Err(Error("Dictionary record lacks terminator".into()));
    }
    let end = start as usize + reader.at;
    let mut reader = Reader {
        bytes: content,
        at: 0,
    };
    let mut pool = None;
    let mut strings = BTreeMap::new();
    let mut magic = reader.word()?;
    if magic == 0xabcd_abcd {
        let length = reader.word()? as usize;
        let mut local = Reader {
            bytes: reader.take(length)?,
            at: 0,
        };
        let mut declarations = vec![];
        while local.at < local.bytes.len() {
            let at = local.at as u32;
            let size = u16::from_be_bytes(local.take(2)?.try_into().unwrap()) as usize;
            let bytes = local.take(size)?;
            if bytes.last() != Some(&0) {
                return Err(Error("Dictionary pool string lacks terminator".into()));
            }
            let value = std::str::from_utf8(&bytes[..size - 1])
                .map_err(|_| Error("Dictionary pool string is not UTF-8".into()))?;
            let alias = format!("pool_{at:08x}");
            strings.insert(at, alias.clone());
            declarations.push(StringDeclaration {
                alias,
                at: Some(at),
                value: value.into(),
            });
        }
        pool = Some(declarations);
        magic = reader.word()?;
    }
    if magic != 0xda7a_da7a {
        return Err(Error("unsupported Dictionary data magic".into()));
    }
    let length = reader.word()? as usize;
    if length != reader.bytes.len() - reader.at || reader.take(1)? != [0x0b] {
        return Err(Error(
            "inconsistent Dictionary body length or root tag".into(),
        ));
    }
    let count = reader.word()? as usize;
    if count > (reader.bytes.len() - reader.at) / 2 {
        return Err(Error("Dictionary entry count exceeds record".into()));
    }
    let mut entries = Vec::new();
    for _ in 0..count {
        entries.push((reader.value(&strings)?, reader.value(&strings)?));
    }
    if reader.at != reader.bytes.len() {
        return Err(Error("Dictionary record has trailing bytes".into()));
    }
    Ok((
        Declaration {
            alias: format!("dict_{start:08x}"),
            at: Some(start),
            pool,
            entries,
        },
        end,
    ))
}
