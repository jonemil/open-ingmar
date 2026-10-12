//! Resolve compiler module/member aliases using an explicit symbol source.
use ciq_assembly::{Error, Item, Operand, Program};
use std::collections::BTreeMap;

/// Build a symbol alias-to-ID map for encoding the supplied assembly program.
///
/// `symbols` must contain only valid `.symbol` declarations. Compiler-generated
/// aliases must match exactly one stored name: modules use `globals/<name>`,
/// while members use their short names. The dedicated `__ciq_sdk_System` alias
/// uses `Toybox_System`; no general SDK module-name inference is performed.
/// Native initialization binds AppBase/initialize, and Number methods bind
/// Number/String methods by their stored names. Native System.exit binds exit.
/// Missing, ambiguous, or conflicting
/// bindings are errors. No PRG symbol records are generated or modified.
pub fn symbol_bindings(
    program: &Program,
    symbols: &Program,
) -> Result<BTreeMap<String, u32>, Error> {
    let fail = |message: String| Error { line: 0, message };
    if symbols
        .items()
        .iter()
        .any(|item| !matches!(item, Item::Symbol(_)))
    {
        return Err(fail(
            "symbol source must contain only .symbol declarations".into(),
        ));
    }
    // The shared encoder validates duplicate aliases/IDs even for empty CODE.
    symbols.encode()?;
    let declarations: Vec<_> = symbols
        .items()
        .iter()
        .filter_map(|item| {
            if let Item::Symbol(declaration) = item {
                Some(declaration)
            } else {
                None
            }
        })
        .collect();
    let mut bindings: BTreeMap<_, _> = declarations
        .iter()
        .map(|d| (d.alias.clone(), d.id))
        .collect();
    for item in program.items() {
        let Item::Instruction { operands, .. } = item else {
            continue;
        };
        for operand in operands {
            let Operand::Symbol(alias) = operand else {
                continue;
            };
            let name = if alias == "__ciq_sdk_System" {
                "Toybox_System".to_string()
            } else if alias == "__ciq_native_AppBase" {
                "AppBase".to_string()
            } else if alias == "__ciq_native_toString" {
                "toString".to_string()
            } else if alias == "__ciq_native_format" {
                "format".to_string()
            } else if alias == "__ciq_native_abs" {
                "abs".to_string()
            } else if alias == "__ciq_native_equals" {
                "equals".to_string()
            } else if alias == "__ciq_native_hashCode" {
                "hashCode".to_string()
            } else if alias == "__ciq_native_weak" {
                "weak".to_string()
            } else if alias == "__ciq_native_compareTo" {
                "compareTo".to_string()
            } else if alias == "__ciq_native_length" {
                "length".to_string()
            } else if alias == "__ciq_native_substring" {
                "substring".to_string()
            } else if alias == "__ciq_native_find" {
                "find".to_string()
            } else if alias == "__ciq_native_toNumber" {
                "toNumber".to_string()
            } else if alias == "__ciq_native_toChar" {
                "toChar".to_string()
            } else if alias == "__ciq_native_toCharArray" {
                "toCharArray".to_string()
            } else if alias == "__ciq_native_size" {
                "size".to_string()
            } else if alias == "__ciq_native_indexOf" {
                "indexOf".to_string()
            } else if alias == "__ciq_native_toNumberWithBase" {
                "toNumberWithBase".to_string()
            } else if alias == "__ciq_native_toLower" {
                "toLower".to_string()
            } else if alias == "__ciq_native_toUpper" {
                "toUpper".to_string()
            } else if alias == "__ciq_native_exit" {
                "exit".to_string()
            } else if alias == "__ciq_native_initialize" {
                "initialize".to_string()
            } else if let Some(module) = alias.strip_prefix("__ciq_module_") {
                format!("globals/{module}")
            } else if let Some(member) = alias.strip_prefix("__ciq_member_") {
                member.to_string()
            } else {
                continue;
            };
            let matches: Vec<_> = declarations.iter().filter(|d| d.name == name).collect();
            if matches.len() != 1 {
                return Err(fail(format!(
                    "expected one symbol named {name:?} for @{alias}; found {}",
                    matches.len()
                )));
            }
            let id = matches[0].id;
            if bindings
                .insert(alias.clone(), id)
                .is_some_and(|old| old != id)
            {
                return Err(fail(format!("conflicting binding for @{alias}")));
            }
        }
    }
    Ok(bindings)
}

/// Bind CODE literal declarations to matching strings in an explicit DATA source.
///
/// Validates DATA layout without encoding method references. Each unplaced
/// literal must match exactly one DATA string by content. Pinned literals must
/// match the record at their stated offset. Referenced undeclared aliases use
/// DATA aliases directly. Missing, ambiguous, or conflicting records are errors;
/// this function neither allocates strings nor changes DATA.
pub fn string_bindings(
    program: &Program,
    data: &ciq_prg::data::Source,
) -> Result<BTreeMap<String, u32>, Error> {
    use ciq_prg::data::Fragment;
    let fail = |message: String| Error { line: 0, message };
    let offsets = data
        .string_bindings()
        .map_err(|error| fail(error.to_string()))?;
    let mut bindings = offsets.clone();
    for item in program.items() {
        let Item::String(literal) = item else {
            continue;
        };
        let matches: Vec<_> = data
            .fragments
            .iter()
            .filter_map(|fragment| {
                let Fragment::String(record) = fragment else {
                    return None;
                };
                let at = offsets[&record.alias];
                (record.value == literal.value && literal.at.is_none_or(|pinned| pinned == at))
                    .then_some(at)
            })
            .collect();
        if matches.len() != 1 {
            return Err(fail(format!(
                "expected one DATA string matching &{}; found {}",
                literal.alias,
                matches.len()
            )));
        }
        let at = matches[0];
        if bindings
            .insert(literal.alias.clone(), at)
            .is_some_and(|old| old != at)
        {
            return Err(fail(format!(
                "conflicting DATA binding for &{}",
                literal.alias
            )));
        }
    }
    // Pinned inline declarations provide their own bindings to the encoder.
    for item in program.items() {
        if let Item::String(literal) = item {
            if literal.at.is_some() {
                bindings.remove(&literal.alias);
            }
        }
    }
    Ok(bindings)
}
