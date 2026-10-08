//! Observed symbol-table payload format. See docs/specs/symbols.md.

pub mod source;

use crate::Error;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Symbol<'a> {
    pub id: u32,
    pub name_offset: u32,
    pub entry_index: usize,
    pub name: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbols<'a> {
    symbols: BTreeMap<u32, Symbol<'a>>,
}

impl<'a> Symbols<'a> {
    pub fn parse(payload: &'a [u8]) -> Result<Self, Error> {
        if payload.len() < 2 {
            return Err(Error {
                offset: 0,
                message: "truncated symbol count",
            });
        }
        let count = u16::from_be_bytes(payload[..2].try_into().unwrap()) as usize;
        let table_end = 2 + count * 8;
        if table_end > payload.len() {
            return Err(Error {
                offset: 2,
                message: "symbol entries exceed section length",
            });
        }
        let mut names = BTreeMap::new();
        let mut offset = table_end;
        while offset < payload.len() {
            if payload.len() - offset < 4 {
                return Err(Error {
                    offset,
                    message: "truncated symbol name record",
                });
            }
            if payload[offset] != 1 {
                return Err(Error {
                    offset,
                    message: "unsupported symbol name tag",
                });
            }
            let length =
                u16::from_be_bytes(payload[offset + 1..offset + 3].try_into().unwrap()) as usize;
            if length > payload.len() - offset - 4 {
                return Err(Error {
                    offset,
                    message: "symbol name exceeds section length",
                });
            }
            let end = offset + 3 + length;
            if payload[end] != 0 {
                return Err(Error {
                    offset: end,
                    message: "symbol name lacks zero terminator",
                });
            }
            let name = std::str::from_utf8(&payload[offset + 3..end]).map_err(|_| Error {
                offset: offset + 3,
                message: "symbol name is not UTF-8",
            })?;
            names.insert(offset, name);
            offset = end + 1;
        }
        let mut symbols = BTreeMap::new();
        for index in 0..count {
            let offset = 2 + index * 8;
            let id = u32::from_be_bytes(payload[offset..offset + 4].try_into().unwrap());
            let name_offset =
                u32::from_be_bytes(payload[offset + 4..offset + 8].try_into().unwrap());
            let name = *names.get(&(name_offset as usize)).ok_or(Error {
                offset: offset + 4,
                message: "symbol reference does not point to a name record",
            })?;
            if symbols
                .insert(
                    id,
                    Symbol {
                        id,
                        name_offset,
                        entry_index: index,
                        name,
                    },
                )
                .is_some()
            {
                return Err(Error {
                    offset,
                    message: "duplicate symbol identifier",
                });
            }
        }
        Ok(Self { symbols })
    }

    pub fn get(&self, id: u32) -> Option<&Symbol<'a>> {
        self.symbols.get(&id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Symbol<'a>> {
        self.symbols.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        // Two entries reference records in reverse physical order.
        let mut bytes = vec![0, 2];
        for (id, offset) in [(0x800005u32, 26u32), (0x800067, 18)] {
            bytes.extend_from_slice(&id.to_be_bytes());
            bytes.extend_from_slice(&offset.to_be_bytes());
        }
        bytes.extend_from_slice(&[1, 0, 4, b'c', b'a', b'l', b'l', 0]);
        bytes.extend_from_slice(&[1, 0, 2, b'M', b'C', 0]);
        bytes
    }

    #[test]
    fn resolves_offsets_instead_of_assuming_name_order() {
        let bytes = fixture();
        let symbols = Symbols::parse(&bytes).unwrap();
        assert_eq!(symbols.get(0x800005).unwrap().name, "MC");
        assert_eq!(symbols.get(0x800067).unwrap().name, "call");
        assert!(symbols.get(123).is_none());
    }

    #[test]
    fn rejects_truncated_tables_and_names() {
        let bytes = fixture();
        for end in 0..bytes.len() {
            assert!(Symbols::parse(&bytes[..end]).is_err(), "prefix {end}");
        }
    }

    #[test]
    fn rejects_duplicate_ids_bad_offsets_tags_terminators_and_utf8() {
        let original = fixture();
        let mut bytes = original.clone();
        let id = bytes[2..6].to_vec();
        bytes[10..14].copy_from_slice(&id);
        assert!(Symbols::parse(&bytes).is_err());
        for offset in [0u32, 19, u32::MAX] {
            let mut bytes = original.clone();
            bytes[6..10].copy_from_slice(&offset.to_be_bytes());
            assert!(Symbols::parse(&bytes).is_err());
        }
        for (offset, value) in [(18, 2), (25, 1), (21, 0xff)] {
            let mut bytes = original.clone();
            bytes[offset] = value;
            assert!(Symbols::parse(&bytes).is_err());
        }
        assert!(Symbols::parse(&[0, 0]).unwrap().iter().next().is_none());
    }
}
