//! Source checking policy, separate from mandatory IR validation and lowering.
use crate::{
    CompilationUnit, Diagnostic, Function, Operation, SourceDiagnostic, ValueId,
    api::{ApiContext, Type},
};
use std::{collections::BTreeMap, fmt};

/// Ambiguity reporting for the currently supported checked source profile.
/// Definite errors and IR invariants remain errors at every level. Disabled
/// checking remains unsupported. Operations
/// retain their independently validated executable profiles.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum TypeCheckLevel {
    /// Report definite errors without reporting ambiguous types.
    #[default]
    Gradual,
    /// Return ambiguity warnings alongside the validated artifact.
    Informative,
    /// Reject reported ambiguities before exposing an artifact.
    Strict,
}
impl std::str::FromStr for TypeCheckLevel {
    type Err = &'static str;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "gradual" => Ok(Self::Gradual),
            "informative" => Ok(Self::Informative),
            "strict" => Ok(Self::Strict),
            _ => Err(
                "type-check level must be gradual, informative, or strict; disabled checking is not implemented",
            ),
        }
    }
}
/// Source compilation settings. Existing entry points retain their API/default
/// behavior; this entry additionally selects ambiguity reporting.
#[derive(Debug, Default, Clone, Copy)]
pub struct CompileOptions {
    /// Explicit API availability checks, or the existing default profile.
    pub api: Option<ApiContext>,
    /// Source ambiguity reporting; mandatory IR validation always applies.
    pub type_check: TypeCheckLevel,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Warning,
    Error,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeDiagnosticKind {
    UntypedParameter,
    AmbiguousOperation,
    AmbiguousBoundary,
}
/// A policy diagnostic with the original source location and stable category.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeDiagnostic {
    pub severity: Severity,
    pub kind: TypeDiagnosticKind,
    pub location: SourceDiagnostic,
}
/// A successfully checked artifact plus informative-level warnings.
#[derive(Debug)]
pub struct CompileOutput<T> {
    pub artifact: T,
    pub warnings: Vec<TypeDiagnostic>,
}
/// A source failure or all strict-mode ambiguity diagnostics. Neither variant
/// exposes an artifact for emission.
#[derive(Debug)]
pub enum CompileError {
    Source(SourceDiagnostic),
    TypeChecking(Vec<TypeDiagnostic>),
}
impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Source(error) => write!(f, "{error}"),
            Self::TypeChecking(errors) => {
                for (index, error) in errors.iter().enumerate() {
                    if index != 0 {
                        writeln!(f)?;
                    }
                    write!(f, "{}", error.location)?;
                }
                Ok(())
            }
        }
    }
}
impl std::error::Error for CompileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Source(error) => Some(error),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Match {
    Yes,
    Maybe,
    No,
}
fn all(parts: impl IntoIterator<Item = Match>) -> Match {
    let mut result = Match::Yes;
    for part in parts {
        match part {
            Match::No => return Match::No,
            Match::Maybe => result = Match::Maybe,
            _ => {}
        }
    }
    result
}
/// Mutable fields and structural signatures retain their invariant contracts.
/// Unknown components make unequal contracts uncertain, not proven matches.
fn invariant(expected: &Type, actual: &Type) -> Match {
    if crate::structural::same_type(expected, actual) {
        Match::Yes
    } else if contains_any(expected) || contains_any(actual) {
        Match::Maybe
    } else {
        Match::No
    }
}
/// Compare container parameters by equality, allowing uncertain union overlap
/// recursively. Nominal inheritance is not container covariance; structural
/// fields and method signatures continue to use their own invariant comparison.
fn container_match(expected: &Type, actual: &Type) -> Match {
    if crate::structural::same_type(expected, actual) {
        return Match::Yes;
    }
    if let (Type::SourceValue(a), Type::SourceValue(b)) = (expected, actual) {
        return if a.native().is_empty()
            && b.native().is_empty()
            && (a.classes().iter().any(|class| b.classes().contains(class))
                || a.nullable() && b.nullable())
        {
            Match::Maybe
        } else {
            Match::No
        };
    }
    if matches!(expected, Type::SourceValue(_)) || matches!(actual, Type::SourceValue(_)) {
        return Match::No;
    }
    if *expected == Type::Any || *actual == Type::Any {
        return Match::Maybe;
    }
    match (expected, actual) {
        (Type::Union(parts), other) | (other, Type::Union(parts)) => {
            if parts
                .iter()
                .any(|part| container_match(part, other) != Match::No)
            {
                Match::Maybe
            } else {
                Match::No
            }
        }
        (Type::Array(a), Type::Array(b)) => container_match(a, b),
        (Type::Dictionary { key: a, value: b }, Type::Dictionary { key: c, value: d }) => {
            all([container_match(a, c), container_match(b, d)])
        }
        _ => Match::No,
    }
}
/// Classify native value boundaries without conflating unknown/overlapping
/// types with disjoint types. Exact class witnesses remain unit obligations.
pub(crate) fn compatibility(expected: &Type, actual: &Type) -> Match {
    if *expected == Type::Void || *actual == Type::Void {
        return if expected == actual {
            Match::Yes
        } else {
            Match::No
        };
    }
    if *expected == Type::Any || crate::structural::same_type(expected, actual) {
        return Match::Yes;
    }
    if *actual == Type::Any {
        return Match::Maybe;
    }
    if let Type::Union(parts) = actual {
        let matches = parts
            .iter()
            .map(|part| compatibility(expected, part))
            .collect::<Vec<_>>();
        return if matches.iter().all(|r| *r == Match::Yes) {
            Match::Yes
        } else if matches.iter().any(|r| *r != Match::No) {
            Match::Maybe
        } else {
            Match::No
        };
    }
    if let Type::Union(parts) = expected {
        let matches = parts
            .iter()
            .map(|part| compatibility(part, actual))
            .collect::<Vec<_>>();
        return if matches.contains(&Match::Yes) {
            Match::Yes
        } else if matches.contains(&Match::Maybe) {
            Match::Maybe
        } else {
            Match::No
        };
    }
    match (expected, actual) {
        (Type::Array(element), Type::TupleValue(tuple)) => all(tuple
            .slots()
            .iter()
            .map(|slot| compatibility(element, &slot.api_type()))),
        (Type::Array(a), Type::Array(b)) => container_match(a, b),
        (Type::Dictionary { key: a, value: b }, Type::Dictionary { key: c, value: d }) => {
            all([container_match(a, c), container_match(b, d)])
        }
        (Type::Structural(shape), _) => {
            if !crate::native::accepts(&Type::named("Toybox.Lang.Object"), actual) {
                return Match::No;
            }
            let fields = shape.fields.iter().map(|(name, expected)| {
                crate::structural::field_type(actual, name)
                    .map_or(Match::No, |actual| invariant(&expected.api_type(), &actual))
            });
            let methods = shape.methods.iter().map(|(name, expected)| {
                let Some(actual) = crate::structural::method_contract(actual, name) else {
                    return Match::No;
                };
                if expected.parameters.len() != actual.parameters.len() {
                    return Match::No;
                }
                all(std::iter::once(invariant(
                    &expected.result.api_type(),
                    &actual.result.api_type(),
                ))
                .chain(
                    expected
                        .parameters
                        .iter()
                        .zip(&actual.parameters)
                        .map(|(a, b)| invariant(&a.api_type(), &b.api_type())),
                ))
            });
            all(fields.chain(methods))
        }
        (Type::Options(a), Type::Options(b)) => all(b
            .iter()
            .filter_map(|(key, value)| a.get(key).map(|expected| compatibility(expected, value)))),
        (Type::TupleValue(a), Type::TupleValue(b)) if a.slots().len() == b.slots().len() => all(a
            .slots()
            .iter()
            .zip(b.slots())
            .map(|(a, b)| compatibility(&a.api_type(), &b.api_type()))),
        (Type::Callback(a), Type::Callback(b)) if a.parameters.len() == b.parameters.len() => all(
            std::iter::once(compatibility(&a.result.api_type(), &b.result.api_type())).chain(
                a.parameters
                    .iter()
                    .zip(&b.parameters)
                    .map(|(a, b)| compatibility(&b.api_type(), &a.api_type())),
            ),
        ),
        (Type::Callback(_), Type::Named(name)) if name == "Toybox.Lang.Method" => Match::Maybe,
        _ => {
            if crate::native::accepts(expected, actual) {
                Match::Yes
            } else {
                Match::No
            }
        }
    }
}
fn contains_any(ty: &Type) -> bool {
    match ty {
        Type::Any => true,
        Type::Array(ty) => contains_any(ty),
        Type::Union(parts) | Type::Tuple(parts) => parts.iter().any(contains_any),
        Type::Dictionary { key, value } => contains_any(key) || contains_any(value),
        Type::Options(fields) => fields.values().any(contains_any),
        Type::TupleValue(tuple) => tuple.slots().iter().any(|ty| contains_any(&ty.api_type())),
        Type::Callback(signature) => {
            contains_any(&signature.result.api_type())
                || signature
                    .parameters
                    .iter()
                    .any(|ty| contains_any(&ty.api_type()))
        }
        Type::Structural(shape) => {
            shape.fields.values().any(|ty| contains_any(&ty.api_type()))
                || shape.methods.values().any(|m| {
                    contains_any(&m.result.api_type())
                        || m.parameters.iter().any(|p| contains_any(&p.api_type()))
                })
        }
        Type::SourceValue(contract) => contract.native().iter().any(contains_any),
        _ => false,
    }
}

