//! Read-only DATA class/module framing. See docs/specs/data-classes.md.
use crate::symbols::source::Error;
pub mod source;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub offset: usize,
    pub key: u32,
    pub kind: u8,
    pub value: u32,
}
impl Field {
    pub fn symbol_id(&self) -> u32 {
        self.key >> 8
    }
    /// Only the ordinary CODE address space is interpreted at present.
    pub fn code_offset(&self) -> Option<u32> {
        (self.kind == 6 && self.value >> 24 == 0x10).then_some(self.value & 0x00ff_ffff)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Class<'a> {
    pub offset: usize,
    pub version: u8,
    pub presence: Option<u8>,
    /// Raw header references; their address spaces are not universally established.
    pub extends: Option<u32>,
    pub statics: Option<u32>,
    pub parent: Option<u32>,
    pub module: Option<u32>,
    pub app_flags: u16,
    pub fields: Vec<Field>,
    pub raw: &'a [u8],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Record<'a> {
    String { offset: usize, value: &'a str },
    Class(Class<'a>),
}

fn word(payload: &[u8], at: &mut usize) -> Result<u32, Error> {
    let bytes = payload
        .get(*at..*at + 4)
        .ok_or_else(|| Error(format!("truncated class header at DATA+0x{at:08x}")))?;
    *at += 4;
    Ok(u32::from_be_bytes(bytes.try_into().unwrap()))
}

/// Walk from the beginning of DATA; unknown records abort without scanning ahead.
pub fn records(payload: &[u8]) -> Result<Vec<Record<'_>>, Error> {
    let mut records = Vec::new();
    let mut at = 0;
    while at < payload.len() {
        let start = at;
        if payload[at] == 1 {
            let offset = u32::try_from(at).map_err(|_| Error("DATA offset exceeds u32".into()))?;
            let (value, end) = super::string_at(payload, offset)?;
            records.push(Record::String {
                offset: start,
                value,
            });
            at = end;
            continue;
        }
        let version = match payload[at] {
            0xc1 => 1,
            0xc2 => 2,
            _ => return Err(Error(format!("unknown DATA record at 0x{at:08x}"))),
        };
        if payload.get(at + 1..at + 4) != Some(&[0xa5, 0x5d, 0xef]) {
            return Err(Error(format!(
                "unsupported class magic at DATA+0x{start:08x}"
            )));
        }
        at += 4;
        let presence = if version == 2 {
            let flags = *payload
                .get(at)
                .ok_or_else(|| Error("truncated class presence flags".into()))?;
            if flags & 0xf0 != 0 {
                return Err(Error("unsupported class presence flags".into()));
            }
            at += 1;
            Some(flags)
        } else {
            None
        };
        let flags = presence.unwrap_or(0x0f);
        let mut refs = [None; 4];
        for (index, value) in refs.iter_mut().enumerate() {
            if flags & (1 << index) != 0 {
                *value = Some(word(payload, &mut at)?);
            }
        }
        let count_width = if version == 2 { 2 } else { 1 };
        let header = payload
            .get(at..at + 2 + count_width)
            .ok_or_else(|| Error(format!("truncated class header at DATA+0x{start:08x}")))?;
        let app_flags = u16::from_be_bytes(header[..2].try_into().unwrap());
        let count = if version == 2 {
            u16::from_be_bytes(header[2..4].try_into().unwrap()) as usize
        } else {
            header[2] as usize
        };
        at += 2 + count_width;
        let width = if version == 2 { 9 } else { 8 };
        if count > (payload.len() - at) / width {
            return Err(Error(format!(
                "class fields exceed DATA payload at 0x{start:08x}"
            )));
        }
        let mut fields = Vec::with_capacity(count);
        for _ in 0..count {
            let offset = at;
            let key = word(payload, &mut at)?;
            let kind = if version == 2 {
                let kind = payload[at];
                at += 1;
                kind
            } else {
                (key & 15) as u8
            };
            let value = word(payload, &mut at)?;
            fields.push(Field {
                offset,
                key,
                kind,
                value,
            });
        }
        records.push(Record::Class(Class {
            offset: start,
            version,
            presence,
            extends: refs[0],
            statics: refs[1],
            parent: refs[2],
            module: refs[3],
            app_flags,
            fields,
            raw: &payload[start..at],
        }));
    }
    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_presence_combinations_preserve_header_and_unknown_fields() {
        for flags in 0..16u8 {
            let mut bytes = vec![0xc2, 0xa5, 0x5d, 0xef, flags];
            let mut expected = [None; 4];
            for (index, reference) in expected.iter_mut().enumerate() {
                if flags & (1 << index) != 0 {
                    *reference = Some(0x40000100 + index as u32);
                    bytes.extend(reference.unwrap().to_be_bytes());
                }
            }
            bytes.extend([0x80, 0x7f, 0, 1]);
            bytes.extend(0x00800067u32.to_be_bytes());
            bytes.push(0xfe); // Unknown type with string-like bytes in its raw value.
            bytes.extend([1, 0, 0, 0]);
            let result = records(&bytes).unwrap();
            let Record::Class(c) = &result[0] else {
                panic!()
            };
            assert_eq!([c.extends, c.statics, c.parent, c.module], expected);
            assert_eq!(c.app_flags, 0x807f);
            assert_eq!(c.fields[0].kind, 0xfe);
            assert_eq!(c.fields[0].key, 0x00800067);
            assert_eq!(c.raw, bytes);
            for end in 1..bytes.len() {
                assert!(records(&bytes[..end]).is_err());
            }
        }
    }
    #[test]
    fn v1_embedded_kind_and_v2_explicit_kind_have_same_method_reference() {
        for version in [1, 2] {
            let mut bytes = vec![0xc0 + version, 0xa5, 0x5d, 0xef];
            if version == 1 {
                bytes.extend([0; 16]);
            } else {
                bytes.push(0);
            }
            bytes.extend([0, 0]);
            if version == 1 {
                bytes.push(1);
            } else {
                bytes.extend([0, 1]);
            }
            bytes.extend(0x80006746u32.to_be_bytes());
            if version == 2 {
                bytes.push(6);
            }
            bytes.extend(0x10000024u32.to_be_bytes());
            bytes.extend([1, 0, 1, b'x', 0]);
            let result = records(&bytes).unwrap();
            let Record::Class(c) = &result[0] else {
                panic!()
            };
            assert_eq!(c.fields[0].symbol_id(), 0x00800067);
            assert_eq!(c.fields[0].code_offset(), Some(0x24));
            assert!(matches!(result[1], Record::String { value: "x", .. }));
        }
    }
    #[test]
    fn invalid_magic_presence_and_unknown_records_fail_without_scanning() {
        for bytes in [
            &[0xc2, 0, 0, 0][..],
            &[0xc2, 0xa5, 0x5d, 0xef, 0x80],
            &[3, 1, 0, 0, 0],
        ] {
            assert!(records(bytes).is_err());
        }
        for (kind, value) in [(3, 0x10000024), (6, 0x30000024), (6, 0x40010024)] {
            assert_eq!(
                Field {
                    offset: 0,
                    key: 0,
                    kind,
                    value
                }
                .code_offset(),
                None
            );
        }
    }
}
