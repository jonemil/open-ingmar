//! Source enum identities and their unit declarations.

/// A nominal enum identity together with the native contract of its members.
/// Source collection may hold an unresolved reference; executable IR requires
/// a resolved backing type, which does not change the VM representation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnumType {
    name: String,
    backing: Option<Box<crate::api::Type>>,
}
impl EnumType {
    /// Construct an enum view with an explicit backing contract.
    pub fn new(name: impl Into<String>, backing: crate::api::Type) -> Self {
        Self::reference(&name.into(), Some(backing))
    }
    /// Return the qualified nominal identity, independent of backing types.
    pub fn name(&self) -> &str {
        &self.name
    }
    /// Return the resolved backing contract, or None during source collection.
    pub fn backing(&self) -> Option<&crate::api::Type> {
        self.backing.as_deref()
    }
    pub(crate) fn reference(name: &str, backing: Option<crate::api::Type>) -> Self {
        Self {
            name: name.into(),
            backing: backing.map(Box::new),
        }
    }
    pub(crate) fn require_backing(&self) -> Result<&crate::api::Type, &'static str> {
        self.backing().ok_or("enum backing type is unresolved")
    }
    pub(crate) fn validate(&self) -> Result<(), &'static str> {
        validate_name(&self.name)?;
        let ty = self.require_backing()?;
        if !crate::structural::type_depth(ty, 1) {
            return Err("enum backing exceeds type depth limit");
        }
        crate::recursive::validate(ty).map_err(|_| "invalid enum backing contract")?;
        crate::unions::result(ty.clone())
            .and_then(crate::FieldKind::value_contract)
            .ok_or("unsupported enum backing type")?;
        Ok(())
    }
    pub(crate) fn parse(text: &str) -> Result<Self, &'static str> {
        if text.matches("enum(").count() > 64 {
            return Err("enum backing exceeds type depth limit");
        }
        let (name, tag) = text.split_once('=').ok_or("expected enum name=backing")?;
        let ty = Self::new(
            name,
            crate::tuples::exact_type(&crate::type_text::call_result(tag)?),
        );
        ty.validate()?;
        Ok(ty)
    }
}
impl std::fmt::Display for EnumType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}=", self.name)?;
        if let Some(ty) = self.backing() {
            let result = crate::unions::result(ty.clone()).ok_or(std::fmt::Error)?;
            write!(f, "{}", crate::type_text::result_tag(&result))
        } else {
            write!(f, "?")
        }
    }
}

/// Project outer enum views for native operations and backing discovery.
/// Container element identities remain invariant and are not erased here.
pub(crate) fn project(ty: &crate::api::Type) -> &crate::api::Type {
    if let crate::api::Type::Enum(identity) = ty {
        identity.backing().map_or(ty, project)
    } else {
        ty
    }
}

pub(crate) fn has_unresolved(ty: &crate::api::Type) -> bool {
    use crate::api::Type;
    match ty {
        Type::Enum(ty) => ty.backing().is_none_or(has_unresolved),
        Type::Recursive(body) | Type::Array(body) => has_unresolved(body),
        Type::Union(parts) | Type::Tuple(parts) => parts.iter().any(has_unresolved),
        Type::TupleValue(tuple) => tuple
            .slots()
            .iter()
            .any(|slot| has_unresolved(&slot.api_type())),
        Type::SourceValue(source) => source.native().iter().any(has_unresolved),
        Type::Options(fields) => fields.values().any(has_unresolved),
        Type::Dictionary { key, value } => has_unresolved(key) || has_unresolved(value),
        Type::Callback(signature) => {
            signature
                .parameters
                .iter()
                .any(|p| has_unresolved(&p.api_type()))
                || has_unresolved(&signature.result.api_type())
        }
        Type::Structural(shape) => shape.api_types().any(|ty| has_unresolved(&ty)),
        _ => false,
    }
}

/// Select when a shared class enum slot receives its initial value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EnumInitialization {
    /// A Number embedded directly in class metadata.
    Number(i32),
    /// A literal/default expression evaluated once by the statics initializer.
    Static,
    /// A computed expression evaluated by each allocation initializer.
    Allocation,
}
impl EnumInitialization {
    pub(crate) fn text(&self) -> String {
        match self {
            Self::Number(n) => n.to_string(),
            Self::Static => "static".into(),
            Self::Allocation => "null".into(),
        }
    }
    pub(crate) fn parse(text: &str) -> Option<Self> {
        Some(match text {
            "static" => Self::Static,
            "null" => Self::Allocation,
            _ => Self::Number(text.parse().ok()?),
        })
    }
}

/// A nominal enum in global, module, or class type scope.
/// Members are separate readonly bindings; this declaration has no VM class.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnumDeclaration {
    /// Enclosing module or class, or None for a global type.
    pub module: Option<String>,
    pub name: String,
    /// Union of the resolved member initializer contracts, without enum erasure.
    pub backing: crate::api::Type,
}

