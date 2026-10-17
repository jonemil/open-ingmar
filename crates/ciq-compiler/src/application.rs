//! Minimal console application metadata and lifecycle generation.
use crate::{CompilationUnit, ReturnType};
use ciq_assembly::{Item, Operand, Program};
use ciq_mc::Opcode;
use ciq_prg::{
    Prg, Section,
    data::{
        self, Fragment,
        classes::source::{Declaration, FieldDeclaration, Key, Reference},
    },
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

const STATICS: u32 = 0x00800002;
const TOYBOX: u32 = 0x00800003;
const SYSTEM: u32 = 0x00800005;
const WATCH_UI: u32 = 0x00800011;
const APPLICATION: u32 = 0x00800015;
const INIT: u32 = 0x00800018;
const INITIALIZE: u32 = 0x0080005e;
const TO_STRING: u32 = 0x008000af;
const FORMAT: u32 = 0x0080009c;
const ABS: u32 = 0x008000a6;
const EQUALS: u32 = 0x008000b3;
const HASH_CODE: u32 = 0x008000ad;
const WEAK: u32 = 0x00800418;
const COMPARE_TO: u32 = 0x00800e50;
const LENGTH: u32 = 0x008000bb;
const SUBSTRING: u32 = 0x008000bd;
const FIND: u32 = 0x008000bc;
const TO_NUMBER: u32 = 0x008000a7;
const TO_FLOAT: u32 = 0x008000a5;
const TO_DOUBLE: u32 = 0x008000a8;
const TO_LONG: u32 = 0x008000a9;
const TO_LONG_WITH_BASE: u32 = 0x00800b1c;
const TO_CHAR: u32 = 0x008004c6;
const TO_CHAR_ARRAY: u32 = 0x008004c4;
const TO_UTF8_ARRAY: u32 = 0x008004c5;
const SIZE: u32 = 0x008000ae;
const DICTIONARY_PUT: u32 = 0x008000b4;
const DICTIONARY_GET: u32 = 0x008000b5;
const DICTIONARY_KEYS: u32 = 0x008000b9;
const DICTIONARY_VALUES: u32 = 0x008000ba;
const HAS_KEY: u32 = 0x008000b6;
const IS_EMPTY: u32 = 0x008000b8;
const INDEX_OF: u32 = 0x008004bf;
const ARRAY_ADD: u32 = 0x0080014e;
const ARRAY_ADD_ALL: u32 = 0x008004be;
const ARRAY_REMOVE: u32 = 0x008000b7;
const ARRAY_REMOVE_ALL: u32 = 0x008004c0;
const REVERSE: u32 = 0x008004c1;
const SLICE: u32 = 0x008004c2;
const TO_NUMBER_WITH_BASE: u32 = 0x008004c3;
const TO_LOWER: u32 = 0x008000be;
const TO_UPPER: u32 = 0x008000bf;
const PRINTLN: u32 = 0x00800067;
const EXIT: u32 = 0x0080006d;
const VIEW: u32 = 0x0080019f;
const BEHAVIOR_DELEGATE: u32 = 0x00800064;
const ON_SELECT: u32 = 0x008003f3;
const REQUEST_UPDATE: u32 = 0x008001a2;
const APP_BASE: u32 = 0x0080029c;
const ON_START: u32 = 0x0080029f;
const ON_STOP: u32 = 0x008002a1;
const GET_INITIAL_VIEW: u32 = 0x008002a0;
const GLOBALS: u32 = 0x008002e6;

/// Application identity and a startup entry or source native onStart callback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsoleApplication {
    /// Sixteen manifest application-ID bytes, independent of signing identity.
    pub id: [u8; 16],
    /// Global function name, or Class.method for a declared AppBase class.
    /// Invoked once by onStart; integer results are printed, void discarded.
    /// Select Class.onStart when that source callback exists; its state is
    /// supplied by the runtime and its result contract is void.
    pub entry: String,
}

