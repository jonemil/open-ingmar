//! Source enum identities and their unit declarations.

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

/// A nominal Number-compatible enum in global, module, or class type scope.
/// Members are separate readonly bindings; this declaration has no VM class.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnumDeclaration {
    /// Enclosing module or class, or None for a global type.
    pub module: Option<String>,
    pub name: String,
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
/// to Number. Definitions are unit obligations even for otherwise unused types.
pub(crate) fn validate_unit(unit: &crate::CompilationUnit) -> Result<(), crate::UnitError> {
    use crate::{Operation, api::Type};
    use std::collections::BTreeSet;
    let fail = |declaration, operation, message: String| crate::UnitError {
        declaration,
        operation,
        message,
    };
    if unit.enums().len() > 256 {
        return Err(fail(None, None, "unit exceeds 256 enum types".into()));
    }
    let mut names = BTreeSet::new();
    for definition in unit.enums() {
        crate::validate_call_names(definition.module.as_deref(), &definition.name)
            .map_err(|e| fail(None, None, e.into()))?;
        if !names.insert(definition.qualified_name()) {
            return Err(fail(None, None, "duplicate enum type".into()));
        }
    }
    fn check(ty: &Type, names: &BTreeSet<String>) -> Result<(), String> {
        let visit = |ty: &Type| check(ty, names);
        match ty {
            Type::Enum(name) => {
                validate_name(name).map_err(str::to_owned)?;
                names
                    .contains(name)
                    .then_some(())
                    .ok_or_else(|| format!("enum type {name} requires a unit declaration"))
            }
            Type::Recursive(body) | Type::Array(body) => visit(body),
            Type::Union(parts) | Type::Tuple(parts) => parts.iter().try_for_each(visit),
            Type::TupleValue(tuple) => tuple
                .slots()
                .iter()
                .try_for_each(|slot| visit(&slot.api_type())),
            Type::SourceValue(source) => source.native().iter().try_for_each(visit),
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
    for constant in unit.shared_constants() {
        check(&constant.value_type.api_type(), &names).map_err(|e| fail(None, None, e))?;
    }
    for class in unit.classes() {
        for field in &class.fields {
            check(&field.result_type().api_type(), &names).map_err(|e| fail(None, None, e))?;
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
            check(&parameter.api_type(), &names).map_err(|e| fail(Some(index), None, e))?;
        }
        check(&body.return_type.api_type(), &names).map_err(|e| fail(Some(index), None, e))?;
        for ty in body.value_types.values() {
            check(&ty.api_type(), &names).map_err(|e| fail(Some(index), None, e))?;
        }
        for (at, op) in body.operations().iter().enumerate() {
            if let Operation::LocalDeclare { value_type, .. } = &op.operation {
                check(&value_type.api_type(), &names)
                    .map_err(|e| fail(Some(index), Some(at), e))?;
            }
        }
    }
    Ok(())
}
