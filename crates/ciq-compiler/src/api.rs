//! Core API declarations, separate from VM bindings and executable support.
//!
//! [`core()`] returns a bounded catalogue. Its introduction-level checks do not
//! establish device, permission, application-type, or runtime-scope support.
use std::collections::BTreeMap;
use std::sync::OnceLock;

/// A Connect IQ API level, ordered by major, minor, then patch component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ApiLevel(pub u16, pub u16, pub u16);

impl std::fmt::Display for ApiLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.0, self.1, self.2)
    }
}

impl std::str::FromStr for ApiLevel {
    type Err = &'static str;

    /// Parse exactly three decimal u16 components, such as `1.3.0`.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let components = text.split('.').collect::<Vec<_>>();
        if components.len() != 3
            || components
                .iter()
                .any(|c| c.is_empty() || !c.bytes().all(|b| b.is_ascii_digit()))
        {
            return Err("API level requires major.minor.patch decimal components");
        }
        let parse = |c: &str| {
            c.parse::<u16>()
                .map_err(|_| "API level component exceeds u16")
        };
        Ok(Self(
            parse(components[0])?,
            parse(components[1])?,
            parse(components[2])?,
        ))
    }
}

/// Explicit API-level inputs for unguarded declaration checking.
/// This does not establish device, permission, app-type, or VM compatibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApiContext {
    target: ApiLevel,
    minimum: ApiLevel,
}

impl ApiContext {
    /// Construct a context with a deployment minimum no higher than the target.
    /// Both levels must be at least the initial Connect IQ API, `1.0.0`.
    pub fn new(target: ApiLevel, minimum: ApiLevel) -> Result<Self, &'static str> {
        if minimum < ApiLevel(1, 0, 0) || target < ApiLevel(1, 0, 0) {
            return Err("target and minimum API must be at least 1.0.0");
        }
        if minimum > target {
            return Err("application minimum API exceeds target API");
        }
        Ok(Self { target, minimum })
    }

    /// Return the caller-supplied target capability level, not the SDK version.
    pub fn target(self) -> ApiLevel {
        self.target
    }

    /// Return the application's caller-supplied deployment requirement.
    pub fn minimum(self) -> ApiLevel {
        self.minimum
    }

    pub(crate) fn require(self, name: &str, since: ApiLevel) -> Result<(), String> {
        if since > self.target {
            return Err(format!(
                "{name} requires API {since}; unavailable on target API {}",
                self.target
            ));
        }
        if since > self.minimum {
            return Err(format!(
                "unguarded {name} requires API {since}; application minimum API is {}",
                self.minimum
            ));
        }
        Ok(())
    }

    /// Check a known class and its enclosing module introduction levels.
    pub fn check_class(self, catalogue: &Catalogue, path: &str) -> Result<(), String> {
        let class = catalogue
            .class(path)
            .ok_or_else(|| format!("class {path} is not known in the API catalogue"))?;
        self.require(path, catalogue.class_level(class))
    }

    /// Check class introduction levels in a value or signature type, including
    /// container parameters. Null/Void have no class level; unions require all alternatives.
    pub fn check_type(self, catalogue: &Catalogue, ty: &Type) -> Result<(), String> {
        match ty {
            Type::Array(element) => {
                self.check_class(catalogue, "Toybox.Lang.Array")?;
                self.check_type(catalogue, element)
            }
            Type::Dictionary { key, value } => {
                self.check_class(catalogue, "Toybox.Lang.Dictionary")?;
                self.check_type(catalogue, key)?;
                self.check_type(catalogue, value)
            }
            Type::Tuple(elements) => {
                self.check_class(catalogue, "Toybox.Lang.Array")?;
                elements
                    .iter()
                    .try_for_each(|ty| self.check_type(catalogue, ty))
            }
            Type::Named(name) if catalogue.enums.contains_key(name) => {
                self.require(name, catalogue.enum_level(name).unwrap())
            }
            Type::Named(name) => self.check_class(catalogue, name),
            Type::Parameter(name) => Err(format!(
                "generic API parameter {name} requires a concrete receiver type"
            )),
            Type::Union(types) => types
                .iter()
                .try_for_each(|ty| self.check_type(catalogue, ty)),
            Type::Null | Type::Void => Ok(()),
        }
    }

    /// Resolve an inherited method and check its unguarded introduction level.
    /// Unknown declarations and unavailable declarations produce different errors.
    pub fn method<'a>(
        self,
        catalogue: &'a Catalogue,
        class: &str,
        member: &str,
    ) -> Result<ResolvedMethod<'a>, String> {
        self.check_class(catalogue, class)?;
        let method = catalogue.method(class, member).ok_or_else(|| {
            format!("native member {class}.{member} is not known in the API catalogue")
        })?;
        self.require(&format!("{class}.{member}"), method.required_api)?;
        Ok(method)
    }

    /// Resolve a module function and check the module and member levels.
    pub fn function<'a>(
        self,
        catalogue: &'a Catalogue,
        module: &str,
        member: &str,
    ) -> Result<&'a Method, String> {
        let declaration = catalogue
            .module(module)
            .ok_or_else(|| format!("module {module} is not known in the API catalogue"))?;
        let method = declaration
            .functions
            .iter()
            .find(|m| m.name == member)
            .ok_or_else(|| {
                format!("native function {module}.{member} is not known in the API catalogue")
            })?;
        self.require(
            &format!("{module}.{member}"),
            declaration.since.max(method.since),
        )?;
        Ok(method)
    }
}

