//! Supported native callback contracts, independent of executable bindings.
use crate::{ClassBase, ParameterType, ReturnType};

pub(crate) struct CallbackSignature {
    pub parameters: &'static [ParameterType],
    pub result: ReturnType,
}

/// Resolve a callback contract for the supported native superclass profile.
/// Other members remain unsupported; this is not a complete API database.
pub(crate) fn callback(base: ClassBase, member: &str) -> Option<CallbackSignature> {
    match (base, member) {
        (ClassBase::ApplicationAppBase, "onStart") => Some(CallbackSignature {
            parameters: &[ParameterType::NullableDictionary],
            result: ReturnType::Void,
        }),
        _ => None,
    }
}