/// An unsupported application configuration or metadata/encoding failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationError {
    /// Failed bootstrap-profile or encoding requirement.
    pub message: String,
}
impl fmt::Display for ApplicationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}
impl std::error::Error for ApplicationError {}
fn error(e: impl fmt::Display) -> ApplicationError {
    ApplicationError {
        message: e.to_string(),
    }
}

fn instruction(opcode: Opcode, operands: Vec<Operand>) -> Item {
    Item::Instruction { opcode, operands }
}
fn sym(name: &str) -> Operand {
    Operand::Symbol(name.into())
}
fn bits(value: u64) -> Operand {
    Operand::Bits(value)
}
fn field(id: u32, flags: u8, kind: u8, value: Reference) -> FieldDeclaration {
    FieldDeclaration {
        key: Key::Raw((id << 8) | u32::from(flags)),
        kind,
        value,
    }
}
fn method(id: u32, flags: u8, label: &str) -> FieldDeclaration {
    field(id, flags, 6, Reference::Code(label.into()))
}
fn class(
    alias: &str,
    references: [Option<Reference>; 4],
    mut fields: Vec<FieldDeclaration>,
) -> Declaration {
    // Runtime lookup failed for an out-of-order global method in the probe.
    fields.sort_by_key(|f| match f.key {
        Key::Raw(key) => key,
        _ => unreachable!(),
    });
    Declaration {
        alias: Some(alias.into()),
        at: None,
        version: 2,
        references,
        app_flags: 0x7f,
        fields,
    }
}

fn lifecycle(
    entry: &str,
    result: ReturnType,
    instance: bool,
    source_start: bool,
    source_initial_view: bool,
) -> Vec<Item> {
    use Opcode::*;
    let mut items = vec![
        Item::Label("__ciq_bootstrap".into()),
        instruction(Argc, vec![bits(1)]),
        instruction(Return, vec![]),
        Item::Label("__ciq_noop".into()),
        instruction(Return, vec![]),
        Item::Label("__ciq_constructor".into()),
        instruction(Getselfv, vec![sym("__ciq_app_base")]),
        instruction(Getsv, vec![sym("__ciq_app_init")]),
        instruction(Invokemz, vec![]),
        instruction(Return, vec![]),
    ];
    if !source_start {
        items.push(Item::Label("__ciq_on_start".into()));
        items.push(instruction(Argc, vec![bits(2)]));
        if result != ReturnType::Void {
            items.push(instruction(
                Getmv,
                vec![sym("__ciq_sdk_System"), sym("__ciq_member_println")],
            ));
            items.push(instruction(Frpush, vec![]));
        }
        let member = sym(&format!("__ciq_member_{entry}"));
        items.push(if instance {
            instruction(Getselfv, vec![member])
        } else {
            instruction(Getmv, vec![sym("__ciq_app_globals"), member])
        });
        items.push(instruction(Invokemz, vec![]));
        if result != ReturnType::Void {
            items.push(instruction(Invokem, vec![bits(2)]));
        }
        items.push(instruction(Popv, vec![]));
        items.push(instruction(Return, vec![]));
    }
    if !source_initial_view {
        items.extend([
            Item::Label("__ciq_initial_view".into()),
            instruction(Argc, vec![bits(1)]),
            instruction(Ipush1, vec![bits(1)]),
            instruction(Newa, vec![]),
            instruction(Dup, vec![bits(0)]),
            instruction(Ipushz, vec![]),
            instruction(
                Getmv,
                vec![sym("__ciq_app_watch_ui"), sym("__ciq_app_view")],
            ),
            instruction(Newc, vec![]),
            instruction(Dup, vec![bits(0)]),
            instruction(Getsv, vec![sym("__ciq_app_initialize")]),
            instruction(Dup, vec![bits(1)]),
            instruction(Invokem, vec![bits(1)]),
            instruction(Popv, vec![]),
            instruction(Aputv, vec![]),
            instruction(Return, vec![]),
        ]);
    }
    items.extend([
        Item::Label("__ciq_initialize".into()),
        instruction(Argc, vec![bits(1)]),
        instruction(Getselfv, vec![sym("__ciq_app_base")]),
        instruction(Getsv, vec![sym("__ciq_app_initialize")]),
        instruction(Invokemz, vec![]),
        instruction(Popv, vec![]),
        instruction(Return, vec![]),
    ]);
    items
}

