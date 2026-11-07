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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MemberIndex {
    pub(crate) methods: BTreeMap<(String, String), (usize, usize)>,
    pub(crate) fields: BTreeMap<(String, String), (usize, usize)>,
    pub(crate) classes: BTreeMap<String, SourceClass>,
}

/// Index effective methods and fields without copying declarations.
/// Private ancestors and initializer/hook methods stay local.
pub(crate) fn member_index(
    classes: &[ClassDeclaration],
    visibility: &crate::visibility::Members,
) -> Result<MemberIndex, String> {
    let hierarchy = for_classes(classes)?;
    let indexes = classes
        .iter()
        .enumerate()
        .map(|(i, c)| (c.name.as_str(), i))
        .collect::<BTreeMap<_, _>>();
    let mut result = BTreeMap::new();
    let mut fields = BTreeMap::new();
    for class in classes {
        for owner in hierarchy[&class.name]
            .ancestors
            .iter()
            .rev()
            .chain(std::iter::once(&class.name))
        {
            let index = indexes[owner.as_str()];
            for (field, declaration) in classes[index].fields.iter().enumerate() {
                if owner != &class.name
                    && visibility.get(&(owner.clone(), declaration.name.clone()))
                        == Some(&crate::Visibility::Private)
                {
                    continue;
                }
                fields.insert(
                    (class.name.clone(), declaration.name.clone()),
                    (index, field),
                );
            }

            for (method, body) in classes[index].methods.iter().enumerate() {
                if owner != &class.name
                    && (body.name() == "initialize"
                        || crate::unit::field_initializer(body.name())
                        || visibility.get(&(owner.clone(), body.name().into()))
                            == Some(&crate::Visibility::Private))
                {
                    continue;
                }
                result.insert((class.name.clone(), body.name().into()), (index, method));
            }
        }
    }
    Ok(MemberIndex {
        methods: result,
        fields,
        classes: hierarchy,
    })
}

/// Fill undecorated source overrides from the nearest inherited contract before
/// compiling bodies. Receiver/access shadowing remains a separately rejected
/// lookup profile; private declarations create distinct selector families.
pub(crate) fn prepare_overrides(
    sources: &[&str],
    parsed: &mut [crate::parser::ParsedUnit],
    types: &BTreeMap<String, SourceClass>,
) -> Result<(), crate::SourceDiagnostic> {
    use crate::{Visibility, source_ir};
    let mut order = parsed
        .iter()
        .enumerate()
        .flat_map(|(source, p)| {
            p.classes
                .iter()
                .enumerate()
                .map(move |(class, c)| (types[&c.name].ancestors.len(), source, class))
        })
        .collect::<Vec<_>>();
    order.sort();
    let mut declarations =
        BTreeMap::<(String, String), (source_ir::CallSignature, Visibility, bool)>::new();
    for (_, source, index) in order {
        let class = &mut parsed[source].classes[index];
        for method in &mut class.methods {
            let access = class
                .visibility
                .iter()
                .find(|(name, _)| name == &method.name)
                .map_or(Visibility::Public, |(_, v)| *v);
            if method.name != "initialize" && !crate::unit::field_initializer(&method.name) {
                if let Some((signature, parent_access, is_static)) = types[&class.name]
                    .ancestors
                    .iter()
                    .find_map(|p| declarations.get(&(p.clone(), method.name.clone())))
                {
                    if *parent_access != Visibility::Private {
                        let fail = |message| crate::SourceDiagnostic {
                            source,
                            diagnostic: crate::Diagnostic::at(
                                sources[source],
                                method.body_span.start,
                                message,
                            ),
                        };
                        if *is_static || access != *parent_access || method.is_static != *is_static
                        {
                            return Err(fail(
                                "inherited access/receiver-kind shadowing is not implemented",
                            ));
                        }
                        if method.parameters.len() != signature.parameters.len() {
                            return Err(fail("override arity differs from inherited method"));
                        }
                        for (slot, parent) in
                            method.parameter_types.iter_mut().zip(&signature.parameters)
                        {
                            if let Some(actual) = slot {
                                if !override_parameter(actual, parent) {
                                    return Err(fail(
                                        "override parameter differs from inherited contract",
                                    ));
                                }
                            } else {
                                *slot = Some(parent.clone());
                            }
                        }
                        if let Some(actual) = &method.result_type {
                            if !override_result(actual, &signature.result) {
                                return Err(fail(
                                    "override result differs from inherited contract",
                                ));
                            }
                        } else {
                            method.result_type = Some(signature.result.clone());
                        }
                    }
                }
            }
            declarations.insert(
                (class.name.clone(), method.name.clone()),
                (
                    source_ir::declared_signature(method),
                    access,
                    method.is_static,
                ),
            );
        }
    }
    Ok(())
}

/// Check a decorated override slot; an undecorated ancestor leaves it dynamic.
pub(crate) fn override_parameter(
    actual: &crate::ParameterType,
    parent: &crate::ParameterType,
) -> bool {
    matches!(parent, crate::ParameterType::Any)
        || crate::structural::same_type(&actual.exact_type(), &parent.exact_type())
}

/// Check an override result against the inherited decorated contract.
pub(crate) fn override_result(actual: &crate::ReturnType, parent: &crate::ReturnType) -> bool {
    matches!(parent, crate::ReturnType::Any) || crate::structural::same_result(actual, parent)
}