fn type_name(ty: &Type) -> String {
    match ty {
        Type::Any => "Any".into(),
        Type::Null => "Null".into(),
        Type::Named(name) => name.strip_prefix("Toybox.").unwrap_or(name).into(),
        _ => crate::unions::parameter(ty.clone())
            .map(|ty| crate::type_text::parameter_tag(&ty).into_owned())
            .unwrap_or_else(|| "Void".into()),
    }
}
struct Issue {
    body: usize,
    operation: Option<usize>,
    kind: TypeDiagnosticKind,
    message: String,
}
fn join_contract(join: &crate::JoinType) -> Type {
    join.source_contract()
        .map(|c| Type::SourceValue(Box::new(c)))
        .unwrap_or_else(|| join.api_type())
}
/// Resolve exact class alternatives with the same witnesses used by unit
/// validation. A projected native base is not an exact source class.
fn boundary_match(
    expected: &Type,
    actual: &Type,
    unit: Option<&CompilationUnit>,
    api: Option<ApiContext>,
) -> Match {
    if let (Type::SourceValue(expected), Type::SourceValue(actual)) = (expected, actual) {
        let witness = |class: &crate::SourceClass, ty: &Type| {
            unit.is_some_and(|u| u.check_source_witness(class, ty, api))
        };
        if expected.accepts_with(actual, witness) {
            Match::Yes
        } else if expected.accepts_boundary_with(actual, witness) {
            Match::Maybe
        } else {
            Match::No
        }
    } else {
        compatibility(expected, actual)
    }
}
fn inspect(
    function: &Function,
    body: usize,
    owner: &str,
    signatures: &BTreeMap<(String, String), Vec<Type>>,
    inherited: bool,
    unit: Option<&CompilationUnit>,
    api: Option<ApiContext>,
) -> Vec<Issue> {
    let mut issues = Vec::new();
    let mut issue = |operation, kind, message| {
        issues.push(Issue {
            body,
            operation,
            kind,
            message,
        })
    };
    if !inherited {
        for (index, parameter) in function.parameter_types().iter().enumerate() {
            if parameter.api_type() == Type::Any {
                issue(
                    None,
                    TypeDiagnosticKind::UntypedParameter,
                    format!(
                        "parameter {} of {owner}{} is untyped",
                        index + 1,
                        function.name()
                    ),
                );
            }
        }
    }
    let origins = crate::source_objects::Origins::for_function(function)
        .expect("policy diagnostics require validated source origins");
    let ty = |value: &ValueId| {
        if function.value_types[value] == crate::ValueType::Any {
            Type::Any
        } else {
            origins
                .contract(*value)
                .map(|c| Type::SourceValue(Box::new(c.clone())))
                .unwrap_or_else(|| function.value_types[value].api_type())
        }
    };
    let mut locals = BTreeMap::new();
    let mut array_builders = BTreeMap::new();
    let mut dictionary_builders = BTreeMap::new();
    let mut record_builders = BTreeMap::new();
    let joins = crate::joins::validate(function.operations())
        .expect("policy diagnostics require validated IR");
    let symbols: BTreeMap<_, _> = function
        .operations()
        .iter()
        .filter_map(|op| {
            if let Operation::Symbol { result, name } = &op.operation {
                Some((*result, name.as_str()))
            } else {
                None
            }
        })
        .collect();
    for (at, located) in function.operations().iter().enumerate() {
        let mut boundaries: Vec<(Type, ValueId)> = Vec::new();
        let mut dynamic = false;
        match &located.operation {
            Operation::ArrayBegin {
                builder,
                element_type,
                ..
            } => {
                array_builders.insert(*builder, element_type.api_type());
            }
            Operation::ArrayFill { builder, value, .. } => {
                if let Some(expected) = array_builders.get(builder) {
                    boundaries.push((expected.clone(), *value));
                }
            }
            Operation::ArrayFillMany {
                builder, values, ..
            } => {
                if let Some(expected) = array_builders.get(builder) {
                    boundaries.extend(values.iter().map(|value| (expected.clone(), *value)));
                }
            }
            Operation::DictionaryBegin {
                builder,
                dictionary_type,
                ..
            } => {
                dictionary_builders.insert(*builder, dictionary_type.api_type());
            }
            Operation::RecordBegin {
                builder, fields, ..
            } => {
                record_builders.insert(*builder, fields);
            }
            Operation::DictionaryFill {
                builder, entries, ..
            } => {
                for (key, value) in entries {
                    if let Some(Type::Dictionary {
                        key: expected_key,
                        value: expected_value,
                    }) = dictionary_builders.get(builder)
                    {
                        boundaries.push(((**expected_key).clone(), *key));
                        boundaries.push(((**expected_value).clone(), *value));
                    }
                    if let Some(expected) = record_builders
                        .get(builder)
                        .and_then(|fields| symbols.get(key).and_then(|name| fields.get(*name)))
                    {
                        boundaries.push((expected.clone(), *value));
                    }
                }
            }

            Operation::Return { value } => {
                boundaries.push((crate::tuples::exact_type(&function.return_type()), *value))
            }
            Operation::JumpValue { target, value } => {
                boundaries.push((join_contract(&joins[target]), *value));
            }
            Operation::SelfFieldAssign { field, value } => {
                if let Some(field) = unit
                    .and_then(|unit| {
                        unit.classes()
                            .iter()
                            .find(|c| c.name == owner.trim_end_matches('.'))
                    })
                    .and_then(|class| class.fields.iter().find(|f| f.name == *field))
                {
                    boundaries.push((crate::tuples::exact_type(&field.result_type()), *value));
                }
            }
            Operation::MemberWrite {
                receiver,
                member,
                value,
                field_type,
            } => {
                let receiver = ty(receiver);
                dynamic = receiver == Type::Any;
                if let Some(expected) = field_type {
                    boundaries.push((crate::tuples::exact_type(expected), *value));
                } else {
                    for receiver in crate::unions::alternatives(&receiver) {
                        if let Ok(expected) = crate::structural::field_contract(receiver, member) {
                            boundaries.push((crate::tuples::exact_type(&expected), *value));
                        }
                    }
                }
            }
            Operation::LocalDeclare { local, value_type } => {
                locals.insert(*local, join_contract(value_type));
            }
            Operation::LocalInitialize { local, value }
            | Operation::LocalAssign { local, value } => {
                if let Some(expected) = locals.get(local) {
                    boundaries.push((expected.clone(), *value));
                }
            }
            Operation::ParameterAssign { index, value } => {
                boundaries.push((function.parameter_types()[*index].exact_type(), *value))
            }
            Operation::GlobalCall {
                member, arguments, ..
            }
            | Operation::GlobalCallStatement { member, arguments } => {
                if let Some(parameters) = signatures.get(&(String::new(), member.clone())) {
                    boundaries.extend(parameters.iter().cloned().zip(arguments.iter().copied()));
                }
            }
            Operation::ModuleCall {
                module,
                member,
                arguments,
                ..
            }
            | Operation::ModuleCallStatement {
                module,
                member,
                arguments,
            } => {
                if let Some(parameters) = signatures.get(&(module.clone(), member.clone())) {
                    boundaries.extend(parameters.iter().cloned().zip(arguments.iter().copied()));
                }
            }
            Operation::NativeModuleCall {
                module,
                member,
                arguments,
                ..
            }
            | Operation::NativeModuleCallStatement {
                module,
                member,
                arguments,
            } => {
                if let Some(parameters) = crate::native::module_parameters(module, member) {
                    boundaries.extend(parameters.into_iter().zip(arguments.iter().copied()));
                }
            }
            Operation::ObjectCall {
                classes,
                member,
                arguments,
                ..
            }
            | Operation::ObjectCallStatement {
                classes,
                member,
                arguments,
                ..
            } => {
                for class in classes {
                    if let Some(parameters) = signatures.get(&(class.clone(), member.clone())) {
                        for boundary in parameters.iter().cloned().zip(arguments.iter().copied()) {
                            if !boundaries.contains(&boundary) {
                                boundaries.push(boundary);
                            }
                        }
                    }
                }
            }
            Operation::SourceNew {
                class, arguments, ..
            } => {
                if let Some(parameters) = signatures.get(&(class.clone(), "initialize".into())) {
                    boundaries.extend(parameters.iter().cloned().zip(arguments.iter().copied()));
                }
            }
            Operation::SelfCall {
                member, arguments, ..
            }
            | Operation::SelfCallStatement { member, arguments } => {
                if let Some(parameters) =
                    signatures.get(&(owner.trim_end_matches('.').into(), member.clone()))
                {
                    boundaries.extend(parameters.iter().cloned().zip(arguments.iter().copied()));
                }
            }
            Operation::NativeCall {
                receiver,
                member,
                arguments,
                ..
            }
            | Operation::NativeCallStatement {
                receiver,
                member,
                arguments,
            } => {
                let receiver_type = ty(receiver);
                dynamic = receiver_type == Type::Any;
                if let Some(parameters) = crate::native::value_parameters(&receiver_type, member) {
                    boundaries.extend(parameters.into_iter().zip(arguments.iter().copied()));
                }
            }
            Operation::Binary { left, right, .. } | Operation::Compare { left, right, .. } => {
                dynamic = ty(left) == Type::Any || ty(right) == Type::Any
            }
            Operation::Not { operand, .. }
            | Operation::Negate { operand, .. }
            | Operation::Positive { operand, .. }
            | Operation::Complement { operand, .. }
            | Operation::Step { operand, .. } => dynamic = ty(operand) == Type::Any,
            Operation::ArrayRead { array, index, .. }
            | Operation::ArrayWrite { array, index, .. } => {
                let receiver = ty(array);
                dynamic = receiver == Type::Any;
                if !crate::record_flow::has_view(&receiver) {
                    boundaries.push((Type::named("Toybox.Lang.Number"), *index));
                }
                if let Operation::ArrayWrite { value, .. } = &located.operation {
                    if let Type::Array(element) = receiver {
                        boundaries.push((*element, *value));
                    }
                }
            }
            Operation::DictionaryRead {
                dictionary, key, ..
            }
            | Operation::DictionaryWrite {
                dictionary, key, ..
            } => {
                if let Type::Dictionary { key: expected, .. } = ty(dictionary) {
                    boundaries.push((*expected, *key));
                }
                if let Operation::DictionaryWrite { value, .. } = &located.operation {
                    if let Type::Dictionary {
                        value: expected, ..
                    } = ty(dictionary)
                    {
                        boundaries.push((*expected, *value));
                    }
                }
            }
            Operation::MemberRead { receiver, .. } | Operation::Has { receiver, .. } => {
                dynamic = ty(receiver) == Type::Any
            }
            _ => {}
        }
        if dynamic {
            issue(
                Some(at),
                TypeDiagnosticKind::AmbiguousOperation,
                "cannot determine the operand or receiver type".into(),
            );
        }
        for (expected, value) in boundaries {
            if boundary_match(&expected, &ty(&value), unit, api) == Match::Maybe {
                issue(
                    Some(at),
                    TypeDiagnosticKind::AmbiguousBoundary,
                    format!(
                        "value of type {} may not satisfy {}",
                        type_name(&ty(&value)),
                        type_name(&expected)
                    ),
                );
            }
        }
    }
    issues
}
fn finish<T>(
    artifact: T,
    issues: Vec<Issue>,
    bodies: &[&Function],
    sources: &[&str],
    origins: &[(usize, usize)],
    level: TypeCheckLevel,
) -> Result<CompileOutput<T>, CompileError> {
    let severity = if level == TypeCheckLevel::Strict {
        Severity::Error
    } else {
        Severity::Warning
    };
    let diagnostics = issues
        .into_iter()
        .map(|issue| {
            let (source, start) = origins.get(issue.body).copied().unwrap_or((0, 0));
            let start = issue
                .operation
                .and_then(|at| bodies[issue.body].operations().get(at))
                .and_then(|op| op.span)
                .map_or(start, |span| span.start);
            TypeDiagnostic {
                severity,
                kind: issue.kind,
                location: SourceDiagnostic {
                    source,
                    diagnostic: Diagnostic::at(
                        sources.get(source).copied().unwrap_or(""),
                        start,
                        issue.message,
                    ),
                },
            }
        })
        .collect::<Vec<_>>();
    if level == TypeCheckLevel::Strict && !diagnostics.is_empty() {
        Err(CompileError::TypeChecking(diagnostics))
    } else {
        Ok(CompileOutput {
            artifact,
            warnings: diagnostics,
        })
    }
}

