//! Core API declarations, separate from VM bindings and executable support.
//!
//! [`core()`] returns a bounded catalogue. Its introduction-level checks do not
//! establish device, permission, application-type, or runtime-scope support.
use std::collections::BTreeMap;
use std::sync::OnceLock;

/// A Connect IQ API level, ordered by major, minor, then patch component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ApiLevel(pub u16, pub u16, pub u16);

/// Semantic types in API signatures; named types use canonical class paths.
/// Null and Void are distinct from classes, and unions retain their alternatives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Named(String),
    Null,
    Void,
    Union(Vec<Type>),
}

impl Type {
    /// Construct a named type without assigning an executable binding.
    pub fn named(path: impl Into<String>) -> Self {
        Self::Named(path.into())
    }
}

/// A runtime-provided method contract; no method body is compiled from it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Method {
    pub name: String,
    pub parameters: Vec<Type>,
    pub result: Type,
    pub since: ApiLevel,
    /// Whether the runtime calls this method as a lifecycle callback.
    pub callback: bool,
}

/// A class declaration with directly declared instance methods.
/// Inherited methods are resolved through the catalogue, not copied here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Class {
    pub name: String,
    pub superclass: Option<String>,
    pub since: ApiLevel,
    pub methods: Vec<Method>,
}

/// An API namespace and its directly declared module functions.
/// Modules do not inherit from Object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Module {
    pub name: String,
    pub since: ApiLevel,
    pub functions: Vec<Method>,
}

/// An inherited method and the minimum API level of its lookup path.
#[derive(Debug, Clone, Copy)]
pub struct ResolvedMethod<'a> {
    pub declaring_class: &'a Class,
    pub method: &'a Method,
    /// Includes the requested class, intervening bases, and the member itself.
    pub required_api: ApiLevel,
}

impl ResolvedMethod<'_> {
    /// Check introduction levels only; other target restrictions remain unknown.
    pub fn available_at(self, level: ApiLevel) -> bool {
        level >= self.required_api
    }
}

/// A bounded API catalogue whose declarations contain no VM identifiers.
/// Entries describe known contracts, not a promise of supported code generation.
#[derive(Debug)]
pub struct Catalogue {
    modules: BTreeMap<String, Module>,
    classes: BTreeMap<String, Class>,
}

impl Catalogue {
    /// Look up a canonical module path; missing entries mean unknown coverage.
    pub fn module(&self, path: &str) -> Option<&Module> {
        self.modules.get(path)
    }

    /// Look up a canonical class path without filtering its API availability.
    pub fn class(&self, path: &str) -> Option<&Class> {
        self.classes.get(path)
    }

    /// Resolve an instance method through the superclass chain, retaining its
    /// declaring class and effective introduction level. Unknown paths return None.
    pub fn method(&self, class: &str, name: &str) -> Option<ResolvedMethod<'_>> {
        let mut current = self.class(class)?;
        let mut required_api = current.since;
        loop {
            required_api = required_api.max(current.since);
            if let Some(method) = current.methods.iter().find(|method| method.name == name) {
                return Some(ResolvedMethod {
                    declaring_class: current,
                    method,
                    required_api: required_api.max(method.since),
                });
            }
            current = self.class(current.superclass.as_deref()?)?;
        }
    }
}

/// Return the shared core Lang/Application/System declaration subset.
/// This catalogue is independent of source syntax and target selection; it does
/// not enable additional source constructs or supply native implementations.
pub fn core() -> &'static Catalogue {
    static CORE: OnceLock<Catalogue> = OnceLock::new();
    CORE.get_or_init(build_core)
}

fn method(name: &str, parameters: Vec<Type>, result: Type, since: ApiLevel) -> Method {
    Method {
        name: name.into(),
        parameters,
        result,
        since,
        callback: false,
    }
}

