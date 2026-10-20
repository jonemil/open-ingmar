//! Observed native identities for the bounded SDK 9.2.0 console profile.
//! API declarations own availability/signatures; this module owns only VM bindings.
use std::collections::BTreeMap;

pub(super) const STATICS: u32 = 0x00800002;
pub(super) const TOYBOX: u32 = 0x00800003;
pub(super) const SYSTEM: u32 = 0x00800005;
pub(super) const WATCH_UI: u32 = 0x00800011;
pub(super) const LANG: u32 = 0x00800007;
pub(super) const OBJECT: u32 = 0x0080004c;
pub(super) const APPLICATION: u32 = 0x00800015;
pub(super) const INIT: u32 = 0x00800018;
pub(super) const INITIALIZE: u32 = 0x0080005e;
pub(super) const TO_STRING: u32 = 0x008000af;
pub(super) const FORMAT: u32 = 0x0080009c;
pub(super) const ABS: u32 = 0x008000a6;
pub(super) const EQUALS: u32 = 0x008000b3;
pub(super) const HASH_CODE: u32 = 0x008000ad;
pub(super) const WEAK: u32 = 0x00800418;
pub(super) const STILL_ALIVE: u32 = 0x00800417;
pub(super) const METHOD: u32 = 0x008000b2;
pub(super) const INVOKE: u32 = 0x008000ac;
pub(super) const COMPARE_TO: u32 = 0x00800e50;
pub(super) const LENGTH: u32 = 0x008000bb;
pub(super) const SUBSTRING: u32 = 0x008000bd;
pub(super) const FIND: u32 = 0x008000bc;
pub(super) const TO_NUMBER: u32 = 0x008000a7;
pub(super) const TO_FLOAT: u32 = 0x008000a5;
pub(super) const TO_DOUBLE: u32 = 0x008000a8;
pub(super) const TO_LONG: u32 = 0x008000a9;
pub(super) const TO_LONG_WITH_BASE: u32 = 0x00800b1c;
pub(super) const TO_CHAR: u32 = 0x008004c6;
pub(super) const TO_CHAR_ARRAY: u32 = 0x008004c4;
pub(super) const TO_UTF8_ARRAY: u32 = 0x008004c5;
pub(super) const SIZE: u32 = 0x008000ae;
pub(super) const DICTIONARY_PUT: u32 = 0x008000b4;
pub(super) const DICTIONARY_GET: u32 = 0x008000b5;
pub(super) const DICTIONARY_KEYS: u32 = 0x008000b9;
pub(super) const DICTIONARY_VALUES: u32 = 0x008000ba;
pub(super) const HAS_KEY: u32 = 0x008000b6;
pub(super) const IS_EMPTY: u32 = 0x008000b8;
pub(super) const INDEX_OF: u32 = 0x008004bf;
pub(super) const ARRAY_ADD: u32 = 0x0080014e;
pub(super) const ARRAY_ADD_ALL: u32 = 0x008004be;
pub(super) const ARRAY_REMOVE: u32 = 0x008000b7;
pub(super) const ARRAY_REMOVE_ALL: u32 = 0x008004c0;
pub(super) const REVERSE: u32 = 0x008004c1;
pub(super) const SLICE: u32 = 0x008004c2;
pub(super) const SORT: u32 = 0x00800e51;
pub(super) const COMPARE: u32 = 0x00800150;
pub(super) const TO_NUMBER_WITH_BASE: u32 = 0x008004c3;
pub(super) const TO_LOWER: u32 = 0x008000be;
pub(super) const TO_UPPER: u32 = 0x008000bf;
pub(super) const PRINTLN: u32 = 0x00800067;
pub(super) const EXIT: u32 = 0x0080006d;
pub(super) const VIEW: u32 = 0x0080019f;
pub(super) const BEHAVIOR_DELEGATE: u32 = 0x00800064;
pub(super) const ON_SELECT: u32 = 0x008003f3;
pub(super) const REQUEST_UPDATE: u32 = 0x008001a2;
pub(super) const APP_BASE: u32 = 0x0080029c;
pub(super) const ON_START: u32 = 0x0080029f;
pub(super) const ON_STOP: u32 = 0x008002a1;
pub(super) const GET_INITIAL_VIEW: u32 = 0x008002a0;
pub(super) const GLOBALS: u32 = 0x008002e6;

