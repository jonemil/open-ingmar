//! Lossless class declarations; see docs/specs/data-classes.md.
use super::{Class, Field};
use crate::{data::address, symbols::source::Error};
use std::fmt::Write;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declaration {
    pub at: u32,
    pub version: u8,
    pub references: [Option<u32>; 4],
    pub app_flags: u16,
    pub fields: Vec<Field>,
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

impl Declaration {
    pub fn from_record(class: &Class<'_>) -> Result<Self, Error> {
        Ok(Self {
            at: u32::try_from(class.offset)
                .map_err(|_| Error("class offset exceeds u32".into()))?,
            version: class.version,
            references: [class.extends, class.statics, class.parent, class.module],
            app_flags: class.app_flags,
            fields: class
                .fields
                .iter()
                .map(|field| Field {
                    offset: 0,
                    ..field.clone()
                })
                .collect(),
        })
    }

    pub fn parse_header(fields: &[String]) -> Result<Self, Error> {
        if fields.len() != 8 || fields[0] != ".class" {
            return Err(Error("expected .class at=... version=... extends=... statics=... parent=... module=... app_flags=...".into()));
        }
        let at = address(value(fields, 1, "at=")?)?;
        let version = u8::try_from(number(value(fields, 2, "version=")?)?)
            .map_err(|_| Error("class version exceeds u8".into()))?;
        let mut references = [None; 4];
        for (index, name) in ["extends=", "statics=", "parent=", "module="]
            .iter()
            .enumerate()
        {
            let text = value(fields, index + 3, name)?;
            references[index] = if text == "none" {
                None
            } else {
                Some(number(text)?)
            };
        }
        let app_flags = u16::try_from(number(value(fields, 7, "app_flags=")?)?)
            .map_err(|_| Error("class app_flags exceeds u16".into()))?;
        Ok(Self {
            at,
            version,
            references,
            app_flags,
            fields: Vec::new(),
        })
    }

    pub fn push_field(&mut self, fields: &[String]) -> Result<(), Error> {
        if fields.len() != 4 || fields[0] != ".field" {
            return Err(Error(
                "expected .field key=0x... type=0x... value=0x...".into(),
            ));
        }
        self.fields.push(Field {
            offset: 0,
            key: number(value(fields, 1, "key=")?)?,
            kind: u8::try_from(number(value(fields, 2, "type=")?)?)
                .map_err(|_| Error("class field type exceeds u8".into()))?,
            value: number(value(fields, 3, "value=")?)?,
        });
        Ok(())
    }

    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        if !matches!(self.version, 1 | 2)
            || (self.version == 1 && self.references.iter().any(Option::is_none))
        {
            return Err(Error(
                "class requires version 1 or 2; version 1 requires all header references".into(),
            ));
        }
        let mut bytes = vec![0xc0 + self.version, 0xa5, 0x5d, 0xef];
        if self.version == 2 {
            bytes.push(
                self.references
                    .iter()
                    .enumerate()
                    .fold(0, |flags, (i, reference)| {
                        flags | (u8::from(reference.is_some()) << i)
                    }),
            );
        }
        for reference in self.references.iter().flatten() {
            bytes.extend(reference.to_be_bytes());
        }
        bytes.extend(self.app_flags.to_be_bytes());
        if self.version == 1 {
            bytes.push(
                u8::try_from(self.fields.len())
                    .map_err(|_| Error("version 1 field count exceeds u8".into()))?,
            );
        } else {
            bytes.extend(
                u16::try_from(self.fields.len())
                    .map_err(|_| Error("version 2 field count exceeds u16".into()))?
                    .to_be_bytes(),
            );
        }
        for field in &self.fields {
            bytes.extend(field.key.to_be_bytes());
            if self.version == 1 {
                if field.kind != (field.key & 15) as u8 {
                    return Err(Error(
                        "version 1 field type must match key's low nibble".into(),
                    ));
                }
            } else {
                bytes.push(field.kind);
            }
            bytes.extend(field.value.to_be_bytes());
        }
        Ok(bytes)
    }

    pub fn to_text(&self) -> String {
        let mut text = format!(".class at=0x{:08x} version=0x{:02x}", self.at, self.version);
        for (name, reference) in ["extends", "statics", "parent", "module"]
            .iter()
            .zip(self.references)
        {
            match reference {
                Some(reference) => write!(text, " {name}=0x{reference:08x}").unwrap(),
                None => write!(text, " {name}=none").unwrap(),
            }
        }
        writeln!(text, " app_flags=0x{:04x}", self.app_flags).unwrap();
        for field in &self.fields {
            writeln!(
                text,
                ".field key=0x{:08x} type=0x{:02x} value=0x{:08x}",
                field.key, field.kind, field.value
            )
            .unwrap();
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
                    at: 0,
                    version,
                    references: std::array::from_fn(|i| {
                        (presence & (1 << i) != 0).then_some(0x40000000 + i as u32)
                    }),
                    app_flags: 0x807f,
                    fields: vec![
                        Field {
                            offset: 0,
                            key: 0x80006746,
                            kind: 6,
                            value: 0x10000030,
                        },
                        Field {
                            offset: 0,
                            key: 0x8000674e,
                            kind: if version == 1 { 14 } else { 254 },
                            value: 0xdeadbeef,
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
            at: 0,
            version: 1,
            references: [Some(0); 4],
            app_flags: 0,
            fields: vec![Field {
                offset: 0,
                key: 6,
                kind: 7,
                value: 0,
            }],
        };
        assert!(legacy.encode().is_err());
        legacy.fields = vec![
            Field {
                offset: 0,
                key: 6,
                kind: 6,
                value: 0
            };
            256
        ];
        assert!(legacy.encode().is_err());
        legacy.version = 2;
        legacy.fields.resize(
            65536,
            Field {
                offset: 0,
                key: 6,
                kind: 6,
                value: 0,
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