/// Generate an unsigned, template-free console watch application from unit IR.
///
/// Uses the bootstrap profile tested with SDK 9.2.0 and Forerunner 265: version-2
/// DATA records, fixed native symbol IDs, native-class imports, generated
/// lifecycle methods, and empty resources/permissions/exceptions. The entry must
/// be a zero-parameter integer-return or void global function, or a method in
/// the sole declared AppBase class, alongside optional source View and
/// BehaviorDelegate subclasses.
/// Class entries use Class.method names and current-receiver lookup.
/// onStart invokes it once and prints only integer
/// results; getInitialView returns a blank native View unless a validated source
/// callback supplies a View or View/delegate positional result. Source callbacks that
/// collide with generated lifecycle/base fields are rejected, except a validated
/// source initialize method, which replaces the default initializer's DATA target.
/// A validated source onStart also replaces its wrapper and requires selecting
/// Class.onStart; the runtime supplies its nullable Dictionary argument directly.
/// A source onStop is bound as the native shutdown callback with the same
/// Dictionary-or-null parameter and Void result; it is not a bootstrap entry.
/// Source getInitialView has no parameters, returns a supported View tuple shape,
/// and replaces the generated callback's DATA field and CODE body. Callback
/// shape validation happens in unit IR, including for resumed text input.
///
/// Allocates user symbols and all CODE/DATA positions deterministically. Global,
/// module, and class methods bind to unit CODE labels; literals use shared DATA layout.
/// Rejects unsupported entry signatures, reserved namespace collisions, and
/// encoding/layout limits. Device budgets and other API/device profiles are not
/// inferred. No SDK files, original PRG, debug tables, or signing key are read.
/// Signing remains a separate operation on the returned complete PRG.
pub fn build_console_application(
    unit: &CompilationUnit,
    app: &ConsoleApplication,
) -> Result<Prg, ApplicationError> {
    let app_classes = unit
        .classes()
        .iter()
        .filter(|class| class.base == crate::ClassBase::ApplicationAppBase)
        .collect::<Vec<_>>();
    if !unit.classes().is_empty() && app_classes.len() != 1 {
        return Err(error(
            "application profile requires exactly one AppBase class",
        ));
    }
    let app_class = app_classes.first().copied();
    let (entry, instance) = if unit.classes().is_empty() {
        let Some(entry) = unit
            .declarations()
            .iter()
            .find(|d| d.module.is_none() && d.function.name() == app.entry)
        else {
            return Err(error("application entry must name a global function"));
        };
        (&entry.function, false)
    } else {
        let class = app_class.unwrap();
        let Some((owner, member)) = app.entry.split_once('.') else {
            return Err(error("class application entry must use Class.method"));
        };
        if owner != class.name {
            return Err(error(
                "application entry must select the declared AppBase class",
            ));
        }
        let Some(entry) = class.methods.iter().find(|f| f.name() == member) else {
            return Err(error("application entry method is not declared"));
        };
        if class.methods.iter().any(|f| f.name() == "AppBase") {
            return Err(error(
                "source methods collide with generated lifecycle or base fields",
            ));
        }
        (entry, true)
    };
    if matches!(
        entry.return_type(),
        ReturnType::NullableInteger
            | ReturnType::NullableString
            | ReturnType::NullableLong
            | ReturnType::NullableDouble
            | ReturnType::NullableFloat
            | ReturnType::NullableBoolean
            | ReturnType::NullableChar
            | ReturnType::TypedDictionary(_)
            | ReturnType::Array(_)
            | ReturnType::Object(_)
            | ReturnType::Tuple(_)
    ) {
        return Err(error(
            "console entry requires a non-null scalar or Void result",
        ));
    }
    let source_start = instance
        && app_class
            .unwrap()
            .methods
            .iter()
            .any(|f| f.name() == "onStart");
    if source_start && entry.name() != "onStart" {
        return Err(error(
            "a source onStart requires selecting Class.onStart as the entry",
        ));
    }
    if !source_start && entry.parameter_count() != 0 {
        return Err(error("application entry must have zero parameters"));
    }
    if instance && entry.name() == "initialize" {
        return Err(error("initialize cannot be selected as the startup entry"));
    }
    let modules: BTreeSet<_> = unit
        .declarations()
        .iter()
        .filter_map(|d| d.module.clone())
        .collect();
    if modules
        .iter()
        .any(|m| matches!(m.as_str(), "globals" | "statics" | "Toybox"))
        || unit
            .declarations()
            .iter()
            .any(|d| d.module.is_none() && d.function.name() == "Toybox")
    {
        return Err(error(
            "application declarations collide with a reserved bootstrap namespace",
        ));
    }
    let mut ids = BTreeMap::from([
        ("initialize".to_owned(), INITIALIZE),
        ("println".into(), PRINTLN),
        ("exit".into(), EXIT),
        ("toString".into(), TO_STRING),
        ("format".into(), FORMAT),
        ("abs".into(), ABS),
        ("equals".into(), EQUALS),
        ("hashCode".into(), HASH_CODE),
        ("weak".into(), WEAK),
        ("compareTo".into(), COMPARE_TO),
        ("length".into(), LENGTH),
        ("substring".into(), SUBSTRING),
        ("find".into(), FIND),
        ("toNumber".into(), TO_NUMBER),
        ("toLong".into(), TO_LONG),
        ("toFloat".into(), TO_FLOAT),
        ("toDouble".into(), TO_DOUBLE),
        ("toLongWithBase".into(), TO_LONG_WITH_BASE),
        ("toChar".into(), TO_CHAR),
        ("toCharArray".into(), TO_CHAR_ARRAY),
        ("toUtf8Array".into(), TO_UTF8_ARRAY),
        ("size".into(), SIZE),
        ("isEmpty".into(), IS_EMPTY),
        ("get".into(), DICTIONARY_GET),
        ("put".into(), DICTIONARY_PUT),
        ("hasKey".into(), HAS_KEY),
        ("keys".into(), DICTIONARY_KEYS),
        ("values".into(), DICTIONARY_VALUES),
        ("indexOf".into(), INDEX_OF),
        ("add".into(), ARRAY_ADD),
        ("addAll".into(), ARRAY_ADD_ALL),
        ("remove".into(), ARRAY_REMOVE),
        ("removeAll".into(), ARRAY_REMOVE_ALL),
        ("reverse".into(), REVERSE),
        ("slice".into(), SLICE),
        ("toNumberWithBase".into(), TO_NUMBER_WITH_BASE),
        ("toLower".into(), TO_LOWER),
        ("toUpper".into(), TO_UPPER),
        ("View".into(), VIEW),
        ("BehaviorDelegate".into(), BEHAVIOR_DELEGATE),
        ("onSelect".into(), ON_SELECT),
        ("requestUpdate".into(), REQUEST_UPDATE),
        ("AppBase".into(), APP_BASE),
        ("onStart".into(), ON_START),
        ("onStop".into(), ON_STOP),
        ("getInitialView".into(), GET_INITIAL_VIEW),
        ("onShow".into(), 0x00800203),
        ("onUpdate".into(), 0x00800204),
        ("onHide".into(), 0x00800205),
        ("setColor".into(), 0x0080020b),
        ("clear".into(), 0x00800239),
        ("drawText".into(), 0x00800246),
    ]);
    for class in unit.classes() {
        if ids.contains_key(&class.name) {
            return Err(error(
                "source class name collides with a pinned native symbol",
            ));
        }
        if class.base == crate::ClassBase::ApplicationAppBase {
            ids.insert(class.name.clone(), 1);
        }
    }
    let names: BTreeSet<_> = unit
        .declarations()
        .iter()
        .map(|d| d.function.name().to_owned())
        .chain(
            unit.classes()
                .iter()
                .flat_map(|c| c.methods.iter().map(|f| f.name().to_owned())),
        )
        .chain(modules.iter().cloned())
        .chain(unit.classes().iter().map(|class| class.name.clone()))
        .chain(
            unit.classes()
                .iter()
                .flat_map(|class| class.fields.iter().map(|field| field.name.clone())),
        )
        .collect();
    let mut next = 2; // Symbol 1 is the synthesized application class.
    for name in names {
        ids.entry(name).or_insert_with(|| {
            let id = next;
            next += 1;
            id
        });
    }
    let mut module_ids = BTreeMap::new();
    for module in &modules {
        module_ids.insert(module.clone(), next);
        next += 1;
    }
    let mut bindings = BTreeMap::from([
        ("__ciq_sdk_System".into(), SYSTEM),
        ("__ciq_sdk_WatchUi".into(), WATCH_UI),
        ("__ciq_native_View".into(), VIEW),
        ("__ciq_native_BehaviorDelegate".into(), BEHAVIOR_DELEGATE),
        ("__ciq_native_requestUpdate".into(), REQUEST_UPDATE),
        ("__ciq_source_globals".into(), GLOBALS),
        ("__ciq_native_setColor".into(), 0x0080020b),
        ("__ciq_native_clear".into(), 0x00800239),
        ("__ciq_native_drawText".into(), 0x00800246),
        ("__ciq_native_AppBase".into(), APP_BASE),
        ("__ciq_native_initialize".into(), INITIALIZE),
        ("__ciq_member_println".into(), PRINTLN),
        ("__ciq_native_exit".into(), EXIT),
        ("__ciq_native_toString".into(), TO_STRING),
        ("__ciq_native_format".into(), FORMAT),
        ("__ciq_native_abs".into(), ABS),
        ("__ciq_native_equals".into(), EQUALS),
        ("__ciq_native_hashCode".into(), HASH_CODE),
        ("__ciq_native_weak".into(), WEAK),
        ("__ciq_native_compareTo".into(), COMPARE_TO),
        ("__ciq_native_length".into(), LENGTH),
        ("__ciq_native_substring".into(), SUBSTRING),
        ("__ciq_native_find".into(), FIND),
        ("__ciq_native_toNumber".into(), TO_NUMBER),
        ("__ciq_native_toLong".into(), TO_LONG),
        ("__ciq_native_toFloat".into(), TO_FLOAT),
        ("__ciq_native_toDouble".into(), TO_DOUBLE),
        ("__ciq_native_toLongWithBase".into(), TO_LONG_WITH_BASE),
        ("__ciq_native_toChar".into(), TO_CHAR),
        ("__ciq_native_toCharArray".into(), TO_CHAR_ARRAY),
        ("__ciq_native_toUtf8Array".into(), TO_UTF8_ARRAY),
        ("__ciq_native_size".into(), SIZE),
        ("__ciq_native_isEmpty".into(), IS_EMPTY),
        ("__ciq_native_get".into(), DICTIONARY_GET),
        ("__ciq_native_put".into(), DICTIONARY_PUT),
        ("__ciq_native_hasKey".into(), HAS_KEY),
        ("__ciq_native_keys".into(), DICTIONARY_KEYS),
        ("__ciq_native_values".into(), DICTIONARY_VALUES),
        ("__ciq_native_indexOf".into(), INDEX_OF),
        ("__ciq_native_add".into(), ARRAY_ADD),
        ("__ciq_native_addAll".into(), ARRAY_ADD_ALL),
        ("__ciq_native_remove".into(), ARRAY_REMOVE),
        ("__ciq_native_removeAll".into(), ARRAY_REMOVE_ALL),
        ("__ciq_native_reverse".into(), REVERSE),
        ("__ciq_native_slice".into(), SLICE),
        ("__ciq_native_toNumberWithBase".into(), TO_NUMBER_WITH_BASE),
        ("__ciq_native_toLower".into(), TO_LOWER),
        ("__ciq_native_toUpper".into(), TO_UPPER),
        ("__ciq_app_globals".into(), GLOBALS),
        ("__ciq_app_base".into(), APP_BASE),
        ("__ciq_app_init".into(), INIT),
        ("__ciq_app_initialize".into(), INITIALIZE),
        ("__ciq_app_watch_ui".into(), WATCH_UI),
        ("__ciq_app_view".into(), VIEW),
    ]);
    for function in unit
        .declarations()
        .iter()
        .map(|d| &d.function)
        .chain(unit.classes().iter().flat_map(|c| &c.methods))
    {
        bindings.insert(
            format!("__ciq_member_{}", function.name()),
            ids[function.name()],
        );
    }
    for (module, id) in &module_ids {
        bindings.insert(format!("__ciq_module_{module}"), *id);
    }
    for class in unit.classes() {
        bindings.insert(format!("__ciq_class_{}", class.name), ids[&class.name]);
        for field in &class.fields {
            bindings.insert(format!("__ciq_field_{}", field.name), ids[&field.name]);
        }
    }
    let source_initial_view = instance
        && app_class
            .unwrap()
            .methods
            .iter()
            .any(|f| f.name() == "getInitialView");
    let mut items = lifecycle(
        entry.name(),
        entry.return_type(),
        instance,
        source_start,
        source_initial_view,
    );
    for source_class in unit
        .classes()
        .iter()
        .filter(|class| class.base.constructed_class().is_some())
    {
        items.extend([
            Item::Label(format!("__ciq_ctor_{}", source_class.name)),
            instruction(
                Opcode::Getselfv,
                vec![sym(&format!(
                    "__ciq_native_{}",
                    source_class.base.api_name().rsplit('.').next().unwrap()
                ))],
            ),
            instruction(Opcode::Getsv, vec![sym("__ciq_app_init")]),
            instruction(Opcode::Invokemz, vec![]),
            instruction(Opcode::Return, vec![]),
        ]);
    }
    items.extend(unit.lower().items().iter().cloned());
    let assembly = Program::new(items).map_err(error)?;
    let labels = assembly.labels().map_err(error)?;
    let module_alias = |name: &str| format!("__ciq_data_module_{}_{}", name.len(), name);
    let mut statics_fields = vec![field(GLOBALS, 0, 10, Reference::Raw(0))];
    for id in module_ids.values() {
        statics_fields.push(field(*id, 0, 10, Reference::Raw(0)));
    }
    let statics = class(
        "__ciq_data_statics",
        [None, None, None, Some(Reference::Raw(STATICS))],
        statics_fields,
    );
    let mut global_fields = vec![
        field(1, 0x50, 7, Reference::Class("__ciq_data_app".into())),
        field(TOYBOX, 0, 0, Reference::Raw(0)),
        method(INIT, 0x30, "__ciq_noop"),
    ];
    for d in unit.declarations().iter().filter(|d| d.module.is_none()) {
        global_fields.push(method(ids[d.function.name()], 0x10, &d.code_label()));
    }
    for module in &modules {
        global_fields.push(field(
            ids[module],
            0x50,
            10,
            Reference::Raw(module_ids[module]),
        ));
    }
    for source_class in unit
        .classes()
        .iter()
        .filter(|class| class.base.constructed_class().is_some())
    {
        global_fields.push(field(
            ids[&source_class.name],
            0x50,
            7,
            Reference::Class(format!("__ciq_data_class_{}", source_class.name)),
        ));
    }
    let globals = class(
        "__ciq_data_globals",
        [None, None, None, Some(Reference::Raw(GLOBALS))],
        global_fields,
    );
    let mut application = class(
        "__ciq_data_app",
        [
            Some(Reference::Raw(0x40000000)),
            None,
            Some(Reference::Raw(GLOBALS)),
            None,
        ],
        vec![
            method(INIT, 0x30, "__ciq_constructor"),
            method(INITIALIZE, 0x10, "__ciq_initialize"),
            field(APP_BASE, 0x70, 7, Reference::Raw(0x40000000)),
            method(ON_START, 0x10, "__ciq_on_start"),
            method(GET_INITIAL_VIEW, 0x10, "__ciq_initial_view"),
        ],
    );
    if let Some(source_class) = app_class {
        application
            .fields
            .extend(source_class.fields.iter().map(|declaration| {
                field(
                    ids[&declaration.name],
                    0,
                    if matches!(declaration.kind, crate::FieldKind::Number(_)) {
                        1
                    } else {
                        0
                    },
                    Reference::Raw(match declaration.kind {
                        crate::FieldKind::Number(initial) => initial as u32,
                        crate::FieldKind::SourceObject(_) => 0,
                    }),
                )
            }));
        if source_class
            .methods
            .iter()
            .any(|f| f.name() == "getInitialView")
        {
            application
                .fields
                .retain(|f| f.key != Key::Raw((GET_INITIAL_VIEW << 8) | 0x10));
        }
        if source_class.methods.iter().any(|f| f.name() == "onStart") {
            application
                .fields
                .retain(|f| f.key != Key::Raw((ON_START << 8) | 0x10));
        }
        if source_class
            .methods
            .iter()
            .any(|f| f.name() == "initialize")
        {
            application
                .fields
                .retain(|f| f.key != Key::Raw((INITIALIZE << 8) | 0x10));
        }
        application.fields.extend(
            source_class
                .methods
                .iter()
                .map(|f| method(ids[f.name()], 0x10, &source_class.code_label(f.name()))),
        );
        application.fields.sort_by_key(|f| match f.key {
            Key::Raw(key) => key,
            _ => unreachable!(),
        });
    }
    let mut classes = vec![statics, globals, application];
    for source_class in unit
        .classes()
        .iter()
        .filter(|class| class.base.constructed_class().is_some())
    {
        let ctor_label = format!("__ciq_ctor_{}", source_class.name);
        let (base_symbol, import_word) = match source_class.base {
            crate::ClassBase::WatchUiView => (VIEW, 0x40000001),
            crate::ClassBase::WatchUiBehaviorDelegate => (BEHAVIOR_DELEGATE, 0x40000002),
            crate::ClassBase::ApplicationAppBase => unreachable!("AppBase emitted separately"),
        };
        let mut fields = vec![
            method(INIT, 0x30, &ctor_label),
            field(base_symbol, 0x70, 7, Reference::Raw(import_word)),
        ];
        fields.extend(source_class.fields.iter().map(|declaration| {
            field(
                ids[&declaration.name],
                0,
                if matches!(declaration.kind, crate::FieldKind::Number(_)) {
                    1
                } else {
                    0
                },
                Reference::Raw(match declaration.kind {
                    crate::FieldKind::Number(initial) => initial as u32,
                    crate::FieldKind::SourceObject(_) => 0,
                }),
            )
        }));
        fields.extend(
            source_class
                .methods
                .iter()
                .map(|f| method(ids[f.name()], 0x10, &source_class.code_label(f.name()))),
        );
        fields.sort_by_key(|field| match field.key {
            Key::Raw(key) => key,
            _ => unreachable!(),
        });
        classes.push(class(
            &format!("__ciq_data_class_{}", source_class.name),
            [
                Some(Reference::Raw(import_word)),
                None,
                Some(Reference::Raw(GLOBALS)),
                None,
            ],
            fields,
        ));
    }
    for module in &modules {
        let mut fields = vec![method(INIT, 0x30, "__ciq_noop")];
        for d in unit
            .declarations()
            .iter()
            .filter(|d| d.module.as_ref() == Some(module))
        {
            fields.push(method(ids[d.function.name()], 0x10, &d.code_label()));
        }
        classes.push(class(
            &module_alias(module),
            [
                None,
                None,
                Some(Reference::Raw(GLOBALS)),
                Some(Reference::Raw(module_ids[module])),
            ],
            fields,
        ));
    }
    // Type-0a module fields require numeric DATA offsets in the shared model.
    let mut positions = BTreeMap::new();
    let mut offset = 0u32;
    for c in &mut classes {
        c.at = Some(offset);
        positions.insert(c.alias.clone().unwrap(), offset);
        offset = offset
            .checked_add(u32::try_from(c.encoded_length().map_err(error)?).map_err(error)?)
            .ok_or_else(|| error("DATA layout exceeds u32"))?;
    }
    for f in &mut classes[0].fields {
        let Key::Raw(key) = f.key else { unreachable!() };
        let id = key >> 8;
        let alias = if id == GLOBALS {
            "__ciq_data_globals".to_owned()
        } else {
            module_alias(module_ids.iter().find(|(_, v)| **v == id).unwrap().0)
        };
        f.value = Reference::Raw(positions[&alias]);
    }
    let mut fragments = classes.into_iter().map(Fragment::Class).collect::<Vec<_>>();
    fragments.extend(assembly.items().iter().filter_map(|i| match i {
        Item::String(s) => Some(Fragment::String(s.clone())),
        Item::Array(a) => Some(Fragment::Array(a.clone())),
        Item::Dictionary(d) => Some(Fragment::Dictionary(d.clone())),
        _ => None,
    }));
    let data = data::Source { fragments };
    let strings = data.string_bindings().map_err(error)?;
    let arrays = data.array_bindings().map_err(error)?;
    let dictionaries = data.dictionary_bindings().map_err(error)?;
    let code = assembly
        .encode_with_data_references(&bindings, &strings, &arrays, &dictionaries)
        .map_err(error)?;
    let data = data
        .encode_with_references(&BTreeMap::new(), &labels)
        .map_err(error)?
        .0;
    let mut header = 0x00060002u32.to_be_bytes().to_vec();
    header.resize(33, 0);
    let mut entry_points = 1u16.to_be_bytes().to_vec();
    entry_points.extend(app.id);
    for word in [GLOBALS, 1, 0x82, 0x26, 1] {
        entry_points.extend(word.to_be_bytes());
    }
    let has_delegate = unit
        .classes()
        .iter()
        .any(|class| class.base == crate::ClassBase::WatchUiBehaviorDelegate);
    let mut imports = (if has_delegate { 3u16 } else { 2u16 })
        .to_be_bytes()
        .to_vec();
    for word in [APPLICATION, APP_BASE, WATCH_UI, VIEW] {
        imports.extend(word.to_be_bytes());
    }
    if has_delegate {
        imports.extend(WATCH_UI.to_be_bytes());
        imports.extend(BEHAVIOR_DELEGATE.to_be_bytes());
    }
    let mut sections = vec![
        (0xd000d00d, header),
        (0x6060c0de, entry_points),
        (ciq_prg::DATA, data),
        (ciq_prg::CODE, code),
        (0xc1a557b1, imports),
        (ciq_prg::RESOURCES, vec![0, 0]),
        (0x6000db01, vec![0, 0]),
        (ciq_prg::EXCEPTIONS, vec![0, 0]),
    ];
    // Deterministic trailing resource padding keeps the unsigned stream aligned.
    let length = sections.iter().map(|(_, b)| 8 + b.len()).sum::<usize>();
    sections[5].1.resize(2 + ((32 - length % 32) % 32), 0);
    Ok(Prg::from_sections(
        sections
            .into_iter()
            .map(|(kind, payload)| Section::new(kind, payload).map_err(error))
            .collect::<Result<_, _>>()?,
    ))
}
