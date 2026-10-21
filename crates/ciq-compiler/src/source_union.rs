//! Finite exact source-class alternatives, separate from erased native types.
use crate::{JoinType, ParameterType, ReturnType, SourceClass, api::Type};

/// A normalized nonempty set of exact source classes, optionally admitting Null.
/// Classes retain their native bases; compilation-unit validation resolves each
/// identity against its declaration. This contract does not express inheritance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceUnion {
    pub(crate) classes: Vec<SourceClass>,
    pub(crate) nullable: bool,
}
impl SourceUnion {
    /// Normalize exact class alternatives, rejecting empty/oversized sets,
    /// malformed identities, and bases that cannot be source-constructed.
    pub fn new(mut classes: Vec<SourceClass>, nullable: bool) -> Result<Self, &'static str> {
        classes.sort();
        classes.dedup();
        if classes.is_empty() || classes.len() + usize::from(nullable) > crate::MAX_PARAMETERS {
            return Err("source union requires 1..=254 alternatives including Null");
        }
        if classes
            .iter()
            .any(|class| SourceClass::parse(&class.to_string()).as_ref() != Some(class))
        {
            return Err("invalid source union class identity or native base");
        }
        Ok(Self { classes, nullable })
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
        }
    }
    pub(crate) fn api_type(&self) -> Type {
        let mut types = self
            .classes
            .iter()
            .map(|c| Type::named(c.base.api_name()))
            .collect::<Vec<_>>();
        if self.nullable {
            types.push(Type::Null);
        }
        crate::unions::normalize(types)
    }
    pub(crate) fn join(self) -> JoinType {
        if self.classes.len() == 1 {
            JoinType::Source {
                class: self.classes[0].clone(),
                nullable: self.nullable,
            }
        } else {
            JoinType::SourceUnion(self)
        }
    }
    pub(crate) fn parameter(self) -> ParameterType {
        if self.classes.len() == 1 {
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
        if self.classes.len() == 1 {
            if self.nullable {
                ReturnType::NullableSourceObject(self.classes[0].clone())
            } else {
                ReturnType::SourceObject(self.classes[0].clone())
            }
        } else {
            ReturnType::SourceUnion(self)
        }
    }
    pub(crate) fn accepts(&self, actual: &Self) -> bool {
        (self.nullable || !actual.nullable)
            && actual.classes.iter().all(|c| self.classes.contains(c))
    }
    pub(crate) fn merge(&self, other: &Self) -> Option<Self> {
        Self::new(
            self.classes.iter().chain(&other.classes).cloned().collect(),
            self.nullable || other.nullable,
        )
        .ok()
    }
    pub(crate) fn text(&self) -> String {
        if self.classes.len() == 1 {
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
        if self.nullable {
            tags.push("null".into());
        }
        format!("source-union({})", tags.join("|"))
    }
    /// Refine exact alternatives using facts about the same storage identity.
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
                    refined.classes.retain(|c| c == class);
                    refined.nullable = false;
                }
                Fact::NotClass(at, ClassTarget::Source(class)) if *at == place => {
                    refined.classes.retain(|c| c != class)
                }
                Fact::Class(at, ClassTarget::Native(class)) if *at == place => {
                    refined.classes.retain(|c| {
                        crate::native::accepts(&Type::named(class), &Type::named(c.base.api_name()))
                    });
                    refined.nullable = false;
                }
                Fact::NotClass(at, ClassTarget::Native(class)) if *at == place => {
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
        if refined.classes.is_empty() {
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
