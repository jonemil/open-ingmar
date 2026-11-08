//! Class-member access policy and runtime selector identities.
use std::collections::BTreeMap;

/// Class-member access, independent of source file boundaries. `hidden` is
/// normalized to Protected; omitted modifiers mean Public.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Visibility {
    #[default]
    Public,
    Protected,
    Private,
}
impl Visibility {
    pub(crate) fn parse(name: &str) -> Option<Self> {
        match name {
            "public" => Some(Self::Public),
            "protected" | "hidden" => Some(Self::Protected),
            "private" => Some(Self::Private),
            _ => None,
        }
    }
    pub(crate) fn text(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Protected => "protected",
            Self::Private => "private",
        }
    }
    pub(crate) fn flags(self) -> u8 {
        if self == Self::Protected { 0x20 } else { 0 }
    }
}
pub(crate) type Members = BTreeMap<(String, String), Visibility>;

/// Return a class-private selector distinct from every ordinary source member.
/// Source declarations reserve the `__ciq_private_` prefix for generated identities.
pub(crate) fn private_symbol(class: &str, member: &str) -> String {
    format!(
        "__ciq_private_{}_{}_{}",
        class.len(),
        crate::names::module_key(class),
        member
    )
}

/// Bind private selectors only after semantic validation. Saved IR keeps source
/// member names; VM lowering uses distinct identities without changing values,
/// receiver classes, contracts, or the source-visible Symbol literals.
pub(crate) fn bind_body<'a>(
    unit: &crate::CompilationUnit,
    class: Option<&str>,
    function: &'a crate::Function,
) -> std::borrow::Cow<'a, crate::Function> {
    if class.is_none_or(|c| !unit.has_private_members(c)) {
        return std::borrow::Cow::Borrowed(function);
    }
    use crate::Operation as O;
    let origins =
        crate::source_objects::Origins::for_function(function).expect("validated receiver origins");
    let mut result = function.clone();
    for located in &mut result.operations {
        match &mut located.operation {
            O::SelfCall { member, .. } | O::SelfCallStatement { member, .. } => {
                if let Some(class) = class {
                    *member = unit.method_symbol(class, member, Some(class));
                }
            }
            O::SelfFieldRead { field, .. } | O::SelfFieldAssign { field, .. } => {
                if let Some(class) = class {
                    *field = unit.member_symbol(class, field);
                }
            }
            O::ClassCall {
                class: receiver,
                member,
                ..
            }
            | O::ClassCallStatement {
                class: receiver,
                member,
                ..
            } => *member = unit.method_symbol(receiver, member, class),
            O::ClassFieldRead { class, field, .. } | O::ClassFieldBegin { class, field, .. } => {
                *field = unit.member_symbol(class, field)
            }
            O::MemberRead {
                receiver, member, ..
            }
            | O::MemberWrite {
                receiver, member, ..
            } => {
                if let Some(receiver_class) = origins
                    .contract(*receiver)
                    .and_then(|c| c.classes().first())
                {
                    *member = unit.instance_field_symbol(&receiver_class.name, member, class);
                }
            }
            _ => {}
        }
    }
    std::borrow::Cow::Owned(result)
}
