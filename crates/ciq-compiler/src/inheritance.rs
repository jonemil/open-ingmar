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

/// Storage and access needed to derive lookup without depending on field types.
pub(crate) struct FieldSpec {
    pub(crate) name: String,
    pub(crate) shared: bool,
    pub(crate) visibility: crate::Visibility,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct FieldViews {
    pub(crate) lexical: BTreeMap<(String, String), String>,
    pub(crate) instance: BTreeMap<(String, String), String>,
}

/// Resolve lexical and instance field owners from the same declarations.
/// Instance lookup prefers visible instance storage throughout the hierarchy,
/// falling back to the closest shared declaration only when no instance exists.
pub(crate) fn field_views(
    hierarchy: &BTreeMap<String, SourceClass>,
    declarations: &BTreeMap<String, Vec<FieldSpec>>,
) -> FieldViews {
    let mut result = FieldViews::default();
    for (class, identity) in hierarchy {
        for owner in identity
            .ancestors
            .iter()
            .rev()
            .chain(std::iter::once(class))
        {
            for field in &declarations[owner] {
                if owner != class && field.visibility == crate::Visibility::Private {
                    continue;
                }
                let key = (class.clone(), field.name.clone());
                result.lexical.insert(key.clone(), owner.clone());
                if !field.shared {
                    result.instance.insert(key, owner.clone());
                }
            }
        }
    }
    for (key, owner) in &result.lexical {
        result
            .instance
            .entry(key.clone())
            .or_insert_with(|| owner.clone());
    }
    result
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MemberIndex {
    pub(crate) methods: BTreeMap<(String, String), (usize, usize)>,
    pub(crate) method_access: BTreeMap<(String, String), MethodSpec>,
    pub(crate) fields: BTreeMap<(String, String), (usize, usize)>,
    pub(crate) instance_fields: BTreeMap<(String, String), (usize, usize)>,
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
    let declarations = classes
        .iter()
        .map(|c| {
            (
                c.name.clone(),
                c.fields
                    .iter()
                    .map(|f| FieldSpec {
                        name: f.name.clone(),
                        shared: f.kind.class_shared(),
                        visibility: visibility
                            .get(&(c.name.clone(), f.name.clone()))
                            .copied()
                            .unwrap_or_default(),
                    })
                    .collect(),
            )
        })
        .collect();
    let views = field_views(&hierarchy, &declarations);
    let bind = |owners: BTreeMap<(String, String), String>| {
        owners
            .into_iter()
            .map(|(key, owner)| {
                let class = indexes[owner.as_str()];
                let field = classes[class]
                    .fields
                    .iter()
                    .position(|f| f.name == key.1)
                    .unwrap();
                (key, (class, field))
            })
            .collect()
    };
    let fields = bind(views.lexical);
    let instance_fields = bind(views.instance);
    for class in classes {
        for owner in hierarchy[&class.name]
            .ancestors
            .iter()
            .rev()
            .chain(std::iter::once(&class.name))
        {
            let index = indexes[owner.as_str()];
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
        method_access: classes
            .iter()
            .flat_map(|c| {
                c.methods.iter().map(|m| {
                    let key = (c.name.clone(), m.name().to_owned());
                    let access = visibility.get(&key).copied().unwrap_or_default();
                    (
                        key,
                        MethodSpec {
                            access,
                            is_static: m.is_static_method(),
                        },
                    )
                })
            })
            .collect(),
        fields,
        instance_fields,
        classes: hierarchy,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MethodSpec {
    pub(crate) access: crate::Visibility,
    pub(crate) is_static: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MethodView {
    /// Closest accessible declaration, for static bodies and selector binding.
    Lexical,
    /// Prefer instance declarations throughout the visible hierarchy.
    Instance,
}

/// Select a method contract under the caller's access and receiver view.
/// Instance views prefer instance declarations across the visible hierarchy;
/// lexical views select the closest accessible declaration. Private ancestors stay local.
/// If no candidate is accessible, retain the closest one for access diagnostics.
pub(crate) fn visible_method<'a>(
    declarations: &'a BTreeMap<(String, String), MethodSpec>,
    class: &str,
    member: &str,
    caller: Option<&str>,
    ancestors: &[String],
    caller_ancestors: &[String],
    view: MethodView,
) -> Option<&'a str> {
    use crate::Visibility;
    let mut fallback = None;
    let mut shared = None;
    for owner in std::iter::once(class).chain(ancestors.iter().map(String::as_str)) {
        if owner != class && (member == "initialize" || crate::unit::field_initializer(member)) {
            continue;
        }
        let Some(((owner, _), spec)) = declarations.get_key_value(&(owner.into(), member.into()))
        else {
            continue;
        };
        let access = spec.access;
        if owner != class && access == Visibility::Private {
            continue;
        }
        fallback.get_or_insert(owner.as_str());
        if access == Visibility::Public
            || caller == Some(owner.as_str())
            || (access == Visibility::Protected && caller_ancestors.contains(owner))
        {
            if view == MethodView::Lexical || !spec.is_static {
                return Some(owner);
            }
            shared.get_or_insert(owner.as_str());
        }
    }
    shared.or(fallback)
}

/// Resolve omitted override annotations before compiling source bodies.
/// The closest nonprivate ancestor constrains a nonprivate declaration only
/// when that ancestor is an instance method. Static ancestors and private
/// children start independent signature families.
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
                if let Some((signature, _, is_static)) = types[&class.name]
                    .ancestors
                    .iter()
                    .filter_map(|p| declarations.get(&(p.clone(), method.name.clone())))
                    .find(|(_, access, _)| *access != Visibility::Private)
                {
                    if !*is_static && access != Visibility::Private {
                        let fail = |message| crate::SourceDiagnostic {
                            source,
                            diagnostic: crate::Diagnostic::at(
                                sources[source],
                                method.body_span.start,
                                message,
                            ),
                        };
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