impl EnumDeclaration {
    /// Return the qualified identity used by signatures and enum values.
    pub fn qualified_name(&self) -> String {
        self.module.as_ref().map_or_else(
            || self.name.clone(),
            |module| format!("{module}.{}", self.name),
        )
    }
}

pub(crate) fn validate_name(name: &str) -> Result<(), &'static str> {
    if name.is_empty() {
        return Err("enum identity must not be empty");
    }
    for part in name.split('.') {
        crate::validate_call_names(None, part)?;
    }
    Ok(())
}

/// Check nominal identities throughout nested contracts without projecting them
/// to native backing types. Definitions are unit obligations even for unused types.
pub(crate) fn validate_unit(unit: &crate::CompilationUnit) -> Result<(), crate::UnitError> {
    use crate::{Operation, api::Type};
    use std::collections::BTreeMap;
    let fail = |declaration, operation, message: String| crate::UnitError {
        declaration,
        operation,
        message,
    };
    if unit.enums().len() > 256 {
        return Err(fail(None, None, "unit exceeds 256 enum types".into()));
    }
    let mut names = BTreeMap::new();
    for definition in unit.enums() {
        crate::validate_call_names(definition.module.as_deref(), &definition.name)
            .map_err(|e| fail(None, None, e.into()))?;
        if names
            .insert(definition.qualified_name(), definition.backing.clone())
            .is_some()
        {
            return Err(fail(None, None, "duplicate enum type".into()));
        }
    }
    fn check(
        ty: &Type,
        names: &BTreeMap<String, Type>,
        classes: &[crate::ClassDeclaration],
    ) -> Result<(), String> {
        let visit = |ty: &Type| check(ty, names, classes);
        match ty {
            Type::Enum(ty) => {
                ty.validate().map_err(str::to_owned)?;
                let backing = names.get(ty.name()).ok_or_else(|| {
                    format!("enum type {} requires a unit declaration", ty.name())
                })?;
                if ty.backing() != Some(backing) {
                    return Err("enum backing does not match its unit declaration".into());
                }
                visit(backing)
            }
            Type::Recursive(body) | Type::Array(body) => visit(body),
            Type::Union(parts) | Type::Tuple(parts) => parts.iter().try_for_each(visit),
            Type::TupleValue(tuple) => tuple
                .slots()
                .iter()
                .try_for_each(|slot| visit(&slot.api_type())),
            Type::SourceValue(source) => {
                for target in source.classes() {
                    if !classes
                        .iter()
                        .any(|c| c.name == target.name && c.base == target.base)
                    {
                        return Err("enum backing source class does not match a declaration".into());
                    }
                }
                source.native().iter().try_for_each(visit)
            }
            Type::Options(fields) => fields.values().try_for_each(visit),
            Type::Dictionary { key, value } => {
                visit(key)?;
                visit(value)
            }
            Type::Callback(signature) => {
                for parameter in &signature.parameters {
                    visit(&parameter.api_type())?;
                }
                visit(&signature.result.api_type())
            }
            Type::Structural(shape) => shape.api_types().try_for_each(|ty| visit(&ty)),
            _ => Ok(()),
        }
    }
    for definition in unit.enums() {
        EnumType::new(definition.qualified_name(), definition.backing.clone())
            .validate()
            .map_err(|e| fail(None, None, e.into()))?;
        check(&definition.backing, &names, unit.classes()).map_err(|e| fail(None, None, e))?;
    }
    for constant in unit.shared_constants() {
        check(&constant.value_type.api_type(), &names, unit.classes())
            .map_err(|e| fail(None, None, e))?;
    }
    for class in unit.classes() {
        for field in &class.fields {
            check(&field.result_type().api_type(), &names, unit.classes())
                .map_err(|e| fail(None, None, e))?;
        }
    }
    for (index, body) in unit
        .declarations()
        .iter()
        .map(|d| &d.function)
        .chain(unit.classes().iter().flat_map(|c| &c.methods))
        .enumerate()
    {
        for parameter in &body.parameter_types {
            check(&parameter.api_type(), &names, unit.classes())
                .map_err(|e| fail(Some(index), None, e))?;
        }
        check(&body.return_type.api_type(), &names, unit.classes())
            .map_err(|e| fail(Some(index), None, e))?;
        for ty in body.value_types.values() {
            check(&ty.api_type(), &names, unit.classes())
                .map_err(|e| fail(Some(index), None, e))?;
        }
        for (at, op) in body.operations().iter().enumerate() {
            if let Operation::LocalDeclare { value_type, .. } = &op.operation {
                check(&value_type.api_type(), &names, unit.classes())
                    .map_err(|e| fail(Some(index), Some(at), e))?;
            }
        }
    }
    Ok(())
}
