//! Runtime class identities used by type tests, separate from source types.

/// Return the symbol of a supported concrete Lang class for executable binding.
/// Lexical interfaces and parameterized source types are not runtime classes.
pub(crate) fn symbol(class: &str) -> Option<u32> {
    Some(match class.strip_prefix("Toybox.Lang.")? {
        "Object" => 0x0080004c,
        "Number" => 0x00800096,
        "Long" => 0x00800097,
        "Float" => 0x00800098,
        "Double" => 0x00800099,
        "Boolean" => 0x0080009a,
        "Char" => 0x008004bd,
        "String" => 0x0080009b,
        "Array" => 0x0080009e,
        "Dictionary" => 0x0080002a,
        "Symbol" => 0x0080009d,
        "Method" => 0x0080009f,
        "WeakReference" => 0x00800416,
        "ByteArray" => 0x008007a2,
        _ => return None,
    })
}
