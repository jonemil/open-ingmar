/// Named operation forms used while checking operands and constructing semantic IR.
/// Several spellings may share a form, such as `.read` and `.take`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Directive {
    Integer,
    Return,
    Add,
    Subtract,
    Local,
    DeclareLocal,
    Read,
    Call,
    Parameter,
    String,
    Println,
    Block,
    Branch,
    Jump,
    Equal,
    Greater,
    Assign,
    AssignParameter,
    ReturnVoid,
    CallDiscard,
    CallGlobal,
    CallGlobalDiscard,
    CallSelf,
    CallSelfDiscard,
    BaseInitialize,
    Null,
    CallNative,
    Boolean,
    CallNativeModuleDiscard,
    Char,
    Negate,
    Not,
    ArrayRead,
    Discard,
    ArrayNew,
    NewNative,
    TupleNew,
    NewSource,
    Constant,
    CallNativeDiscard,
    ReadSelf,
    AssignSelf,
    CallObject,
    CallObjectDiscard,
    Multiply,
    Divide,
    Remainder,
    Less,
    LessEqual,
    GreaterEqual,
    NotEqual,
    ArrayWrite,
    BitAnd,
    BitOr,
    BitXor,
    ShiftLeft,
    ShiftRight,
    Positive,
    Complement,
    ArrayBegin,
    ArrayFill,
    ArrayFinish,
    ArrayFillMany,
    ArrayConstant,
    Increment,
    Decrement,
    BlockValue,
    JumpValue,
    DictionaryNew,
    DictionaryConstant,
    DictionaryRead,
    DictionaryWrite,
    DictionaryBegin,
    DictionaryFill,
    DictionaryFinish,
    Long,
    Float,
    Double,
    Symbol,
    SelfValue,
    Instanceof,
    Has,
    Cast,
    ArrayAllocate,
    ByteArrayNew,
    ByteArrayConstant,
    ByteArrayBegin,
    ByteOptionsBegin,
    CallNativeModule,
}

use crate::{
    BinaryOp, BlockId, ComparisonOp, Diagnostic, Function, LocalId, LocatedOperation, Operation,
    Span, ValueId,
};

/// Parse provisional `ciq-ir` text into a validated semantic function.
///
/// Applies the same validation as typed [`Function`] construction. Invalid
/// syntax or semantic operations produce diagnostics at input byte offsets.
pub fn parse_ir(source: &str) -> Result<Function, Diagnostic> {
    parse_ir_context(source, None)
}

/// Parse function IR with explicit unguarded API-level checks. The context is
/// a build input and is not read from, or written into, semantic IR text.
pub fn parse_ir_with_api(
    source: &str,
    context: crate::api::ApiContext,
) -> Result<Function, Diagnostic> {
    parse_ir_context(source, Some(context))
}

