//! Named, lossless class declarations; see docs/specs/data-classes.md.
use super::Class;
use crate::{
    data::address,
    symbols::source::{Error, valid_alias},
};
use std::{collections::BTreeMap, fmt::Write};

type Bindings = BTreeMap<String, u32>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reference {
    Raw(u32),
    Symbol(String),
    Class(String),
    String(String),
    Code(String),
}
impl Reference {
    fn parse(text: &str) -> Result<Self, Error> {
        if let Some(alias) = text.strip_prefix("code:") {
            if !valid_alias(alias) {
                return Err(Error("invalid code label".into()));
            }
            return Ok(Self::Code(alias.into()));
        }
        for (prefix, kind) in [('@', 0), ('%', 1), ('&', 2)] {
            if let Some(alias) = text.strip_prefix(prefix) {
                if !valid_alias(alias) {
                    return Err(Error("invalid reference alias".into()));
                }
                return Ok(match kind {
                    0 => Self::Symbol(alias.into()),
                    1 => Self::Class(alias.into()),
                    _ => Self::String(alias.into()),
                });
            }
        }
        Ok(Self::Raw(number(text)?))
    }
    fn text(&self) -> String {
        match self {
            Self::Raw(n) => format!("0x{n:08x}"),
            Self::Symbol(a) => format!("@{a}"),
            Self::Class(a) => format!("%{a}"),
            Self::String(a) => format!("&{a}"),
            Self::Code(a) => format!("code:{a}"),
        }
    }
    fn resolve(
        &self,
        symbols: &Bindings,
        classes: &Bindings,
        strings: &Bindings,
        code: &Bindings,
    ) -> Result<u32, Error> {
        let (map, alias, role) = match self {
            Self::Raw(n) => return Ok(*n),
            Self::Symbol(a) => (symbols, a, "symbol"),
            Self::Class(a) => (classes, a, "class"),
            Self::String(a) => (strings, a, "string"),
            Self::Code(a) => {
                let offset = *code
                    .get(a)
                    .ok_or_else(|| Error(format!("undefined code label {a}")))?;
                if offset >= 0x1000000 {
                    return Err(Error("code label exceeds 24-bit address space".into()));
                }
                return Ok(0x10000000 | offset);
            }
        };
        map.get(alias)
            .copied()
            .ok_or_else(|| Error(format!("undefined {role} alias {alias}")))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Key {
    Raw(u32),
    Symbol { alias: String, bits: u8 },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldDeclaration {
    pub key: Key,
    pub kind: u8,
    pub value: Reference,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declaration {
    pub alias: Option<String>,
    pub at: Option<u32>,
    pub version: u8,
    pub references: [Option<Reference>; 4],
    pub app_flags: u16,
    pub fields: Vec<FieldDeclaration>,
}

fn number(text: &str) -> Result<u32, Error> {
    let hex = text
        .strip_prefix("0x")
        .ok_or_else(|| Error("class integers require 0x prefixes".into()))?;
    if hex.is_empty() || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(Error("invalid class integer".into()));
    }
    u32::from_str_radix(hex, 16).map_err(|_| Error("class integer exceeds u32".into()))
}
fn value<'a>(fields: &'a [String], index: usize, name: &str) -> Result<&'a str, Error> {
    fields
        .get(index)
        .and_then(|s| s.strip_prefix(name))
        .ok_or_else(|| Error(format!("expected class attribute {name}")))
}
fn byte(text: &str) -> Result<u8, Error> {
    u8::try_from(number(text)?).map_err(|_| Error("class attribute exceeds u8".into()))
}

impl Declaration {
    pub fn from_record(class: &Class<'_>) -> Result<Self, Error> {
        Ok(Self {
            alias: None,
            at: Some(
                u32::try_from(class.offset)
                    .map_err(|_| Error("class offset exceeds u32".into()))?,
            ),
            version: class.version,
            references: [class.extends, class.statics, class.parent, class.module]
                .map(|r| r.map(Reference::Raw)),
            app_flags: class.app_flags,
            fields: class
                .fields
                .iter()
                .map(|f| FieldDeclaration {
                    key: Key::Raw(f.key),
                    kind: f.kind,
                    value: Reference::Raw(f.value),
                })
                .collect(),
        })
    }
    pub fn parse_header(fields: &[String]) -> Result<Self, Error> {
        let mut index = 1;
        let alias = if fields.get(index).is_some_and(|s| !s.contains('=')) {
            let alias = fields[index].clone();
            index += 1;
            if !valid_alias(&alias) {
                return Err(Error("invalid class alias".into()));
            }
            Some(alias)
        } else {
            None
        };
        let at = if fields.get(index).is_some_and(|s| s.starts_with("at=")) {
            let at = address(value(fields, index, "at=")?)?;
            index += 1;
            Some(at)
        } else {
            None
        };
        if fields.first().map(String::as_str) != Some(".class") || fields.len() != index + 6 {
            return Err(Error("expected .class [alias] [at=...] version=... extends=... statics=... parent=... module=... app_flags=...".into()));
        }
        let version = byte(value(fields, index, "version=")?)?;
        let mut references = [None, None, None, None];
        for (i, name) in ["extends=", "statics=", "parent=", "module="]
            .iter()
            .enumerate()
        {
            let text = value(fields, index + i + 1, name)?;
            references[i] = if text == "none" {
                None
            } else {
                Some(Reference::parse(text)?)
            };
        }
        let app_flags = u16::try_from(number(value(fields, index + 5, "app_flags=")?)?)
            .map_err(|_| Error("class app_flags exceeds u16".into()))?;
        Ok(Self {
            alias,
            at,
            version,
            references,
            app_flags,
            fields: Vec::new(),
        })
    }
    pub fn push_field(&mut self, fields: &[String]) -> Result<(), Error> {
        let (key, index) = if fields.len() == 4
            && fields[0] == ".field"
            && fields[1].starts_with("key=")
        {
            (Key::Raw(number(value(fields, 1, "key=")?)?), 2)
        } else if fields.len() == 5 && fields[0] == ".field" {
            let alias = value(fields, 1, "symbol=")?
                .strip_prefix('@')
                .filter(|s| valid_alias(s))
                .ok_or_else(|| Error("expected symbol=@alias".into()))?;
            (
                Key::Symbol {
                    alias: alias.into(),
                    bits: byte(value(fields, 2, "bits=")?)?,
                },
                3,
            )
        } else {
            return Err(Error("expected .field key=... type=... value=... or .field symbol=@alias bits=... type=... value=...".into()));
        };
        self.fields.push(FieldDeclaration {
            key,
            kind: byte(value(fields, index, "type=")?)?,
            value: Reference::parse(value(fields, index + 1, "value=")?)?,
        });
        Ok(())
    }
    pub fn encoded_length(&self) -> Result<usize, Error> {
        if !matches!(self.version, 1 | 2)
            || (self.version == 1 && self.references.iter().any(Option::is_none))
        {
            return Err(Error(
                "class requires version 1 or 2; version 1 requires all header references".into(),
            ));
        }
        if self.fields.len() > if self.version == 1 { 255 } else { 65535 } {
            return Err(Error("class field count exceeds format width".into()));
        }
        if self.alias.as_ref().is_some_and(|a| !valid_alias(a)) {
            return Err(Error("invalid class alias".into()));
        }
        for (i, r) in self.references.iter().enumerate() {
            if r.as_ref().is_some_and(|r| {
                !matches!(r, Reference::Raw(_)) && !(i >= 2 && matches!(r, Reference::Symbol(_)))
            }) {
                return Err(Error(
                    "only parent/module headers accept symbol references".into(),
                ));
            }
        }
        for f in &self.fields {
            match &f.value {
                Reference::Raw(_) => {}
                Reference::Class(_) if f.kind == 7 => {}
                Reference::String(_) if f.kind == 3 => {}
                Reference::Code(_) if f.kind == 6 => {}
                _ => {
                    return Err(Error(
                        "typed field value does not match class/string field type".into(),
                    ));
                }
            }
            let bits = match &f.key {
                Key::Raw(k) => (k & 255) as u8,
                Key::Symbol { bits, .. } => *bits,
            };
            if self.version == 1 && f.kind != bits & 15 {
                return Err(Error(
                    "version 1 field type must match key's low nibble".into(),
                ));
            }
        }
        Ok(if self.version == 1 {
            23 + self.fields.len() * 8
        } else {
            9 + self.references.iter().flatten().count() * 4 + self.fields.len() * 9
        })
    }
    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        self.encode_with(&Bindings::new(), &Bindings::new(), &Bindings::new())
    }
    pub fn encode_with(
        &self,
        symbols: &Bindings,
        classes: &Bindings,
        strings: &Bindings,
    ) -> Result<Vec<u8>, Error> {
        self.encode_with_code(symbols, classes, strings, &Bindings::new())
    }

