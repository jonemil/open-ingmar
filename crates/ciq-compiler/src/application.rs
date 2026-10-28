//! Minimal console application metadata and lifecycle generation.
use crate::native_bindings::{
    APP_BASE, APPLICATION, BEHAVIOR_DELEGATE, GET_INITIAL_VIEW, GLOBALS, INIT, INITIALIZE, LANG,
    OBJECT, ON_START, STATICS, TOYBOX, VIEW, WATCH_UI,
};
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

/// Encode a field's literal default using the application's shared Symbol IDs.
/// Fields requiring source initialization and nullable defaults use Null slots.
fn instance_field(
    declaration: &crate::InstanceField,
    ids: &BTreeMap<String, u32>,
) -> FieldDeclaration {
    let (kind, value) = match &declaration.kind {
        crate::FieldKind::Number(initial) => (1, *initial as u32),
        crate::FieldKind::Symbol {
            initial: Some(name),
            ..
        } => (8, ids[name]),
        crate::FieldKind::Constant { .. }
        | crate::FieldKind::Value { .. }
        | crate::FieldKind::Symbol { initial: None, .. }
        | crate::FieldKind::Numeric { .. }
        | crate::FieldKind::Scalar { .. }
        | crate::FieldKind::LangObject { .. }
        | crate::FieldKind::Array { .. }
        | crate::FieldKind::Dictionary { .. }
        | crate::FieldKind::SourceObject(_)
        | crate::FieldKind::NullableSourceObject(_)
        | crate::FieldKind::Method { .. }
        | crate::FieldKind::WeakReference { .. }
        | crate::FieldKind::ByteArray { .. }
        | crate::FieldKind::Interface { .. } => (0, 0),
    };
    field(ids[&declaration.name], 0, kind, Reference::Raw(value))
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
    if !matches!(
        entry.return_type(),
        ReturnType::Any
            | ReturnType::Integer
            | ReturnType::Long
            | ReturnType::Float
            | ReturnType::Double
            | ReturnType::Numeric
            | ReturnType::IntegerAlias
            | ReturnType::Decimal
            | ReturnType::String
            | ReturnType::Boolean
            | ReturnType::Char
            | ReturnType::Void
    ) {
        return Err(error(
            "console entry requires a non-null scalar or Void result, or Any",
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
    if (instance && entry.name() == "initialize")
        || entry.name().starts_with("__ciq_initialize_shared_")
    {
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
    let mut ids = crate::native_bindings::member_ids();
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
        .chain(unit.shared_constants().iter().map(|c| c.name.clone()))
        .chain(modules.iter().cloned())
        .chain(unit.classes().iter().map(|class| class.name.clone()))
        .chain(
            unit.declarations()
                .iter()
                .map(|d| &d.function)
                .chain(unit.classes().iter().flat_map(|c| &c.methods))
                .flat_map(|f| f.operations())
                .filter_map(|op| {
                    if let crate::Operation::Symbol { name, .. }
                    | crate::Operation::MemberRead { member: name, .. }
                    | crate::Operation::MemberWrite { member: name, .. }
                    | crate::Operation::NativeCall { member: name, .. }
                    | crate::Operation::NativeCallStatement { member: name, .. } = &op.operation
                    {
                        Some(name.clone())
                    } else {
                        None
                    }
                }),
        )
        .chain(
            unit.classes()
                .iter()
                .flat_map(|class| class.fields.iter().map(|field| field.name.clone())),
        )
        .chain(
            unit.classes()
                .iter()
                .flat_map(|class| &class.fields)
                .filter_map(|field| field.kind.symbol_default().map(str::to_owned)),
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
    let mut bindings = crate::native_bindings::code_bindings();

    for (name, id) in &ids {
        bindings.insert(format!("__ciq_member_{name}"), *id);
        bindings.insert(format!("__ciq_native_{name}"), *id);
    }
    for (module, id) in &module_ids {
        bindings.insert(format!("__ciq_module_{module}"), *id);
    }
    let literal_names: BTreeSet<_> = unit
        .declarations()
        .iter()
        .map(|d| &d.function)
        .chain(unit.classes().iter().flat_map(|c| &c.methods))
        .flat_map(|f| f.operations())
        .filter_map(|op| {
            if let crate::Operation::Symbol { name, .. } = &op.operation {
                Some(name.clone())
            } else {
                None
            }
        })
        .chain(
            unit.classes()
                .iter()
                .flat_map(|class| &class.fields)
                .filter_map(|field| field.kind.symbol_default().map(str::to_owned)),
        )
        .collect();
    for name in &literal_names {
        bindings.insert(format!("__ciq_symbol_{name}"), ids[name]);
    }
    for class in unit.classes() {
        bindings.insert(format!("__ciq_class_{}", class.name), ids[&class.name]);
        for field in &class.fields {
            bindings.insert(format!("__ciq_field_{}", field.name), ids[&field.name]);
        }
    }
    for constant in unit.shared_constants() {
        bindings.insert(
            format!("__ciq_field_{}", constant.name),
            ids[&constant.name],
        );
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
        .filter(|class| class.base.constructed_type().is_some())
    {
        if source_class.base == crate::ClassBase::LangObject {
            items.extend([
                Item::Label(format!("__ciq_ctor_{}", source_class.name)),
                instruction(Opcode::Return, vec![]),
            ]);
            continue;
        }
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
    // Allocation hooks have no ordinary function prologue. Invoke the checked
    // field initializer through a reserved slot to give its locals a call frame.
    for source_class in unit.classes().iter().filter(|class| {
        class
            .methods
            .iter()
            .any(|f| f.name() == crate::unit::FIELD_INITIALIZER)
    }) {
        let ctor = if source_class.base == crate::ClassBase::ApplicationAppBase {
            "__ciq_constructor".to_owned()
        } else {
            format!("__ciq_ctor_{}", source_class.name)
        };
        let start = items
            .iter()
            .position(|item| matches!(item,Item::Label(label) if label==&ctor))
            .unwrap();
        let at = (start + 1..items.len())
            .find(|&at| {
                matches!(
                    &items[at],
                    Item::Instruction {
                        opcode: Opcode::Return,
                        ..
                    }
                )
            })
            .unwrap();
        let mut initialize = Vec::new();
        if source_class.base != crate::ClassBase::LangObject {
            initialize.push(instruction(Opcode::Popv, vec![]));
        }
        initialize.extend([
            instruction(
                Opcode::Getselfv,
                vec![sym(&format!(
                    "__ciq_member_{}",
                    crate::unit::FIELD_INITIALIZER
                ))],
            ),
            instruction(Opcode::Invokemz, vec![]),
            instruction(Opcode::Popv, vec![]),
            instruction(Opcode::Return, vec![]),
        ]);
        items.splice(at..=at, initialize);
    }
    let shared_hook = |module: Option<&str>| {
        module.map_or_else(
            || "__ciq_shared_globals".into(),
            |module| format!("__ciq_shared_module_{}_{}", module.len(), module),
        )
    };
    let shared_owners = unit
        .shared_constants()
        .iter()
        .map(|c| c.module.as_deref())
        .collect::<BTreeSet<_>>();
    for module in &shared_owners {
        items.push(Item::Label(shared_hook(*module)));
        for constant in unit
            .shared_constants()
            .iter()
            .filter(|c| c.module.as_deref() == *module)
        {
            items.extend([
                instruction(
                    Opcode::Getselfv,
                    vec![sym(&format!("__ciq_member_{}", constant.initializer()))],
                ),
                instruction(Opcode::Invokemz, vec![]),
                instruction(Opcode::Popv, vec![]),
            ]);
        }
        items.push(instruction(Opcode::Return, vec![]));
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
        method(
            INIT,
            0x30,
            if shared_owners.contains(&None) {
                "__ciq_shared_globals"
            } else {
                "__ciq_noop"
            },
        ),
    ];
    for constant in unit
        .shared_constants()
        .iter()
        .filter(|c| c.module.is_none())
    {
        global_fields.push(field(ids[&constant.name], 0, 0, Reference::Raw(0)));
    }
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
        .filter(|class| class.base.constructed_type().is_some())
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
        application.fields.extend(
            source_class
                .fields
                .iter()
                .map(|declaration| instance_field(declaration, &ids)),
        );
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
    let has_delegate = unit
        .classes()
        .iter()
        .any(|class| class.base == crate::ClassBase::WatchUiBehaviorDelegate);
    let has_object = unit
        .classes()
        .iter()
        .any(|class| class.base == crate::ClassBase::LangObject);
    let object_import = if has_delegate { 3 } else { 2 };
    let mut classes = vec![statics, globals, application];
    for source_class in unit
        .classes()
        .iter()
        .filter(|class| class.base.constructed_type().is_some())
    {
        let ctor_label = format!("__ciq_ctor_{}", source_class.name);
        let (base_symbol, import_word) = match source_class.base {
            crate::ClassBase::LangObject => (OBJECT, 0x40000000 | object_import),
            crate::ClassBase::WatchUiView => (VIEW, 0x40000001),
            crate::ClassBase::WatchUiBehaviorDelegate => (BEHAVIOR_DELEGATE, 0x40000002),
            crate::ClassBase::ApplicationAppBase => unreachable!("AppBase emitted separately"),
        };
        let mut fields = vec![
            method(INIT, 0x30, &ctor_label),
            field(base_symbol, 0x70, 7, Reference::Raw(import_word)),
        ];
        fields.extend(
            source_class
                .fields
                .iter()
                .map(|declaration| instance_field(declaration, &ids)),
        );
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
        let hook = if shared_owners.contains(&Some(module.as_str())) {
            shared_hook(Some(module))
        } else {
            "__ciq_noop".into()
        };
        let mut fields = vec![method(INIT, 0x30, &hook)];
        for constant in unit
            .shared_constants()
            .iter()
            .filter(|c| c.module.as_ref() == Some(module))
        {
            fields.push(field(ids[&constant.name], 0, 0, Reference::Raw(0)));
        }
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
        Item::ByteArray(a) => Some(Fragment::ByteArray(a.clone())),
        Item::Dictionary(d) => Some(Fragment::Dictionary(d.clone())),
        _ => None,
    }));
    let data = data::Source { fragments };
    let strings = data.string_bindings().map_err(error)?;
    let arrays = data.array_bindings().map_err(error)?;
    let dictionaries = data.dictionary_bindings().map_err(error)?;
    let byte_arrays = data.byte_array_bindings().map_err(error)?;
    let code = assembly
        .encode_with_byte_arrays(&bindings, &strings, &arrays, &dictionaries, &byte_arrays)
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
    let mut imports = ((if has_delegate { 3u16 } else { 2u16 }) + u16::from(has_object))
        .to_be_bytes()
        .to_vec();
    for word in [APPLICATION, APP_BASE, WATCH_UI, VIEW] {
        imports.extend(word.to_be_bytes());
    }
    if has_delegate {
        imports.extend(WATCH_UI.to_be_bytes());
        imports.extend(BEHAVIOR_DELEGATE.to_be_bytes());
    }
    if has_object {
        imports.extend(LANG.to_be_bytes());
        imports.extend(OBJECT.to_be_bytes());
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
    if !literal_names.is_empty() {
        let symbols = ciq_prg::symbols::source::Source {
            declarations: literal_names
                .iter()
                .map(|name| ciq_prg::symbols::source::Declaration {
                    alias: name.clone(),
                    name: name.clone(),
                    id: ids[name],
                    name_at: None,
                    entry_index: None,
                })
                .collect(),
        };
        sections.push((ciq_prg::SYMBOLS, symbols.encode().map_err(error)?));
    }
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
