//! Source superclass resolution and validated nominal ancestry.
use crate::{ClassBase, ClassDeclaration, SourceClass};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone)]
pub(crate) enum Parent {
    Native(ClassBase),
    Source(String),
}

/// Resolve every class to its terminal native base and ordered source ancestors.
/// Reject unknown parents, cycles, and chains exceeding the shared 64-level bound.
pub(crate) fn resolve(
    parents: &BTreeMap<String, Parent>,
) -> Result<BTreeMap<String, SourceClass>, (String, String)> {
    let mut result = BTreeMap::new();
    for name in parents.keys() {
        let mut path = Vec::new();
        let mut seen = BTreeSet::new();
        let mut at = name.as_str();
        let base = loop {
            if !seen.insert(at) {
                return Err((
                    name.clone(),
                    format!("cyclic source superclass chain at {at}"),
                ));
            }
            if path.len() >= 64 {
                return Err((
                    name.clone(),
                    "source superclass chain exceeds 64 classes".into(),
                ));
            }
            path.push(at.to_owned());
            match parents.get(at) {
                Some(Parent::Native(base)) => break *base,
                Some(Parent::Source(parent)) => at = parent,
                None => return Err((name.clone(), format!("unresolved source superclass {at}"))),
            }
        };
        result.insert(
            name.clone(),
            SourceClass {
                name: name.clone(),
                base,
                ancestors: path.into_iter().skip(1).collect(),
            },
        );
    }
    Ok(result)
}

/// Rebuild ancestry from unit declarations and check their native projections.
/// Saved type contracts must agree with this graph rather than supplying parents.
pub(crate) fn for_classes(
    classes: &[ClassDeclaration],
) -> Result<BTreeMap<String, SourceClass>, String> {
    let parents = classes
        .iter()
        .map(|c| {
            (
                c.name.clone(),
                c.superclass
                    .as_ref()
                    .map_or(Parent::Native(c.base), |p| Parent::Source(p.clone())),
            )
        })
        .collect();
    let types = resolve(&parents).map_err(|(_, message)| message)?;
    for class in classes {
        if types[&class.name].base != class.base {
            return Err(format!(
                "native projection disagrees with superclass chain for {}",
                class.name
            ));
        }
    }
    Ok(types)
}
