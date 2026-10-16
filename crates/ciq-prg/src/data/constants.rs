//! Shared flat constant-container framing and record-local scalar values.
use super::StringDeclaration;
use crate::symbols::source::{Error, valid_alias};
use std::collections::BTreeMap;

pub(super) struct Record {
    pub root: u8,
    pub pool: Option<Vec<StringDeclaration>>,
    pub values: Vec<Value>,
    pub end: usize,
}

/// Scalar in a flat Array or Dictionary record, retaining raw Number/Char bits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Null,
    Number(u32),
    Char(u32),
    Boolean(bool),
    /// Alias of a string in this record's local pool, not a DATA string alias.
    String(String),
}

impl Value {
    pub(super) fn parse(s: &str) -> Result<Self, Error> {
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
            .ok_or_else(|| Error("invalid constant scalar".into()))?;
        if bits.is_empty() || !bits.bytes().all(|c| c.is_ascii_hexdigit()) {
            return Err(Error("invalid constant bit pattern".into()));
        }
        let bits = u32::from_str_radix(bits, 16)
            .map_err(|_| Error("constant scalar exceeds u32".into()))?;
        match kind {
            "number" => Ok(Self::Number(bits)),
            "char" => Ok(Self::Char(bits)),
            _ => Err(Error("unsupported constant scalar".into())),
        }
    }
    pub(super) fn text(&self) -> String {
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
            .ok_or_else(|| Error("constant length overflow".into()))?;
        let result = self
            .bytes
            .get(self.at..end)
            .ok_or_else(|| Error("truncated constant record".into()))?;
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
                _ => return Err(Error("invalid constant Boolean".into())),
            },
            3 => Value::String(
                pool.get(&self.word()?)
                    .ok_or_else(|| {
                        Error("constant string reference is not a pool boundary".into())
                    })?
                    .clone(),
            ),
            _ => return Err(Error("unsupported constant value tag".into())),
        })
    }
}

fn pool_bytes(
    pool: Option<&[StringDeclaration]>,
) -> Result<(Vec<u8>, BTreeMap<String, u32>), Error> {
    let mut bytes = Vec::new();
    let mut offsets = BTreeMap::new();
    for s in pool.into_iter().flatten() {
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
            .ok_or_else(|| Error("constant string plus terminator exceeds u16".into()))?;
        bytes.extend(length.to_be_bytes());
        bytes.extend(s.value.as_bytes());
        bytes.push(0);
        u32::try_from(bytes.len()).map_err(|_| Error("constant string pool exceeds u32".into()))?;
    }
    Ok((bytes, offsets))
}

/// Encode the supplied root/count and ordered scalars with a record-local pool.
/// Derive block lengths and reject invalid pool placement or String references.
pub(super) fn encode<'a>(
    pool: Option<&[StringDeclaration]>,
    root: u8,
    count: usize,
    values: impl IntoIterator<Item = &'a Value>,
) -> Result<Vec<u8>, Error> {
    let (pool_data, offsets) = pool_bytes(pool)?;
    let mut body = Vec::new();
    body.push(root);
    body.extend(
        u32::try_from(count)
            .map_err(|_| Error("constant entry count exceeds u32".into()))?
            .to_be_bytes(),
    );
    for value in values {
        value.encode(&mut body, &offsets)?;
    }
    let mut data = Vec::new();
    if pool.is_some() {
        data.extend(0xabcd_abcdu32.to_be_bytes());
        data.extend(
            u32::try_from(pool_data.len())
                .map_err(|_| Error("constant pool exceeds u32".into()))?
                .to_be_bytes(),
        );
        data.extend(pool_data);
    }
    data.extend(0xda7a_da7au32.to_be_bytes());
    data.extend(
        u32::try_from(body.len())
            .map_err(|_| Error("constant body exceeds u32".into()))?
            .to_be_bytes(),
    );
    data.extend(body);
    let mut out = vec![3];
    out.extend(
        u32::try_from(data.len())
            .map_err(|_| Error("constant record exceeds u32".into()))?
            .to_be_bytes(),
    );
    out.extend(data);
    out.push(0);
    Ok(out)
}

/// Decode one flat Array or Dictionary at a known DATA boundary.
/// Validate the complete framing, pool references, scalar count, and terminator;
/// reject unsupported roots/nested values without scanning for another record.
pub(super) fn decode(payload: &[u8], start: u32) -> Result<Record, Error> {
    let mut reader = Reader {
        bytes: payload
            .get(start as usize..)
            .filter(|_| (start as usize) < super::ADDRESS_LIMIT)
            .ok_or_else(|| Error("unsupported constant DATA address".into()))?,
        at: 0,
    };
    if reader.take(1)? != [3] {
        return Err(Error("unsupported constant outer tag".into()));
    }
    let length = reader.word()? as usize;
    let content = reader.take(length)?;
    if reader.take(1)? != [0] {
        return Err(Error("constant record lacks terminator".into()));
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
                return Err(Error("constant pool string lacks terminator".into()));
            }
            let value = std::str::from_utf8(&bytes[..size - 1])
                .map_err(|_| Error("constant pool string is not UTF-8".into()))?;
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
        return Err(Error("unsupported constant data magic".into()));
    }
    let length = reader.word()? as usize;
    if length != reader.bytes.len() - reader.at {
        return Err(Error(
            "inconsistent constant body length or root tag".into(),
        ));
    }
    let root = reader.take(1)?[0];
    let arity = match root {
        5 => 1,
        0x0b => 2,
        _ => return Err(Error("unsupported constant root tag".into())),
    };
    let count = (reader.word()? as usize)
        .checked_mul(arity)
        .ok_or_else(|| Error("constant count overflow".into()))?;
    if count > reader.bytes.len() - reader.at {
        return Err(Error("constant entry count exceeds record".into()));
    }
    let mut values = Vec::new();
    for _ in 0..count {
        values.push(reader.value(&strings)?);
    }
    if reader.at != reader.bytes.len() {
        return Err(Error("constant record has trailing bytes".into()));
    }
    Ok(Record {
        root,
        pool,
        values,
        end,
    })
}