pub(crate) fn parse_ir_context(
    source: &str,
    context: Option<crate::api::ApiContext>,
) -> Result<Function, Diagnostic> {
    let mut rows = Vec::new();
    let mut offset = 0;
    for line in source.split_inclusive('\n') {
        let mut quoted = false;
        let mut escaped = false;
        for (column, c) in line.char_indices() {
            if escaped {
                escaped = false;
            } else if quoted && c == '\\' {
                escaped = true;
            } else if c == '"' {
                quoted = !quoted;
            } else if !quoted && c == '#' {
                break;
            } else if !quoted && c == ',' {
                return Err(Diagnostic::at(
                    source,
                    offset + column,
                    "IR fields require whitespace separators",
                ));
            }
        }
        let fields = ciq_prg::symbols::source::tokens(line)
            .map_err(|error| Diagnostic::at(source, offset, error.to_string()))?;
        if !fields.is_empty() {
            rows.push((offset, fields));
        }
        offset += line.len();
    }
    let fail = |offset, message: &str| Diagnostic::at(source, offset, message);
    if !rows
        .first()
        .is_some_and(|(_, fields)| fields == &["ciq-ir"])
    {
        return Err(fail(
            rows.first().map_or(0, |row| row.0),
            "expected 'ciq-ir' header",
        ));
    }
    let Some((name_at, declaration)) = rows.get(1) else {
        return Err(fail(source.len(), "expected .function <name>"));
    };
    if !(2..=6).contains(&declaration.len()) || declaration[0] != ".function" {
        return Err(fail(*name_at, "expected .function <name>"));
    }
    let mut parameter_count = 0;
    let mut parameters = false;
    let mut parameter_types = None;
    let mut instance_method = false;
    let mut result_type = None;
    for field in declaration.iter().skip(2) {
        if let Some(result) = field.strip_prefix("result=") {
            if result_type.is_some() {
                return Err(fail(*name_at, "duplicate result contract"));
            }
            result_type = Some(
                crate::type_text::function_result(result)
                    .map_err(|message| fail(*name_at, message))?,
            );
        } else if field == "receiver=instance" && !instance_method {
            instance_method = true;
        } else if let Some(types) = field.strip_prefix("parameter-types=") {
            if parameter_types.is_some() {
                return Err(fail(*name_at, "duplicate parameter types"));
            }
            parameter_types = Some(
                types
                    .split(':')
                    .map(|tag| {
                        crate::type_text::parameter(tag).map_err(|message| fail(*name_at, message))
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            );
        } else if let Some(digits) = field
            .strip_prefix("parameters=")
            .filter(|s| !parameters && !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
        {
            parameter_count = digits
                .parse()
                .map_err(|_| fail(*name_at, "parameter count exceeds usize"))?;
            parameters = true;
        } else {
            return Err(fail(*name_at, "invalid or duplicate function metadata"));
        }
    }
    if parameter_count > crate::MAX_PARAMETERS {
        return Err(fail(*name_at, "function exceeds 254-parameter limit"));
    }
    let mut operations = Vec::new();
    let mut operation_offsets = Vec::new();
    let mut ended = false;
    for (index, (at, fields)) in rows.iter().enumerate().skip(2) {
        if fields.as_slice() == [".end"] {
            if index + 1 != rows.len() {
                return Err(fail(rows[index + 1].0, "unexpected input after .end"));
            }
            ended = true;
            break;
        }
        let call_width =
            || fields.len() - usize::from(fields.last().is_some_and(|f| f.starts_with("span=")));
        let (directive, width) = match fields.first().map(String::as_str) {
            Some(".call-global") => (Directive::CallGlobal, call_width()),
            Some(".call-global-discard") => (Directive::CallGlobalDiscard, call_width()),
            Some(".call-self") => (Directive::CallSelf, call_width()),
            Some(".call-self-discard") => (Directive::CallSelfDiscard, call_width()),
            Some(".return-void") => (Directive::ReturnVoid, 1),
            Some(".call-discard") => (
                Directive::CallDiscard,
                fields.len() - usize::from(fields.last().is_some_and(|f| f.starts_with("span="))),
            ),
            Some(".call-object") => (Directive::CallObject, call_width()),
            Some(".call-object-discard") => (Directive::CallObjectDiscard, call_width()),
            Some(".integer") => (Directive::Integer, 3),
            Some(".read-self") => (Directive::ReadSelf, call_width()),
            Some(".assign-self") => (Directive::AssignSelf, 3),
            Some(".boolean") => (Directive::Boolean, 3),
            Some(".char") => (Directive::Char, 3),
            Some(".long") => (Directive::Long, 3),
            Some(".float") => (Directive::Float, 3),
            Some(".double") => (Directive::Double, 3),
            Some(".increment") => (Directive::Increment, 3),
            Some(".decrement") => (Directive::Decrement, 3),
            Some(".positive") => (Directive::Positive, 3),
            Some(".complement") => (Directive::Complement, 3),
            Some(".negate") => (Directive::Negate, 3),
            Some(".not") => (Directive::Not, 3),
            Some(".instanceof") => (Directive::Instanceof, 4),
            Some(".has") => (Directive::Has, 4),
            Some(".cast") => (Directive::Cast, 4),
            Some(".array-read") => (Directive::ArrayRead, 4),
            Some(".dictionary-read") => (Directive::DictionaryRead, 4),
            Some(".dictionary-write") => (Directive::DictionaryWrite, 4),
            Some(".array-write") => (Directive::ArrayWrite, 4),
            Some(".call-native-module-discard") => {
                (Directive::CallNativeModuleDiscard, call_width())
            }
            Some(".call-native-module") => (Directive::CallNativeModule, call_width()),
            Some(".return") => (Directive::Return, 2),
            Some(".add") => (Directive::Add, 4),
            Some(".subtract") => (Directive::Subtract, 4),
            Some(".multiply") => (Directive::Multiply, 4),
            Some(".divide") => (Directive::Divide, 4),
            Some(".remainder") => (Directive::Remainder, 4),
            Some(".bit-and") => (Directive::BitAnd, 4),
            Some(".bit-or") => (Directive::BitOr, 4),
            Some(".bit-xor") => (Directive::BitXor, 4),
            Some(".shift-left") => (Directive::ShiftLeft, 4),
            Some(".shift-right") => (Directive::ShiftRight, 4),
            Some(".local") => (Directive::Local, 3),
            Some(".declare-local") => (Directive::DeclareLocal, 3),
            Some(".read" | ".take") => (Directive::Read, 3),
            Some(".symbol") => (Directive::Symbol, 3),
            Some(".self") => (Directive::SelfValue, 2),
            Some(".call") => (
                Directive::Call,
                fields.len()
                    - usize::from(
                        fields
                            .last()
                            .is_some_and(|field| field.starts_with("span=")),
                    ),
            ),
            Some(".parameter") => (Directive::Parameter, 3),
            Some(".string") => (Directive::String, 3),
            Some(".println") => (Directive::Println, 2),
            Some(".discard") => (Directive::Discard, 2),
            Some(".array-constant") => (Directive::ArrayConstant, call_width()),
            Some(".array-new") => (Directive::ArrayNew, 3),
            Some(".array-allocate") => (Directive::ArrayAllocate, 4),
            Some(".byte-array-new") => (Directive::ByteArrayNew, 3),
            Some(".byte-options-begin") => (Directive::ByteOptionsBegin, 3),
            Some(".byte-array-constant") => (Directive::ByteArrayConstant, 3),
            Some(".byte-array-begin") => (Directive::ByteArrayBegin, 3),
            Some(".dictionary-begin") => (Directive::DictionaryBegin, 4),
            Some(".dictionary-fill") => (Directive::DictionaryFill, call_width()),
            Some(".dictionary-finish") => (Directive::DictionaryFinish, 3),
            Some(".array-begin") => (Directive::ArrayBegin, 4),
            Some(".array-fill") => (Directive::ArrayFill, 4),
            Some(".array-finish") => (Directive::ArrayFinish, 3),
            Some(".array-fill-many") => (Directive::ArrayFillMany, call_width()),
            Some(".new-native") => (Directive::NewNative, 3),
            Some(".tuple-new") => (Directive::TupleNew, call_width()),
            Some(".new-source") => (Directive::NewSource, call_width()),
            Some(".constant") => (Directive::Constant, 4),
            Some(".call-native-discard") => (Directive::CallNativeDiscard, call_width()),
            Some(".dictionary-constant") => (Directive::DictionaryConstant, call_width()),
            Some(".dictionary-new") => (
                Directive::DictionaryNew,
                if fields.get(2).is_some_and(|s| s.starts_with("type=")) {
                    3
                } else {
                    2
                },
            ),
            Some(".block") => (Directive::Block, 2),
            Some(".block-value") => (Directive::BlockValue, 3),
            Some(".jump-value") => (Directive::JumpValue, 3),
            Some(".branch") => (Directive::Branch, 4),
            Some(".jump") => (Directive::Jump, 2),
            Some(".equal") => (Directive::Equal, 4),
            Some(".greater") => (Directive::Greater, 4),
            Some(".less") => (Directive::Less, 4),
            Some(".less-equal") => (Directive::LessEqual, 4),
            Some(".greater-equal") => (Directive::GreaterEqual, 4),
            Some(".not-equal") => (Directive::NotEqual, 4),
            Some(".null") => (Directive::Null, 2),
            Some(".call-native") => (Directive::CallNative, call_width()),
            Some(".assign") => (Directive::Assign, 3),
            Some(".assign-parameter") => (Directive::AssignParameter, 3),
            Some(".base-initialize") => (Directive::BaseInitialize, 2),
            _ => {
                return Err(fail(*at, "unknown IR directive"));
            }
        };
        if ((directive == Directive::Call && width < 4)
            || (directive == Directive::CallDiscard && width < 3)
            || (matches!(directive, Directive::CallSelf | Directive::CallGlobal) && width < 3)
            || (matches!(
                directive,
                Directive::CallSelfDiscard | Directive::CallGlobalDiscard
            ) && width < 2)
            || (directive == Directive::CallNative && width < 4))
            || (directive == Directive::CallNativeModuleDiscard && width < 3)
            || (directive == Directive::CallNativeModule && width < 4)
            || fields.len() < width
            || fields.len() > width + 1
        {
            return Err(fail(*at, "incorrect IR operation field count"));
        }
        let parse_value = |text: &str| -> Result<ValueId, Diagnostic> {
            let digits = text
                .strip_prefix('%')
                .filter(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
                .ok_or_else(|| fail(*at, "value IDs use % followed by decimal u32"))?;
            Ok(ValueId(
                digits
                    .parse()
                    .map_err(|_| fail(*at, "value ID exceeds u32"))?,
            ))
        };
        let parse_local = |text: &str| -> Result<LocalId, Diagnostic> {
            let digits = text
                .strip_prefix('$')
                .filter(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
                .ok_or_else(|| fail(*at, "local IDs use $ followed by decimal u32"))?;
            Ok(LocalId(
                digits
                    .parse()
                    .map_err(|_| fail(*at, "local ID exceeds u32"))?,
            ))
        };
        let parse_builder = |text: &str| -> Result<crate::BuilderId, Diagnostic> {
            let digits = text
                .strip_prefix('&')
                .filter(|d| !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit()))
                .ok_or_else(|| fail(*at, "builder IDs use & followed by decimal u32"))?;
            Ok(crate::BuilderId(
                digits
                    .parse()
                    .map_err(|_| fail(*at, "builder ID exceeds u32"))?,
            ))
        };
        let parse_count = |text: &str| -> Result<u32, Diagnostic> {
            if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
                return Err(fail(*at, "builder lengths and indexes require decimal u32"));
            }
            text.parse()
                .map_err(|_| fail(*at, "builder count exceeds u32"))
        };
        let parse_block = |text: &str| -> Result<BlockId, Diagnostic> {
            let digits = text
                .strip_prefix('^')
                .filter(|d| !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit()))
                .ok_or_else(|| fail(*at, "block IDs use ^ followed by decimal u32"))?;
            Ok(BlockId(
                digits
                    .parse()
                    .map_err(|_| fail(*at, "block ID exceeds u32"))?,
            ))
        };
        let span = if let Some(field) = fields.get(width) {
            let (start, end) = field
                .strip_prefix("span=")
                .and_then(|s| s.split_once(':'))
                .ok_or_else(|| fail(*at, "expected span=<start>:<end>"))?;
            let position = |digits: &str| {
                if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
                    return Err(fail(*at, "span offsets must be decimal usize"));
                }
                digits
                    .parse()
                    .map_err(|_| fail(*at, "span offset exceeds usize"))
            };
            Some(Span {
                start: position(start)?,
                end: position(end)?,
            })
        } else {
            None
        };
        let operation = if matches!(directive, Directive::Instanceof | Directive::Cast)
            && (fields[3].starts_with("source(") || fields[3].starts_with("nullable-source("))
        {
            let (class, nullable) = match crate::type_text::call_result(&fields[3]) {
                Ok(crate::ReturnType::SourceObject(class)) => (class, false),
                Ok(crate::ReturnType::NullableSourceObject(class))
                    if directive == Directive::Cast =>
                {
                    (class, true)
                }
                _ => return Err(fail(*at, "invalid source type target")),
            };
            let result = parse_value(&fields[1])?;
            let operand = parse_value(&fields[2])?;
            if directive == Directive::Instanceof {
                Operation::SourceInstanceOf {
                    result,
                    operand,
                    class,
                }
            } else {
                Operation::SourceCast {
                    result,
                    operand,
                    class,
                    nullable,
                }
            }
        } else {
            match directive {
                Directive::Cast => Operation::Cast {
                    result: parse_value(&fields[1])?,
                    operand: parse_value(&fields[2])?,
                    target: crate::casts::parse_type(&fields[3])
                        .ok_or_else(|| fail(*at, "unsupported lexical cast target"))?,
                },
                Directive::Instanceof => Operation::InstanceOf {
                    result: parse_value(&fields[1])?,
                    operand: parse_value(&fields[2])?,
                    class: fields[3].clone(),
                },
                Directive::Has => Operation::Has {
                    result: parse_value(&fields[1])?,
                    receiver: parse_value(&fields[2])?,
                    symbol: parse_value(&fields[3])?,
                },
                Directive::CallObject => {
                    if width < 5 {
                        return Err(fail(
                            *at,
                            "expected .call-object result receiver class member [arguments]",
                        ));
                    }
                    let (result_type, arguments_at) = call_contract(fields, 5, *at, source)?;
                    Operation::ObjectCall {
                        result: parse_value(&fields[1])?,
                        receiver: parse_value(&fields[2])?,
                        class: fields[3].clone(),
                        member: fields[4].clone(),
                        result_type,
                        arguments: fields[arguments_at..width]
                            .iter()
                            .map(|f| parse_value(f))
                            .collect::<Result<_, _>>()?,
                    }
                }
                Directive::CallObjectDiscard => {
                    if width < 4 {
                        return Err(fail(
                            *at,
                            "expected .call-object-discard receiver class member [arguments]",
                        ));
                    }
                    Operation::ObjectCallStatement {
                        receiver: parse_value(&fields[1])?,
                        class: fields[2].clone(),
                        member: fields[3].clone(),
                        arguments: fields[4..width]
                            .iter()
                            .map(|f| parse_value(f))
                            .collect::<Result<_, _>>()?,
                    }
                }
                Directive::ReadSelf => {
                    if width < 3 {
                        return Err(fail(*at, "expected .read-self result field [result=type]"));
                    }
                    let (result_type, arguments_at) = call_contract(fields, 3, *at, source)?;
                    if arguments_at != width {
                        return Err(fail(*at, "unexpected field read operands"));
                    }
                    Operation::SelfFieldRead {
                        result: parse_value(&fields[1])?,
                        field: fields[2].clone(),
                        result_type,
                    }
                }
                Directive::AssignSelf => Operation::SelfFieldAssign {
                    field: fields[1].clone(),
                    value: parse_value(&fields[2])?,
                },
                Directive::NewSource => {
                    if width < 4 {
                        return Err(fail(
                            *at,
                            "expected .new-source result class base [arguments]",
                        ));
                    }
                    Operation::SourceNew {
                        result: parse_value(&fields[1])?,
                        class: fields[2].clone(),
                        base: match fields[3].as_str() {
                            "Lang.Object" => crate::ClassBase::LangObject,
                            "WatchUi.View" => crate::ClassBase::WatchUiView,
                            "WatchUi.BehaviorDelegate" => crate::ClassBase::WatchUiBehaviorDelegate,
                            "Application.AppBase" => crate::ClassBase::ApplicationAppBase,
                            _ => return Err(fail(*at, "unsupported source construction base")),
                        },
                        arguments: fields[4..width]
                            .iter()
                            .map(|field| parse_value(field))
                            .collect::<Result<_, _>>()?,
                    }
                }
                Directive::Constant => Operation::NativeConstant {
                    result: parse_value(&fields[1])?,
                    module: fields[2].clone(),
                    member: fields[3].clone(),
                },
                Directive::CallNativeDiscard => Operation::NativeCallStatement {
                    receiver: parse_value(&fields[1])?,
                    member: fields[2].clone(),
                    arguments: fields[3..width]
                        .iter()
                        .map(|v| parse_value(v))
                        .collect::<Result<_, _>>()?,
                },
                Directive::TupleNew => {
                    if !(3..=4).contains(&width) {
                        return Err(fail(*at, "tuple allocation requires one or two slots"));
                    }
                    Operation::TupleNew {
                        result: fields
                            .get(1)
                            .ok_or_else(|| fail(*at, "tuple allocation requires a result"))
                            .and_then(|field| parse_value(field))?,
                        elements: fields
                            .iter()
                            .take(width)
                            .skip(2)
                            .map(|field| parse_value(field))
                            .collect::<Result<_, _>>()?,
                    }
                }
                Directive::NewNative => {
                    if fields[2] == "Toybox.Lang.WeakReference" {
                        Operation::WeakReferenceNew {
                            result: parse_value(&fields[1])?,
                        }
                    } else {
                        Operation::NativeNew {
                            result: parse_value(&fields[1])?,
                            class: crate::NativeClass::from_name(&fields[2]).ok_or_else(|| {
                                fail(*at, "unsupported native construction class")
                            })?,
                        }
                    }
                }
                Directive::ArrayConstant => {
                    if width < 4 {
                        return Err(fail(*at, "constant array requires a type and elements"));
                    }
                    Operation::ArrayConstant {
                        result: parse_value(&fields[1])?,
                        element_type: crate::type_text::element(&fields[2])
                            .filter(|e| !matches!(e, crate::ArrayElement::Object(_)))
                            .ok_or_else(|| {
                                fail(*at, "constant arrays require supported scalar elements")
                            })?,
                        values: if crate::NumericAlias::parse_tag(&fields[2]).is_some()
                            || matches!(
                                fields[2].as_str(),
                                "comparable"
                                    | "nullable-comparable"
                                    | "comparator"
                                    | "nullable-comparator"
                                    | "object"
                                    | "nullable-object"
                            ) {
                            let mut values = Vec::new();
                            let mut next = 3;
                            while next < width {
                                values.push(
                                    crate::ArrayConstantValue::parse(&fields[..width], &mut next)
                                        .ok_or_else(|| {
                                        fail(*at, "invalid tagged scalar Array constant")
                                    })?,
                                );
                            }
                            values
                        } else {
                            fields[3..width]
                                .iter()
                                .map(|s| {
                                    if s == "null" {
                                        return Ok(crate::ArrayConstantValue::Null);
                                    }
                                    if matches!(fields[2].as_str(), "boolean" | "nullable-boolean")
                                    {
                                        return match s.as_str() {
                                            "true" => Ok(crate::ArrayConstantValue::Boolean(true)),
                                            "false" => {
                                                Ok(crate::ArrayConstantValue::Boolean(false))
                                            }
                                            _ => Err(fail(
                                                *at,
                                                "Boolean array constants require true or false",
                                            )),
                                        };
                                    }
                                    if matches!(fields[2].as_str(), "string" | "nullable-string") {
                                        return s
                                            .strip_prefix('"')
                                            .map(|s| crate::ArrayConstantValue::String(s.into()))
                                            .ok_or_else(|| {
                                                fail(
                                                    *at,
                                                    "String array constants require quoted text",
                                                )
                                            });
                                    }
                                    if matches!(
                                        fields[2].as_str(),
                                        "long"
                                            | "nullable-long"
                                            | "double"
                                            | "nullable-double"
                                            | "float"
                                            | "nullable-float"
                                    ) {
                                        let wide = !matches!(
                                            fields[2].as_str(),
                                            "float" | "nullable-float"
                                        );
                                        let hex = s
                                            .strip_prefix("0x")
                                            .filter(|s| s.len() == if wide { 16 } else { 8 })
                                            .ok_or_else(|| {
                                                fail(
                                                    *at,
                                                    "numeric array constant has wrong bit width",
                                                )
                                            })?;
                                        if !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
                                            return Err(fail(
                                                *at,
                                                "invalid numeric array constant bits",
                                            ));
                                        }
                                        let bits = u64::from_str_radix(hex, 16)
                                            .map_err(|_| fail(*at, "invalid numeric array bits"))?;
                                        return Ok(match fields[2].as_str() {
                                            "long" | "nullable-long" => {
                                                crate::ArrayConstantValue::Long(bits)
                                            }
                                            "double" | "nullable-double" => {
                                                crate::ArrayConstantValue::Double(bits)
                                            }
                                            _ => crate::ArrayConstantValue::Float(bits as u32),
                                        });
                                    }
                                    let hex = s
                                        .strip_prefix("0x")
                                        .filter(|s| {
                                            s.len() == 8 && s.bytes().all(|b| b.is_ascii_hexdigit())
                                        })
                                        .ok_or_else(|| {
                                            fail(*at, "array constants require eight hex digits")
                                        })?;
                                    u32::from_str_radix(hex, 16)
                                        .map(|bits| {
                                            if matches!(
                                                fields[2].as_str(),
                                                "char" | "nullable-char"
                                            ) {
                                                crate::ArrayConstantValue::Char(bits)
                                            } else {
                                                crate::ArrayConstantValue::Number(bits)
                                            }
                                        })
                                        .map_err(|_| fail(*at, "invalid array constant"))
                                })
                                .collect::<Result<_, _>>()?
                        },
                    }
                }
                Directive::ArrayBegin => Operation::ArrayBegin {
                    builder: parse_builder(&fields[1])?,
                    element_type: crate::type_text::element(&fields[2])
                        .ok_or_else(|| fail(*at, "unsupported builder element type"))?,
                    length: parse_count(&fields[3])?,
                },
                Directive::ArrayFillMany => {
                    if width < 4 {
                        return Err(fail(*at, "array fill group requires at least one value"));
                    }
                    Operation::ArrayFillMany {
                        builder: parse_builder(&fields[1])?,
                        start: parse_count(&fields[2])?,
                        values: fields[3..width]
                            .iter()
                            .map(|v| parse_value(v))
                            .collect::<Result<_, _>>()?,
                    }
                }
                Directive::ArrayFill => Operation::ArrayFill {
                    builder: parse_builder(&fields[1])?,
                    index: parse_count(&fields[2])?,
                    value: parse_value(&fields[3])?,
                },
                Directive::ArrayFinish => Operation::ArrayFinish {
                    result: parse_value(&fields[1])?,
                    builder: parse_builder(&fields[2])?,
                },
                Directive::ByteOptionsBegin => Operation::ByteOptionsBegin {
                    builder: parse_builder(&fields[1])?,
                    length: fields[2]
                        .parse()
                        .map_err(|_| fail(*at, "invalid option field count"))?,
                },
                Directive::ByteArrayConstant => {
                    let hex = fields[2]
                        .strip_prefix("0x")
                        .ok_or_else(|| fail(*at, "ByteArray payload requires 0x hex pairs"))?;
                    if hex.len() % 2 != 0 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
                        return Err(fail(*at, "ByteArray payload requires hex pairs"));
                    }
                    Operation::ByteArrayConstant {
                        result: parse_value(&fields[1])?,
                        values: (0..hex.len())
                            .step_by(2)
                            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
                            .collect(),
                    }
                }
                Directive::ByteArrayBegin => Operation::ByteArrayBegin {
                    builder: parse_builder(&fields[1])?,
                    length: parse_count(&fields[2])?,
                },
                Directive::ByteArrayNew => Operation::ByteArrayAllocate {
                    result: parse_value(&fields[1])?,
                    length: parse_value(&fields[2])?,
                },
                Directive::ArrayNew | Directive::ArrayAllocate => {
                    let element_type = crate::type_text::element(&fields[2])
                        .ok_or_else(|| fail(*at, "unsupported array element type"))?;
                    let result = parse_value(&fields[1])?;
                    if directive == Directive::ArrayAllocate {
                        Operation::ArrayAllocate {
                            result,
                            element_type,
                            length: parse_value(&fields[3])?,
                        }
                    } else {
                        Operation::ArrayNew {
                            result,
                            element_type,
                        }
                    }
                }
                Directive::Discard => Operation::Discard {
                    value: parse_value(&fields[1])?,
                },
                Directive::DictionaryBegin => Operation::DictionaryBegin {
                    builder: parse_builder(&fields[1])?,
                    dictionary_type: crate::DictionaryType::parse(&fields[2])
                        .ok_or_else(|| fail(*at, "unsupported Dictionary contract"))?,
                    length: parse_count(&fields[3])?,
                },
                Directive::DictionaryFill => {
                    if width < 5 || (width - 3) % 2 != 0 {
                        return Err(fail(*at, "Dictionary fill requires key/value pairs"));
                    }
                    Operation::DictionaryFill {
                        builder: parse_builder(&fields[1])?,
                        start: parse_count(&fields[2])?,
                        entries: fields[3..width]
                            .chunks_exact(2)
                            .map(|pair| Ok((parse_value(&pair[0])?, parse_value(&pair[1])?)))
                            .collect::<Result<_, Diagnostic>>()?,
                    }
                }
                Directive::DictionaryFinish => Operation::DictionaryFinish {
                    result: parse_value(&fields[1])?,
                    builder: parse_builder(&fields[2])?,
                },
                Directive::DictionaryRead => Operation::DictionaryRead {
                    result: parse_value(&fields[1])?,
                    dictionary: parse_value(&fields[2])?,
                    key: parse_value(&fields[3])?,
                },
                Directive::DictionaryWrite => Operation::DictionaryWrite {
                    dictionary: parse_value(&fields[1])?,
                    key: parse_value(&fields[2])?,
                    value: parse_value(&fields[3])?,
                },
                Directive::ArrayWrite => Operation::ArrayWrite {
                    array: parse_value(&fields[1])?,
                    index: parse_value(&fields[2])?,
                    value: parse_value(&fields[3])?,
                },
                Directive::ArrayRead => Operation::ArrayRead {
                    result: parse_value(&fields[1])?,
                    array: parse_value(&fields[2])?,
                    index: parse_value(&fields[3])?,
                },
                Directive::Not => Operation::Not {
                    result: parse_value(&fields[1])?,
                    operand: parse_value(&fields[2])?,
                },
                Directive::Complement => Operation::Complement {
                    result: parse_value(&fields[1])?,
                    operand: parse_value(&fields[2])?,
                },
                Directive::Increment | Directive::Decrement => Operation::Step {
                    result: parse_value(&fields[1])?,
                    operand: parse_value(&fields[2])?,
                    increment: directive == Directive::Increment,
                },
                Directive::Positive => Operation::Positive {
                    result: parse_value(&fields[1])?,
                    operand: parse_value(&fields[2])?,
                },
                Directive::Negate => Operation::Negate {
                    result: parse_value(&fields[1])?,
                    operand: parse_value(&fields[2])?,
                },
                Directive::Char => {
                    let digits = fields[2]
                        .strip_prefix("0x")
                        .filter(|digits| {
                            digits.len() == 8 && digits.bytes().all(|b| b.is_ascii_hexdigit())
                        })
                        .ok_or_else(|| {
                            fail(*at, "Char literal requires 0x followed by eight hex digits")
                        })?;
                    Operation::Char {
                        result: parse_value(&fields[1])?,
                        value: u32::from_str_radix(digits, 16)
                            .map_err(|_| fail(*at, "invalid Char bits"))?,
                    }
                }
                Directive::CallNativeModule => Operation::NativeModuleCall {
                    result: parse_value(&fields[1])?,
                    module: fields[2].clone(),
                    member: fields[3].clone(),
                    arguments: fields[4..width]
                        .iter()
                        .map(|f| parse_value(f))
                        .collect::<Result<_, _>>()?,
                },
                Directive::CallNativeModuleDiscard => Operation::NativeModuleCallStatement {
                    module: fields[1].clone(),
                    member: fields[2].clone(),
                    arguments: fields[3..width]
                        .iter()
                        .map(|f| parse_value(f))
                        .collect::<Result<_, _>>()?,
                },
                Directive::Boolean => Operation::Boolean {
                    result: parse_value(&fields[1])?,
                    value: match fields[2].as_str() {
                        "true" => true,
                        "false" => false,
                        _ => return Err(fail(*at, "Boolean literal requires true or false")),
                    },
                },
                Directive::CallNative => Operation::NativeCall {
                    result: parse_value(&fields[1])?,
                    receiver: parse_value(&fields[2])?,
                    member: fields[3].clone(),
                    arguments: fields[4..width]
                        .iter()
                        .map(|f| parse_value(f))
                        .collect::<Result<_, _>>()?,
                },
                Directive::Null => Operation::Null {
                    result: parse_value(&fields[1])?,
                },
                Directive::BaseInitialize => Operation::NativeBaseInitialize {
                    base: match fields[1].as_str() {
                        "Application.AppBase" => crate::ClassBase::ApplicationAppBase,
                        "WatchUi.View" => crate::ClassBase::WatchUiView,
                        "WatchUi.BehaviorDelegate" => crate::ClassBase::WatchUiBehaviorDelegate,
                        _ => return Err(fail(*at, "unsupported native initializer base")),
                    },
                },
                Directive::CallGlobal => {
                    let (result_type, arguments_at) = call_contract(fields, 3, *at, source)?;
                    Operation::GlobalCall {
                        result: parse_value(&fields[1])?,
                        result_type,
                        member: fields[2].clone(),
                        arguments: fields[arguments_at..width]
                            .iter()
                            .map(|f| parse_value(f))
                            .collect::<Result<_, _>>()?,
                    }
                }
                Directive::CallGlobalDiscard => Operation::GlobalCallStatement {
                    member: fields[1].clone(),
                    arguments: fields[2..width]
                        .iter()
                        .map(|f| parse_value(f))
                        .collect::<Result<_, _>>()?,
                },
                Directive::CallSelf => {
                    let (result_type, arguments_at) = call_contract(fields, 3, *at, source)?;
                    Operation::SelfCall {
                        result: parse_value(&fields[1])?,
                        result_type,
                        member: fields[2].clone(),
                        arguments: fields[arguments_at..width]
                            .iter()
                            .map(|f| parse_value(f))
                            .collect::<Result<_, _>>()?,
                    }
                }
                Directive::CallSelfDiscard => Operation::SelfCallStatement {
                    member: fields[1].clone(),
                    arguments: fields[2..width]
                        .iter()
                        .map(|f| parse_value(f))
                        .collect::<Result<_, _>>()?,
                },
                Directive::ReturnVoid => Operation::ReturnVoid,
                Directive::CallDiscard => Operation::ModuleCallStatement {
                    module: fields[1].clone(),
                    member: fields[2].clone(),
                    arguments: fields[3..width]
                        .iter()
                        .map(|f| parse_value(f))
                        .collect::<Result<_, _>>()?,
                },
                Directive::Assign => Operation::LocalAssign {
                    local: parse_local(&fields[1])?,
                    value: parse_value(&fields[2])?,
                },
                Directive::AssignParameter => {
                    let digits = &fields[1];
                    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
                        return Err(fail(*at, "parameter index requires decimal usize"));
                    }
                    Operation::ParameterAssign {
                        index: digits
                            .parse()
                            .map_err(|_| fail(*at, "parameter index requires decimal usize"))?,
                        value: parse_value(&fields[2])?,
                    }
                }
                Directive::DictionaryConstant => {
                    if width < 5 {
                        return Err(fail(*at, "Dictionary constant requires a type and entries"));
                    }
                    let dictionary_type = crate::DictionaryType::parse(&fields[2])
                        .ok_or_else(|| fail(*at, "unsupported Dictionary contract"))?;
                    let mut next = 3;
                    let mut entries = Vec::new();
                    while next < width {
                        let key = crate::DictionaryScalar::parse(&fields[..width], &mut next)
                            .ok_or_else(|| fail(*at, "invalid Dictionary scalar key"))?;
                        let value = crate::DictionaryScalar::parse(&fields[..width], &mut next)
                            .ok_or_else(|| fail(*at, "invalid Dictionary scalar value"))?;
                        entries.push((key, value));
                    }
                    Operation::DictionaryConstant {
                        result: parse_value(&fields[1])?,
                        dictionary_type,
                        entries,
                    }
                }
                Directive::DictionaryNew => Operation::DictionaryNew {
                    dictionary_type: if width == 3 {
                        Some(
                            crate::DictionaryType::parse(fields[2].strip_prefix("type=").unwrap())
                                .ok_or_else(|| fail(*at, "unsupported Dictionary contract"))?,
                        )
                    } else {
                        None
                    },
                    result: parse_value(&fields[1])?,
                },
                Directive::BlockValue => Operation::BlockParameter {
                    result: parse_value(&fields[1])?,
                    value_type: crate::type_text::local_contract(&fields[2])
                        .ok_or_else(|| fail(*at, "unsupported block value type"))?,
                },
                Directive::JumpValue => Operation::JumpValue {
                    target: parse_block(&fields[1])?,
                    value: parse_value(&fields[2])?,
                },
                Directive::Block => Operation::Block {
                    block: parse_block(&fields[1])?,
                },
                Directive::Branch => Operation::Branch {
                    condition: parse_value(&fields[1])?,
                    on_true: parse_block(&fields[2])?,
                    on_false: parse_block(&fields[3])?,
                },
                Directive::Jump => Operation::Jump {
                    target: parse_block(&fields[1])?,
                },
                Directive::Equal
                | Directive::Greater
                | Directive::Less
                | Directive::LessEqual
                | Directive::GreaterEqual
                | Directive::NotEqual => Operation::Compare {
                    result: parse_value(&fields[1])?,
                    kind: match directive {
                        Directive::Equal => ComparisonOp::Equal,
                        Directive::Greater => ComparisonOp::Greater,
                        Directive::Less => ComparisonOp::Less,
                        Directive::LessEqual => ComparisonOp::LessEqual,
                        Directive::GreaterEqual => ComparisonOp::GreaterEqual,
                        Directive::NotEqual => ComparisonOp::NotEqual,
                        _ => unreachable!(),
                    },
                    left: parse_value(&fields[2])?,
                    right: parse_value(&fields[3])?,
                },
                Directive::String => {
                    let value = fields[2]
                        .strip_prefix('"')
                        .ok_or_else(|| fail(*at, "string values require quotes"))?;
                    Operation::String {
                        result: parse_value(&fields[1])?,
                        value: value.into(),
                    }
                }
                Directive::Println => Operation::Println {
                    value: parse_value(&fields[1])?,
                },
                Directive::Float | Directive::Double => {
                    let digits = fields[2]
                        .strip_prefix("0x")
                        .ok_or_else(|| fail(*at, "floating bits require 0x hexadecimal"))?;
                    let width = if directive == Directive::Float { 8 } else { 16 };
                    if digits.len() != width || !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
                        return Err(fail(
                            *at,
                            "floating bits require exactly 8 or 16 hexadecimal digits for their width",
                        ));
                    }
                    let bits = u64::from_str_radix(digits, 16)
                        .map_err(|_| fail(*at, "invalid floating bits"))?;
                    let result = parse_value(&fields[1])?;
                    if directive == Directive::Float {
                        Operation::Float {
                            result,
                            bits: bits as u32,
                        }
                    } else {
                        Operation::Double { result, bits }
                    }
                }
                Directive::Long => {
                    let digits = &fields[2];
                    let magnitude = digits.strip_prefix('-').unwrap_or(digits);
                    if magnitude.is_empty() || !magnitude.bytes().all(|b| b.is_ascii_digit()) {
                        return Err(fail(
                            *at,
                            "long requires decimal digits with optional leading minus",
                        ));
                    }
                    Operation::Long {
                        result: parse_value(&fields[1])?,
                        value: digits.parse().map_err(|_| fail(*at, "long exceeds i64"))?,
                    }
                }
                Directive::SelfValue => Operation::SelfValue {
                    result: parse_value(&fields[1])?,
                },
                Directive::Symbol => Operation::Symbol {
                    result: parse_value(&fields[1])?,
                    name: fields[2].clone(),
                },
                Directive::Integer => {
                    let digits = &fields[2];
                    let magnitude = digits.strip_prefix('-').unwrap_or(digits);
                    if magnitude.is_empty() || !magnitude.bytes().all(|b| b.is_ascii_digit()) {
                        return Err(fail(
                            *at,
                            "integer requires decimal digits with optional leading minus",
                        ));
                    }
                    Operation::Integer {
                        result: parse_value(&fields[1])?,
                        value: digits
                            .parse()
                            .map_err(|_| fail(*at, "integer exceeds i32"))?,
                    }
                }
                Directive::Return => Operation::Return {
                    value: parse_value(&fields[1])?,
                },
                Directive::DeclareLocal => Operation::LocalDeclare {
                    local: parse_local(&fields[1])?,
                    value_type: crate::type_text::local_contract(&fields[2])
                        .ok_or_else(|| fail(*at, "invalid local declaration contract"))?,
                },
                Directive::Local => Operation::LocalInitialize {
                    local: parse_local(&fields[1])?,
                    value: parse_value(&fields[2])?,
                },
                Directive::Read => {
                    let result = parse_value(&fields[1])?;
                    let local = parse_local(&fields[2])?;
                    if fields[0] == ".take" {
                        Operation::LocalTake { result, local }
                    } else {
                        Operation::LocalRead { result, local }
                    }
                }
                Directive::Call => {
                    let (result_type, arguments_at) = call_contract(fields, 4, *at, source)?;
                    Operation::ModuleCall {
                        result: parse_value(&fields[1])?,
                        result_type,
                        module: fields[2].clone(),
                        member: fields[3].clone(),
                        arguments: fields[arguments_at..width]
                            .iter()
                            .map(|field| parse_value(field))
                            .collect::<Result<_, _>>()?,
                    }
                }
                Directive::Parameter => {
                    let digits = &fields[2];
                    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
                        return Err(fail(*at, "parameter index must be decimal usize"));
                    }
                    Operation::ParameterRead {
                        result: parse_value(&fields[1])?,
                        index: digits
                            .parse()
                            .map_err(|_| fail(*at, "parameter index exceeds usize"))?,
                    }
                }
                Directive::Add
                | Directive::Subtract
                | Directive::Multiply
                | Directive::Divide
                | Directive::Remainder
                | Directive::BitAnd
                | Directive::BitOr
                | Directive::BitXor
                | Directive::ShiftLeft
                | Directive::ShiftRight => Operation::Binary {
                    result: parse_value(&fields[1])?,
                    kind: match directive {
                        Directive::Add => BinaryOp::Add,
                        Directive::Subtract => BinaryOp::Subtract,
                        Directive::Multiply => BinaryOp::Multiply,
                        Directive::Divide => BinaryOp::Divide,
                        Directive::Remainder => BinaryOp::Remainder,
                        Directive::BitAnd => BinaryOp::BitAnd,
                        Directive::BitOr => BinaryOp::BitOr,
                        Directive::BitXor => BinaryOp::BitXor,
                        Directive::ShiftLeft => BinaryOp::ShiftLeft,
                        Directive::ShiftRight => BinaryOp::ShiftRight,
                        _ => unreachable!("all non-binary directives parsed above"),
                    },
                    left: parse_value(&fields[2])?,
                    right: parse_value(&fields[3])?,
                },
            }
        };
        operations.push(LocatedOperation { operation, span });
        operation_offsets.push(*at);
    }
    if !ended {
        return Err(fail(source.len(), "expected .end"));
    }
    let types = if let Some(types) = parameter_types {
        if types.len() != parameter_count {
            return Err(fail(*name_at, "parameter types require a matching count"));
        }
        types
    } else {
        vec![crate::ParameterType::Integer; parameter_count]
    };
    let result = Function::construct_for_api(
        declaration[1].to_string(),
        types,
        operations,
        instance_method,
        context,
        result_type,
    );
    result.map_err(|error| {
        Diagnostic::at(
            source,
            error
                .operation
                .map_or(*name_at, |index| operation_offsets[index]),
            error.message,
        )
    })
}

// A value call carries its expected result contract; unit validation checks
// that it agrees with the callee before lowering.
fn call_contract(
    fields: &[String],
    at: usize,
    offset: usize,
    text: &str,
) -> Result<(crate::ReturnType, usize), Diagnostic> {
    if let Some(field) = fields.get(at).and_then(|s| s.strip_prefix("result=")) {
        let result = crate::type_text::call_result(field)
            .map_err(|message| Diagnostic::at(text, offset, message))?;
        Ok((result, at + 1))
    } else {
        Ok((crate::ReturnType::Integer, at))
    }
}
