//! Class-containing unions that distinguish exact identities from native types.
use crate::{JoinType, ParameterType, ReturnType, SourceClass, api::Type};

/// A normalized union containing exact source classes and optional native/Null
/// alternatives. Native alternatives do not establish exact source identities.
/// Classes retain their native bases; compilation-unit validation resolves each
/// identity against its declaration. This contract does not express inheritance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceUnion {
    pub(crate) classes: Vec<SourceClass>,
    pub(crate) nullable: bool,
    pub(crate) native: Vec<Type>,
}

/// Match value contracts with caller-supplied source and native proofs. Tuple
/// slots retain exact identities; class bases cannot stand in for interface
/// implementations. Ordinary container domains keep their invariant rules.
pub(crate) fn matches_type_with(
    expected: &Type,
    actual: &Type,
    source: &impl Fn(&SourceClass, &Type) -> bool,
    native: &impl Fn(&Type, &Type) -> bool,
) -> bool {
    matches_in(
        expected,
        actual,
        source,
        native,
        &crate::recursive::Comparison::default(),
    )
}
fn matches_in(
    expected: &Type,
    actual: &Type,
    source: &impl Fn(&SourceClass, &Type) -> bool,
    native: &impl Fn(&Type, &Type) -> bool,
    comparison: &crate::recursive::Comparison,
) -> bool {
    comparison
        .scoped(expected, actual, || {
            matches_step(expected, actual, source, native, comparison)
        })
        .unwrap_or(true)
}
fn matches_step(
    expected: &Type,
    actual: &Type,
    source: &impl Fn(&SourceClass, &Type) -> bool,
    native: &impl Fn(&Type, &Type) -> bool,
    comparison: &crate::recursive::Comparison,
) -> bool {
    if let Some(body) = crate::recursive::unfold(expected) {
        return matches_in(&body, actual, source, native, comparison);
    }
    if let Some(body) = crate::recursive::unfold(actual) {
        return matches_in(expected, &body, source, native, comparison);
    }

    let class_matches = |target: &Type, class: &SourceClass| {
        if crate::structural::contains(target)
            || matches!(
                crate::ValueType::from_api_type(target),
                Some(crate::ValueType::Interface(_))
            )
        {
            source(class, target)
        } else {
            native(target, &Type::named(class.base.api_name()))
        }
    };
    crate::tuples::matches_with(
        expected,
        actual,
        &|expected, actual| match (expected, actual) {
            (Type::SourceValue(expected), Type::SourceValue(actual)) => {
                (expected.nullable() || !actual.nullable())
                    && actual.native().iter().all(|ty| {
                        matches_in(&expected.native_type(), ty, source, native, comparison)
                    })
                    && actual.classes().iter().all(|class| {
                        expected.classes().contains(class)
                            || expected.native().iter().any(|ty| class_matches(ty, class))
                    })
            }
            (Type::SourceValue(expected), actual) => {
                matches_in(&expected.native_type(), actual, source, native, comparison)
            }
            (expected, Type::SourceValue(actual)) => {
                (!actual.nullable() || native(expected, &Type::Null))
                    && actual
                        .native()
                        .iter()
                        .all(|ty| matches_in(expected, ty, source, native, comparison))
                    && actual.classes().iter().all(|class| {
                        crate::unions::alternatives(expected)
                            .iter()
                            .any(|ty| class_matches(ty, class))
                    })
            }
            _ => native(expected, actual),
        },
    )
}