/// Compile a declaration unit and report ambiguity under the selected checked
/// policy. Existing source/IR errors still fail before policy diagnostics.
pub fn compile_sources_with_options(
    sources: &[&str],
    options: CompileOptions,
) -> Result<CompileOutput<CompilationUnit>, CompileError> {
    let (unit, origins) =
        crate::unit::compile_sources_mapped(sources, options.api).map_err(CompileError::Source)?;
    if options.type_check == TypeCheckLevel::Gradual {
        return Ok(CompileOutput {
            artifact: unit,
            warnings: vec![],
        });
    }
    let mut signatures = BTreeMap::new();
    let mut bodies = Vec::new();
    let mut owners = Vec::new();
    let mut inherited = Vec::new();
    for declaration in unit.declarations() {
        let module = declaration.module.clone().unwrap_or_default();
        signatures.insert(
            (module.clone(), declaration.function.name().into()),
            declaration
                .function
                .parameter_types()
                .iter()
                .map(crate::ParameterType::exact_type)
                .collect(),
        );
        bodies.push(&declaration.function);
        owners.push(if module.is_empty() {
            module
        } else {
            format!("{module}.")
        });
        inherited.push(false);
    }
    for class in unit.classes() {
        for function in &class.methods {
            signatures.insert(
                (class.name.clone(), function.name().into()),
                function
                    .parameter_types()
                    .iter()
                    .map(crate::ParameterType::exact_type)
                    .collect(),
            );
            bodies.push(function);
            owners.push(format!("{}.", class.name));
            inherited.push(crate::native::callback(class.base, function.name()).is_some());
        }
    }
    let issues = bodies
        .iter()
        .enumerate()
        .flat_map(|(at, function)| {
            inspect(
                function,
                at,
                &owners[at],
                &signatures,
                inherited[at],
                Some(&unit),
                options.api,
            )
        })
        .collect();
    // Body references are needed only while locating diagnostics.
    let result = finish((), issues, &bodies, sources, &origins, options.type_check)?;
    Ok(CompileOutput {
        artifact: unit,
        warnings: result.warnings,
    })
}