/// Semantic types in API signatures; named types use canonical class paths.
/// Null and Void are distinct from classes, and unions retain their alternatives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    /// Mutable Array with an explicit element contract. Array element contracts
    /// are invariant; this does not imply executable support for every element.
    Array(Box<Type>),
    /// Mutable Dictionary with invariant key and value contracts. This describes
    /// API types without enabling executable Dictionary lookup or mutation.
    Dictionary {
        key: Box<Type>,
        value: Box<Type>,
    },
    /// Positional Array shape with an exact number of typed slots.
    Tuple(Vec<Type>),
    Named(String),
    /// A generic class parameter substituted from a concrete receiver type.
    Parameter(String),
    Null,
    Void,
    Union(Vec<Type>),
}

impl Type {
    /// Construct a named type without assigning an executable binding.
    pub fn named(path: impl Into<String>) -> Self {
        Self::Named(path.into())
    }

    /// Construct a Dictionary signature type with explicit key/value contracts.
    /// Class availability and executable support must be checked separately.
    pub fn dictionary(key: Type, value: Type) -> Self {
        Self::Dictionary {
            key: Box::new(key),
            value: Box::new(value),
        }
    }
}

fn substitute_parameters(ty: &Type, bindings: &[(&str, &Type)]) -> Option<Type> {
    Some(match ty {
        Type::Parameter(name) => {
            let (_, argument) = bindings.iter().find(|(parameter, _)| *parameter == name)?;
            substitute_parameters(argument, &[])?
        }
        Type::Array(element) => Type::Array(Box::new(substitute_parameters(element, bindings)?)),
        Type::Dictionary { key, value } => Type::dictionary(
            substitute_parameters(key, bindings)?,
            substitute_parameters(value, bindings)?,
        ),
        Type::Tuple(elements) => Type::Tuple(
            elements
                .iter()
                .map(|ty| substitute_parameters(ty, bindings))
                .collect::<Option<_>>()?,
        ),
        Type::Union(types) => {
            fn append(types: &mut Vec<Type>, ty: Type) {
                match ty {
                    Type::Union(alternatives) => {
                        for alternative in alternatives {
                            append(types, alternative);
                        }
                    }
                    ty if !types.contains(&ty) => types.push(ty),
                    _ => {}
                }
            }
            let mut alternatives = Vec::new();
            for ty in types {
                append(&mut alternatives, substitute_parameters(ty, bindings)?);
            }
            if alternatives.len() == 1 {
                alternatives.pop()?
            } else {
                Type::Union(alternatives)
            }
        }
        other => other.clone(),
    })
}

