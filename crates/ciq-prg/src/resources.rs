//! Read-only resource string indexes.

use crate::symbols::source::Error;
use std::collections::BTreeSet;

pub const STRINGS_CATEGORY: u32 = 0x0080_00a2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceString<'a> {
    pub language: u32,
    pub id: u32,
    pub offset: u32,
    pub value: &'a str,
}

fn table(payload: &[u8], offset: u32) -> Result<Vec<(u32, u32)>, Error> {
    let at = offset as usize;
    if at > payload.len() || payload.len() - at < 2 {
        return Err(Error("truncated resource index count".into()));
    }
    let count = u16::from_be_bytes(payload[at..at + 2].try_into().unwrap()) as usize;
    if count > (payload.len() - at - 2) / 8 {
        return Err(Error("resource index exceeds payload".into()));
    }
    let mut entries = Vec::new();
    let mut ids = BTreeSet::new();
    for index in 0..count {
        let start = at + 2 + index * 8;
        let id = u32::from_be_bytes(payload[start..start + 4].try_into().unwrap());
        let offset = u32::from_be_bytes(payload[start + 4..start + 8].try_into().unwrap());
        if !ids.insert(id) {
            return Err(Error("duplicate resource index ID".into()));
        }
        entries.push((id, offset));
    }
    Ok(entries)
}

/// Follow the string category, language indexes, and resource indexes only.
pub fn strings(payload: &[u8]) -> Result<Vec<ResourceString<'_>>, Error> {
    let mut strings = Vec::new();
    let mut visited = BTreeSet::from([0]);
    for (category, at) in table(payload, 0)? {
        if category != STRINGS_CATEGORY {
            continue;
        }
        if !visited.insert(at) {
            return Err(Error("cyclic resource string indexes".into()));
        }
        for (language, at) in table(payload, at)? {
            if !visited.insert(at) {
                return Err(Error(
                    "shared or cyclic resource language indexes are unsupported".into(),
                ));
            }
            for (id, offset) in table(payload, at)? {
                let (value, _) = crate::data::string_at(payload, offset)?;
                strings.push(ResourceString {
                    language,
                    id,
                    offset,
                    value,
                });
            }
        }
    }
    Ok(strings)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Vec<u8> {
        let mut bytes = Vec::new();
        for (id, at) in [(STRINGS_CATEGORY, 10u32), (0x8002e7, 20), (0x83, 30)] {
            bytes.extend_from_slice(&1u16.to_be_bytes());
            bytes.extend_from_slice(&id.to_be_bytes());
            bytes.extend_from_slice(&at.to_be_bytes());
        }
        bytes.extend_from_slice(&[1, 0, 2, b'h', b'i', 0]);
        bytes
    }
    #[test]
    fn follows_language_and_resource_ids_without_treating_them_as_addresses() {
        let bytes = fixture();
        let result = strings(&bytes).unwrap();
        assert_eq!(
            result,
            vec![ResourceString {
                language: 0x8002e7,
                id: 0x83,
                offset: 30,
                value: "hi"
            }]
        );
        assert!(strings(&[0, 0]).unwrap().is_empty());
    }
    #[test]
    fn rejects_truncated_indexes_records_and_cyclic_offsets() {
        let bytes = fixture();
        for end in 0..bytes.len() {
            assert!(strings(&bytes[..end]).is_err(), "prefix {end}");
        }
        for at in [0u32, 35, u32::MAX] {
            let mut broken = bytes.clone();
            broken[6..10].copy_from_slice(&at.to_be_bytes());
            assert!(strings(&broken).is_err());
        }
    }
}