fn build_core() -> Catalogue {
    let initial = ApiLevel(1, 0, 0);
    let object = "Toybox.Lang.Object";
    let number = "Toybox.Lang.Number";
    let string = "Toybox.Lang.String";
    let nullable_dictionary = Type::Union(vec![Type::named("Toybox.Lang.Dictionary"), Type::Null]);
    let mut classes = BTreeMap::new();
    for name in [
        object,
        number,
        string,
        "Toybox.Lang.Boolean",
        "Toybox.Lang.Char",
        "Toybox.Lang.Dictionary",
        "Toybox.Application.AppBase",
    ] {
        classes.insert(
            name.into(),
            Class {
                name: name.into(),
                superclass: (name != object).then(|| object.into()),
                since: initial,
                methods: Vec::new(),
            },
        );
    }
    classes.get_mut(object).unwrap().methods = vec![
        method("toString", vec![], Type::named(string), initial),
        method(
            "equals",
            vec![Type::Union(vec![Type::named(object), Type::Null])],
            Type::named("Toybox.Lang.Boolean"),
            initial,
        ),
        method("hashCode", vec![], Type::named(number), initial),
    ];
    classes.get_mut(number).unwrap().methods = vec![
        method("abs", vec![], Type::named(number), initial),
        method(
            "format",
            vec![Type::named(string)],
            Type::named(string),
            initial,
        ),
        method(
            "toChar",
            vec![],
            Type::named("Toybox.Lang.Char"),
            ApiLevel(1, 3, 0),
        ),
        method(
            "compareTo",
            vec![Type::named(object)],
            Type::named(number),
            ApiLevel(5, 0, 0),
        ),
    ];
    let mut on_start = method("onStart", vec![nullable_dictionary], Type::Void, initial);
    on_start.callback = true;
    classes
        .get_mut("Toybox.Application.AppBase")
        .unwrap()
        .methods = vec![method("initialize", vec![], Type::Void, initial), on_start];
    let mut modules = BTreeMap::new();
    for name in ["Toybox.Lang", "Toybox.Application", "Toybox.System"] {
        modules.insert(
            name.into(),
            Module {
                name: name.into(),
                since: initial,
                functions: vec![],
            },
        );
    }
    modules
        .get_mut("Toybox.System")
        .unwrap()
        .functions
        .push(method(
            "println",
            vec![Type::Union(vec![Type::named(object), Type::Null])],
            Type::Void,
            initial,
        ));
    Catalogue { modules, classes }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn class_existence_does_not_enable_later_members() {
        let api = core();
        assert_eq!(
            api.class("Toybox.Lang.Number").unwrap().since,
            ApiLevel(1, 0, 0)
        );
        let to_char = api.method("Toybox.Lang.Number", "toChar").unwrap();
        assert!(!to_char.available_at(ApiLevel(1, 2, 9)));
        assert!(to_char.available_at(ApiLevel(1, 3, 0)));
        let compare = api.method("Toybox.Lang.Number", "compareTo").unwrap();
        assert!(!compare.available_at(ApiLevel(4, 9, 9)));
        assert!(compare.available_at(ApiLevel(5, 0, 0)));
        assert!(
            api.method("Toybox.Lang.Number", "abs")
                .unwrap()
                .available_at(ApiLevel(1, 0, 0))
        );
    }

    #[test]
    fn inheritance_retains_owner_and_class_availability() {
        let mut api = build_core();
        api.classes.insert(
            "Example.LaterNumber".into(),
            Class {
                name: "Example.LaterNumber".into(),
                superclass: Some("Toybox.Lang.Number".into()),
                since: ApiLevel(6, 0, 0),
                methods: vec![],
            },
        );
        let inherited = api.method("Example.LaterNumber", "toString").unwrap();
        assert_eq!(inherited.declaring_class.name, "Toybox.Lang.Object");
        assert!(!inherited.available_at(ApiLevel(5, 0, 0)));
        assert!(inherited.available_at(ApiLevel(6, 0, 0)));
        assert!(api.method("Toybox.Lang.Number", "missing").is_none());
        assert!(
            api.class("Toybox.Lang.Object")
                .unwrap()
                .superclass
                .is_none()
        );
        assert!(api.class("Toybox.System").is_none());
    }

    #[test]
    fn core_inheritance_and_signature_references_are_closed() {
        let api = core();
        fn check_type(api: &Catalogue, ty: &Type) {
            match ty {
                Type::Named(name) => assert!(api.class(name).is_some(), "unknown {name}"),
                Type::Union(types) => types.iter().for_each(|ty| check_type(api, ty)),
                Type::Null | Type::Void => {}
            }
        }
        for class in api.classes.values() {
            let mut path = std::collections::BTreeSet::new();
            let mut current = class;
            loop {
                assert!(path.insert(&current.name), "inheritance cycle");
                let Some(base) = &current.superclass else {
                    break;
                };
                current = api.class(base).expect("unknown superclass");
            }
        }
        for method in api
            .classes
            .values()
            .flat_map(|c| &c.methods)
            .chain(api.modules.values().flat_map(|m| &m.functions))
        {
            check_type(api, &method.result);
            for parameter in &method.parameters {
                check_type(api, parameter);
            }
        }
    }
}