impl SourceUnion {
    /// Normalize exact class alternatives, rejecting empty/oversized sets,
    /// malformed identities, and unsupported source-reference bases.
    pub fn new(classes: Vec<SourceClass>, nullable: bool) -> Result<Self, &'static str> {
        Self::with_native(classes, Vec::new(), nullable)
    }
    /// Normalize a class-containing union with supported native alternatives.
    /// Reject malformed classes, unsupported native value contracts, and more
    /// than 254 normalized alternatives. Native Null contributes nullability.
    pub fn with_native(
        mut classes: Vec<SourceClass>,
        native: Vec<Type>,
        nullable: bool,
    ) -> Result<Self, &'static str> {
        classes.sort();
        classes.dedup();
        if classes.is_empty() {
            return Err("class-containing union requires an exact source alternative");
        }
        if classes
            .iter()
            .any(|c| SourceClass::parse(&c.to_string()).as_ref() != Some(c))
        {
            return Err("invalid source union class identity or native base");
        }
        let normalized = crate::unions::normalize(native);
        let mut nullable = nullable;
        let mut native = Vec::new();
        for ty in crate::unions::alternatives(&normalized) {
            if *ty == Type::Null {
                nullable = true;
            } else if !matches!(ty, Type::SourceValue(_))
                && crate::unions::parameter(ty.clone()).is_some()
            {
                native.push(ty.clone());
            } else {
                return Err("unsupported native source-union alternative");
            }
        }
        if classes.len() + native.len() + usize::from(nullable) > crate::MAX_PARAMETERS {
            return Err("source union exceeds 254 alternatives including Null");
        }
        Ok(Self {
            classes,
            nullable,
            native,
        })
    }
    /// Return normalized native alternatives without source-base projections.
    pub fn native(&self) -> &[Type] {
        &self.native
    }
    /// Return normalized class identities, without erasing their native bases.
    pub fn classes(&self) -> &[SourceClass] {
        &self.classes
    }
    /// Report whether the contract also admits Null.
    pub fn nullable(&self) -> bool {
        self.nullable
    }
    pub(crate) fn single(class: SourceClass, nullable: bool) -> Self {
        Self {
            classes: vec![class],
            nullable,
            native: Vec::new(),
        }
    }
    pub(crate) fn api_type(&self) -> Type {
        let mut types = self
            .classes
            .iter()
            .map(|c| Type::named(c.base.api_name()))
            .collect::<Vec<_>>();
        types.extend(self.native.iter().cloned());
        if self.nullable {
            types.push(Type::Null);
        }
        crate::unions::normalize(types)
    }
    pub(crate) fn join(self) -> JoinType {
        if self.native == [Type::Any] {
            return JoinType::Api(Type::Any);
        }
        if self.classes.is_empty() {
            return JoinType::Api(self.api_type());
        }
        if self.classes.len() == 1 && self.native.is_empty() {
            JoinType::Source {
                class: self.classes[0].clone(),
                nullable: self.nullable,
            }
        } else {
            JoinType::SourceUnion(self)
        }
    }
    pub(crate) fn parameter(self) -> ParameterType {
        if self.native == [Type::Any] {
            return ParameterType::Any;
        }
        if self.classes.len() == 1 && self.native.is_empty() {
            if self.nullable {
                ParameterType::NullableSourceObject(self.classes[0].clone())
            } else {
                ParameterType::SourceObject(self.classes[0].clone())
            }
        } else {
            ParameterType::SourceUnion(self)
        }
    }
    pub(crate) fn result(self) -> ReturnType {
        if self.native == [Type::Any] {
            return ReturnType::Any;
        }
        if self.classes.len() == 1 && self.native.is_empty() {
            if self.nullable {
                ReturnType::NullableSourceObject(self.classes[0].clone())
            } else {
                ReturnType::SourceObject(self.classes[0].clone())
            }
        } else {
            ReturnType::SourceUnion(self)
        }
    }
    pub(crate) fn native_type(&self) -> Type {
        let mut types = self.native.clone();
        if self.nullable {
            types.push(Type::Null);
        }
        crate::unions::normalize(types)
    }
    pub(crate) fn accepts_api(&self, actual: &Type) -> bool {
        crate::native::accepts(&self.native_type(), actual)
    }
    pub(crate) fn accepts_with(
        &self,
        actual: &Self,
        witness: impl Fn(&SourceClass, &Type) -> bool,
    ) -> bool {
        self.accepts_using(actual, &crate::native::accepts, witness)
    }
    /// Match exact/native alternatives while preserving the caller's recursive
    /// proof context and its separate source-interface witness obligation.
    pub(crate) fn accepts_using(
        &self,
        actual: &Self,
        native: &impl Fn(&Type, &Type) -> bool,
        witness: impl Fn(&SourceClass, &Type) -> bool,
    ) -> bool {
        (self.nullable || !actual.nullable)
            && actual
                .native
                .iter()
                .all(|ty| native(&self.native_type(), ty))
            && actual.classes.iter().all(|c| {
                self.classes.contains(c)
                    || self
                        .native
                        .iter()
                        .any(|ty| native(ty, &Type::named(c.base.api_name())) || witness(c, ty))
            })
    }
    /// Accept a runtime boundary with a shared class/Null alternative, native
    /// overlap, or a unit-established interface witness.
    /// A native base cannot establish an exact class or structural witness.
    /// This does not replace `accepts_with` for proven membership checks.
    pub(crate) fn accepts_boundary_with(
        &self,
        actual: &Self,
        witness: impl Fn(&SourceClass, &Type) -> bool,
    ) -> bool {
        (self.nullable && actual.nullable)
            || actual
                .native
                .iter()
                .any(|ty| crate::native::accepts_boundary(&self.native_type(), ty))
            || actual.classes.iter().any(|class| {
                self.classes.contains(class)
                    || self.native.iter().any(|ty| {
                        crate::native::accepts(ty, &Type::named(class.base.api_name()))
                            || witness(class, ty)
                    })
            })
    }

    /// Defer interface witnesses to unit validation at a value boundary.
    pub(crate) fn accepts_boundary(&self, actual: &Self) -> bool {
        self.accepts_boundary_with(actual, |_, ty| {
            crate::structural::contains(ty)
                || matches!(
                    crate::ValueType::from_api_type(ty),
                    Some(crate::ValueType::Interface(_))
                )
        })
    }
    pub(crate) fn merge(&self, other: &Self) -> Option<Self> {
        Self::with_native(
            self.classes.iter().chain(&other.classes).cloned().collect(),
            self.native.iter().chain(&other.native).cloned().collect(),
            self.nullable || other.nullable,
        )
        .ok()
    }
    pub(crate) fn merge_api(&self, ty: &Type) -> Option<Self> {
        Self::with_native(
            self.classes.clone(),
            self.native
                .iter()
                .cloned()
                .chain(std::iter::once(ty.clone()))
                .collect(),
            self.nullable,
        )
        .ok()
    }
    pub(crate) fn text(&self) -> String {
        if self.classes.len() == 1 && self.native.is_empty() {
            return if self.nullable {
                format!("nullable-source({})", self.classes[0])
            } else {
                format!("source({})", self.classes[0])
            };
        }
        let mut tags = self
            .classes
            .iter()
            .map(|c| format!("source({c})"))
            .collect::<Vec<_>>();
        tags.extend(self.native.iter().map(|ty| {
            crate::type_text::parameter_tag(
                &crate::unions::parameter(ty.clone()).expect("validated native alternative"),
            )
            .into_owned()
        }));
        if self.nullable {
            tags.push("null".into());
        }
        format!("source-union({})", tags.join("|"))
    }
    /// Refine source and native alternatives using the same storage facts.
    /// A runtime source-class test can recover an exact identity from an opaque
    /// native alternative; a mere native base projection cannot do so.
    /// An empty intersection retains the original contract: there is no bottom
    /// type or unreachable-code proof in this type model.
    pub(crate) fn refine(
        &self,
        place: crate::type_flow::Place,
        facts: &std::collections::BTreeSet<crate::type_flow::Fact>,
    ) -> JoinType {
        use crate::type_flow::{ClassTarget, Fact};
        if facts.contains(&Fact::Null(place)) && !facts.contains(&Fact::NonNull(place)) {
            return JoinType::Api(Type::Null);
        }
        let mut refined = self.clone();
        for fact in facts {
            match fact {
                Fact::NonNull(at) if *at == place => refined.nullable = false,
                Fact::Class(at, ClassTarget::Source(class)) if *at == place => {
                    let base = Type::named(class.base.api_name());
                    let opaque_match = refined
                        .native
                        .iter()
                        .any(|ty| crate::native::accepts(ty, &base));
                    refined.classes.retain(|c| c == class);
                    if opaque_match && !refined.classes.contains(class) {
                        refined.classes.push(class.clone());
                    }
                    refined.native.clear();
                    refined.nullable = false;
                }
                Fact::NotClass(at, ClassTarget::Source(class)) if *at == place => {
                    refined.classes.retain(|c| c != class)
                }
                Fact::Class(at, ClassTarget::Native(class)) if *at == place => {
                    refined.classes.retain(|c| {
                        crate::native::accepts(&Type::named(class), &Type::named(c.base.api_name()))
                    });
                    let target = Type::named(class);
                    refined.native = refined
                        .native
                        .into_iter()
                        .filter_map(|ty| {
                            if crate::native::accepts(&target, &ty) {
                                Some(ty)
                            } else if crate::native::accepts(&ty, &target)
                                && crate::ValueType::from_api_type(&target).is_some()
                            {
                                Some(target.clone())
                            } else {
                                None
                            }
                        })
                        .collect();
                    refined.nullable = false;
                }
                Fact::NotClass(at, ClassTarget::Native(class)) if *at == place => {
                    refined
                        .native
                        .retain(|ty| !crate::native::accepts(&Type::named(class), ty));
                    refined.classes.retain(|c| {
                        !crate::native::accepts(
                            &Type::named(class),
                            &Type::named(c.base.api_name()),
                        )
                    })
                }
                _ => {}
            }
        }
        if refined.classes.is_empty() && refined.native.is_empty() {
            if refined.nullable {
                JoinType::Api(Type::Null)
            } else {
                self.clone().join()
            }
        } else {
            refined.join()
        }
    }
}
impl ParameterType {
    pub(crate) fn source_contract(&self) -> Option<SourceUnion> {
        match self {
            Self::SourceObject(c) => Some(SourceUnion::single(c.clone(), false)),
            Self::NullableSourceObject(c) => Some(SourceUnion::single(c.clone(), true)),
            Self::SourceUnion(c) => Some(c.clone()),
            _ => None,
        }
    }
}
impl ReturnType {
    pub(crate) fn source_contract(&self) -> Option<SourceUnion> {
        match self {
            Self::SourceObject(c) => Some(SourceUnion::single(c.clone(), false)),
            Self::NullableSourceObject(c) => Some(SourceUnion::single(c.clone(), true)),
            Self::SourceUnion(c) => Some(c.clone()),
            _ => None,
        }
    }
}
impl JoinType {
    pub(crate) fn source_contract(&self) -> Option<SourceUnion> {
        match self {
            Self::Source { class, nullable } => Some(SourceUnion::single(class.clone(), *nullable)),
            Self::SourceUnion(c) => Some(c.clone()),
            _ => None,
        }
    }
}
