//! Resolve compiler module/member aliases using an explicit symbol source.
use ciq_assembly::{Error, Item, Operand, Program};
use std::collections::BTreeMap;

/// Resolve CODE references without generating or changing a PRG symbol table.
/// Compiler module aliases denote `globals/<name>`, not the short field name.
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
            let name = if let Some(module) = alias.strip_prefix("__ciq_module_") {
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