/// Compile one global function with checked-policy diagnostics. Unit-only
/// declarations and unresolved-callee limitations match `compile_function`.
pub fn compile_function_with_options(
    source: &str,
    options: CompileOptions,
) -> Result<CompileOutput<Function>, CompileError> {
    let function = crate::compile_fragment(source, options.api).map_err(|diagnostic| {
        CompileError::Source(SourceDiagnostic {
            source: 0,
            diagnostic,
        })
    })?;
    if options.type_check == TypeCheckLevel::Gradual {
        return Ok(CompileOutput {
            artifact: function,
            warnings: vec![],
        });
    }
    let issues = inspect(&function, 0, "", &BTreeMap::new(), false, None, options.api);
    let result = finish(
        (),
        issues,
        &[&function],
        &[source],
        &[(0, 0)],
        options.type_check,
    )?;
    Ok(CompileOutput {
        artifact: function,
        warnings: result.warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn typed_join_reports_the_incoming_unknown_edge() {
        let ir = "ciq-ir\n.function f parameters=1 parameter-types=any result=integer\n.block ^0\n.boolean %0 true\n.branch %0 ^1 ^2\n.block ^1\n.parameter %1 0\n.jump-value ^3 %1\n.block ^2\n.integer %2 22\n.jump-value ^3 %2\n.block ^3\n.block-value %3 integer\n.return %3\n.end\n";
        let function = crate::parse_function_ir(ir).unwrap();
        let issues = inspect(&function, 0, "", &BTreeMap::new(), false, None, None);
        let boundary: Vec<_> = issues
            .iter()
            .filter(|issue| issue.kind == TypeDiagnosticKind::AmbiguousBoundary)
            .collect();
        assert_eq!(boundary.len(), 1);
        assert!(matches!(
            function.operations()[boundary[0].operation.unwrap()].operation,
            Operation::JumpValue { .. }
        ));
        let typed = ir.replace("parameter-types=any", "parameter-types=integer");
        let function = crate::parse_function_ir(&typed).unwrap();
        assert!(inspect(&function, 0, "", &BTreeMap::new(), false, None, None).is_empty());
        let dynamic_join = ir
            .replace("result=integer", "result=any")
            .replace("%3 integer", "%3 any");
        let function = crate::parse_function_ir(&dynamic_join).unwrap();
        assert!(
            !inspect(&function, 0, "", &BTreeMap::new(), false, None, None)
                .iter()
                .any(|i| i.kind == TypeDiagnosticKind::AmbiguousBoundary)
        );
    }

    #[test]
    fn structural_matches_keep_unknown_members_distinct_from_missing_members() {
        let make = |field: crate::ReturnType, result: crate::ReturnType| {
            Type::Structural(Box::new(crate::StructuralInterface {
                fields: BTreeMap::from([("value".into(), field)]),
                methods: BTreeMap::from([(
                    "read".into(),
                    crate::StructuralMethod {
                        parameters: vec![],
                        result,
                    },
                )]),
                declaration: None,
            }))
        };
        let expected = make(crate::ReturnType::Integer, crate::ReturnType::Integer);
        let unknown = make(crate::ReturnType::Any, crate::ReturnType::Integer);
        assert_eq!(compatibility(&expected, &unknown), Match::Maybe);
        assert_eq!(
            compatibility(
                &expected,
                &make(crate::ReturnType::Any, crate::ReturnType::String)
            ),
            Match::No
        );
        assert_eq!(compatibility(&expected, &Type::Null), Match::No);
        assert_eq!(compatibility(&expected, &Type::Any), Match::Maybe);
        let mut missing = expected.clone();
        if let Type::Structural(shape) = &mut missing {
            shape.methods.clear();
        }
        assert_eq!(compatibility(&expected, &missing), Match::No);
        assert_eq!(compatibility(&expected, &expected), Match::Yes);
    }

    #[test]
    fn native_matches_separate_dynamic_overlap_and_disjoint_contracts() {
        let number = Type::named("Toybox.Lang.Number");
        let string = Type::named("Toybox.Lang.String");
        let union = Type::Union(vec![number.clone(), string.clone()]);
        assert_eq!(compatibility(&number, &Type::Any), Match::Maybe);
        assert_eq!(compatibility(&Type::Any, &number), Match::Yes);
        assert_eq!(compatibility(&number, &string), Match::No);
        assert_eq!(compatibility(&number, &union), Match::Maybe);
        assert_eq!(compatibility(&union, &number), Match::Yes);
        assert_eq!(compatibility(&Type::Void, &Type::Any), Match::No);
        assert_eq!(
            compatibility(
                &Type::Array(Box::new(number.clone())),
                &Type::Array(Box::new(Type::Any))
            ),
            Match::Maybe
        );
        assert_eq!(
            compatibility(
                &Type::dictionary(string.clone(), number.clone()),
                &Type::dictionary(number.clone(), Type::Any)
            ),
            Match::No
        );
        let expected = Type::Options(BTreeMap::from([("x".into(), number)]));
        assert_eq!(
            compatibility(&expected, &Type::Options(BTreeMap::new())),
            Match::Yes
        );
        assert_eq!(
            compatibility(
                &expected,
                &Type::Options(BTreeMap::from([("x".into(), Type::Any)]))
            ),
            Match::Maybe
        );
    }
}
