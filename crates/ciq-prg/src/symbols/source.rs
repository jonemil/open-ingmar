//! Shared symbol declarations and layout; see docs/specs/symbol-assembly.md.

use super::Symbols;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Write};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declaration {
    pub alias: String,
    pub name: String,
    pub id: u32,
    pub name_at: Option<u32>,
    pub entry_index: Option<usize>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Source {
    pub declarations: Vec<Declaration>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error(pub String);
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Error {}

pub fn valid_alias(alias: &str) -> bool {
    let mut bytes = alias.bytes();
    bytes
        .next()
        .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
        && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// Tokenize comments and quoted names without treating # inside a name as a comment.
pub fn tokens(line: &str) -> Result<Vec<String>, Error> {
    let mut chars = line.chars().peekable();
    let mut result = Vec::new();
    while let Some(c) = chars.next() {
        if c.is_whitespace() || c == ',' {
            continue;
        }
        if c == '#' {
            break;
        }
        let mut token = String::new();
        if c == '"' {
            let mut closed = false;
            while let Some(c) = chars.next() {
                if c == '"' {
                    closed = true;
                    break;
                }
                if c != '\\' {
                    token.push(c);
                    continue;
                }
                let c = chars
                    .next()
                    .ok_or_else(|| Error("unfinished string escape".into()))?;
                token.push(match c {
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    '0' => '\0',
                    '\\' => '\\',
                    '"' => '"',
                    'u' => {
                        if chars.next() != Some('{') {
                            return Err(Error("expected { after \\u".into()));
                        }
                        let mut hex = String::new();
                        let mut closed = false;
                        for c in chars.by_ref() {
                            if c == '}' {
                                closed = true;
                                break;
                            }
                            hex.push(c);
                        }
                        if !closed
                            || hex.is_empty()
                            || hex.len() > 6
                            || !hex.bytes().all(|b| b.is_ascii_hexdigit())
                        {
                            return Err(Error("invalid Unicode escape".into()));
                        }
                        char::from_u32(u32::from_str_radix(&hex, 16).unwrap())
                            .ok_or_else(|| Error("invalid Unicode scalar".into()))?
                    }
                    _ => return Err(Error(format!("unsupported string escape \\{c}"))),
                });
            }
            if !closed {
                return Err(Error("unterminated quoted name".into()));
            }
            if chars
                .peek()
                .is_some_and(|c| !c.is_whitespace() && *c != '#' && *c != ',')
            {
                return Err(Error("expected separator after quoted name".into()));
            }
            // Preserve quotedness for the declaration parser.
            token.insert(0, '"');
        } else {
            token.push(c);
            while chars
                .peek()
                .is_some_and(|c| !c.is_whitespace() && *c != '#' && *c != ',')
            {
                token.push(chars.next().unwrap());
            }
        }
        result.push(token);
    }
    Ok(result)
}

fn hex(value: &str) -> Result<u32, Error> {
    let digits = value
        .strip_prefix("0x")
        .ok_or_else(|| Error("expected 0x hexadecimal value".into()))?;
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(Error("invalid hexadecimal value".into()));
    }
    u32::from_str_radix(digits, 16).map_err(|_| Error("hexadecimal value exceeds u32".into()))
}

impl Declaration {
    pub fn parse(fields: &[String]) -> Result<Self, Error> {
        if fields.len() < 4
            || fields[0] != ".symbol"
            || !valid_alias(&fields[1])
            || !fields[2].starts_with('"')
        {
            return Err(Error(
                "expected .symbol <alias> \"name\" id=0x... [name_at=0x...] [entry_index=...]"
                    .into(),
            ));
        }
        let mut id = None;
        let mut name_at = None;
        let mut entry_index = None;
        let mut keys = BTreeSet::new();
        for field in &fields[3..] {
            let (key, value) = field
                .split_once('=')
                .ok_or_else(|| Error("expected key=value constraint".into()))?;
            if !keys.insert(key) {
                return Err(Error(format!("duplicate {key} constraint")));
            }
            match key {
                "id" => id = Some(hex(value)?),
                "name_at" => name_at = Some(hex(value)?),
                "entry_index" => {
                    if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
                        return Err(Error("entry_index must be decimal".into()));
                    }
                    entry_index = Some(
                        value
                            .parse()
                            .map_err(|_| Error("entry_index exceeds usize".into()))?,
                    );
                }
                _ => return Err(Error(format!("unknown symbol constraint {key}"))),
            }
        }
        Ok(Self {
            alias: fields[1].clone(),
            name: fields[2][1..].into(),
            id: id.ok_or_else(|| {
                Error("symbol id is required; automatic ID allocation is unsupported".into())
            })?,
            name_at,
            entry_index,
        })
    }
}

impl Source {
    pub fn parse(text: &str) -> Result<Self, Error> {
        let mut source = Self::default();
        for (line, text) in text.lines().enumerate() {
            let fields =
                tokens(text).map_err(|e| Error(format!("symbol line {}: {e}", line + 1)))?;
            if !fields.is_empty() {
                source.declarations.push(
                    Declaration::parse(&fields)
                        .map_err(|e| Error(format!("symbol line {}: {e}", line + 1)))?,
                );
            }
        }
        source.bindings()?;
        Ok(source)
    }

    pub fn bindings(&self) -> Result<BTreeMap<String, u32>, Error> {
        let mut aliases = BTreeMap::new();
        let mut ids = BTreeSet::new();
        for d in &self.declarations {
            if !valid_alias(&d.alias) {
                return Err(Error(format!("invalid alias {:?}", d.alias)));
            }
            if aliases.insert(d.alias.clone(), d.id).is_some() {
                return Err(Error(format!("duplicate symbol alias {}", d.alias)));
            }
            if !ids.insert(d.id) {
                return Err(Error(format!("duplicate symbol ID 0x{:08x}", d.id)));
            }
        }
        Ok(aliases)
    }

    pub fn alias(symbols: &Symbols<'_>, id: u32) -> String {
        let symbol = symbols.get(id).unwrap();
        if valid_alias(symbol.name)
            && !symbol.name.starts_with("sym_")
            && symbols.iter().filter(|s| s.name == symbol.name).count() == 1
        {
            symbol.name.into()
        } else {
            format!("sym_{id:08x}")
        }
    }

    pub fn from_symbols(symbols: &Symbols<'_>) -> Self {
        Self {
            declarations: symbols
                .iter()
                .map(|s| Declaration {
                    alias: Self::alias(symbols, s.id),
                    name: s.name.into(),
                    id: s.id,
                    name_at: Some(s.name_offset),
                    entry_index: Some(s.entry_index),
                })
                .collect(),
        }
    }

    pub fn to_text(&self) -> String {
        let mut text = String::new();
        for d in &self.declarations {
            write!(text, ".symbol {} {:?} id=0x{:08x}", d.alias, d.name, d.id).unwrap();
            if let Some(at) = d.name_at {
                write!(text, " name_at=0x{at:08x}").unwrap();
            }
            if let Some(index) = d.entry_index {
                write!(text, " entry_index={index}").unwrap();
            }
            text.push('\n');
        }
        text
    }

    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        self.bindings()?;
        let count = self.declarations.len();
        if count > u16::MAX as usize {
            return Err(Error("symbol count exceeds u16".into()));
        }
        let table_end = 2 + count * 8;
        let mut order = vec![None; count];
        for (i, d) in self.declarations.iter().enumerate() {
            if let Some(slot) = d.entry_index {
                if slot >= count || order[slot].replace(i).is_some() {
                    return Err(Error("conflicting or out-of-range entry_index".into()));
                }
            }
            if d.name.len() > u16::MAX as usize {
                return Err(Error("symbol name exceeds u16 UTF-8 length".into()));
            }
        }
        let mut free = order.iter_mut().filter(|slot| slot.is_none());
        for (i, d) in self.declarations.iter().enumerate() {
            if d.entry_index.is_none() {
                *free.next().unwrap() = Some(i);
            }
        }
        let mut records: BTreeMap<usize, &str> = BTreeMap::new();
        let mut offsets = vec![0; count];
        for (i, d) in self.declarations.iter().enumerate() {
            if let Some(at) = d.name_at {
                let at = at as usize;
                if at < table_end {
                    return Err(Error("name_at overlaps symbol entry table".into()));
                }
                if let Some(name) = records.insert(at, &d.name) {
                    if name != d.name {
                        return Err(Error("different names pinned at the same offset".into()));
                    }
                }
                offsets[i] = at;
            }
        }
        let maximum = table_end
            + self
                .declarations
                .iter()
                .map(|d| d.name.len() + 4)
                .sum::<usize>();
        let mut end = table_end;
        for (&at, name) in &records {
            if at < end {
                return Err(Error("overlapping pinned name records".into()));
            }
            end = at + name.len() + 4;
            if end > maximum {
                return Err(Error(
                    "name_at requires unsupported padding between packed records".into(),
                ));
            }
        }
        for (i, d) in self.declarations.iter().enumerate() {
            if d.name_at.is_some() {
                continue;
            }
            let length = d.name.len() + 4;
            let mut at = table_end;
            for (&next, name) in &records {
                if at + length <= next {
                    break;
                }
                at = next + name.len() + 4;
            }
            records.insert(at, &d.name);
            offsets[i] = at;
        }
        let mut payload = vec![0; table_end];
        payload[..2].copy_from_slice(&(count as u16).to_be_bytes());
        for (slot, index) in order.into_iter().enumerate() {
            let i = index.unwrap();
            let start = 2 + slot * 8;
            payload[start..start + 4].copy_from_slice(&self.declarations[i].id.to_be_bytes());
            let at =
                u32::try_from(offsets[i]).map_err(|_| Error("name offset exceeds u32".into()))?;
            payload[start + 4..start + 8].copy_from_slice(&at.to_be_bytes());
        }
        for (at, name) in records {
            if at != payload.len() {
                return Err(Error(
                    "layout leaves unsupported gaps between packed name records".into(),
                ));
            }
            payload.push(1);
            payload.extend_from_slice(&(name.len() as u16).to_be_bytes());
            payload.extend_from_slice(name.as_bytes());
            payload.push(0);
        }
        Ok(payload)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unconstrained_layout_is_deterministic_and_escapes_round_trip() {
        let source = Source::parse(".symbol system \"Toybox_System\" id=0x00800005\n.symbol print \"print#\\n\\\"λ\\u{7}\\0\" id=0x00800067 # comment\n").unwrap();
        assert_eq!(Source::parse(&source.to_text()).unwrap(), source);
        let bytes = source.encode().unwrap();
        let symbols = Symbols::parse(&bytes).unwrap();
        assert_eq!(symbols.get(0x800005).unwrap().name_offset, 18);
        assert_eq!(
            symbols.get(0x800067).unwrap().name,
            source.declarations[1].name
        );
        assert_eq!(source.encode().unwrap(), bytes);
        assert_eq!(Source::from_symbols(&symbols).encode().unwrap(), bytes);
    }

    #[test]
    fn honors_independent_table_order_and_shared_name_placement() {
        let source = Source::parse(".symbol a \"same\" id=0x1 name_at=0x1a entry_index=1\n.symbol b \"same\" id=0x2 name_at=0x1a entry_index=0\n.symbol c \"other\" id=0x3 entry_index=2\n").unwrap();
        let bytes = source.encode().unwrap();
        let table = Symbols::parse(&bytes).unwrap();
        assert_eq!(table.get(1).unwrap().entry_index, 1);
        assert_eq!(table.get(2).unwrap().entry_index, 0);
        assert_eq!(
            table.get(1).unwrap().name_offset,
            table.get(2).unwrap().name_offset
        );
        assert_eq!(Source::from_symbols(&table).encode().unwrap(), bytes);
        let source =
            Source::parse(".symbol a \"a\" id=0x1 name_at=0x17\n.symbol b \"b\" id=0x2\n").unwrap();
        let bytes = source.encode().unwrap();
        assert_eq!(
            Symbols::parse(&bytes).unwrap().get(2).unwrap().name_offset,
            18
        );
    }

    #[test]
    fn rejects_conflicting_and_unrepresentable_layouts() {
        for text in [
            ".symbol a \"a\" id=0x1\n.symbol a \"b\" id=0x2",
            ".symbol a \"a\" id=0x1\n.symbol b \"b\" id=0x1",
            ".symbol a \"a\" id=0x1 entry_index=1",
            ".symbol a \"a\" id=0x1 name_at=0x0",
            ".symbol a \"a\" id=0x1 name_at=0xffffffff",
            ".symbol a \"aa\" id=0x1 name_at=0x12\n.symbol b \"bb\" id=0x2 name_at=0x13",
            ".symbol a \"a\" id=0x1 name_at=0x12\n.symbol b \"b\" id=0x2 name_at=0x12",
            ".symbol a \"a\" id=0x1 entry_index=0\n.symbol b \"b\" id=0x2 entry_index=0",
            ".symbol a \"a\" id=0x1 name_at=0xb",
        ] {
            assert!(
                Source::parse(text).and_then(|s| s.encode()).is_err(),
                "{text}"
            );
        }
    }

    #[test]
    fn rejects_bad_declarations_and_escapes() {
        for text in [
            ".symbol a \"x\"",
            ".symbol 1bad \"x\" id=0x1",
            ".symbol a x id=0x1",
            ".symbol a \"x\" id=0x1 id=0x2",
            ".symbol a \"x\" id=0x1 unknown=0",
            ".symbol a \"x\" id=0x+1",
            ".symbol a \"x\" id=0x1 entry_index=-1",
            ".symbol a \"unterminated",
            ".symbol a \"\\q\" id=0x1",
            ".symbol a \"\\u{d800}\" id=0x1",
            ".symbol a \"\\u{110000}\" id=0x1",
            ".symbol a \"x\"junk id=0x1",
        ] {
            assert!(Source::parse(text).is_err(), "{text}");
        }
        let mut source = Source::parse(".symbol a \"x\" id=0x1").unwrap();
        source.declarations[0].name = "λ".repeat(32768);
        assert!(source.encode().is_err());
    }
}
