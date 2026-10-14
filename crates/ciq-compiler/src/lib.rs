//! Restricted Monkey C frontend.
//!
//! [`compile`] parses Monkey C source and [`parse_ir`] parses saved IR text.
//! Both produce a validated [`Function`], which [`backend::lower`] converts
//! into typed VM assembly. Typed IR construction uses the same validation.
//! [`compile_sources`] collects global/module functions and instance classes
//! into a [`CompilationUnit`] and checks module/current-receiver calls.
//! [`parse_unit_ir`] resumes the same validated unit from saved text.
//! [`application::build_console_application`] adds generated lifecycle/metadata
//! and returns an unsigned PRG for the supported console bootstrap profile.

pub mod api;
pub mod application;
pub mod backend;
mod bindings;
mod control_flow;
pub use bindings::{string_bindings, symbol_bindings};
mod ir_text;
mod names;
mod native;
mod null_flow;
mod operators;
mod parser;
mod source_ir;
mod source_objects;
mod source_types;
mod unit;
pub use ir_text::{parse_ir, parse_ir_with_api};
pub use unit::{
    ClassBase, ClassDeclaration, CompilationUnit, FieldKind, FunctionDeclaration, InstanceField,
    SourceDiagnostic, UnitError, compile_sources, compile_sources_with_api, parse_unit_ir,
    parse_unit_ir_with_api,
};

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub offset: usize,
    pub line: usize,
    pub column: usize,
    pub message: String,
}

impl Diagnostic {
    fn at(source: &str, offset: usize, message: impl Into<String>) -> Self {
        let prefix = &source[..offset];
        Self {
            offset,
            line: prefix.bytes().filter(|b| *b == b'\n').count() + 1,
            column: prefix.rsplit('\n').next().unwrap().chars().count() + 1,
            message: message.into(),
        }
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}: {}", self.line, self.column, self.message)
    }
}

impl std::error::Error for Diagnostic {}

/// A semantic value identity, independent of VM stack slots and symbols.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ValueId(pub u32);

/// Semantic local identity; the backend independently assigns VM slots.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LocalId(pub u32);

/// A semantic block identity, independent of assembly labels and CODE offsets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BlockId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Comparisons producing branch conditions; equality also supports null checks.
pub enum ComparisonOp {
    Equal,
    Greater,
    Less,
    LessEqual,
    GreaterEqual,
    NotEqual,
}
impl ComparisonOp {
    pub(crate) fn is_equality(self) -> bool {
        matches!(self, Self::Equal | Self::NotEqual)
    }
}

pub(crate) const MAX_PARAMETERS: usize = 254;
pub(crate) const MAX_LOCALS: usize = 255;
pub(crate) const MAX_OPERATIONS: usize = 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
}

/// Native class identities supported as non-null object references.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeClass {
    View,
    BehaviorDelegate,
    Dc,
}

impl NativeClass {
    /// Return this class's canonical API identity, independent of symbol binding.
    pub fn name(self) -> &'static str {
        match self {
            Self::View => "Toybox.WatchUi.View",
            Self::BehaviorDelegate => "Toybox.WatchUi.BehaviorDelegate",
            Self::Dc => "Toybox.Graphics.Dc",
        }
    }
    fn from_name(name: &str) -> Option<Self> {
        [Self::View, Self::BehaviorDelegate, Self::Dc]
            .into_iter()
            .find(|class| class.name() == name)
    }
}

/// Supported invariant Array element contracts, independent of VM storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrayElement {
    Number,
    Char,
    Object(NativeClass),
}

impl ArrayElement {
    /// Return the native class type represented by this element contract.
    pub fn api_type(self) -> api::Type {
        api::Type::named(match self {
            Self::Number => "Toybox.Lang.Number",
            Self::Char => "Toybox.Lang.Char",
            Self::Object(class) => class.name(),
        })
    }

    fn from_api_type(ty: &api::Type) -> Option<Self> {
        [Self::Number, Self::Char, Self::Object(NativeClass::View)]
            .into_iter()
            .find(|element| element.api_type() == *ty)
    }

    fn text(self) -> &'static str {
        match self {
            Self::Number => "number",
            Self::Char => "char",
            Self::Object(NativeClass::View) => "view",
            Self::Object(NativeClass::Dc) => "dc",
            Self::Object(NativeClass::BehaviorDelegate) => "behavior-delegate",
        }
    }

    fn array_text(self) -> &'static str {
        match self {
            Self::Number => "number-array",
            Self::Char => "char-array",
            Self::Object(NativeClass::View) => "view-array",
            Self::Object(NativeClass::Dc) => "dc-array",
            Self::Object(NativeClass::BehaviorDelegate) => "behavior-delegate-array",
        }
    }
}

/// Supported positional native callback results, independent of runtime storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TupleShape {
    OneView,
    ViewDelegate,
}

impl TupleShape {
    /// Return the exact API slot types represented by this shape.
    pub fn api_type(self) -> api::Type {
        match self {
            Self::OneView => api::Type::Tuple(vec![api::Type::named(NativeClass::View.name())]),
            Self::ViewDelegate => api::Type::Tuple(vec![
                api::Type::named(NativeClass::View.name()),
                api::Type::named("Toybox.WatchUi.InputDelegate"),
            ]),
        }
    }
}

/// The supported function result contract, declared explicitly or inferred
/// from uniform supported value/Void returns.
/// Void functions produce no semantic value; parameter contracts are separate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReturnType {
    Integer,
    NullableInteger,
    String,
    NullableString,
    Boolean,
    Char,
    Object(NativeClass),
    /// Exact non-null source View subclass, resolved within a compilation unit.
    SourceObject(String),
    Tuple(TupleShape),
    /// Non-null Array with an invariant supported element contract.
    Array(ArrayElement),
    Void,
}

/// Supported explicit parameter contracts, independent of VM slot allocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParameterType {
    Integer,
    NullableInteger,
    String,
    NullableString,
    Boolean,
    Char,
    Object(NativeClass),
    /// Exact non-null source View subclass, resolved within a compilation unit.
    SourceObject(String),
    /// Non-null Array with an invariant supported element contract.
    Array(ArrayElement),
    /// An opaque native Dictionary or null; contents are not yet accessible.
    NullableDictionary,
}

impl ParameterType {
    /// Return the API type represented by this supported parameter contract.
    /// Source objects map to their native View base for API checks; exact source
    /// assignability is checked separately against unit declarations.
    pub fn api_type(&self) -> api::Type {
        match self {
            Self::NullableInteger => ValueType::NullableInteger.api_type(),
            Self::NullableString => ValueType::NullableString.api_type(),
            Self::Integer => ValueType::Integer.api_type(),
            Self::String => ValueType::String.api_type(),
            Self::Boolean => ValueType::Boolean.api_type(),
            Self::Char => ValueType::Char.api_type(),
            Self::Object(class) => ValueType::Object(*class).api_type(),
            Self::SourceObject(_) => ValueType::Object(NativeClass::View).api_type(),
            Self::Array(element) => ValueType::Array(*element).api_type(),
            Self::NullableDictionary => ValueType::NullableDictionary.api_type(),
        }
    }
}

