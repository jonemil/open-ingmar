//! Supported DATA strings, classes, scalar arrays/Dictionaries, and opaque spans.

use crate::symbols::source::{Error, tokens, valid_alias};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

pub mod arrays;
pub mod classes;
pub mod dictionaries;

pub const ADDRESS_LIMIT: usize = 0x0100_0000;
type Bindings = BTreeMap<String, u32>;

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
    Array(arrays::Declaration),
    Dictionary(dictionaries::Declaration),
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
            classes::Record::Class(_)
            | classes::Record::Array(_)
            | classes::Record::Dictionary(_) => None,
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
        Self::parse_with_symbols(text, &BTreeMap::new())
    }

    pub fn parse_with_symbols(text: &str, symbols: &BTreeMap<String, u32>) -> Result<Self, Error> {
        Self::parse_with_references(text, symbols, &BTreeMap::new())
    }

    pub fn parse_with_references(
        text: &str,
        symbols: &Bindings,
        code: &Bindings,
    ) -> Result<Self, Error> {
        let source = Self::parse_layout(text)?;
        source.encode_with_references(symbols, code)?;
        Ok(source)
    }

    /// Parse DATA assembly and validate layout, retaining unresolved field aliases.
    ///
    /// Produces a source whose string/array offsets can be bound before CODE encoding.
    /// Does not validate referenced symbols or methods; final encoding must
    /// resolve them through [`Self::encode_with_references`].
    pub fn parse_layout(text: &str) -> Result<Self, Error> {
        let mut source = Self::default();
        let mut class: Option<classes::source::Declaration> = None;
        let mut array: Option<arrays::Declaration> = None;
        let mut dictionary: Option<dictionaries::Declaration> = None;
        for (index, line) in text.lines().enumerate() {
            let mut parse = || -> Result<Option<Fragment>, Error> {
                let fields = tokens(line)?;
                if fields.is_empty() {
                    return Ok(None);
                }
                if let Some(declaration) = dictionary.as_mut() {
                    if fields == [".enddictionary"] {
                        return Ok(Some(Fragment::Dictionary(dictionary.take().unwrap())));
                    }
                    declaration.push_field(&fields)?;
                    return Ok(None);
                }
                if let Some(declaration) = class.as_mut() {
                    if fields == [".endclass"] {
                        return Ok(Some(Fragment::Class(class.take().unwrap())));
                    }
                    declaration.push_field(&fields)?;
                    return Ok(None);
                }
                if let Some(declaration) = array.as_mut() {
                    if fields == [".endarray"] {
                        return Ok(Some(Fragment::Array(array.take().unwrap())));
                    }
                    declaration.push_element(&fields)?;
                    return Ok(None);
                }
                if fields[0] == ".dictionary" {
                    dictionary = Some(dictionaries::Declaration::parse_header(&fields)?);
                    return Ok(None);
                }
                if fields[0] == ".class" {
                    class = Some(classes::source::Declaration::parse_header(&fields)?);
                    return Ok(None);
                }
                if fields[0] == ".array" {
                    array = Some(arrays::Declaration::parse_header(&fields)?);
                    return Ok(None);
                }
                if fields[0] == ".string" {
                    return Ok(Some(Fragment::String(StringDeclaration::parse(&fields)?)));
                }
                if fields.len() != 3 || fields[0] != ".bytes" {
                    return Err(Error(
                        "expected .class, .array, .dictionary, .string, or .bytes at=0x... <hex-bytes>".into(),
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
        if dictionary.is_some() {
            return Err(Error("unterminated Dictionary declaration".into()));
        }
        if class.is_some() {
            return Err(Error("unterminated .class declaration".into()));
        }
        if array.is_some() {
            return Err(Error("unterminated .array declaration".into()));
        }
        source.layout()?;
        Ok(source)
    }

    /// Decode a complete supported DATA walk into lossless editable declarations.
    /// Retains class keys/references and scalar array types and value bits.
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
                classes::Record::Array(array) => Fragment::Array(array),
                classes::Record::Dictionary(d) => Fragment::Dictionary(d),
            });
        }
        if source.encode()?.0 != payload {
            return Err(Error("structured DATA representation changed bytes".into()));
        }
        Ok(source)
    }

    /// Name only verified class/string targets and known symbol-bearing fields.
    pub fn name_classes(
        &mut self,
        symbols: Option<&crate::symbols::Symbols<'_>>,
        named_symbols: bool,
    ) -> Result<(), Error> {
        use classes::source::{Key, Reference};
        let symbol_alias = |id| {
            symbols.and_then(|table| {
                table
                    .get(id)
                    .map(|_| crate::symbols::source::Source::alias(table, id))
            })
        };
        let mut candidates = BTreeMap::new();
        for fragment in &self.fragments {
            let Fragment::Class(c) = fragment else {
                continue;
            };
            let Some(at) = c.at else { continue };
            let alias = match &c.references[3] {
                Some(Reference::Raw(id)) => symbol_alias(*id),
                _ => None,
            };
            candidates.insert(at, alias);
        }
        // A class name can live in a referring field rather than its own header.
        for fragment in &self.fragments {
            let Fragment::Class(c) = fragment else {
                continue;
            };
            for field in &c.fields {
                if let (7, Key::Raw(key), Reference::Raw(target)) =
                    (field.kind, &field.key, &field.value)
                {
                    if let Some(candidate) = candidates.get_mut(target) {
                        if candidate.is_none() {
                            *candidate = symbol_alias(key >> 8);
                        }
                    }
                }
            }
        }
        let mut counts = BTreeMap::new();
        for alias in candidates.values().flatten() {
            *counts.entry(alias.clone()).or_insert(0usize) += 1;
        }
        let aliases: BTreeMap<_, _> = candidates
            .into_iter()
            .map(|(at, candidate)| {
                let alias = candidate
                    .filter(|a| counts[a] == 1 && !a.starts_with("class_"))
                    .unwrap_or_else(|| format!("class_{at:08x}"));
                (at, alias)
            })
            .collect();
        let strings: BTreeMap<_, _> = self
            .fragments
            .iter()
            .filter_map(|f| match f {
                Fragment::String(s) => s.at.map(|at| (at, s.alias.clone())),
                _ => None,
            })
            .collect();
        for fragment in &mut self.fragments {
            let Fragment::Class(c) = fragment else {
                continue;
            };
            if let Some(at) = c.at {
                c.alias = aliases.get(&at).cloned();
            }
            if named_symbols {
                for reference in &mut c.references[2..] {
                    if let Some(Reference::Raw(id)) = reference {
                        if let Some(alias) = symbol_alias(*id) {
                            *reference = Some(Reference::Symbol(alias));
                        }
                    }
                }
            }
            for field in &mut c.fields {
                if named_symbols {
                    if let Key::Raw(key) = field.key {
                        if let Some(alias) = symbol_alias(key >> 8) {
                            field.key = Key::Symbol {
                                alias,
                                bits: (key & 255) as u8,
                            };
                        }
                    }
                }
                if let Reference::Raw(target) = field.value {
                    if field.kind == 7 {
                        if let Some(alias) = aliases.get(&target) {
                            field.value = Reference::Class(alias.clone());
                        }
                    } else if field.kind == 3 {
                        if let Some(alias) = strings.get(&target) {
                            field.value = Reference::String(alias.clone());
                        }
                    }
                }
            }
        }
        Ok(())
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
        self.encode_with_symbols(&BTreeMap::new())
    }

    pub fn encode_with_symbols(
        &self,
        symbols: &BTreeMap<String, u32>,
    ) -> Result<(Vec<u8>, BTreeMap<String, u32>), Error> {
        self.encode_with_references(symbols, &BTreeMap::new())
    }

    pub fn encode_with_references(
        &self,
        symbols: &Bindings,
        code: &Bindings,
    ) -> Result<(Vec<u8>, Bindings), Error> {
        let (strings, classes) = self.layout()?;
        let mut payload = Vec::new();
        for fragment in &self.fragments {
            match fragment {
                Fragment::Bytes { bytes, .. } => payload.extend(bytes),
                Fragment::Array(a) => payload.extend(a.encode()?),
                Fragment::Dictionary(d) => payload.extend(d.encode()?),
                Fragment::Class(c) => {
                    payload.extend(c.encode_with_code(symbols, &classes, &strings, code)?)
                }
                Fragment::String(s) => {
                    payload.push(1);
                    payload.extend((s.value.len() as u16).to_be_bytes());
                    payload.extend(s.value.as_bytes());
                    payload.push(0);
                }
            }
        }
        Ok((payload, strings))
    }

    /// Resolve DATA string aliases to offsets after validating fragment layout.
    ///
    /// Checks placement, lengths, and duplicate aliases without resolving class
    /// fields. Use this before CODE encoding when DATA methods refer back to CODE.
    pub fn string_bindings(&self) -> Result<BTreeMap<String, u32>, Error> {
        self.layout().map(|(strings, _)| strings)
    }

    /// Resolve constant-array aliases to DATA offsets after validating layout.
    /// String and array bindings stay separate so CODE cannot confuse their kinds.
    pub fn array_bindings(&self) -> Result<Bindings, Error> {
        self.layout()?;
        let mut offset = 0usize;
        let mut arrays = Bindings::new();
        for fragment in &self.fragments {
            offset += match fragment {
                Fragment::Array(a) => {
                    arrays.insert(a.alias.clone(), offset as u32);
                    a.encoded_length()?
                }
                Fragment::String(s) => s.value.len() + 4,
                Fragment::Dictionary(d) => d.encoded_length()?,
                Fragment::Class(c) => c.encoded_length()?,
                Fragment::Bytes { bytes, .. } => bytes.len(),
            };
        }
        Ok(arrays)
    }

    /// Bind Dictionary aliases to record starts, separately from Array/String aliases.
    pub fn dictionary_bindings(&self) -> Result<Bindings, Error> {
        self.layout()?;
        let mut offset = 0usize;
        let mut bindings = Bindings::new();
        for fragment in &self.fragments {
            offset += match fragment {
                Fragment::Dictionary(d) => {
                    bindings.insert(d.alias.clone(), offset as u32);
                    d.encoded_length()?
                }
                Fragment::Array(a) => a.encoded_length()?,
                Fragment::String(s) => s.value.len() + 4,
                Fragment::Class(c) => c.encoded_length()?,
                Fragment::Bytes { bytes, .. } => bytes.len(),
            };
        }
        Ok(bindings)
    }

    fn layout(&self) -> Result<(Bindings, Bindings), Error> {
        let mut offset = 0usize;
        let mut strings = BTreeMap::new();
        let mut classes = BTreeMap::new();
        let mut arrays = BTreeSet::new();
        let mut dictionaries = BTreeSet::new();
        for fragment in &self.fragments {
            let (at, size) = match fragment {
                Fragment::Bytes { at, bytes } => (Some(*at), bytes.len()),
                Fragment::Dictionary(d) => {
                    if !valid_alias(&d.alias) || !dictionaries.insert(d.alias.clone()) {
                        return Err(Error("invalid or duplicate Dictionary alias".into()));
                    }
                    (d.at, d.encoded_length()?)
                }
                Fragment::Array(a) => {
                    if !valid_alias(&a.alias) || !arrays.insert(a.alias.clone()) {
                        return Err(Error(format!(
                            "invalid or duplicate array alias {}",
                            a.alias
                        )));
                    }
                    (a.at, a.encoded_length()?)
                }
                Fragment::Class(c) => {
                    if let Some(alias) = &c.alias {
                        if !valid_alias(alias)
                            || classes.insert(alias.clone(), offset as u32).is_some()
                        {
                            return Err(Error(format!("invalid or duplicate class alias {alias}")));
                        }
                    }
                    (c.at, c.encoded_length()?)
                }
                Fragment::String(s) => {
                    if !valid_alias(&s.alias)
                        || strings.insert(s.alias.clone(), offset as u32).is_some()
                    {
                        return Err(Error(format!(
                            "invalid or duplicate string alias {}",
                            s.alias
                        )));
                    }
                    if s.value.len() > u16::MAX as usize {
                        return Err(Error("string UTF-8 byte length exceeds u16".into()));
                    }
                    (s.at, s.value.len() + 4)
                }
            };
            if at.is_some_and(|at| at as usize != offset) {
                return Err(Error(format!(
                    "pinned DATA offset does not match next offset 0x{offset:08x}; relocation is unsupported"
                )));
            }
            if size > ADDRESS_LIMIT - offset {
                return Err(Error("DATA payload exceeds supported address range".into()));
            }
            offset += size;
        }
        Ok((strings, classes))
    }

    pub fn to_text(&self) -> String {
        let mut text = String::new();
        for fragment in &self.fragments {
            match fragment {
                Fragment::String(s) => text.push_str(&s.to_text()),
                Fragment::Class(c) => text.push_str(&c.to_text()),
                Fragment::Array(a) => text.push_str(&a.to_text()),
                Fragment::Dictionary(d) => text.push_str(&d.to_text()),
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
    fn string_layout_can_precede_method_binding_but_encoding_cannot() {
        let text = ".class owner version=0x02 extends=none statics=none parent=none module=none app_flags=0x0000\n.field symbol=@method bits=0x10 type=0x06 value=code:probe\n.endclass\n.string greeting \"hello\"\n";
        assert!(Source::parse(text).is_err());
        let source = Source::parse_layout(text).unwrap();
        assert_eq!(source.string_bindings().unwrap()["greeting"], 18);
        assert!(source.encode().is_err());
        let (payload, strings) = source
            .encode_with_references(
                &Bindings::from([("method".into(), 0x42)]),
                &Bindings::from([("probe".into(), 0x123)]),
            )
            .unwrap();
        assert_eq!(strings["greeting"], 18);
        assert_eq!(string_at(&payload, 18).unwrap().0, "hello");
        assert!(
            Source::parse_layout(&text.replace(
                ".string greeting",
                ".bytes at=0x00000000 ff\n.string greeting"
            ))
            .is_err()
        );
    }

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

#[cfg(test)]
mod named_class_tests {
    use super::*;
    use crate::symbols::{Symbols, source::Source as SymbolSource};
    use classes::{
        Record,
        source::{Key, Reference},
    };

    const HEADER: &str =
        "version=0x02 extends=none statics=none parent=none module=none app_flags=0x0";

    #[test]
    fn forward_class_and_string_references_allocate_and_follow_layout() {
        let symbols = BTreeMap::from([("child".into(), 0x81)]);
        let text = format!(
            ".class root {HEADER}\n.field symbol=@child bits=0x50 type=0x07 value=%target\n.field key=0x80001000 type=0x03 value=&target\n.endclass\n.class target {HEADER}\n.endclass\n.string target \"hi\"\n"
        );
        let source = Source::parse_with_symbols(&text, &symbols).unwrap();
        let (bytes, strings) = source.encode_with_symbols(&symbols).unwrap();
        assert_eq!(strings["target"], 36);
        let records = classes::records(&bytes).unwrap();
        let Record::Class(root) = &records[0] else {
            panic!()
        };
        assert_eq!(root.fields[0].key, 0x8150);
        assert_eq!(root.fields[0].value, 27);
        assert_eq!(root.fields[1].value, 36);
        assert_eq!(
            Source::parse_with_symbols(&source.to_text(), &symbols).unwrap(),
            source
        );
        assert!(Source::parse(&text).is_err()); // external symbols are required
        let moved = text.replace(".class root", ".string prefix \"x\"\n.class root");
        let (bytes, _) = Source::parse_with_symbols(&moved, &symbols)
            .unwrap()
            .encode_with_symbols(&symbols)
            .unwrap();
        let Record::Class(root) = &classes::records(&bytes).unwrap()[1] else {
            panic!()
        };
        assert_eq!(root.fields[0].value, 32);
        assert_eq!(root.fields[1].value, 41);
    }

    #[test]
    fn rejects_wrong_namespaces_unknown_aliases_duplicate_classes_and_oversized_ids() {
        let base = format!(
            ".class root {HEADER}\n.field symbol=@child bits=0x00 type=0x07 value=%target\n.endclass\n.class target {HEADER}\n.endclass\n"
        );
        let symbols = BTreeMap::from([("child".into(), 0x81)]);
        for text in [
            base.replace("%target", "&target"),
            base.replace("%target", "%missing"),
            base.replace("@child", "@missing"),
            base.replace("type=0x07", "type=0x06"),
            base.replace(".class target", ".class root"),
            base.replace("bits=0x00", "bits=0x100"),
        ] {
            assert!(
                Source::parse_with_symbols(&text, &symbols).is_err(),
                "{text}"
            );
        }
        let oversized = BTreeMap::from([("child".into(), 0x1000000)]);
        assert!(Source::parse_with_symbols(&base, &oversized).is_err());
        assert!(
            Source::parse(&format!(
                ".class a {}\n.endclass",
                HEADER.replace("extends=none", "extends=@child")
            ))
            .is_err()
        );
    }

    #[test]
    fn exported_names_preserve_bytes_and_use_verified_reference_kinds() {
        let symbol_bytes = SymbolSource::parse(".symbol globals \"globals\" id=0x8002e6\n.symbol child \"Child\" id=0x81\n.symbol title \"title\" id=0x82\n").unwrap().encode().unwrap();
        let symbols = Symbols::parse(&symbol_bytes).unwrap();
        let bindings = SymbolSource::from_symbols(&symbols).bindings().unwrap();
        let numeric = format!(
            ".class at=0x0 {}\n.field key=0x8150 type=0x07 value=0x1f\n.field key=0x8200 type=0x03 value=0x28\n.endclass\n.class at=0x1f {HEADER}\n.endclass\n.string title \"hi\" at=0x28\n",
            HEADER.replace("module=none", "module=0x8002e6")
        );
        let bytes = Source::parse(&numeric).unwrap().encode().unwrap().0;
        let mut source = Source::from_records(&bytes).unwrap();
        source.name_classes(Some(&symbols), true).unwrap();
        let text = source.to_text();
        assert!(text.contains(".class globals at=0x00000000"));
        assert!(text.contains(".class Child at=0x0000001f"));
        assert!(text.contains("symbol=@Child bits=0x50 type=0x07 value=%Child"));
        assert!(text.contains("value=&str_00000028"));
        assert_eq!(
            Source::parse_with_symbols(&text, &bindings)
                .unwrap()
                .encode_with_symbols(&bindings)
                .unwrap()
                .0,
            bytes
        );
        let Fragment::Class(c) = &source.fragments[0] else {
            panic!()
        };
        assert!(matches!(c.fields[0].key, Key::Symbol { .. }));
        assert!(matches!(c.fields[0].value, Reference::Class(_)));
    }
}

#[cfg(test)]
mod code_reference_tests {
    use super::*;
    #[test]
    fn method_code_references_are_typed_and_24_bit() {
        let text = ".class app version=0x02 extends=none statics=none parent=none module=none app_flags=0x0\n.field key=0x8146 type=0x06 value=code:probe\n.endclass\n";
        let labels = BTreeMap::from([("probe".into(), 0x30)]);
        let source = Source::parse_with_references(text, &BTreeMap::new(), &labels).unwrap();
        let (bytes, _) = source
            .encode_with_references(&BTreeMap::new(), &labels)
            .unwrap();
        let classes::Record::Class(c) = &classes::records(&bytes).unwrap()[0] else {
            panic!()
        };
        assert_eq!(c.fields[0].value, 0x10000030);
        assert!(Source::parse(text).is_err());
        assert!(
            Source::parse_with_references(
                &text.replace("type=0x06", "type=0x07"),
                &BTreeMap::new(),
                &labels
            )
            .is_err()
        );
        let oversized = BTreeMap::from([("probe".into(), 0x1000000)]);
        assert!(Source::parse_with_references(text, &BTreeMap::new(), &oversized).is_err());
    }
}
