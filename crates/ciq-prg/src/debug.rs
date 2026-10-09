//! DEBUG strings and LINE_NUMBERS; see docs/specs/line-numbers-and-debug.md.
use crate::{
    data::{Fragment, Source as DataSource, StringDeclaration},
    symbols::source::{Error, tokens, valid_alias},
};
use std::{collections::BTreeMap, fmt::Write};

type Bindings = BTreeMap<String, u32>;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DebugSource {
    pub strings: Vec<StringDeclaration>,
}
impl DebugSource {
    pub fn parse(text: &str) -> Result<Self, Error> {
        let data = DataSource::parse(text)?;
        let mut strings = Vec::new();
        for fragment in data.fragments {
            let Fragment::String(string) = fragment else {
                return Err(Error(
                    "DEBUG source accepts only .string declarations".into(),
                ));
            };
            strings.push(string);
        }
        Ok(Self { strings })
    }
    pub fn from_payload(payload: &[u8]) -> Result<Self, Error> {
        let data = DataSource::from_records(payload)?;
        let mut strings = Vec::new();
        for fragment in data.fragments {
            let Fragment::String(mut string) = fragment else {
                return Err(Error("unsupported non-string DEBUG record".into()));
            };
            string.alias = format!("dbg_{:08x}", string.at.unwrap());
            strings.push(string);
        }
        let source = Self { strings };
        if source.encode()?.0 != payload {
            return Err(Error("DEBUG source changed bytes".into()));
        }
        Ok(source)
    }
    pub fn encode(&self) -> Result<(Vec<u8>, Bindings), Error> {
        DataSource {
            fragments: self.strings.iter().cloned().map(Fragment::String).collect(),
        }
        .encode()
    }
    pub fn to_text(&self) -> String {
        self.strings
            .iter()
            .map(StringDeclaration::to_text)
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pc {
    Raw(u32),
    Code(String),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StringRef {
    Raw(u32),
    Named(String),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub pc: Pc,
    pub file: StringRef,
    pub function: StringRef,
    pub line: u32,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LineSource {
    pub entries: Vec<Line>,
}

fn hex(text: &str) -> Result<u32, Error> {
    let text = text
        .strip_prefix("0x")
        .ok_or_else(|| Error("expected 0x-prefixed metadata value".into()))?;
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(Error("invalid metadata value".into()));
    }
    u32::from_str_radix(text, 16).map_err(|_| Error("metadata value exceeds u32".into()))
}
fn alias(text: &str) -> Result<String, Error> {
    if valid_alias(text) {
        Ok(text.into())
    } else {
        Err(Error("invalid metadata alias".into()))
    }
}
impl StringRef {
    fn parse(text: &str) -> Result<Self, Error> {
        if let Some(a) = text.strip_prefix('&') {
            Ok(Self::Named(alias(a)?))
        } else {
            Ok(Self::Raw(hex(text)?))
        }
    }
    fn resolve(&self, strings: &Bindings) -> Result<u32, Error> {
        match self {
            Self::Raw(n) => Ok(*n),
            Self::Named(a) => strings
                .get(a)
                .copied()
                .ok_or_else(|| Error(format!("undefined DEBUG string {a}"))),
        }
    }
    fn text(&self) -> String {
        match self {
            Self::Raw(n) => format!("0x{n:08x}"),
            Self::Named(a) => format!("&{a}"),
        }
    }
}
impl LineSource {
    pub fn from_payload(payload: &[u8]) -> Result<Self, Error> {
        let count = payload
            .get(..2)
            .ok_or_else(|| Error("truncated line-table count".into()))?;
        let count = u16::from_be_bytes(count.try_into().unwrap()) as usize;
        if payload.len() != 2 + count * 16 {
            return Err(Error("line-table length does not match count".into()));
        }
        let entries = payload[2..]
            .chunks_exact(16)
            .map(|entry| {
                let word = |at| u32::from_be_bytes(entry[at..at + 4].try_into().unwrap());
                Line {
                    pc: Pc::Raw(word(0)),
                    file: StringRef::Raw(word(4)),
                    function: StringRef::Raw(word(8)),
                    line: word(12),
                }
            })
            .collect();
        Ok(Self { entries })
    }
    pub fn parse(text: &str) -> Result<Self, Error> {
        let mut entries = Vec::new();
        for (index, line) in text.lines().enumerate() {
            let parse = || -> Result<Option<Line>, Error> {
                let fields = tokens(line)?;
                if fields.is_empty() {
                    return Ok(None);
                }
                if fields.len() != 5 || fields[0] != ".line" {
                    return Err(Error(
                        "expected .line pc=... file=... function=... line=<decimal>".into(),
                    ));
                }
                let attribute = |at: usize, prefix: &str| {
                    fields[at]
                        .strip_prefix(prefix)
                        .ok_or_else(|| Error(format!("expected line attribute {prefix}")))
                };
                let pc = attribute(1, "pc=")?;
                let pc = if let Some(a) = pc.strip_prefix("code:") {
                    Pc::Code(alias(a)?)
                } else {
                    Pc::Raw(hex(pc)?)
                };
                let file = StringRef::parse(attribute(2, "file=")?)?;
                let function = StringRef::parse(attribute(3, "function=")?)?;
                let line = attribute(4, "line=")?;
                if line.is_empty() || !line.bytes().all(|b| b.is_ascii_digit()) {
                    return Err(Error("source line must be decimal u32".into()));
                }
                Ok(Some(Line {
                    pc,
                    file,
                    function,
                    line: line
                        .parse()
                        .map_err(|_| Error("source line exceeds u32".into()))?,
                }))
            };
            if let Some(line) =
                parse().map_err(|e| Error(format!("line source {}: {e}", index + 1)))?
            {
                entries.push(line);
            }
        }
        if entries.len() > u16::MAX as usize {
            return Err(Error("line count exceeds u16".into()));
        }
        Ok(Self { entries })
    }
    pub fn name_references(&mut self, code: &BTreeMap<u32, String>, debug: &BTreeMap<u32, String>) {
        for line in &mut self.entries {
            if let Pc::Raw(pc) = line.pc {
                if pc >> 24 == 0x10 {
                    if let Some(a) = code.get(&(pc & 0xffffff)) {
                        line.pc = Pc::Code(a.clone());
                    }
                }
            }
            for reference in [&mut line.file, &mut line.function] {
                if let StringRef::Raw(offset) = reference {
                    if let Some(a) = debug.get(offset) {
                        *reference = StringRef::Named(a.clone());
                    }
                }
            }
        }
    }
    pub fn encode(&self, code: &Bindings, debug: &Bindings) -> Result<Vec<u8>, Error> {
        let count = u16::try_from(self.entries.len())
            .map_err(|_| Error("line count exceeds u16".into()))?;
        let mut payload = count.to_be_bytes().to_vec();
        for line in &self.entries {
            let pc = match &line.pc {
                Pc::Raw(n) => *n,
                Pc::Code(a) => {
                    let offset = *code
                        .get(a)
                        .ok_or_else(|| Error(format!("undefined code label {a}")))?;
                    if offset >= 0x1000000 {
                        return Err(Error("line CODE offset exceeds 24 bits".into()));
                    }
                    0x10000000 | offset
                }
            };
            for value in [
                pc,
                line.file.resolve(debug)?,
                line.function.resolve(debug)?,
                line.line,
            ] {
                payload.extend(value.to_be_bytes());
            }
        }
        Ok(payload)
    }
    pub fn to_text(&self) -> String {
        let mut text = String::new();
        for line in &self.entries {
            let pc = match &line.pc {
                Pc::Raw(n) => format!("0x{n:08x}"),
                Pc::Code(a) => format!("code:{a}"),
            };
            writeln!(
                text,
                ".line pc={pc} file={} function={} line={}",
                line.file.text(),
                line.function.text(),
                line.line
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
    fn debug_strings_roundtrip_preserve_offsets_and_reject_unknown_records() {
        let source = DebugSource::parse(
            ".string file \"a#\\nλ\" at=0x0\n.string function \"probe\" at=0x9\n",
        )
        .unwrap();
        let (bytes, bindings) = source.encode().unwrap();
        assert_eq!(bindings["function"], 9);
        let exported = DebugSource::from_payload(&bytes).unwrap();
        assert_eq!(
            DebugSource::parse(&exported.to_text())
                .unwrap()
                .encode()
                .unwrap()
                .0,
            bytes
        );
        for end in 1..9 {
            assert!(DebugSource::from_payload(&bytes[..end]).is_err());
        }
        assert!(DebugSource::from_payload(&[3, 0, 0, 0]).is_err());
        assert!(DebugSource::parse(".bytes at=0x0 ff").is_err());
        assert!(DebugSource::parse(".string a \"x\"\n.string a \"y\"").is_err());
        assert!(DebugSource::parse(".string a \"longer\"\n.string b \"x\" at=0x5").is_err());
    }
    #[test]
    fn line_order_duplicates_and_noncode_addresses_roundtrip() {
        let text = ".line pc=0x10000030 file=0x0 function=0x8 line=22\n.line pc=0x10000030 file=0x0 function=0x8 line=23\n.line pc=0x40000030 file=0x123 function=0x456 line=0\n";
        let source = LineSource::parse(text).unwrap();
        let bytes = source.encode(&Bindings::new(), &Bindings::new()).unwrap();
        assert_eq!(LineSource::from_payload(&bytes).unwrap(), source);
        let mut named = source;
        named.name_references(
            &BTreeMap::from([(0x30, "probe".into())]),
            &BTreeMap::from([(0, "file".into()), (8, "function".into())]),
        );
        let moved = named
            .encode(
                &BTreeMap::from([("probe".into(), 0x45)]),
                &BTreeMap::from([("file".into(), 0), ("function".into(), 8)]),
            )
            .unwrap();
        let decoded = LineSource::from_payload(&moved).unwrap();
        assert_eq!(decoded.entries[0].pc, Pc::Raw(0x10000045));
        assert_eq!(decoded.entries[1].pc, Pc::Raw(0x10000045));
        assert_eq!(decoded.entries[2].pc, Pc::Raw(0x40000030));
        assert_eq!(LineSource::parse(&named.to_text()).unwrap(), named);
    }
    #[test]
    fn malformed_and_unresolved_line_inputs_fail() {
        for bytes in [&[][..], &[0], &[0, 1], &[0, 0, 0]] {
            assert!(LineSource::from_payload(bytes).is_err());
        }
        for text in [
            ".line",
            ".line pc=code:1bad file=0x0 function=0x0 line=1",
            ".line pc=0x0 file=@wrong function=0x0 line=1",
            ".line pc=0x0 file=0x0 function=0x0 line=-1",
            ".line pc=0x0 file=0x0 function=0x0 line=4294967296",
        ] {
            assert!(LineSource::parse(text).is_err());
        }
        let source =
            LineSource::parse(".line pc=code:probe file=&file function=&function line=1").unwrap();
        assert!(source.encode(&Bindings::new(), &Bindings::new()).is_err());
        assert!(
            source
                .encode(
                    &Bindings::from([("probe".into(), 0x1000000)]),
                    &Bindings::new()
                )
                .is_err()
        );
        assert!(
            source
                .encode(&Bindings::from([("probe".into(), 0)]), &Bindings::new())
                .is_err()
        );
        let too_many = LineSource {
            entries: vec![
                Line {
                    pc: Pc::Raw(0),
                    file: StringRef::Raw(0),
                    function: StringRef::Raw(0),
                    line: 0
                };
                65536
            ],
        };
        assert!(too_many.encode(&Bindings::new(), &Bindings::new()).is_err());
    }
}