impl ReturnType {
    /// Return the API type represented by this supported result contract.
    /// Source objects map to their native View base without erasing their
    /// separate source-class return requirement.
    pub fn api_type(&self) -> api::Type {
        match self {
            Self::NullableInteger => ValueType::NullableInteger.api_type(),
            Self::NullableString => ValueType::NullableString.api_type(),
            Self::Integer => ValueType::Integer.api_type(),
            Self::String => ValueType::String.api_type(),
            Self::Boolean => ValueType::Boolean.api_type(),
            Self::Char => ValueType::Char.api_type(),
            Self::Object(class) => ValueType::Object(*class).api_type(),
            Self::SourceObject(_) => ValueType::Object(NativeClass::View).api_type(),
            Self::Tuple(shape) => shape.api_type(),
            Self::Array(element) => ValueType::Array(*element).api_type(),
            Self::Void => api::Type::Void,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ValueType {
    Integer,
    NullableInteger,
    String,
    NullableString,
    Boolean,
    Char,
    Object(NativeClass),
    Tuple(TupleShape),
    FontDefinition,
    Array(ArrayElement),
    Null,
    NullableDictionary,
}

impl ValueType {
    /// Return the scalar alternative of a supported nullable value. Opaque
    /// callback Dictionaries retain their union because non-null use is unsupported.
    fn without_null(self) -> Self {
        match self {
            Self::NullableInteger => Self::Integer,
            Self::NullableString => Self::String,
            _ => self,
        }
    }

    /// Map a supported value category to its core API type without changing storage.
    fn api_type(self) -> api::Type {
        match self {
            Self::Integer => api::Type::named("Toybox.Lang.Number"),
            Self::NullableInteger => api::Type::Union(vec![
                api::Type::named("Toybox.Lang.Number"),
                api::Type::Null,
            ]),
            Self::String => api::Type::named("Toybox.Lang.String"),
            Self::NullableString => api::Type::Union(vec![
                api::Type::named("Toybox.Lang.String"),
                api::Type::Null,
            ]),
            Self::Boolean => api::Type::named("Toybox.Lang.Boolean"),
            Self::Char => api::Type::named("Toybox.Lang.Char"),
            Self::Object(class) => api::Type::named(class.name()),
            Self::Tuple(shape) => shape.api_type(),
            Self::FontDefinition => api::Type::named("Toybox.Graphics.FontDefinition"),
            Self::Array(element) => api::Type::Array(Box::new(element.api_type())),
            Self::Null => api::Type::Null,
            Self::NullableDictionary => api::Type::Union(vec![
                api::Type::named("Toybox.Lang.Dictionary"),
                api::Type::Null,
            ]),
        }
    }

    /// Adapt an API type to the fixed executable value subset; other types
    /// remain unsupported even if the catalogue contains their declarations.
    fn from_api_type(ty: &api::Type) -> Option<Self> {
        match ty {
            ty if *ty == TupleShape::OneView.api_type() => Some(Self::Tuple(TupleShape::OneView)),
            ty if *ty == TupleShape::ViewDelegate.api_type() => {
                Some(Self::Tuple(TupleShape::ViewDelegate))
            }
            api::Type::Array(element) => ArrayElement::from_api_type(element).map(Self::Array),
            api::Type::Named(name) => match name.as_str() {
                "Toybox.Graphics.FontDefinition" => Some(Self::FontDefinition),
                "Toybox.Lang.Number" => Some(Self::Integer),
                "Toybox.Lang.String" => Some(Self::String),
                "Toybox.Lang.Boolean" => Some(Self::Boolean),
                "Toybox.Lang.Char" => Some(Self::Char),
                name => NativeClass::from_name(name).map(Self::Object),
            },
            api::Type::Null => Some(Self::Null),
            _ if *ty == Self::NullableDictionary.api_type() => Some(Self::NullableDictionary),
            _ if *ty == Self::NullableInteger.api_type() => Some(Self::NullableInteger),
            _ if *ty == Self::NullableString.api_type() => Some(Self::NullableString),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operation {
    /// Invoke a source instance method on a receiver with proven constructor
    /// class. Unit validation checks ownership and the declared call contract.
    ObjectCall {
        result: ValueId,
        result_type: ReturnType,
        receiver: ValueId,
        class: String,
        member: String,
        arguments: Vec<ValueId>,
    },
    /// Invoke a proven source instance method and discard its runtime result.
    ObjectCallStatement {
        receiver: ValueId,
        class: String,
        member: String,
        arguments: Vec<ValueId>,
    },
    /// Read a Number or non-null source reference on the current instance.
    /// Unit validation checks the claimed result against the field declaration
    /// and prevents reference reads before constructor initialization.
    SelfFieldRead {
        result: ValueId,
        field: String,
        result_type: ReturnType,
    },
    /// Replace a declared field on the current instance, preserving its type.
    SelfFieldAssign {
        field: String,
        value: ValueId,
    },
    /// Define a supported native constant with its declared enum/value contract.
    NativeConstant {
        result: ValueId,
        module: String,
        member: String,
    },
    /// Allocate a supported source object and invoke its initializer with ordered arguments.
    /// Unit validation resolves the class and checks its initializer contract.
    SourceNew {
        result: ValueId,
        class: String,
        /// Native base projection; unit validation checks the class declaration.
        base: ClassBase,
        arguments: Vec<ValueId>,
    },
    /// Invoke a catalogue-resolved Void member on a value receiver for effects.
    NativeCallStatement {
        receiver: ValueId,
        member: String,
        arguments: Vec<ValueId>,
    },
    /// Construct a fresh View result, optionally followed by a non-null delegate.
    /// Slots must be already evaluated local/parameter reads, so allocation
    /// cannot move before operand effects. The shape has no Array mutation profile.
    TupleNew {
        result: ValueId,
        elements: Vec<ValueId>,
    },
    /// Allocate and initialize one supported native object with no arguments.
    /// Constructor availability is checked separately from class availability;
    /// allocation and initialization failures belong to the runtime.
    NativeNew {
        result: ValueId,
        class: NativeClass,
    },
    /// Allocate an empty array with the specified element contract.
    /// The element contract is explicit; allocation failures belong to the VM.
    ArrayNew {
        result: ValueId,
        element_type: ArrayElement,
    },
    /// Read one element from a non-null typed array using a non-null Number index.
    /// Evaluate array before index, once each. Runtime bounds failures remain
    /// observable; indexes are not clamped or adjusted by the compiler.
    ArrayRead {
        result: ValueId,
        array: ValueId,
        index: ValueId,
    },
    /// Replace a Number-array element without producing a value or resizing.
    /// Evaluate receiver, Number index, and Number value once in that order;
    /// runtime bounds checks occur at the write, after operand effects.
    ArrayWrite {
        array: ValueId,
        index: ValueId,
        value: ValueId,
    },
    /// Invert one non-null Boolean and produce Boolean, evaluating it once.
    /// Other types are rejected rather than converted to truth values.
    Not {
        result: ValueId,
        operand: ValueId,
    },
    /// Negate one Number value, preserving the runtime's signed 32-bit behavior.
    /// Nullable values require a non-null proof before use.
    Negate {
        result: ValueId,
        operand: ValueId,
    },
    /// Define a Char from its raw 32-bit representation. This preserves VM bits,
    /// including values outside Unicode scalar ranges; source literals are narrower.
    Char {
        result: ValueId,
        value: u32,
    },
    /// Invoke a catalogue-resolved native module function as a Void statement.
    /// Names are canonical API identities, independent of executable bindings.
    NativeModuleCallStatement {
        module: String,
        member: String,
        arguments: Vec<ValueId>,
    },
    /// Define a Boolean literal, distinct from integer zero/one and null.
    Boolean {
        result: ValueId,
        value: bool,
    },
    /// Start a uniquely named block; the first declared block is entry.
    Block {
        block: BlockId,
    },
    /// Compare supported operands, defining a Boolean in this block.
    Compare {
        result: ValueId,
        kind: ComparisonOp,
        left: ValueId,
        right: ValueId,
    },
    /// Consume a Boolean and terminate the block with two block targets.
    Branch {
        condition: ValueId,
        on_true: BlockId,
        on_false: BlockId,
    },
    /// Terminate the block with an unconditional edge.
    Jump {
        target: BlockId,
    },
    Integer {
        result: ValueId,
        value: i32,
    },
    Null {
        result: ValueId,
    },
    String {
        result: ValueId,
        value: String,
    },
    Println {
        value: ValueId,
    },
    /// Evaluate and consume a supported non-Void value without retaining it.
    /// Calls and operand effects still execute once in their validated order.
    Discard {
        value: ValueId,
    },
    /// Invoke the native superclass initializer on the current instance.
    /// The supported call has no explicit arguments and no semantic result.
    NativeBaseInitialize {
        base: ClassBase,
    },
    Binary {
        result: ValueId,
        kind: BinaryOp,
        left: ValueId,
        right: ValueId,
    },
    ParameterRead {
        result: ValueId,
        index: usize,
    },
    ModuleCall {
        result: ValueId,
        result_type: ReturnType,
        module: String,
        member: String,
        arguments: Vec<ValueId>,
    },
    /// Invoke a runtime-provided member on an explicit value receiver.
    /// Validation resolves its inherited contract and supported native lowering.
    NativeCall {
        result: ValueId,
        receiver: ValueId,
        member: String,
        arguments: Vec<ValueId>,
    },
    /// Invoke a qualified user method for effects, discarding any VM result.
    /// Unit validation checks its parameter contracts and exact arity.
    ModuleCallStatement {
        module: String,
        member: String,
        arguments: Vec<ValueId>,
    },
    /// Invoke a value-returning member on this method's instance receiver.
    SelfCall {
        result: ValueId,
        result_type: ReturnType,
        member: String,
        arguments: Vec<ValueId>,
    },
    /// Invoke a member on this method's receiver and discard its result.
    SelfCallStatement {
        member: String,
        arguments: Vec<ValueId>,
    },
    LocalInitialize {
        local: LocalId,
        value: ValueId,
    },
    /// Replace an initialized local's value without changing its supported
    /// type or allocating a new local. The right-hand value is consumed first.
    LocalAssign {
        local: LocalId,
        value: ValueId,
    },
    LocalRead {
        result: ValueId,
        local: LocalId,
    },
    Return {
        value: ValueId,
    },
    /// Terminate the block without producing a semantic result.
    ReturnVoid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocatedOperation {
    pub operation: Operation,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrError {
    pub operation: Option<usize>,
    pub message: String,
}
impl fmt::Display for IrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}
impl std::error::Error for IrError {}

/// A validated semantic IR function with a name, parameters, and operations.
///
/// Expression values have one use within their defining block and refer only to
/// earlier definitions. Explicit blocks may have cyclic edges and terminate with a
/// return, branch, or jump; an unmarked body is one implicit block. Local reads
/// and assignments require initialization on every incoming path. Assignments
/// retain each local's initializer type. Calls, prints, and assignments require
/// operations in expression evaluation order. Operations cannot be mutated
/// after validation; VM frame slots are assigned during lowering.
/// Locals retain supported value types; printing accepts Number, String, Boolean,
/// or Char. Operator contracts determine arithmetic result types. User calls carry
/// result contracts; unit validation checks them against callee signatures.
/// Native callback parameters may carry opaque nullable Dictionary state.
/// Returns consistently produce one supported type or no value. Statement calls
/// discard results; unit validation rejects using a void callee as a value.
/// Instance methods permit self calls and require class ownership at unit
/// validation. Other functions cannot assume an instance receiver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    name: String,
    parameter_count: usize,
    parameter_types: Vec<ParameterType>,
    return_type: ReturnType,
    instance_method: bool,
    operations: Vec<LocatedOperation>,
    value_types: std::collections::BTreeMap<ValueId, ValueType>,
}

impl Function {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn parameter_count(&self) -> usize {
        self.parameter_count
    }

    /// Return the validated result contract shared by all return terminators.
    pub fn return_type(&self) -> ReturnType {
        self.return_type.clone()
    }

    /// Whether this body requires an instance receiver in its hidden context slot.
    pub fn is_instance_method(&self) -> bool {
        self.instance_method
    }

    pub fn operations(&self) -> &[LocatedOperation] {
        &self.operations
    }

    /// Check this immutable function's known unguarded API references against
    /// explicit target and deployment levels. Returns an operation-indexed error
    /// where possible; does not change IR or establish device compatibility.
    pub fn validate_api(&self, context: api::ApiContext) -> Result<(), IrError> {
        Self::construct_for_api(
            self.name.clone(),
            self.parameter_types.clone(),
            self.operations.clone(),
            self.instance_method,
            Some(context),
            Some(self.return_type.clone()),
        )
        .map(|_| ())
    }

    /// Construct and validate a zero-parameter function from IR operations.
    ///
    /// Uses [`Self::with_parameters`] with a parameter count of zero.
    pub fn new(name: String, operations: Vec<LocatedOperation>) -> Result<Self, IrError> {
        Self::with_parameters(name, 0, operations)
    }

    /// Construct and validate a function with positional parameters.
    ///
    /// Parameter count includes unused parameters and excludes the VM context.
    /// Rejects invalid names, spans, value/local dependencies, effect ordering,
    /// incompatible value types or inconsistent returns, malformed or
    /// unreachable blocks, cross-block expression uses, and unsupported
    /// operation or parameter/local counts.
    /// External callee arity and types are not checked here.
    pub fn with_parameters(
        name: String,
        parameter_count: usize,
        operations: Vec<LocatedOperation>,
    ) -> Result<Self, IrError> {
        Self::integer_parameters(name, parameter_count, operations, false)
    }

    /// Construct and validate an instance method with integer positional parameters.
    ///
    /// Enables current-receiver calls. The receiver occupies the hidden VM context
    /// slot, not an explicit parameter. Class ownership and callee signatures are
    /// checked by compilation-unit validation; other Function invariants apply.
    pub fn method(
        name: String,
        parameter_count: usize,
        operations: Vec<LocatedOperation>,
    ) -> Result<Self, IrError> {
        Self::integer_parameters(name, parameter_count, operations, true)
    }

    /// Construct instance-method IR with explicit parameter contracts.
    /// Unit validation checks these against native callbacks or user signatures.
    pub fn method_with_parameter_types(
        name: String,
        parameter_types: Vec<ParameterType>,
        operations: Vec<LocatedOperation>,
    ) -> Result<Self, IrError> {
        Self::construct(name, parameter_types, operations, true)
    }

    /// Construct and validate function IR with explicit positional types.
    /// Callee parameter and result contracts are checked by unit validation.
    pub fn with_parameter_types(
        name: String,
        parameter_types: Vec<ParameterType>,
        operations: Vec<LocatedOperation>,
    ) -> Result<Self, IrError> {
        Self::construct(name, parameter_types, operations, false)
    }

    /// Construct function IR with explicit parameter and result contracts.
    /// Every return must be assignable to the result; nullable contracts accept
    /// Null, their scalar type, or the same union. Other validation is unchanged.
    pub fn with_signature(
        name: String,
        parameter_types: Vec<ParameterType>,
        result: ReturnType,
        operations: Vec<LocatedOperation>,
    ) -> Result<Self, IrError> {
        Self::construct_for_api(name, parameter_types, operations, false, None, Some(result))
    }

    /// Construct instance-method IR with explicit parameter and result contracts.
    /// Unit validation checks ownership, callees, and native callback contracts.
    pub fn method_with_signature(
        name: String,
        parameter_types: Vec<ParameterType>,
        result: ReturnType,
        operations: Vec<LocatedOperation>,
    ) -> Result<Self, IrError> {
        Self::construct_for_api(name, parameter_types, operations, true, None, Some(result))
    }

    /// Return explicit parameter types in source order, excluding the receiver.
    pub fn parameter_types(&self) -> &[ParameterType] {
        &self.parameter_types
    }

    fn integer_parameters(
        name: String,
        count: usize,
        operations: Vec<LocatedOperation>,
        instance: bool,
    ) -> Result<Self, IrError> {
        if count > MAX_PARAMETERS {
            return Err(IrError {
                operation: None,
                message: "function exceeds 254-parameter limit".into(),
            });
        }
        Self::construct(
            name,
            vec![ParameterType::Integer; count],
            operations,
            instance,
        )
    }

    fn construct(
        name: String,
        parameter_types: Vec<ParameterType>,
        operations: Vec<LocatedOperation>,
        instance_method: bool,
    ) -> Result<Self, IrError> {
        Self::construct_for_api(
            name,
            parameter_types,
            operations,
            instance_method,
            None,
            None,
        )
    }

    fn construct_for_api(
        name: String,
        parameter_types: Vec<ParameterType>,
        operations: Vec<LocatedOperation>,
        instance_method: bool,
        context: Option<api::ApiContext>,
        declared_result: Option<ReturnType>,
    ) -> Result<Self, IrError> {
        let source_parameters = parameter_types.iter().filter_map(|ty| {
            if let ParameterType::SourceObject(class) = ty {
                Some(class.as_str())
            } else {
                None
            }
        });
        let source_result = declared_result.as_ref().and_then(|ty| {
            if let ReturnType::SourceObject(class) = ty {
                Some(class.as_str())
            } else {
                None
            }
        });
        for class in source_parameters.chain(source_result) {
            validate_call_names(None, class).map_err(|message| IrError {
                operation: None,
                message: message.into(),
            })?;
        }
        if parameter_types.iter().any(|ty| {
            matches!(
                ty,
                ParameterType::Array(ArrayElement::Object(class)) if *class != NativeClass::View
            )
        }) || matches!(declared_result, Some(ReturnType::Array(ArrayElement::Object(class))) if class != NativeClass::View)
        {
            return Err(IrError {
                operation: None,
                message: "object arrays support only the View element contract".into(),
            });
        }
        if instance_method && native::callback(ClassBase::ApplicationAppBase, &name).is_some() {
            if let Some(context) = context {
                context
                    .method(api::core(), "Toybox.Application.AppBase", &name)
                    .map_err(|message| IrError {
                        operation: None,
                        message,
                    })?;
            }
        }
        if let Some(context) = context {
            for ty in parameter_types.iter().map(|ty| ty.api_type()) {
                context
                    .check_type(api::core(), &ty)
                    .map_err(|message| IrError {
                        operation: None,
                        message,
                    })?;
            }
        }
        let parameter_count = parameter_types.len();
        if parameter_count > MAX_PARAMETERS {
            return Err(IrError {
                operation: None,
                message: "function exceeds 254-parameter limit".into(),
            });
        }
        let valid_name = name
            .bytes()
            .next()
            .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
            && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_');
        if !valid_name {
            return Err(IrError {
                operation: None,
                message: "invalid function name".into(),
            });
        }
        if operations.is_empty() || operations.len() > MAX_OPERATIONS {
            return Err(IrError {
                operation: None,
                message: "IR requires 1..=1024 operations with a final return".into(),
            });
        }
        let mut return_type = None;
        let mut definitions = std::collections::BTreeMap::<ValueId, usize>::new();
        let structure = control_flow::validate(&operations)?;
        let mut value_blocks = std::collections::BTreeMap::new();
        let mut available_locals = std::collections::BTreeSet::new();
        let local_inputs = control_flow::local_inputs(&structure, &operations);
        let non_null_inputs = null_flow::inputs(&structure, &operations);
        let mut non_null = std::collections::BTreeSet::new();
        let mut types = std::collections::BTreeMap::new();
        let mut locals = std::collections::BTreeSet::new();
        let mut uses = std::collections::BTreeMap::new();
        let mut local_types = std::collections::BTreeMap::new();
        let mut origins = source_objects::Origins::new(parameter_types.clone());
        for (index, located) in operations.iter().enumerate() {
            let fail = |message: &str| IrError {
                operation: Some(index),
                message: message.into(),
            };
            if located.span.is_some_and(|span| span.start > span.end) {
                return Err(fail("source span ends before it starts"));
            }
            let type_of = |v| types.get(&v).copied().unwrap_or(ValueType::Integer);
            origins.observe(&located.operation).map_err(fail)?;
            let (result, operands) = match located.operation {
                Operation::ObjectCall {
                    receiver,
                    ref class,
                    ref member,
                    ref arguments,
                    ..
                }
                | Operation::ObjectCallStatement {
                    receiver,
                    ref class,
                    ref member,
                    ref arguments,
                } => {
                    validate_call_names(Some(class), member).map_err(fail)?;
                    validate_call_arguments(arguments, &types).map_err(fail)?;
                    if !matches!(
                        type_of(receiver),
                        ValueType::Object(NativeClass::View | NativeClass::BehaviorDelegate)
                    ) {
                        return Err(fail(
                            "source object calls require a supported non-null object receiver",
                        ));
                    }
                    let result = if let Operation::ObjectCall {
                        result,
                        ref result_type,
                        ..
                    } = located.operation
                    {
                        let ty = call_result_type(result_type.clone()).map_err(fail)?;
                        types.insert(result, ty);
                        Some(result)
                    } else {
                        None
                    };
                    (
                        result,
                        std::iter::once(receiver)
                            .chain(arguments.iter().copied())
                            .collect(),
                    )
                }
                Operation::Block { block } => {
                    available_locals = local_inputs[&block].clone();
                    non_null = non_null_inputs[&block].clone();
                    (None, vec![])
                }
                Operation::Compare {
                    result,
                    left,
                    right,
                    kind,
                } => {
                    let checked_type = |value| {
                        if kind.is_equality()
                            && (type_of(left) == ValueType::Null
                                || type_of(right) == ValueType::Null)
                        {
                            if let Some(&at) = definitions.get(&value) {
                                let declared = match operations[at].operation {
                                    Operation::LocalRead { local, .. } => {
                                        local_types.get(&local).copied()
                                    }
                                    Operation::ParameterRead { index, .. } => parameter_types
                                        .get(index)
                                        .and_then(|ty| ValueType::from_api_type(&ty.api_type())),
                                    _ => None,
                                };
                                if let Some(
                                    ty @ (ValueType::NullableInteger | ValueType::NullableString),
                                ) = declared
                                {
                                    return ty;
                                }
                            }
                        }
                        type_of(value)
                    };
                    let result_type = operators::comparison_result(
                        kind,
                        &checked_type(left).api_type(),
                        &checked_type(right).api_type(),
                    )
                    .and_then(|ty| ValueType::from_api_type(&ty))
                    .ok_or_else(|| {
                        fail("operand types do not match a supported comparison contract")
                    })?;
                    types.insert(result, result_type);
                    (Some(result), vec![left, right])
                }
                Operation::Branch { condition, .. } => {
                    if type_of(condition) != ValueType::Boolean {
                        return Err(fail("branch requires a Boolean value"));
                    }
                    (None, vec![condition])
                }
                Operation::Jump { .. } => (None, vec![]),
                Operation::Char { result, .. } => {
                    types.insert(result, ValueType::Char);
                    (Some(result), vec![])
                }
                Operation::Boolean { result, .. } => {
                    types.insert(result, ValueType::Boolean);
                    (Some(result), vec![])
                }
                Operation::SelfFieldRead {
                    result,
                    ref field,
                    ref result_type,
                } => {
                    if !instance_method {
                        return Err(fail("self field reads require an instance receiver"));
                    }
                    validate_call_names(None, field).map_err(fail)?;
                    if !matches!(
                        result_type,
                        ReturnType::Integer | ReturnType::SourceObject(_)
                    ) {
                        return Err(fail("unsupported instance field result contract"));
                    }
                    types.insert(result, call_result_type(result_type.clone()).map_err(fail)?);
                    (Some(result), vec![])
                }
                Operation::SelfFieldAssign { ref field, value } => {
                    if !instance_method {
                        return Err(fail("self field assignments require an instance receiver"));
                    }
                    validate_call_names(None, field).map_err(fail)?;
                    if !matches!(
                        type_of(value),
                        ValueType::Integer | ValueType::Object(NativeClass::View)
                    ) {
                        return Err(fail("instance fields require Number or source View values"));
                    }
                    (None, vec![value])
                }
                Operation::Integer { result, .. } => (Some(result), vec![]),
                Operation::Null { result } => {
                    types.insert(result, ValueType::Null);
                    (Some(result), vec![])
                }
                Operation::String { result, ref value } => {
                    if value.len() > u16::MAX as usize {
                        return Err(fail("string UTF-8 byte length exceeds u16"));
                    }
                    types.insert(result, ValueType::String);
                    (Some(result), vec![])
                }
                Operation::NativeBaseInitialize { base } => {
                    if let Some(context) = context {
                        context
                            .method(api::core(), base.api_name(), "initialize")
                            .map_err(|message| fail(&message))?;
                    }
                    if !instance_method {
                        return Err(fail(
                            "native base initialization requires an instance method",
                        ));
                    }
                    (None, vec![])
                }
                Operation::Println { value } => {
                    if let Some(context) = context {
                        context
                            .function(api::core(), "Toybox.System", "println")
                            .map_err(|message| fail(&message))?;
                    }
                    if !matches!(
                        type_of(value),
                        ValueType::Integer
                            | ValueType::String
                            | ValueType::Boolean
                            | ValueType::Char
                    ) {
                        return Err(fail(
                            "println currently accepts only Number, String, Boolean, or Char",
                        ));
                    }
                    (None, vec![value])
                }
                Operation::NativeModuleCallStatement {
                    ref module,
                    ref member,
                    ref arguments,
                } => {
                    let types = arguments
                        .iter()
                        .map(|v| type_of(*v).api_type())
                        .collect::<Vec<_>>();
                    native::module_statement(module, member, &types, context)
                        .map_err(|message| fail(&message))?;
                    (None, arguments.clone())
                }
                Operation::NativeConstant {
                    result,
                    ref module,
                    ref member,
                } => {
                    native::constant(module, member, context).map_err(|message| fail(&message))?;
                    types.insert(result, ValueType::FontDefinition);
                    (Some(result), vec![])
                }
                Operation::SourceNew {
                    result,
                    ref class,
                    base,
                    ref arguments,
                } => {
                    validate_call_names(None, class).map_err(fail)?;
                    let native_class = base.constructed_class().ok_or_else(|| {
                        fail("native base has no supported source construction profile")
                    })?;
                    native::constructor(native_class, context).map_err(|message| fail(&message))?;
                    validate_call_arguments(arguments, &types).map_err(fail)?;
                    types.insert(result, ValueType::Object(native_class));
                    (Some(result), arguments.clone())
                }
                Operation::NativeCallStatement {
                    receiver,
                    ref member,
                    ref arguments,
                } => {
                    let arguments_types = arguments
                        .iter()
                        .map(|v| type_of(*v).api_type())
                        .collect::<Vec<_>>();
                    let result = native::value_call(
                        &type_of(receiver).api_type(),
                        member,
                        &arguments_types,
                        context,
                    )
                    .map_err(|message| fail(&message))?;
                    if result != api::Type::Void {
                        return Err(fail("native statement requires a Void result"));
                    }
                    (
                        None,
                        std::iter::once(receiver)
                            .chain(arguments.iter().copied())
                            .collect(),
                    )
                }
                Operation::TupleNew {
                    result,
                    ref elements,
                } => {
                    let shape = match elements.len() {
                        1 => TupleShape::OneView,
                        2 => TupleShape::ViewDelegate,
                        _ => return Err(fail("initial View tuple requires one or two slots")),
                    };
                    let api::Type::Tuple(expected) = shape.api_type() else {
                        unreachable!()
                    };
                    if !expected.iter().zip(elements).all(|(expected, value)| {
                        native::accepts(expected, &type_of(*value).api_type())
                    }) {
                        return Err(fail("initial View tuple slot types do not match its shape"));
                    }
                    if !elements.iter().all(|value| {
                        definitions.get(value).is_some_and(|at| {
                            matches!(
                                operations[*at].operation,
                                Operation::LocalRead { .. } | Operation::ParameterRead { .. }
                            )
                        })
                    }) {
                        return Err(fail(
                            "tuple slots require already evaluated local or parameter reads",
                        ));
                    }
                    types.insert(result, ValueType::Tuple(shape));
                    (Some(result), elements.clone())
                }
                Operation::NativeNew { result, class } => {
                    native::constructor(class, context).map_err(|message| fail(&message))?;
                    types.insert(result, ValueType::Object(class));
                    (Some(result), vec![])
                }
                Operation::ArrayNew {
                    result,
                    element_type,
                } => {
                    if matches!(element_type, ArrayElement::Object(class) if class != NativeClass::View)
                    {
                        return Err(fail("object arrays support only the View element contract"));
                    }
                    let ty = ValueType::Array(element_type);
                    if let Some(context) = context {
                        context
                            .check_type(api::core(), &ty.api_type())
                            .map_err(|message| fail(&message))?;
                    }
                    types.insert(result, ty);
                    (Some(result), vec![])
                }
                Operation::ArrayRead {
                    result,
                    array,
                    index,
                } => {
                    let ValueType::Array(element) = type_of(array) else {
                        return Err(fail("array reads require a supported array receiver"));
                    };
                    if type_of(index) != ValueType::Integer {
                        return Err(fail("array reads require a non-null Number index"));
                    }
                    types.insert(
                        result,
                        ValueType::from_api_type(&element.api_type()).unwrap(),
                    );
                    (Some(result), vec![array, index])
                }
                Operation::ArrayWrite {
                    array,
                    index,
                    value,
                } => {
                    if type_of(array) != ValueType::Array(ArrayElement::Number) {
                        return Err(fail("array writes require a Number-array receiver"));
                    }
                    if type_of(index) != ValueType::Integer || type_of(value) != ValueType::Integer
                    {
                        return Err(fail("array writes require non-null Number index and value"));
                    }
                    (None, vec![array, index, value])
                }
                Operation::Not { result, operand } => {
                    let result_type = operators::not_result(&type_of(operand).api_type())
                        .and_then(|ty| ValueType::from_api_type(&ty))
                        .ok_or_else(|| fail("logical negation requires a non-null Boolean"))?;
                    types.insert(result, result_type);
                    (Some(result), vec![operand])
                }
                Operation::Negate { result, operand } => {
                    let result_type = operators::negate_result(&type_of(operand).api_type())
                        .and_then(|ty| ValueType::from_api_type(&ty))
                        .ok_or_else(|| fail("negation requires a non-null Number"))?;
                    types.insert(result, result_type);
                    (Some(result), vec![operand])
                }
                Operation::Binary {
                    result,
                    left,
                    right,
                    kind,
                } => {
                    let result_type = operators::binary_result(
                        kind,
                        &type_of(left).api_type(),
                        &type_of(right).api_type(),
                    )
                    .and_then(|ty| ValueType::from_api_type(&ty))
                    .ok_or_else(|| {
                        fail("operand types do not match a supported arithmetic contract")
                    })?;
                    types.insert(result, result_type);
                    (Some(result), vec![left, right])
                }
                Operation::ParameterRead { result, index } => {
                    if index >= parameter_count {
                        return Err(fail("parameter index is outside the function signature"));
                    }
                    let ty = ValueType::from_api_type(&parameter_types[index].api_type())
                        .expect("parameter contracts have supported value types");
                    types.insert(
                        result,
                        if non_null.contains(&null_flow::Place::Parameter(index)) {
                            ty.without_null()
                        } else {
                            ty
                        },
                    );
                    (Some(result), vec![])
                }
                Operation::Discard { value } => (None, vec![value]),
                Operation::NativeCall {
                    result,
                    receiver,
                    ref member,
                    ref arguments,
                } => {
                    let argument_types = arguments
                        .iter()
                        .map(|v| type_of(*v).api_type())
                        .collect::<Vec<_>>();
                    let result_type = native::value_call(
                        &type_of(receiver).api_type(),
                        member,
                        &argument_types,
                        context,
                    )
                    .map_err(|message| fail(&message))?;
                    let result_type = ValueType::from_api_type(&result_type)
                        .ok_or_else(|| fail("native call result type has no supported lowering"))?;
                    types.insert(result, result_type);
                    let mut operands = vec![receiver];
                    operands.extend(arguments);
                    (Some(result), operands)
                }
                Operation::ModuleCall {
                    ref module,
                    ref member,
                    ref arguments,
                    ..
                }
                | Operation::ModuleCallStatement {
                    ref module,
                    ref member,
                    ref arguments,
                } => {
                    validate_call_names(Some(module), member).map_err(fail)?;
                    validate_call_arguments(arguments, &types).map_err(fail)?;
                    let result = if let Operation::ModuleCall {
                        result,
                        ref result_type,
                        ..
                    } = located.operation
                    {
                        types.insert(result, call_result_type(result_type.clone()).map_err(fail)?);
                        Some(result)
                    } else {
                        None
                    };
                    (result, arguments.clone())
                }
                Operation::SelfCall {
                    ref member,
                    ref arguments,
                    ..
                }
                | Operation::SelfCallStatement {
                    ref member,
                    ref arguments,
                } => {
                    if !instance_method {
                        return Err(fail("self calls require an instance method"));
                    }
                    validate_call_names(None, member).map_err(fail)?;
                    validate_call_arguments(arguments, &types).map_err(fail)?;
                    let result = if let Operation::SelfCall {
                        result,
                        ref result_type,
                        ..
                    } = located.operation
                    {
                        types.insert(result, call_result_type(result_type.clone()).map_err(fail)?);
                        Some(result)
                    } else {
                        None
                    };
                    (result, arguments.clone())
                }
                Operation::LocalInitialize { local, value } => {
                    non_null.remove(&null_flow::Place::Local(local));
                    local_types.insert(local, type_of(value));
                    if !locals.insert(local) {
                        return Err(fail("duplicate local initialization"));
                    }
                    available_locals.insert(local);
                    if locals.len() > MAX_LOCALS {
                        return Err(fail("function exceeds 255-local limit"));
                    }
                    if parameter_count + locals.len() > MAX_LOCALS {
                        return Err(fail("parameters plus locals exceed 255-slot limit"));
                    }
                    (None, vec![value])
                }
                Operation::LocalAssign { local, value } => {
                    if !locals.contains(&local) || !available_locals.contains(&local) {
                        return Err(fail(
                            "assignment requires an initialized local on every incoming path",
                        ));
                    }
                    let incoming = type_of(value);
                    if local_types[&local] != incoming
                        && !(local_types[&local] == ValueType::NullableDictionary
                            && incoming == ValueType::Null)
                        && !(local_types[&local] == ValueType::NullableInteger
                            && matches!(incoming, ValueType::Integer | ValueType::Null))
                        && !(local_types[&local] == ValueType::NullableString
                            && matches!(incoming, ValueType::String | ValueType::Null))
                    {
                        return Err(fail("assignment must preserve the local's value type"));
                    }
                    non_null.remove(&null_flow::Place::Local(local));
                    (None, vec![value])
                }
                Operation::LocalRead { result, local } => {
                    if !locals.contains(&local) || !available_locals.contains(&local) {
                        return Err(fail(
                            "local read must refer to an earlier initialized local",
                        ));
                    }
                    let ty = local_types[&local];
                    types.insert(
                        result,
                        if non_null.contains(&null_flow::Place::Local(local)) {
                            ty.without_null()
                        } else {
                            ty
                        },
                    );
                    (Some(result), vec![])
                }
                Operation::ReturnVoid => {
                    if declared_result
                        .as_ref()
                        .is_some_and(|ty| *ty != ReturnType::Void)
                    {
                        return Err(fail("void return does not match function result contract"));
                    }
                    if return_type
                        .replace(ReturnType::Void)
                        .is_some_and(|previous| previous != ReturnType::Void)
                    {
                        return Err(fail("function cannot mix value and void returns"));
                    }
                    if !structure.explicit && index + 1 != operations.len() {
                        return Err(fail("return must be the final operation"));
                    }
                    (None, vec![])
                }
                Operation::Return { value } => {
                    let result = if let Some(contract) = &declared_result {
                        if let ReturnType::SourceObject(class) = contract {
                            if origins.class(value) != Some(class.as_str()) {
                                return Err(fail(
                                    "return does not preserve its declared source class",
                                ));
                            }
                        }
                        if !native::accepts(&contract.api_type(), &type_of(value).api_type()) {
                            return Err(fail("return type does not match function contract"));
                        }
                        contract.clone()
                    } else {
                        match type_of(value) {
                            ValueType::Integer => ReturnType::Integer,
                            ValueType::String => ReturnType::String,
                            ValueType::Boolean => ReturnType::Boolean,
                            ValueType::Char => ReturnType::Char,
                            ValueType::Object(class) => ReturnType::Object(class),
                            ValueType::Tuple(shape) => ReturnType::Tuple(shape),
                            ValueType::Array(element) => ReturnType::Array(element),
                            _ => {
                                return Err(fail(
                                    "nullable returns require an explicit result contract",
                                ));
                            }
                        }
                    };
                    if return_type
                        .replace(result.clone())
                        .is_some_and(|previous| previous != result)
                    {
                        return Err(fail("function cannot mix different return types"));
                    }
                    if !structure.explicit && index + 1 != operations.len() {
                        return Err(fail("return must be the final operation"));
                    }
                    (None, vec![value])
                }
            };
            if let Some(result) = result {
                if let Some(context) = context {
                    if let Some(ty) = types.get(&result) {
                        context
                            .check_type(api::core(), &ty.api_type())
                            .map_err(|message| fail(&message))?;
                    }
                }
                types.entry(result).or_insert(ValueType::Integer);
            }
            for operand in operands {
                if !definitions.contains_key(&operand) {
                    return Err(fail("operand must refer to an earlier defined value"));
                }
                if value_blocks[&operand] != structure.blocks[index] {
                    return Err(fail(
                        "expression values cannot cross block boundaries; use local reads",
                    ));
                }
                *uses.entry(operand).or_insert(0usize) += 1;
            }
            if let Some(result) = result {
                value_blocks.insert(result, structure.blocks[index]);
                if definitions.insert(result, index).is_some() {
                    return Err(fail("duplicate value definition"));
                }
            }
        }
        if let (Some(context), Some(result)) = (context, declared_result) {
            context
                .check_type(api::core(), &result.api_type())
                .map_err(|message| IrError {
                    operation: None,
                    message,
                })?;
        }
        if !matches!(
            operations.last().unwrap().operation,
            Operation::Return { .. } | Operation::ReturnVoid
        ) {
            return Err(IrError {
                operation: None,
                message: "IR requires a final return".into(),
            });
        }
        for (value, index) in &definitions {
            if uses.get(value) != Some(&1) {
                return Err(IrError {
                    operation: Some(*index),
                    message: "supported expression IR requires exactly one use per defined value"
                        .into(),
                });
            }
        }
        if operations.iter().any(|located| {
            matches!(
                located.operation,
                Operation::ModuleCall { .. }
                    | Operation::Binary {
                        kind: BinaryOp::Divide | BinaryOp::Remainder,
                        ..
                    }
                    | Operation::ArrayRead { .. }
                    | Operation::ArrayWrite { .. }
                    | Operation::ArrayNew { .. }
                    | Operation::NativeNew { .. }
                    | Operation::TupleNew { .. }
                    | Operation::SourceNew { .. }
                    | Operation::NativeCallStatement { .. }
                    | Operation::NativeCall { .. }
                    | Operation::ObjectCall { .. }
                    | Operation::ObjectCallStatement { .. }
                    | Operation::NativeModuleCallStatement { .. }
                    | Operation::ModuleCallStatement { .. }
                    | Operation::SelfCall { .. }
                    | Operation::SelfCallStatement { .. }
                    | Operation::Println { .. }
                    | Operation::Discard { .. }
                    | Operation::LocalAssign { .. }
                    | Operation::SelfFieldRead { .. }
                    | Operation::SelfFieldAssign { .. }
                    | Operation::NativeBaseInitialize { .. }
            )
        }) {
            // Calls, prints, and writes make operation order observable. A read
            // defined before a write must not be emitted after that write.
            enum Task {
                Value(ValueId),
                Emit(usize),
            }
            let mut schedule = Vec::new();
            for (index, located) in operations.iter().enumerate() {
                let roots = match &located.operation {
                    Operation::ArrayWrite {
                        array,
                        index,
                        value,
                    } => vec![*array, *index, *value],
                    Operation::LocalInitialize { value, .. }
                    | Operation::LocalAssign { value, .. }
                    | Operation::SelfFieldAssign { value, .. }
                    | Operation::Return { value }
                    | Operation::Println { value }
                    | Operation::Discard { value } => vec![*value],
                    Operation::Branch { condition, .. } => vec![*condition],
                    Operation::ObjectCallStatement {
                        receiver,
                        arguments,
                        ..
                    }
                    | Operation::NativeCallStatement {
                        receiver,
                        arguments,
                        ..
                    } => std::iter::once(*receiver)
                        .chain(arguments.iter().copied())
                        .collect(),
                    Operation::ModuleCallStatement { arguments, .. }
                    | Operation::NativeModuleCallStatement { arguments, .. }
                    | Operation::SelfCallStatement { arguments, .. } => arguments.clone(),
                    Operation::Block { .. }
                    | Operation::Jump { .. }
                    | Operation::ReturnVoid
                    | Operation::NativeBaseInitialize { .. } => {
                        schedule.push(index);
                        continue;
                    }
                    _ => continue,
                };
                let mut tasks: Vec<_> = roots.into_iter().rev().map(Task::Value).collect();
                while let Some(task) = tasks.pop() {
                    match task {
                        Task::Emit(at) => schedule.push(at),
                        Task::Value(value) => {
                            let at = definitions[&value];
                            tasks.push(Task::Emit(at));
                            match &operations[at].operation {
                                Operation::Negate { operand, .. }
                                | Operation::Not { operand, .. } => {
                                    tasks.push(Task::Value(*operand))
                                }
                                Operation::ArrayRead {
                                    array: left,
                                    index: right,
                                    ..
                                }
                                | Operation::Binary { left, right, .. }
                                | Operation::Compare { left, right, .. } => {
                                    tasks.extend([Task::Value(*right), Task::Value(*left)])
                                }
                                Operation::ObjectCall {
                                    receiver,
                                    arguments,
                                    ..
                                }
                                | Operation::NativeCall {
                                    receiver,
                                    arguments,
                                    ..
                                } => {
                                    tasks.extend(arguments.iter().rev().map(|v| Task::Value(*v)));
                                    tasks.push(Task::Value(*receiver));
                                }
                                Operation::TupleNew {
                                    elements: arguments,
                                    ..
                                }
                                | Operation::SourceNew { arguments, .. }
                                | Operation::ModuleCall { arguments, .. }
                                | Operation::SelfCall { arguments, .. } => tasks.extend(
                                    arguments.iter().rev().map(|value| Task::Value(*value)),
                                ),
                                _ => {}
                            }
                        }
                    }
                }
                schedule.push(index);
            }
            if let Some((expected, actual)) = schedule
                .iter()
                .enumerate()
                .find(|(expected, actual)| *expected != **actual)
            {
                return Err(IrError {
                    operation: Some(*actual),
                    message: format!(
                        "functions with calls, division/remainder, array reads, prints, or assignments require expression evaluation order in IR (expected operation {expected})"
                    ),
                });
            }
        }
        Ok(Self {
            name,
            parameter_count,
            parameter_types,
            instance_method,
            return_type: return_type.expect("final return establishes result type"),
            operations,
            value_types: types,
        })
    }

    /// Lower this IR function into a typed VM assembly program.
    ///
    /// [`backend::lower`] selects instructions and assigns frame slots. Module
    /// and member aliases remain unresolved for later binding. Deduplicated
    /// literal declarations carry content without DATA placement; bind them with
    /// [`string_bindings`] before encoding CODE. Output is not a complete PRG.
    pub fn lower(&self) -> ciq_assembly::Program {
        backend::lower(self)
    }

    /// Lower this function and serialize its VM assembly as `.mcasm` text.
    pub fn assembly(&self) -> String {
        self.lower().to_text()
    }

    /// Serialize this function as provisional `ciq-ir` text.
    ///
    /// Retains semantic IDs, operation order, receiver metadata, and optional
    /// source spans.
    /// It can be read by [`parse_ir`]; it is not Garmin MIR.
    pub fn to_ir_text(&self) -> String {
        let parameters = if self.parameter_count == 0 {
            String::new()
        } else {
            format!(" parameters={}", self.parameter_count)
        };
        let receiver = if self.instance_method {
            " receiver=instance"
        } else {
            ""
        };
        let parameter_types = if self
            .parameter_types
            .iter()
            .all(|t| *t == ParameterType::Integer)
        {
            String::new()
        } else {
            format!(
                " parameter-types={}",
                self.parameter_types
                    .iter()
                    .map(|t| {
                        match t {
                            ParameterType::SourceObject(class) => {
                                return format!("source({class})");
                            }
                            ParameterType::NullableInteger => "nullable-integer",
                            ParameterType::NullableString => "nullable-string",
                            ParameterType::Integer => "integer",
                            ParameterType::String => "string",
                            ParameterType::Boolean => "boolean",
                            ParameterType::Char => "char",
                            ParameterType::Object(NativeClass::View) => "view",
                            ParameterType::Object(NativeClass::Dc) => "dc",
                            ParameterType::Object(NativeClass::BehaviorDelegate) => {
                                "behavior-delegate"
                            }
                            ParameterType::Array(element) => element.array_text(),
                            ParameterType::NullableDictionary => "nullable-dictionary",
                        }
                        .to_owned()
                    })
                    .collect::<Vec<_>>()
                    .join(":")
            )
        };
        let result = match self.return_type {
            ReturnType::NullableInteger
            | ReturnType::NullableString
            | ReturnType::SourceObject(_) => call_type_text(&self.return_type),
            _ => String::new(),
        };
        let mut text = format!(
            "ciq-ir\n.function {}{parameters}{receiver}{parameter_types}{result}\n",
            self.name
        );
        for operation in &self.operations {
            text.push_str(&match operation.operation {
                Operation::Block { block } => format!(".block ^{}", block.0),
                Operation::Branch {
                    condition,
                    on_true,
                    on_false,
                } => format!(".branch %{} ^{} ^{}", condition.0, on_true.0, on_false.0),
                Operation::Jump { target } => format!(".jump ^{}", target.0),
                Operation::Compare {
                    result,
                    kind,
                    left,
                    right,
                } => format!(
                    ".{} %{} %{} %{}",
                    match kind {
                        ComparisonOp::Equal => "equal",
                        ComparisonOp::Greater => "greater",
                        ComparisonOp::Less => "less",
                        ComparisonOp::LessEqual => "less-equal",
                        ComparisonOp::GreaterEqual => "greater-equal",
                        ComparisonOp::NotEqual => "not-equal",
                    },
                    result.0,
                    left.0,
                    right.0
                ),
                Operation::Null { result } => format!(".null %{}", result.0),
                Operation::NativeModuleCallStatement {
                    ref module,
                    ref member,
                    ref arguments,
                } => {
                    let mut line = format!(".call-native-module-discard {module} {member}");
                    for argument in arguments {
                        line.push_str(&format!(" %{}", argument.0));
                    }
                    line
                }
                Operation::Char { result, value } => format!(".char %{} 0x{value:08x}", result.0),
                Operation::Boolean { result, value } => format!(".boolean %{} {value}", result.0),
                Operation::SelfFieldRead {
                    result,
                    ref field,
                    ref result_type,
                } => {
                    format!(
                        ".read-self %{} {field}{}",
                        result.0,
                        call_type_text(result_type)
                    )
                }
                Operation::SelfFieldAssign { ref field, value } => {
                    format!(".assign-self {field} %{}", value.0)
                }
                Operation::Integer { result, value } => format!(".integer %{} {value}", result.0),
                Operation::String { result, ref value } => {
                    format!(".string %{} {value:?}", result.0)
                }
                Operation::Println { value } => format!(".println %{}", value.0),
                Operation::NativeConstant {
                    result,
                    ref module,
                    ref member,
                } => format!(".constant %{} {module} {member}", result.0),
                Operation::SourceNew {
                    result,
                    ref class,
                    base,
                    ref arguments,
                } => {
                    let args = arguments
                        .iter()
                        .map(|value| format!(" %{}", value.0))
                        .collect::<String>();
                    format!(".new-source %{} {class} {}{args}", result.0, base.text())
                }
                Operation::ObjectCall {
                    result,
                    ref result_type,
                    receiver,
                    ref class,
                    ref member,
                    ref arguments,
                } => {
                    let args = arguments
                        .iter()
                        .map(|v| format!(" %{}", v.0))
                        .collect::<String>();
                    format!(
                        ".call-object %{} %{} {class} {member}{}{args}",
                        result.0,
                        receiver.0,
                        call_type_text(result_type)
                    )
                }
                Operation::ObjectCallStatement {
                    receiver,
                    ref class,
                    ref member,
                    ref arguments,
                } => {
                    let args = arguments
                        .iter()
                        .map(|v| format!(" %{}", v.0))
                        .collect::<String>();
                    format!(
                        ".call-object-discard %{} {class} {member}{args}",
                        receiver.0
                    )
                }
                Operation::NativeCallStatement {
                    receiver,
                    ref member,
                    ref arguments,
                } => format!(
                    ".call-native-discard %{} {member}{}",
                    receiver.0,
                    arguments
                        .iter()
                        .map(|v| format!(" %{}", v.0))
                        .collect::<String>()
                ),
                Operation::TupleNew {
                    result,
                    ref elements,
                } => format!(
                    ".tuple-new %{}{}",
                    result.0,
                    elements
                        .iter()
                        .map(|value| format!(" %{}", value.0))
                        .collect::<String>()
                ),
                Operation::NativeNew { result, class } => {
                    format!(".new-native %{} {}", result.0, class.name())
                }
                Operation::ArrayNew {
                    result,
                    element_type,
                } => format!(".array-new %{} {}", result.0, element_type.text()),
                Operation::ArrayRead {
                    result,
                    array,
                    index,
                } => format!(".array-read %{} %{} %{}", result.0, array.0, index.0),
                Operation::ArrayWrite {
                    array,
                    index,
                    value,
                } => format!(".array-write %{} %{} %{}", array.0, index.0, value.0),
                Operation::Not { result, operand } => {
                    format!(".not %{} %{}", result.0, operand.0)
                }
                Operation::Negate { result, operand } => {
                    format!(".negate %{} %{}", result.0, operand.0)
                }
                Operation::Binary {
                    result,
                    kind,
                    left,
                    right,
                } => format!(
                    ".{} %{} %{} %{}",
                    match kind {
                        BinaryOp::Add => "add",
                        BinaryOp::Subtract => "subtract",
                        BinaryOp::Multiply => "multiply",
                        BinaryOp::Divide => "divide",
                        BinaryOp::Remainder => "remainder",
                    },
                    result.0,
                    left.0,
                    right.0
                ),
                Operation::Discard { value } => format!(".discard %{}", value.0),
                Operation::NativeCall {
                    result,
                    receiver,
                    ref member,
                    ref arguments,
                } => {
                    let arguments = arguments
                        .iter()
                        .map(|v| format!(" %{}", v.0))
                        .collect::<String>();
                    format!(
                        ".call-native %{} %{} {member}{arguments}",
                        result.0, receiver.0
                    )
                }
                Operation::ModuleCall {
                    result,
                    ref result_type,
                    ref module,
                    ref member,
                    ref arguments,
                } => {
                    let arguments = arguments
                        .iter()
                        .map(|value| format!(" %{}", value.0))
                        .collect::<String>();
                    format!(
                        ".call %{} {module} {member}{}{arguments}",
                        result.0,
                        call_type_text(result_type)
                    )
                }
                Operation::ModuleCallStatement {
                    ref module,
                    ref member,
                    ref arguments,
                } => {
                    let arguments = arguments
                        .iter()
                        .map(|value| format!(" %{}", value.0))
                        .collect::<String>();
                    format!(".call-discard {module} {member}{arguments}")
                }
                Operation::SelfCall {
                    result,
                    ref result_type,
                    ref member,
                    ref arguments,
                } => {
                    let arguments = arguments
                        .iter()
                        .map(|v| format!(" %{}", v.0))
                        .collect::<String>();
                    format!(
                        ".call-self %{} {member}{}{arguments}",
                        result.0,
                        call_type_text(result_type)
                    )
                }
                Operation::SelfCallStatement {
                    ref member,
                    ref arguments,
                } => {
                    let arguments = arguments
                        .iter()
                        .map(|v| format!(" %{}", v.0))
                        .collect::<String>();
                    format!(".call-self-discard {member}{arguments}")
                }
                Operation::NativeBaseInitialize { base } => {
                    format!(".base-initialize {}", base.text())
                }
                Operation::ReturnVoid => ".return-void".into(),
                Operation::ParameterRead { result, index } => {
                    format!(".parameter %{} {index}", result.0)
                }
                Operation::LocalInitialize { local, value } => {
                    format!(".local ${} %{}", local.0, value.0)
                }
                Operation::LocalAssign { local, value } => {
                    format!(".assign ${} %{}", local.0, value.0)
                }
                Operation::LocalRead { result, local } => {
                    format!(".read %{} ${}", result.0, local.0)
                }
                Operation::Return { value } => format!(".return %{}", value.0),
            });
            if let Some(span) = operation.span {
                text.push_str(&format!(" span={}:{}", span.start, span.end));
            }
            text.push('\n');
        }
        text.push_str(".end\n");
        text
    }
}

/// Parse one supported Monkey C function into validated semantic IR.
///
/// Accepts leading native name declarations for Application and System and
/// resolves their supported aliases before IR construction.
/// Resolves parameters and scoped locals, lowers conditions and while loops
/// (including innermost break/continue) to blocks, and retains source spans.
/// Value functions require a consistent typed return on every path;
/// otherwise bare returns and fallthrough produce void returns. Mixed return
/// types, unreachable statements, and local/parameter shadowing are rejected.
/// Assignments require a visible initialized local and preserve its supported type.
/// Unsupported syntax, unresolved names, and invalid IR produce source
/// diagnostics rather than partial output. VM instruction selection and symbol
/// binding occur later.
pub fn compile(source: &str) -> Result<Function, Diagnostic> {
    source_ir::build(source, parser::parse(source)?, None)
}

/// Parse a supported function and check unguarded API references against explicit
/// target and deployment levels before lowering support. No device is inferred.
pub fn compile_with_api(source: &str, context: api::ApiContext) -> Result<Function, Diagnostic> {
    source_ir::build(source, parser::parse(source)?, Some(context))
}

fn validate_call_names(module: Option<&str>, member: &str) -> Result<(), &'static str> {
    let valid = |name: &str| {
        name.bytes()
            .next()
            .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
            && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
    };
    if !valid(member) || module.is_some_and(|m| !valid(m)) {
        return Err("invalid module or member name");
    }
    if module == Some("System") {
        return Err("System calls require supported native statement forms");
    }
    if module == Some("self") {
        return Err("self calls require explicit receiver IR operations");
    }
    Ok(())
}
fn validate_call_arguments(
    arguments: &[ValueId],
    types: &std::collections::BTreeMap<ValueId, ValueType>,
) -> Result<(), &'static str> {
    if arguments.iter().any(|v| {
        types.get(v).is_some_and(|t| {
            !matches!(
                t,
                ValueType::Integer
                    | ValueType::String
                    | ValueType::Boolean
                    | ValueType::Char
                    | ValueType::Object(_)
                    | ValueType::Array(_)
                    | ValueType::Null
                    | ValueType::NullableInteger
                    | ValueType::NullableString
            )
        })
    }) {
        return Err("user calls require supported scalar, array, or nullable arguments");
    }
    if arguments.len() > MAX_PARAMETERS {
        return Err("call exceeds 254-argument limit");
    }
    Ok(())
}

fn call_result_type(result: ReturnType) -> Result<ValueType, &'static str> {
    match result {
        ReturnType::NullableInteger => Ok(ValueType::NullableInteger),
        ReturnType::NullableString => Ok(ValueType::NullableString),
        ReturnType::Integer => Ok(ValueType::Integer),
        ReturnType::String => Ok(ValueType::String),
        ReturnType::Boolean => Ok(ValueType::Boolean),
        ReturnType::Char => Ok(ValueType::Char),
        ReturnType::Object(class) => Ok(ValueType::Object(class)),
        ReturnType::SourceObject(class) => {
            validate_call_names(None, &class)?;
            Ok(ValueType::Object(NativeClass::View))
        }
        ReturnType::Tuple(shape) => Ok(ValueType::Tuple(shape)),
        ReturnType::Array(element) => Ok(ValueType::Array(element)),
        ReturnType::Void => Err("void function cannot be used as a value"),
    }
}

fn call_type_text(result: &ReturnType) -> String {
    match result {
        ReturnType::SourceObject(class) => return format!(" result=source({class})"),
        ReturnType::NullableInteger => " result=nullable-integer",
        ReturnType::NullableString => " result=nullable-string",
        ReturnType::Integer => "",
        ReturnType::String => " result=string",
        ReturnType::Boolean => " result=boolean",
        ReturnType::Char => " result=char",
        ReturnType::Object(NativeClass::View) => " result=view",
        ReturnType::Object(NativeClass::Dc) => " result=dc",
        ReturnType::Object(NativeClass::BehaviorDelegate) => " result=behavior-delegate",
        ReturnType::Tuple(TupleShape::OneView) => " result=one-view",
        ReturnType::Tuple(TupleShape::ViewDelegate) => " result=view-delegate",
        ReturnType::Array(ArrayElement::Char) => " result=char-array",
        ReturnType::Array(ArrayElement::Number) => " result=number-array",
        ReturnType::Array(ArrayElement::Object(NativeClass::View)) => " result=view-array",
        ReturnType::Array(ArrayElement::Object(NativeClass::Dc)) => " result=dc-array",
        ReturnType::Array(ArrayElement::Object(NativeClass::BehaviorDelegate)) => {
            " result=behavior-delegate-array"
        }
        ReturnType::Void => unreachable!("validated value calls cannot be void"),
    }
    .to_owned()
}
