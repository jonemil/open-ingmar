//! Referenced DATA strings and opaque spans; see docs/specs/data-strings.md.

use crate::symbols::source::{Error, tokens, valid_alias};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

pub mod classes;

pub const ADDRESS_LIMIT: usize = 0x0100_0000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringDeclaration {
    pub alias: String,
    pub value: String,
    pub at: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fragment {
    Bytes { at: u32, bytes: Vec<u8> },
    String(StringDeclaration),
    Class(classes::source::Declaration),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Source {
    pub fragments: Vec<Fragment>,
}

pub fn address(value: &str) -> Result<u32, Error> {
    let hex = value
        .strip_prefix("0x")
        .ok_or_else(|| Error("address must start with 0x".into()))?;
    if hex.is_empty() || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(Error("invalid hexadecimal address".into()));
    }
    let value = u32::from_str_radix(hex, 16).map_err(|_| Error("address exceeds u32".into()))?;
    if value as usize >= ADDRESS_LIMIT {
        return Err(Error(
            "only zero-prefix DATA addresses are supported".into(),
        ));
    }
    Ok(value)
}

/// Decode exactly one referenced record; never scan opaque data for apparent tags.
pub fn string_at(payload: &[u8], at: u32) -> Result<(&str, usize), Error> {
    let start = at as usize;
    if start >= ADDRESS_LIMIT || start > payload.len() || payload.len() - start < 4 {
        return Err(Error("truncated or unsupported DATA string address".into()));
    }
    if payload[start] != 1 {
        return Err(Error("unsupported DATA string tag".into()));
    }
    let length = u16::from_be_bytes(payload[start + 1..start + 3].try_into().unwrap()) as usize;
    if length > payload.len() - start - 4 {
        return Err(Error("DATA string exceeds payload".into()));
    }
    let end = start + 3 + length;
    if payload[end] != 0 {
        return Err(Error("DATA string lacks zero terminator".into()));
    }
    let text = std::str::from_utf8(&payload[start + 3..end])
        .map_err(|_| Error("DATA string is not UTF-8".into()))?;
    Ok((text, end + 1))
}

/// Walk known record boundaries while keeping class/module fields uninterpreted.
/// An unknown record aborts this walk; callers can still use validated code references.
pub fn record_string_offsets(payload: &[u8]) -> Result<Vec<u32>, Error> {
    classes::records(payload)?
        .into_iter()
        .filter_map(|record| match record {
            classes::Record::String { offset, .. } => {
                Some(u32::try_from(offset).map_err(|_| Error("DATA offset exceeds u32".into())))
            }
            classes::Record::Class(_) => None,
        })
        .collect()
}

impl StringDeclaration {
    pub fn parse(fields: &[String]) -> Result<Self, Error> {
        if !(fields.len() == 3 || fields.len() == 4)
            || fields[0] != ".string"
            || !valid_alias(&fields[1])
            || !fields[2].starts_with('"')
        {
            return Err(Error(
                "expected .string <alias> \"value\" [at=0x...]".into(),
            ));
        }
        let at = fields
            .get(3)
            .map(|field| {
                address(
                    field
                        .strip_prefix("at=")
                        .ok_or_else(|| Error("expected at=0x... placement".into()))?,
                )
            })
            .transpose()?;
        let value = fields[2][1..].to_owned();
        if value.len() > u16::MAX as usize {
            return Err(Error("string UTF-8 byte length exceeds u16".into()));
        }
        Ok(Self {
            alias: fields[1].clone(),
            value,
            at,
        })
    }

    pub fn to_text(&self) -> String {
        let mut text = format!(".string {} {:?}", self.alias, self.value);
        if let Some(at) = self.at {
            write!(text, " at=0x{at:08x}").unwrap();
        }
        text.push('\n');
        text
    }
}

impl Source {
    pub fn parse(text: &str) -> Result<Self, Error> {
        let mut source = Self::default();
        let mut class: Option<classes::source::Declaration> = None;
        for (index, line) in text.lines().enumerate() {
            let mut parse = || -> Result<Option<Fragment>, Error> {
                let fields = tokens(line)?;
                if fields.is_empty() {
                    return Ok(None);
                }
                if let Some(declaration) = class.as_mut() {
                    if fields == [".endclass"] {
                        return Ok(Some(Fragment::Class(class.take().unwrap())));
                    }
                    declaration.push_field(&fields)?;
                    return Ok(None);
                }
                if fields[0] == ".class" {
                    class = Some(classes::source::Declaration::parse_header(&fields)?);
                    return Ok(None);
                }
                if fields[0] == ".string" {
                    return Ok(Some(Fragment::String(StringDeclaration::parse(&fields)?)));
                }
                if fields.len() != 3 || fields[0] != ".bytes" {
                    return Err(Error(
                        "expected .class, .string, or .bytes at=0x... <hex-bytes>".into(),
                    ));
                }
                let at = address(
                    fields[1]
                        .strip_prefix("at=")
                        .ok_or_else(|| Error("expected at=0x... placement".into()))?,
                )?;
                let hex = &fields[2];
                if hex.is_empty()
                    || hex.len() % 2 != 0
                    || !hex.bytes().all(|b| b.is_ascii_hexdigit())
                {
                    return Err(Error(
                        "raw bytes require pairs of hexadecimal digits".into(),
                    ));
                }
                let bytes = (0..hex.len())
                    .step_by(2)
                    .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
                    .collect();
                Ok(Some(Fragment::Bytes { at, bytes }))
            };
            if let Some(fragment) =
                parse().map_err(|e| Error(format!("DATA line {}: {e}", index + 1)))?
            {
                source.fragments.push(fragment);
            }
        }
        if class.is_some() {
            return Err(Error("unterminated .class declaration".into()));
        }
        source.encode()?;
        Ok(source)
    }

    /// Serialize a complete supported DATA walk, retaining all class keys and references.
    pub fn from_records(payload: &[u8]) -> Result<Self, Error> {
        let mut source = Self::default();
        for record in classes::records(payload)? {
            source.fragments.push(match record {
                classes::Record::String { offset, value } => Fragment::String(StringDeclaration {
                    alias: format!("str_{offset:08x}"),
                    value: value.into(),
                    at: Some(
                        u32::try_from(offset)
                            .map_err(|_| Error("DATA offset exceeds u32".into()))?,
                    ),
                }),
                classes::Record::Class(class) => {
                    Fragment::Class(classes::source::Declaration::from_record(&class)?)
                }
            });
        }
        if source.encode()?.0 != payload {
            return Err(Error("structured DATA representation changed bytes".into()));
        }
        Ok(source)
    }

    /// Build a partial structural representation using instruction-derived addresses.
    pub fn from_references(
        payload: &[u8],
        addresses: impl IntoIterator<Item = u32>,
    ) -> Result<Self, Error> {
        let mut source = Self::default();
        let mut current = 0;
        let addresses: BTreeSet<_> = addresses.into_iter().collect();
        for at in addresses {
            let (value, end) = string_at(payload, at)?;
            if (at as usize) < current {
                return Err(Error("overlapping referenced string records".into()));
            }
            if at as usize > current {
                source.fragments.push(Fragment::Bytes {
                    at: current as u32,
                    bytes: payload[current..at as usize].to_vec(),
                });
            }
            source.fragments.push(Fragment::String(StringDeclaration {
                alias: format!("str_{at:08x}"),
                value: value.into(),
                at: Some(at),
            }));
            current = end;
        }
        if current < payload.len() {
            source.fragments.push(Fragment::Bytes {
                at: current as u32,
                bytes: payload[current..].to_vec(),
            });
        }
        // Verify both constraints and address representability before exporting.
        if source.encode()?.0 != payload {
            return Err(Error("DATA representation changed bytes".into()));
        }
        Ok(source)
    }

    pub fn encode(&self) -> Result<(Vec<u8>, BTreeMap<String, u32>), Error> {
        let mut payload = Vec::new();
        let mut bindings = BTreeMap::new();
        for fragment in &self.fragments {
            let at = match fragment {
                Fragment::Bytes { at, .. } => Some(*at),
                Fragment::String(s) => s.at,
                Fragment::Class(c) => Some(c.at),
            };
            if at.is_some_and(|at| at as usize != payload.len()) {
                return Err(Error(format!(
                    "pinned DATA offset does not match next offset 0x{:08x}; relocation is unsupported",
                    payload.len()
                )));
            }
            match fragment {
                Fragment::Class(c) => {
                    let bytes = c.encode()?;
                    if bytes.len() > ADDRESS_LIMIT - payload.len() {
                        return Err(Error("DATA payload exceeds supported address range".into()));
                    }
                    payload.extend(bytes);
                }
                Fragment::Bytes { bytes, .. } => {
                    if bytes.len() > ADDRESS_LIMIT - payload.len() {
                        return Err(Error("DATA payload exceeds supported address range".into()));
                    }
                    payload.extend_from_slice(bytes);
                }
                Fragment::String(s) => {
                    if !valid_alias(&s.alias) || bindings.contains_key(&s.alias) {
                        return Err(Error(format!(
                            "invalid or duplicate string alias {}",
                            s.alias
                        )));
                    }
                    if s.value.len() > u16::MAX as usize {
                        return Err(Error("string UTF-8 byte length exceeds u16".into()));
                    }
                    if s.value.len() + 4 > ADDRESS_LIMIT - payload.len() {
                        return Err(Error("DATA payload exceeds supported address range".into()));
                    }
                    bindings.insert(s.alias.clone(), payload.len() as u32);
                    payload.push(1);
                    payload.extend_from_slice(&(s.value.len() as u16).to_be_bytes());
                    payload.extend_from_slice(s.value.as_bytes());
                    payload.push(0);
                }
            }
        }
        Ok((payload, bindings))
    }

    pub fn to_text(&self) -> String {
        let mut text = String::new();
        for fragment in &self.fragments {
            match fragment {
                Fragment::String(s) => text.push_str(&s.to_text()),
                Fragment::Class(c) => text.push_str(&c.to_text()),
                Fragment::Bytes { at, bytes } => {
                    for (index, chunk) in bytes.chunks(32).enumerate() {
                        write!(text, ".bytes at=0x{:08x} ", *at as usize + index * 32).unwrap();
                        for byte in chunk {
                            write!(text, "{byte:02x}").unwrap();
                        }
                        text.push('\n');
                    }
                }
            }
        }
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authored_strings_allocate_by_order_and_utf8_byte_length() {
        let source = Source::parse(".string first \"λ#\\n\\0\"\n.string second \"\"\n").unwrap();
        let (bytes, bindings) = source.encode().unwrap();
        assert_eq!(bindings["first"], 0);
        assert_eq!(bindings["second"], 9);
        assert_eq!(string_at(&bytes, 0).unwrap().0, "λ#\n\0");
        assert_eq!(string_at(&bytes, 9).unwrap().0, "");
        assert_eq!(
            Source::parse(&source.to_text()).unwrap().encode().unwrap(),
            (bytes, bindings)
        );
    }

    #[test]
    fn preserves_opaque_bytes_and_shared_references_without_scanning() {
        let bytes = [0xff, 1, 0, 0, 0, 1, 0, 2, b'h', b'i', 0, 0xc1, 0xff];
        let source = Source::from_references(&bytes, [5, 5]).unwrap();
        assert_eq!(source.fragments.len(), 3);
        let text = source.to_text();
        assert_eq!(text.matches(".string ").count(), 1);
        assert_eq!(Source::parse(&text).unwrap().encode().unwrap().0, bytes);
        let resized = text.replace("\"hi\"", "\"longer\"");
        assert!(Source::parse(&resized).is_err());
    }

    #[test]
    fn rejects_truncation_bad_encoding_offsets_and_overlaps() {
        let bytes = [1, 0, 2, b'h', b'i', 0];
        for end in 0..bytes.len() {
            assert!(string_at(&bytes[..end], 0).is_err());
        }
        for at in [1, 6, u32::MAX] {
            assert!(string_at(&bytes, at).is_err());
        }
        for (at, byte) in [(0, 2), (5, 1), (3, 0xff)] {
            let mut broken = bytes;
            broken[at] = byte;
            assert!(string_at(&broken, 0).is_err());
        }
        // A second apparently valid record embedded inside the first is not accepted.
        let nested = [1, 0, 4, 1, 0, 0, 0, 0];
        assert!(Source::from_references(&nested, [0, 3]).is_err());
    }

    #[test]
    fn rejects_invalid_source_and_unrepresentable_lengths() {
        for text in [
            ".string a \"x\" at=0x1000000",
            ".string a \"x\" at=0x1",
            ".string a \"x\"\n.string a \"y\"",
            ".string 1bad \"x\"",
            ".bytes at=0x0 ff\n.bytes at=0x0 ff",
            ".bytes at=0x0 ff0",
            ".bytes at=0x0 zz",
            ".string a x",
            ".string a \"x\" unknown=0",
        ] {
            assert!(Source::parse(text).is_err(), "{text}");
        }
        let source = Source {
            fragments: vec![Fragment::String(StringDeclaration {
                alias: "large".into(),
                value: "λ".repeat(32768),
                at: None,
            })],
        };
        assert!(source.encode().is_err());
    }
}

#[cfg(test)]
mod record_walk_tests {
    use super::*;
    #[test]
    fn walks_class_record_boundaries_without_scanning_field_bytes() {
        let mut payload = vec![0xc2, 0xa5, 0x5d, 0xef, 0, 0, 0, 0, 1];
        // One nine-byte field containing bytes that look like an empty string.
        payload.extend_from_slice(&[1, 0, 0, 0, 1, 0, 0, 0, 0]);
        payload.extend_from_slice(&[1, 0, 1, b'x', 0]);
        assert_eq!(record_string_offsets(&payload).unwrap(), vec![18]);
        assert_eq!(
            Source::from_references(&payload, record_string_offsets(&payload).unwrap())
                .unwrap()
                .encode()
                .unwrap()
                .0,
            payload
        );
        for end in 0..18 {
            if end != 0 {
                assert!(record_string_offsets(&payload[..end]).is_err());
            }
        }
        let mut bad = payload;
        bad[4] = 0x80;
        assert!(record_string_offsets(&bad).is_err());
        assert!(record_string_offsets(&[3, 0, 0, 0, 0, 0]).is_err());
        let mut v1 = vec![0xc1, 0xa5, 0x5d, 0xef];
        v1.extend_from_slice(&[0; 19]);
        v1.extend_from_slice(&[1, 0, 0, 0]);
        assert_eq!(record_string_offsets(&v1).unwrap(), vec![23]);
    }
}