/// Native names shared by symbol allocation and callable aliases.
const MEMBERS: &[(&str, u32)] = &[
    ("Object", OBJECT),
    ("initialize", INITIALIZE),
    ("println", PRINTLN),
    ("exit", EXIT),
    ("toString", TO_STRING),
    ("format", FORMAT),
    ("abs", ABS),
    ("equals", EQUALS),
    ("hashCode", HASH_CODE),
    ("weak", WEAK),
    ("stillAlive", STILL_ALIVE),
    ("method", METHOD),
    ("invoke", INVOKE),
    ("compareTo", COMPARE_TO),
    ("length", LENGTH),
    ("substring", SUBSTRING),
    ("find", FIND),
    ("toNumber", TO_NUMBER),
    ("toLong", TO_LONG),
    ("toFloat", TO_FLOAT),
    ("toDouble", TO_DOUBLE),
    ("toLongWithBase", TO_LONG_WITH_BASE),
    ("toChar", TO_CHAR),
    ("toCharArray", TO_CHAR_ARRAY),
    ("toUtf8Array", TO_UTF8_ARRAY),
    ("size", SIZE),
    ("isEmpty", IS_EMPTY),
    ("get", DICTIONARY_GET),
    ("put", DICTIONARY_PUT),
    ("hasKey", HAS_KEY),
    ("keys", DICTIONARY_KEYS),
    ("values", DICTIONARY_VALUES),
    ("indexOf", INDEX_OF),
    ("add", ARRAY_ADD),
    ("addAll", ARRAY_ADD_ALL),
    ("remove", ARRAY_REMOVE),
    ("removeAll", ARRAY_REMOVE_ALL),
    ("reverse", REVERSE),
    ("slice", SLICE),
    ("sort", SORT),
    ("compare", COMPARE),
    ("toNumberWithBase", TO_NUMBER_WITH_BASE),
    ("toLower", TO_LOWER),
    ("toUpper", TO_UPPER),
    ("decodeNumber", 0x00800a25),
    ("encodeNumber", 0x00800a26),
    ("offset", 0x00800aa9),
    ("endianness", 0x00800aae),
    ("View", VIEW),
    ("BehaviorDelegate", BEHAVIOR_DELEGATE),
    ("onSelect", ON_SELECT),
    ("requestUpdate", REQUEST_UPDATE),
    ("AppBase", APP_BASE),
    ("onStart", ON_START),
    ("onStop", ON_STOP),
    ("getInitialView", GET_INITIAL_VIEW),
    ("onShow", 0x00800203),
    ("onUpdate", 0x00800204),
    ("onHide", 0x00800205),
    ("setColor", 0x0080020b),
    ("clear", 0x00800239),
    ("drawText", 0x00800246),
];

/// Lang runtime class bindings shared by type tests and application generation.
const LANG_CLASSES: &[(&str, u32)] = &[
    ("Object", OBJECT),
    ("Number", 0x00800096),
    ("Long", 0x00800097),
    ("Float", 0x00800098),
    ("Double", 0x00800099),
    ("Boolean", 0x0080009a),
    ("Char", 0x008004bd),
    ("String", 0x0080009b),
    ("Array", 0x0080009e),
    ("Dictionary", 0x0080002a),
    ("Symbol", 0x0080009d),
    ("Method", 0x0080009f),
    ("WeakReference", 0x00800416),
    ("ByteArray", 0x008007a2),
];

/// Resolve a supported concrete Lang class, excluding interfaces and generic types.
pub(super) fn lang_class(path: &str) -> Option<u32> {
    let name = path.strip_prefix("Toybox.Lang.")?;
    LANG_CLASSES
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, id)| *id)
}

/// Return pinned native identities without allocating IDs for user declarations.
pub(super) fn member_ids() -> BTreeMap<String, u32> {
    MEMBERS
        .iter()
        .map(|(name, id)| ((*name).into(), *id))
        .collect()
}

/// Bind compiler aliases from the same identities used by generated metadata.
/// Extra unused aliases do not create symbols or DATA records.
pub(super) fn code_bindings() -> BTreeMap<String, u32> {
    let mut bindings = BTreeMap::from([
        ("__ciq_sdk_System".into(), SYSTEM),
        ("__ciq_sdk_Lang".into(), LANG),
        ("__ciq_sdk_WatchUi".into(), WATCH_UI),
        ("__ciq_source_globals".into(), GLOBALS),
        ("__ciq_member_println".into(), PRINTLN),
        ("__ciq_app_globals".into(), GLOBALS),
        ("__ciq_app_base".into(), APP_BASE),
        ("__ciq_app_init".into(), INIT),
        ("__ciq_app_initialize".into(), INITIALIZE),
        ("__ciq_app_watch_ui".into(), WATCH_UI),
        ("__ciq_app_view".into(), VIEW),
    ]);
    for (name, id) in MEMBERS {
        bindings.insert(format!("__ciq_native_{name}"), *id);
    }
    for (name, id) in LANG_CLASSES {
        bindings.insert(format!("__ciq_type_{name}"), *id);
    }
    bindings
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bindings_retain_unique_ids_and_catalogued_runtime_classes() {
        let ids = member_ids();
        assert_eq!(ids.len(), MEMBERS.len());
        let unique: std::collections::BTreeSet<_> = ids.values().collect();
        assert_eq!(unique.len(), ids.len());
        let aliases = code_bindings();
        for (name, id) in LANG_CLASSES {
            let path = format!("Toybox.Lang.{name}");
            assert!(crate::api::core().class(&path).is_some(), "{path}");
            assert_eq!(lang_class(&path), Some(*id));
            assert_eq!(aliases[&format!("__ciq_type_{name}")], *id);
        }
        for (name, id) in MEMBERS {
            assert_eq!(aliases[&format!("__ciq_native_{name}")], *id);
        }
        assert_eq!(lang_class("Toybox.Lang.Comparable"), None);
        assert_eq!(lang_class("Toybox.Lang.NumberFormat"), None);
    }
}
