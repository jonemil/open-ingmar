//! Member candidates for exact source receivers and their native alternatives.
use crate::{ParameterType, ReturnType, SourceClass, SourceUnion, StructuralMethod, api::Type};

pub(crate) struct Targets {
    pub(crate) source: Vec<StructuralMethod>,
    pub(crate) native: Vec<Type>,
    pub(crate) missing: bool,
}

/// Resolve every retained receiver alternative without discarding missing members.
/// Source declarations override native members; native profiles are checked later.
pub(crate) fn targets(
    receiver: &SourceUnion,
    member: &str,
    mut declaration: impl FnMut(&SourceClass) -> Option<StructuralMethod>,
) -> Targets {
    let mut targets = Targets {
        source: Vec::new(),
        native: Vec::new(),
        missing: receiver.nullable(),
    };
    for class in receiver.classes() {
        if let Some(signature) = declaration(class) {
            targets.source.push(signature);
        } else {
            targets.add_native(Type::named(class.base.api_name()), member);
        }
    }
    for ty in receiver.native() {
        targets.add_native(ty.clone(), member);
    }
    targets
}

impl Targets {
    pub(crate) fn is_empty(&self) -> bool {
        self.source.is_empty() && self.native.is_empty()
    }

    fn add_native(&mut self, ty: Type, member: &str) {
        if crate::native::value_parameters(&ty, member).is_some() {
            self.native.push(ty);
        } else {
            self.missing = true;
        }
    }

    /// Return available parameter contracts for contextual literals and diagnostics.
    /// Missing members supply no construction context and remain uncertain.
    pub(crate) fn parameters(&self, member: &str) -> Vec<Vec<ParameterType>> {
        self.source
            .iter()
            .map(|s| s.parameters.clone())
            .chain(self.native.iter().filter_map(|ty| {
                crate::native::value_parameters(ty, member)?
                    .into_iter()
                    .map(crate::unions::parameter)
                    .collect()
            }))
            .collect()
    }

    /// Resolve a value result, validating native executable profiles and arguments.
    /// Void or an unsupported result join remains unusable as a value. Partial
    /// lookup retains Any rather than claiming a result for an unknown member.
    pub(crate) fn result(
        &self,
        member: &str,
        mut arguments: impl FnMut(&Type) -> Vec<Type>,
        context: Option<crate::api::ApiContext>,
    ) -> Result<ReturnType, String> {
        let mut results = self
            .source
            .iter()
            .map(|s| s.result.clone())
            .collect::<Vec<_>>();
        for ty in &self.native {
            let result = crate::native::value_call(ty, member, &arguments(ty), context)?;
            results.push(crate::unions::result(result).ok_or("unsupported native member result")?);
        }
        if results.is_empty() {
            return Err("no receiver alternative declares the requested member".into());
        }
        if results.contains(&ReturnType::Void) {
            return Ok(ReturnType::Void);
        }
        if self.missing {
            return Ok(ReturnType::Any);
        }
        let mut results = results.into_iter();
        let first = results.next().unwrap();
        Ok(results
            .try_fold(first, |a, b| crate::joins::call_result(&a, &b))
            .unwrap_or(ReturnType::Void))
    }
}