/// A runtime-provided method contract; no method body is compiled from it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Method {
    pub name: String,
    pub parameters: Vec<Type>,
    /// Generic declared result, before exact receiver specialization.
    pub result: Type,
    /// Exact canonical receiver paths with specialized results. Other receivers
    /// retain `result`; superclass lookup does not propagate these overrides.
    pub receiver_results: BTreeMap<String, Type>,
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
    /// Effective result for the requested receiver, after exact specialization.
    pub result: &'a Type,
    /// Includes the requested class, bases, enclosing modules, and the member.
    pub required_api: ApiLevel,
}

/// A method declaration with container parameters replaced by receiver types.
/// Introduction levels and executable support remain separate checks.
#[derive(Debug)]
pub struct InstantiatedMethod<'a> {
    pub declaration: ResolvedMethod<'a>,
    pub parameters: Vec<Type>,
    pub result: Type,
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
    enums: BTreeMap<String, ApiLevel>,
    constants: BTreeMap<(String, String), Constant>,
}

/// A published native constant, retaining its semantic type and API level.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Constant {
    pub ty: Type,
    pub value: i32,
    pub since: ApiLevel,
}

impl Catalogue {
    /// Resolve a method and substitute a typed receiver's container parameters
    /// throughout its argument/result types. Nullable substitutions are flattened
    /// and deduplicated. Return None for unknown receivers/members or unresolved
    /// parameters, including key/value methods on an opaque named Dictionary.
    pub fn method_for_type(&self, receiver: &Type, member: &str) -> Option<InstantiatedMethod<'_>> {
        let bindings;
        let (class, substitutions): (&str, &[(&str, &Type)]) = match receiver {
            Type::Named(name) => (name, &[]),
            Type::Array(element) => {
                bindings = vec![("Element", element.as_ref())];
                ("Toybox.Lang.Array", &bindings)
            }
            Type::Dictionary { key, value } => {
                bindings = vec![("Key", key.as_ref()), ("Value", value.as_ref())];
                ("Toybox.Lang.Dictionary", &bindings)
            }
            _ => return None,
        };
        let declaration = self.method(class, member)?;
        let parameters = declaration
            .method
            .parameters
            .iter()
            .map(|ty| substitute_parameters(ty, substitutions))
            .collect::<Option<Vec<_>>>()?;
        let result = substitute_parameters(declaration.result, substitutions)?;
        Some(InstantiatedMethod {
            declaration,
            parameters,
            result,
        })
    }

    /// Resolve a recorded native constant in its canonical module.
    pub fn constant(&self, module: &str, member: &str) -> Option<&Constant> {
        self.constants.get(&(module.into(), member.into()))
    }
    /// Resolve a bounded enum type without treating it as an object class.
    pub fn enum_level(&self, name: &str) -> Option<ApiLevel> {
        let level = *self.enums.get(name)?;
        let module = name.rsplit_once('.')?.0;
        Some(level.max(self.modules.get(module)?.since))
    }
    fn class_level(&self, class: &Class) -> ApiLevel {
        let mut level = class.since;
        let mut path = class.name.as_str();
        while let Some((parent, _)) = path.rsplit_once('.') {
            if let Some(module) = self.module(parent) {
                level = level.max(module.since);
            }
            path = parent;
        }
        level
    }

    /// Look up a canonical module path; missing entries mean unknown coverage.
    pub fn module(&self, path: &str) -> Option<&Module> {
        self.modules.get(path)
    }

    /// Look up a canonical class path without filtering its API availability.
    pub fn class(&self, path: &str) -> Option<&Class> {
        self.classes.get(path)
    }

    /// Resolve an instance method through the superclass chain, retaining its
    /// declaring class, effective result, and introduction level. Result overrides
    /// match the requested class exactly. Unknown paths return None.
    pub fn method(&self, class: &str, name: &str) -> Option<ResolvedMethod<'_>> {
        let mut current = self.class(class)?;
        let mut required_api = self.class_level(current);
        loop {
            required_api = required_api.max(self.class_level(current));
            if let Some(method) = current.methods.iter().find(|method| method.name == name) {
                return Some(ResolvedMethod {
                    declaring_class: current,
                    method,
                    result: method.receiver_results.get(class).unwrap_or(&method.result),
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
        receiver_results: BTreeMap::new(),
        since,
        callback: false,
    }
}

fn build_core() -> Catalogue {
    let initial = ApiLevel(1, 0, 0);
    let object = "Toybox.Lang.Object";
    let number = "Toybox.Lang.Number";
    let long = "Toybox.Lang.Long";
    let string = "Toybox.Lang.String";
    let nullable_dictionary = Type::Union(vec![Type::named("Toybox.Lang.Dictionary"), Type::Null]);
    let mut classes = BTreeMap::new();
    for name in [
        object,
        number,
        "Toybox.Lang.Long",
        "Toybox.Lang.Float",
        "Toybox.Lang.Double",
        string,
        "Toybox.Lang.Boolean",
        "Toybox.Lang.Char",
        "Toybox.Lang.Array",
        "Toybox.Lang.Dictionary",
        "Toybox.Lang.WeakReference",
        "Toybox.Application.AppBase",
        "Toybox.WatchUi.View",
        "Toybox.WatchUi.InputDelegate",
        "Toybox.WatchUi.BehaviorDelegate",
        "Toybox.Graphics.Dc",
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
    classes.get_mut("Toybox.Lang.Dictionary").unwrap().methods = vec![
        method(
            "get",
            vec![Type::Parameter("Key".into())],
            Type::Union(vec![Type::Parameter("Value".into()), Type::Null]),
            initial,
        ),
        method(
            "put",
            vec![
                Type::Parameter("Key".into()),
                Type::Parameter("Value".into()),
            ],
            Type::Void,
            initial,
        ),
        method(
            "hasKey",
            vec![Type::Parameter("Key".into())],
            Type::named("Toybox.Lang.Boolean"),
            initial,
        ),
        method(
            "remove",
            vec![Type::Parameter("Key".into())],
            Type::Void,
            initial,
        ),
        method(
            "keys",
            vec![],
            Type::Array(Box::new(Type::Parameter("Key".into()))),
            initial,
        ),
        method(
            "values",
            vec![],
            Type::Array(Box::new(Type::Parameter("Value".into()))),
            initial,
        ),
        method("toString", vec![], Type::named(string), ApiLevel(1, 0, 1)),
        method("size", vec![], Type::named(number), initial),
        method(
            "isEmpty",
            vec![],
            Type::named("Toybox.Lang.Boolean"),
            initial,
        ),
    ];
    classes.get_mut(object).unwrap().methods = vec![
        method("toString", vec![], Type::named(string), initial),
        method(
            "equals",
            vec![Type::Union(vec![Type::named(object), Type::Null])],
            Type::named("Toybox.Lang.Boolean"),
            initial,
        ),
        method("hashCode", vec![], Type::named(number), initial),
        method(
            "weak",
            vec![],
            Type::named("Toybox.Lang.WeakReference"),
            ApiLevel(1, 2, 0),
        ),
    ];
    classes.get_mut("Toybox.Lang.WeakReference").unwrap().since = ApiLevel(1, 2, 0);
    let weak = classes.get_mut(object).unwrap().methods.last_mut().unwrap();
    weak.receiver_results = [
        number,
        long,
        "Toybox.Lang.Float",
        "Toybox.Lang.Double",
        string,
        "Toybox.Lang.Boolean",
    ]
    .into_iter()
    .map(|name| (name.into(), Type::named(name)))
    .collect();
    let char_type = "Toybox.Lang.Char";
    let char_initial = ApiLevel(1, 3, 0);
    let char_class = classes.get_mut(char_type).unwrap();
    char_class.since = char_initial;
    char_class.methods = vec![
        method("toLower", vec![], Type::named(char_type), char_initial),
        method("toUpper", vec![], Type::named(char_type), char_initial),
        method("toNumber", vec![], Type::named(number), char_initial),
        method("toString", vec![], Type::named(string), char_initial),
        method(
            "compareTo",
            vec![Type::named(object)],
            Type::named(number),
            ApiLevel(5, 0, 0),
        ),
    ];
    for name in ["Toybox.Lang.Float", "Toybox.Lang.Double"] {
        let mut methods = vec![
            method("abs", vec![], Type::named(name), initial),
            method(
                "format",
                vec![Type::named(string)],
                Type::named(string),
                initial,
            ),
            method("toNumber", vec![], Type::named(number), initial),
            method("toLong", vec![], Type::named(long), initial),
            method(
                "compareTo",
                vec![Type::named(object)],
                Type::named(number),
                ApiLevel(5, 0, 0),
            ),
        ];
        if name == "Toybox.Lang.Double" {
            methods.push(method("toString", vec![], Type::named(string), initial));
            methods.push(method(
                "equals",
                vec![Type::Union(vec![Type::named(object), Type::Null])],
                Type::named("Toybox.Lang.Boolean"),
                ApiLevel(1, 3, 0),
            ));
        }
        classes.get_mut(name).unwrap().methods = methods;
    }
    classes.get_mut(long).unwrap().methods = vec![
        method("abs", vec![], Type::named(long), initial),
        method("toNumber", vec![], Type::named(number), initial),
        method("toLong", vec![], Type::named(long), initial),
        method("toString", vec![], Type::named(string), initial),
        method(
            "format",
            vec![Type::named(string)],
            Type::named(string),
            initial,
        ),
        method(
            "equals",
            vec![Type::Union(vec![Type::named(object), Type::Null])],
            Type::named("Toybox.Lang.Boolean"),
            ApiLevel(1, 3, 0),
        ),
        method(
            "compareTo",
            vec![Type::named(object)],
            Type::named(number),
            ApiLevel(5, 0, 0),
        ),
    ];
    classes.get_mut(number).unwrap().methods = vec![
        method("toLong", vec![], Type::named(long), initial),
        method("abs", vec![], Type::named(number), initial),
        method("toNumber", vec![], Type::named(number), initial),
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
    classes.get_mut("Toybox.Lang.Boolean").unwrap().methods = vec![method(
        "compareTo",
        vec![Type::named(object)],
        Type::named(number),
        ApiLevel(5, 0, 0),
    )];
    classes.get_mut("Toybox.Lang.Array").unwrap().methods = vec![
        method("size", vec![], Type::named(number), initial),
        method("toString", vec![], Type::named(string), initial),
        method(
            "add",
            vec![Type::Parameter("Element".into())],
            Type::Array(Box::new(Type::Parameter("Element".into()))),
            char_initial,
        ),
        method(
            "addAll",
            vec![Type::Array(Box::new(Type::Parameter("Element".into())))],
            Type::Array(Box::new(Type::Parameter("Element".into()))),
            char_initial,
        ),
        method(
            "remove",
            vec![Type::Parameter("Element".into())],
            Type::named("Toybox.Lang.Boolean"),
            char_initial,
        ),
        method(
            "removeAll",
            vec![Type::Parameter("Element".into())],
            Type::named("Toybox.Lang.Boolean"),
            char_initial,
        ),
        method(
            "indexOf",
            vec![Type::Parameter("Element".into())],
            Type::named(number),
            char_initial,
        ),
        method(
            "reverse",
            vec![],
            Type::Array(Box::new(Type::Parameter("Element".into()))),
            char_initial,
        ),
        method(
            "slice",
            vec![
                Type::Union(vec![Type::named(number), Type::Null]),
                Type::Union(vec![Type::named(number), Type::Null]),
            ],
            Type::Array(Box::new(Type::Parameter("Element".into()))),
            char_initial,
        ),
    ];
    classes.get_mut(string).unwrap().methods = vec![
        method(
            "toLong",
            vec![],
            Type::Union(vec![Type::named(long), Type::Null]),
            ApiLevel(3, 1, 0),
        ),
        method(
            "toLongWithBase",
            vec![Type::named(number)],
            Type::Union(vec![Type::named(long), Type::Null]),
            ApiLevel(3, 1, 0),
        ),
        method(
            "toUtf8Array",
            vec![],
            Type::Array(Box::new(Type::named(number))),
            char_initial,
        ),
        method(
            "toCharArray",
            vec![],
            Type::Array(Box::new(Type::named(char_type))),
            char_initial,
        ),
        method(
            "compareTo",
            vec![Type::named(object)],
            Type::named(number),
            ApiLevel(5, 0, 0),
        ),
        method(
            "equals",
            vec![Type::Union(vec![Type::named(object), Type::Null])],
            Type::named("Toybox.Lang.Boolean"),
            initial,
        ),
        method("hashCode", vec![], Type::named(number), initial),
        method(
            "toNumberWithBase",
            vec![Type::named(number)],
            Type::Union(vec![Type::named(number), Type::Null]),
            ApiLevel(1, 4, 1),
        ),
        method(
            "substring",
            vec![Type::Union(vec![Type::named(number), Type::Null]); 2],
            Type::Union(vec![Type::named(string), Type::Null]),
            initial,
        ),
        method(
            "find",
            vec![Type::named(string)],
            Type::Union(vec![Type::named(number), Type::Null]),
            initial,
        ),
        method(
            "toNumber",
            vec![],
            Type::Union(vec![Type::named(number), Type::Null]),
            initial,
        ),
        method("length", vec![], Type::named(number), initial),
        method("toLower", vec![], Type::named(string), initial),
        method("toUpper", vec![], Type::named(string), initial),
        method("toString", vec![], Type::named(string), initial),
    ];
    let mut on_start = method(
        "onStart",
        vec![nullable_dictionary.clone()],
        Type::Void,
        initial,
    );
    on_start.callback = true;
    let mut on_stop = method("onStop", vec![nullable_dictionary], Type::Void, initial);
    on_stop.callback = true;
    let mut initial_view = method(
        "getInitialView",
        vec![],
        Type::Union(vec![
            Type::Tuple(vec![Type::named("Toybox.WatchUi.View")]),
            Type::Tuple(vec![
                Type::named("Toybox.WatchUi.View"),
                Type::named("Toybox.WatchUi.InputDelegate"),
            ]),
        ]),
        initial,
    );
    initial_view.callback = true;
    classes
        .get_mut("Toybox.Application.AppBase")
        .unwrap()
        .methods = vec![
        method("initialize", vec![], Type::Void, initial),
        on_start,
        on_stop,
        initial_view,
    ];
    let mut view_methods = vec![method("initialize", vec![], Type::Void, ApiLevel(2, 1, 0))];
    for (name, parameters) in [
        ("onShow", vec![]),
        ("onHide", vec![]),
        ("onUpdate", vec![Type::named("Toybox.Graphics.Dc")]),
    ] {
        let mut declaration = method(name, parameters, Type::Void, initial);
        declaration.callback = true;
        view_methods.push(declaration);
    }
    classes.get_mut("Toybox.WatchUi.View").unwrap().methods = view_methods;
    let behavior = classes.get_mut("Toybox.WatchUi.BehaviorDelegate").unwrap();
    behavior.superclass = Some("Toybox.WatchUi.InputDelegate".into());
    let mut on_select = method(
        "onSelect",
        vec![],
        Type::named("Toybox.Lang.Boolean"),
        ApiLevel(1, 2, 0),
    );
    on_select.callback = true;
    behavior.methods = vec![method("initialize", vec![], Type::Void, initial), on_select];

    classes.get_mut("Toybox.Graphics.Dc").unwrap().methods = vec![
        method("clear", vec![], Type::Void, initial),
        method(
            "setColor",
            vec![Type::named(number); 2],
            Type::Void,
            initial,
        ),
        method(
            "drawText",
            vec![
                Type::named(number),
                Type::named(number),
                Type::named("Toybox.Graphics.FontDefinition"),
                Type::named(string),
                Type::named(number),
            ],
            Type::Void,
            initial,
        ),
    ];
    for name in [number, long, "Toybox.Lang.Float", "Toybox.Lang.Double"] {
        let methods = &mut classes.get_mut(name).unwrap().methods;
        methods.push(method(
            "toFloat",
            vec![],
            Type::named("Toybox.Lang.Float"),
            initial,
        ));
        methods.push(method(
            "toDouble",
            vec![],
            Type::named("Toybox.Lang.Double"),
            initial,
        ));
    }
    for (name, result, since) in [
        ("toFloat", "Toybox.Lang.Float", initial),
        ("toDouble", "Toybox.Lang.Double", ApiLevel(3, 1, 0)),
    ] {
        classes.get_mut(string).unwrap().methods.push(method(
            name,
            vec![],
            Type::Union(vec![Type::named(result), Type::Null]),
            since,
        ));
    }
    let mut modules = BTreeMap::new();
    for name in [
        "Toybox.Lang",
        "Toybox.Application",
        "Toybox.System",
        "Toybox.WatchUi",
        "Toybox.Graphics",
    ] {
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
    modules
        .get_mut("Toybox.System")
        .unwrap()
        .functions
        .push(method("exit", vec![], Type::Void, initial));
    modules
        .get_mut("Toybox.WatchUi")
        .unwrap()
        .functions
        .push(method("requestUpdate", vec![], Type::Void, initial));
    Catalogue {
        modules,
        classes,
        enums: BTreeMap::from([("Toybox.Graphics.FontDefinition".into(), initial)]),
        constants: BTreeMap::from([(
            ("Toybox.Graphics".into(), "FONT_MEDIUM".into()),
            Constant {
                ty: Type::named("Toybox.Graphics.FontDefinition"),
                value: 3,
                since: initial,
            },
        )]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substitution_recurses_through_container_and_tuple_signatures() {
        let key = Type::named("Toybox.Lang.String");
        let value = Type::Union(vec![Type::named("Toybox.Lang.Number"), Type::Null]);
        let template = Type::Tuple(vec![Type::dictionary(
            Type::Parameter("Key".into()),
            Type::Array(Box::new(Type::Union(vec![
                Type::Parameter("Value".into()),
                Type::Null,
            ]))),
        )]);
        let actual = substitute_parameters(&template, &[("Key", &key), ("Value", &value)]);
        assert_eq!(
            actual,
            Some(Type::Tuple(vec![Type::dictionary(
                key.clone(),
                Type::Array(Box::new(value)),
            )]))
        );
        assert!(substitute_parameters(&template, &[("Key", &key)]).is_none());
        let unbound = Type::Array(Box::new(Type::Parameter("Other".into())));
        assert!(substitute_parameters(&template, &[("Key", &key), ("Value", &unbound)]).is_none());
    }

    #[test]
    fn enclosing_module_and_inherited_base_levels_are_required() {
        let mut catalogue = build_core();
        catalogue.modules.get_mut("Toybox.Lang").unwrap().since = ApiLevel(2, 0, 0);
        let old = ApiContext::new(ApiLevel(1, 0, 0), ApiLevel(1, 0, 0)).unwrap();
        assert!(
            old.check_class(&catalogue, "Toybox.Lang.Number")
                .unwrap_err()
                .contains("requires API 2.0.0")
        );
        assert_eq!(
            catalogue
                .method("Toybox.Application.AppBase", "toString")
                .unwrap()
                .required_api,
            ApiLevel(2, 0, 0)
        );
        catalogue
            .classes
            .get_mut("Toybox.Lang.Object")
            .unwrap()
            .since = ApiLevel(3, 0, 0);
        assert_eq!(
            catalogue
                .method("Toybox.Lang.Number", "toString")
                .unwrap()
                .required_api,
            ApiLevel(3, 0, 0)
        );
    }

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
    fn receiver_result_overrides_do_not_leak_to_subclasses() {
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
        let resolved = api.method("Example.LaterNumber", "weak").unwrap();
        assert_eq!(*resolved.result, Type::named("Toybox.Lang.WeakReference"));
        assert_eq!(resolved.required_api, ApiLevel(6, 0, 0));
    }

    #[test]
    fn core_inheritance_and_signature_references_are_closed() {
        let api = core();
        fn check_type(api: &Catalogue, ty: &Type) {
            match ty {
                Type::Array(element) => {
                    assert!(api.class("Toybox.Lang.Array").is_some());
                    check_type(api, element);
                }
                Type::Dictionary { key, value } => {
                    assert!(api.class("Toybox.Lang.Dictionary").is_some());
                    check_type(api, key);
                    check_type(api, value);
                }
                Type::Named(name) => assert!(
                    api.class(name).is_some() || api.enum_level(name).is_some(),
                    "unknown {name}"
                ),
                Type::Parameter(_) => {}
                Type::Union(types) | Type::Tuple(types) => {
                    types.iter().for_each(|ty| check_type(api, ty))
                }
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
            for (receiver, result) in &method.receiver_results {
                assert!(api.class(receiver).is_some(), "unknown receiver {receiver}");
                check_type(api, result);
            }
            for parameter in &method.parameters {
                check_type(api, parameter);
            }
        }
    }
}