    pub fn encode_with_code(
        &self,
        symbols: &Bindings,
        classes: &Bindings,
        strings: &Bindings,
        code: &Bindings,
    ) -> Result<Vec<u8>, Error> {
        self.encoded_length()?;
        let mut bytes = vec![0xc0 + self.version, 0xa5, 0x5d, 0xef];
        if self.version == 2 {
            bytes.push(
                self.references
                    .iter()
                    .enumerate()
                    .fold(0, |flags, (i, r)| flags | (u8::from(r.is_some()) << i)),
            );
        }
        for reference in self.references.iter().flatten() {
            bytes.extend(
                reference
                    .resolve(symbols, classes, strings, code)?
                    .to_be_bytes(),
            );
        }
        bytes.extend(self.app_flags.to_be_bytes());
        if self.version == 1 {
            bytes.push(self.fields.len() as u8);
        } else {
            bytes.extend((self.fields.len() as u16).to_be_bytes());
        }
        for f in &self.fields {
            let key = match &f.key {
                Key::Raw(k) => *k,
                Key::Symbol { alias, bits } => {
                    let id = symbols
                        .get(alias)
                        .ok_or_else(|| Error(format!("undefined symbol alias {alias}")))?;
                    if *id > 0xffffff {
                        return Err(Error("class field symbol ID exceeds 24 bits".into()));
                    }
                    (id << 8) | u32::from(*bits)
                }
            };
            bytes.extend(key.to_be_bytes());
            if self.version == 2 {
                bytes.push(f.kind);
            }
            bytes.extend(
                f.value
                    .resolve(symbols, classes, strings, code)?
                    .to_be_bytes(),
            );
        }
        Ok(bytes)
    }
    pub fn to_text(&self) -> String {
        let mut text = String::from(".class");
        if let Some(alias) = &self.alias {
            write!(text, " {alias}").unwrap();
        }
        if let Some(at) = self.at {
            write!(text, " at=0x{at:08x}").unwrap();
        }
        write!(text, " version=0x{:02x}", self.version).unwrap();
        for (name, r) in ["extends", "statics", "parent", "module"]
            .iter()
            .zip(&self.references)
        {
            write!(
                text,
                " {name}={}",
                r.as_ref()
                    .map(Reference::text)
                    .unwrap_or_else(|| "none".into())
            )
            .unwrap();
        }
        writeln!(text, " app_flags=0x{:04x}", self.app_flags).unwrap();
        for (index, f) in self.fields.iter().enumerate() {
            match &f.key {
                Key::Raw(k) => write!(text, ".field key=0x{k:08x}").unwrap(),
                Key::Symbol { alias, bits } => {
                    write!(text, ".field symbol=@{alias} bits=0x{bits:02x}").unwrap()
                }
            }
            write!(text, " type=0x{:02x} value={}", f.kind, f.value.text()).unwrap();
            let mut comment = false;
            if let Some(at) = self.at {
                let header = if self.version == 1 {
                    23
                } else {
                    9 + self.references.iter().flatten().count() * 4
                };
                let width = if self.version == 1 { 8 } else { 9 };
                write!(
                    text,
                    " # DATA+0x{:08x}",
                    u64::from(at) + (header + index * width) as u64
                )
                .unwrap();
                comment = true;
            }
            if f.kind == 6 {
                if let Reference::Raw(n) = f.value {
                    if n >> 24 == 0x10 {
                        write!(
                            text,
                            "{}CODE+0x{:08x}",
                            if comment { " -> " } else { " # " },
                            n & 0xffffff
                        )
                        .unwrap();
                    }
                }
            }
            text.push('\n');
        }
        text.push_str(".endclass\n");
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{
        Source,
        classes::{Record, records},
    };

    #[test]
    fn roundtrips_all_presence_combinations_and_legacy_fields() {
        for version in [1, 2] {
            for presence in 0..16u8 {
                if version == 1 && presence != 15 {
                    continue;
                }
                let declaration = Declaration {
                    at: Some(0),
                    alias: None,
                    version,
                    references: std::array::from_fn(|i| {
                        (presence & (1 << i) != 0).then_some(Reference::Raw(0x40000000 + i as u32))
                    }),
                    app_flags: 0x807f,
                    fields: vec![
                        FieldDeclaration {
                            key: Key::Raw(0x80006746),
                            kind: 6,
                            value: Reference::Raw(0x10000030),
                        },
                        FieldDeclaration {
                            key: Key::Raw(0x8000674e),
                            kind: if version == 1 { 14 } else { 254 },
                            value: Reference::Raw(0xdeadbeef),
                        },
                    ],
                };
                let bytes = declaration.encode().unwrap();
                let parsed = records(&bytes).unwrap();
                let Record::Class(class) = &parsed[0] else {
                    panic!()
                };
                assert_eq!(Declaration::from_record(class).unwrap(), declaration);
                let text = declaration.to_text();
                assert_eq!(Source::parse(&text).unwrap().encode().unwrap().0, bytes);
                assert_eq!(Source::from_records(&bytes).unwrap().to_text(), text);
            }
        }
    }

    #[test]
    fn rejects_malformed_blocks_widths_legacy_type_mismatch_and_count_overflow() {
        let header = ".class at=0x0 version=0x02 extends=none statics=none parent=none module=none app_flags=0x0\n";
        for tail in [
            "",
            ".class\n",
            ".string a \"x\"\n",
            ".field key=0x1 type=0x100 value=0x0\n.endclass\n",
            ".endclass extra\n",
        ] {
            assert!(Source::parse(&format!("{header}{tail}")).is_err());
        }
        assert!(Source::parse(".field key=0x1 type=0x6 value=0x0").is_err());
        for invalid in [
            header.replace("version=0x02", "version=0x03"),
            header.replace("app_flags=0x0", "app_flags=0x10000"),
            header.replace("version=0x02", "version=0x01"),
        ] {
            assert!(Source::parse(&format!("{invalid}.endclass\n")).is_err());
        }
        let mut legacy = Declaration {
            at: Some(0),
            alias: None,
            version: 1,
            references: std::array::from_fn(|_| Some(Reference::Raw(0))),
            app_flags: 0,
            fields: vec![FieldDeclaration {
                key: Key::Raw(6),
                kind: 7,
                value: Reference::Raw(0),
            }],
        };
        assert!(legacy.encode().is_err());
        legacy.fields = vec![
            FieldDeclaration {
                key: Key::Raw(6),
                kind: 6,
                value: Reference::Raw(0)
            };
            256
        ];
        assert!(legacy.encode().is_err());
        legacy.version = 2;
        legacy.fields.resize(
            65536,
            FieldDeclaration {
                key: Key::Raw(6),
                kind: 6,
                value: Reference::Raw(0),
            },
        );
        assert!(legacy.encode().is_err());
    }

    #[test]
    fn method_edit_preserves_every_other_byte_and_pinned_record_layout() {
        let text = ".class at=0x0 version=0x02 extends=none statics=none parent=0x8002e6 module=none app_flags=0x0\n.field key=0x8146 type=0x06 value=0x10000030\n.field key=0x8146 type=0xff value=0xdeadbeef\n.endclass\n.string text \"hello\" at=0x1f\n";
        let original = Source::parse(text).unwrap().encode().unwrap().0;
        let changed = Source::parse(&text.replace("0x10000030", "0x10000040"))
            .unwrap()
            .encode()
            .unwrap()
            .0;
        let differences: Vec<_> = original
            .iter()
            .zip(&changed)
            .enumerate()
            .filter(|(_, (a, b))| a != b)
            .map(|(i, _)| i)
            .collect();
        assert_eq!(differences, vec![21]);
        assert!(Source::parse(&text.replace("hello", "longer")).is_ok()); // final record has no following pinned span
        assert!(
            Source::parse(&text.replace(
                ".string text",
                ".field key=0x1 type=0x0 value=0x0\n.string text"
            ))
            .is_err()
        );
        let extended = text.replace(".endclass", ".field key=0x1 type=0x0 value=0x0\n.endclass");
        assert!(Source::parse(&extended).is_err()); // would move the pinned string
    }
}
