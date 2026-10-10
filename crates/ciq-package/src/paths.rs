use crate::{Error, Limits, Member, Result};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn relative(name: &str, directory: bool, limits: Limits) -> Result<String> {
    if name.len() > limits.path_bytes
        || name.contains(['\\', ':'])
        || name.chars().any(char::is_control)
    {
        return Err(Error::new("unsafe or oversized member path"));
    }
    let path = if directory {
        name.strip_suffix('/').unwrap_or(name)
    } else {
        name
    };
    if path.is_empty() {
        return Err(Error::new("empty member path"));
    }
    for component in path.split('/') {
        let stem = component
            .split('.')
            .next()
            .unwrap_or("")
            .to_ascii_uppercase();
        let reserved = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
            || (stem.len() == 4
                && (stem.starts_with("COM") || stem.starts_with("LPT"))
                && matches!(stem.as_bytes()[3], b'1'..=b'9'));
        if component.is_empty()
            || matches!(component, "." | "..")
            || component.ends_with(['.', ' '])
            || reserved
        {
            return Err(Error::new(format!("unsafe member path: {name}")));
        }
    }
    Ok(path.to_owned())
}
pub(crate) fn validate(members: &[Member], limits: Limits) -> Result<()> {
    if members.len() > limits.entries {
        return Err(Error::new("archive entry count exceeds limit"));
    }
    let mut seen = BTreeSet::new();
    let mut paths: BTreeMap<String, (String, bool)> = BTreeMap::new();
    let mut total = 0u64;
    for member in members {
        let path = relative(&member.name, member.directory, limits)?;
        if !seen.insert(path.clone()) {
            return Err(Error::new("duplicate archive path"));
        }
        if member.size > limits.member_bytes || (member.directory && member.size != 0) {
            return Err(Error::new("invalid or oversized member size"));
        }
        total = total
            .checked_add(member.size)
            .ok_or_else(|| Error::new("archive size overflow"))?;
        if total > limits.total_bytes {
            return Err(Error::new("archive total decoded size exceeds limit"));
        }
        let parts: Vec<_> = path.split('/').collect();
        for i in 1..=parts.len() {
            let prefix = parts[..i].join("/");
            let directory = i < parts.len() || member.directory;
            let key = prefix.to_lowercase();
            if let Some((old, old_directory)) = paths.get(&key) {
                if old != &prefix || *old_directory != directory || !directory {
                    return Err(Error::new("colliding archive paths"));
                }
            } else {
                paths.insert(key, (prefix, directory));
            }
        }
    }
    Ok(())
}
