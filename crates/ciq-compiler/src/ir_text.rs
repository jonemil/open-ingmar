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
            result_type = Some(match result {
                "dictionary" => crate::ReturnType::Dictionary,
                "object" => crate::ReturnType::LangObject,
                "nullable-object" => crate::ReturnType::NullableObject,
                "weak-reference" => crate::ReturnType::WeakReference,
                "method" => crate::ReturnType::Method,
                "symbol" => crate::ReturnType::Symbol,
                tag if tag.starts_with("source(") && tag.ends_with(')') => {
                    crate::ReturnType::SourceObject(
                        crate::SourceClass::parse(&tag[7..tag.len() - 1]).ok_or_else(|| {
                            Diagnostic::at(source, *name_at, "invalid source reference contract")
                        })?,
                    )
                }
                tag if tag.starts_with("dictionary(") => crate::ReturnType::TypedDictionary(
                    crate::DictionaryType::parse(tag)
                        .ok_or_else(|| fail(*name_at, "unsupported Dictionary contract"))?,
                ),
                "nullable-integer" => crate::ReturnType::NullableInteger,
                "nullable-long" => crate::ReturnType::NullableLong,
                "nullable-double" => crate::ReturnType::NullableDouble,
                "nullable-float" => crate::ReturnType::NullableFloat,
                "nullable-string" => crate::ReturnType::NullableString,
                "nullable-boolean" => crate::ReturnType::NullableBoolean,
                "nullable-char" => crate::ReturnType::NullableChar,
                _ => return Err(fail(*name_at, "unsupported function result contract")),
            });
        } else if field == "receiver=instance" && !instance_method {
            instance_method = true;
        } else if let Some(types) = field.strip_prefix("parameter-types=") {
            if parameter_types.is_some() {
                return Err(fail(*name_at, "duplicate parameter types"));
            }
            parameter_types = Some(
                types
                    .split(':')
                    .map(|t| match t {
                        tag if tag.starts_with("source(") && tag.ends_with(')') => {
                            Ok(crate::ParameterType::SourceObject(
                                crate::SourceClass::parse(&tag[7..tag.len() - 1]).ok_or_else(
                                    || fail(*name_at, "invalid source reference contract"),
                                )?,
                            ))
                        }
                        tag if tag.starts_with("dictionary(") => crate::DictionaryType::parse(tag)
                            .map(crate::ParameterType::TypedDictionary)
                            .ok_or_else(|| fail(*name_at, "unsupported Dictionary contract")),
                        "nullable-integer" => Ok(crate::ParameterType::NullableInteger),
                        "nullable-long" => Ok(crate::ParameterType::NullableLong),
                        "nullable-double" => Ok(crate::ParameterType::NullableDouble),
                        "nullable-float" => Ok(crate::ParameterType::NullableFloat),
                        "nullable-string" => Ok(crate::ParameterType::NullableString),
                        "nullable-boolean" => Ok(crate::ParameterType::NullableBoolean),
                        "nullable-char" => Ok(crate::ParameterType::NullableChar),
                        "integer" => Ok(crate::ParameterType::Integer),
                        "long" => Ok(crate::ParameterType::Long),
                        "double" => Ok(crate::ParameterType::Double),
                        "float" => Ok(crate::ParameterType::Float),
                        "string" => Ok(crate::ParameterType::String),
                        "boolean" => Ok(crate::ParameterType::Boolean),
                        "char" => Ok(crate::ParameterType::Char),
                        "dictionary" => Ok(crate::ParameterType::Dictionary),
                        "object" => Ok(crate::ParameterType::LangObject),
                        "nullable-object" => Ok(crate::ParameterType::NullableObject),
                        "weak-reference" => Ok(crate::ParameterType::WeakReference),
                        "method" => Ok(crate::ParameterType::Method),
                        "symbol" => Ok(crate::ParameterType::Symbol),
                        "view" => Ok(crate::ParameterType::Object(crate::NativeClass::View)),
                        "dc" => Ok(crate::ParameterType::Object(crate::NativeClass::Dc)),
                        "behavior-delegate" => Ok(crate::ParameterType::Object(
                            crate::NativeClass::BehaviorDelegate,
                        )),
                        "view-array" => Ok(crate::ParameterType::Array(
                            crate::ArrayElement::Object(crate::NativeClass::View),
                        )),
                        "char-array" => Ok(crate::ParameterType::Array(crate::ArrayElement::Char)),
                        "boolean-array" => {
                            Ok(crate::ParameterType::Array(crate::ArrayElement::Boolean))
                        }
                        "nullable-char-array" => Ok(crate::ParameterType::Array(
                            crate::ArrayElement::NullableChar,
                        )),
                        "nullable-boolean-array" => Ok(crate::ParameterType::Array(
                            crate::ArrayElement::NullableBoolean,
                        )),
                        "nullable-number-array" => Ok(crate::ParameterType::Array(
                            crate::ArrayElement::NullableNumber,
                        )),
                        "nullable-string-array" => Ok(crate::ParameterType::Array(
                            crate::ArrayElement::NullableString,
                        )),
                        "string-array" => {
                            Ok(crate::ParameterType::Array(crate::ArrayElement::String))
                        }
                        "long-array" => Ok(crate::ParameterType::Array(crate::ArrayElement::Long)),
                        "nullable-long-array" => Ok(crate::ParameterType::Array(
                            crate::ArrayElement::NullableLong,
                        )),
                        "float-array" => {
                            Ok(crate::ParameterType::Array(crate::ArrayElement::Float))
                        }
                        "nullable-float-array" => Ok(crate::ParameterType::Array(
                            crate::ArrayElement::NullableFloat,
                        )),
                        "double-array" => {
                            Ok(crate::ParameterType::Array(crate::ArrayElement::Double))
                        }
                        "nullable-double-array" => Ok(crate::ParameterType::Array(
                            crate::ArrayElement::NullableDouble,
                        )),
                        "number-array" => {
                            Ok(crate::ParameterType::Array(crate::ArrayElement::Number))
                        }
                        "nullable-dictionary" => Ok(crate::ParameterType::NullableDictionary),
                        _ => Err(fail(*name_at, "unsupported parameter type")),
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
        let (kind, width) = match fields.first().map(String::as_str) {
            Some(".call-self") => (18, call_width()),
            Some(".call-self-discard") => (19, call_width()),
            Some(".return-void") => (16, 1),
            Some(".call-discard") => (
                17,
                fields.len() - usize::from(fields.last().is_some_and(|f| f.starts_with("span="))),
            ),
            Some(".call-object") => (40, call_width()),
            Some(".call-object-discard") => (41, call_width()),
            Some(".integer") => (0, 3),
            Some(".read-self") => (38, call_width()),
            Some(".assign-self") => (39, 3),
            Some(".boolean") => (23, 3),
            Some(".char") => (25, 3),
            Some(".long") => (74, 3),
            Some(".float") => (75, 3),
            Some(".double") => (76, 3),
            Some(".increment") => (62, 3),
            Some(".decrement") => (63, 3),
            Some(".positive") => (55, 3),
            Some(".complement") => (56, 3),
            Some(".negate") => (26, 3),
            Some(".not") => (27, 3),
            Some(".array-read") => (28, 4),
            Some(".dictionary-read") => (68, 4),
            Some(".dictionary-write") => (69, 4),
            Some(".array-write") => (49, 4),
            Some(".call-native-module-discard") => (24, call_width()),
            Some(".return") => (1, 2),
            Some(".add") => (2, 4),
            Some(".subtract") => (3, 4),
            Some(".multiply") => (42, 4),
            Some(".divide") => (43, 4),
            Some(".remainder") => (44, 4),
            Some(".bit-and") => (50, 4),
            Some(".bit-or") => (51, 4),
            Some(".bit-xor") => (52, 4),
            Some(".shift-left") => (53, 4),
            Some(".shift-right") => (54, 4),
            Some(".local") => (4, 3),
            Some(".read" | ".take") => (5, 3),
            Some(".symbol") => (91, 3),
            Some(".self") => (92, 2),
            Some(".call") => (
                6,
                fields.len()
                    - usize::from(
                        fields
                            .last()
                            .is_some_and(|field| field.starts_with("span=")),
                    ),
            ),
            Some(".parameter") => (7, 3),
            Some(".string") => (8, 3),
            Some(".println") => (9, 2),
            Some(".discard") => (29, 2),
            Some(".array-constant") => (61, call_width()),
            Some(".array-new") => (30, 3),
            Some(".dictionary-begin") => (70, 4),
            Some(".dictionary-fill") => (71, call_width()),
            Some(".dictionary-finish") => (72, 3),
            Some(".array-begin") => (57, 4),
            Some(".array-fill") => (58, 4),
            Some(".array-finish") => (59, 3),
            Some(".array-fill-many") => (60, call_width()),
            Some(".new-native") => (31, 3),
            Some(".tuple-new") => (32, call_width()),
            Some(".new-source") => (33, call_width()),
            Some(".constant") => (34, 4),
            Some(".call-native-discard") => (35, call_width()),
            Some(".dictionary-constant") => (67, call_width()),
            Some(".dictionary-new") => (
                66,
                if fields.get(2).is_some_and(|s| s.starts_with("type=")) {
                    3
                } else {
                    2
                },
            ),
            Some(".block") => (10, 2),
            Some(".block-value") => (64, 3),
            Some(".jump-value") => (65, 3),
            Some(".branch") => (11, 4),
            Some(".jump") => (12, 2),
            Some(".equal") => (13, 4),
            Some(".greater") => (14, 4),
            Some(".less") => (45, 4),
            Some(".less-equal") => (46, 4),
            Some(".greater-equal") => (47, 4),
            Some(".not-equal") => (48, 4),
            Some(".null") => (21, 2),
            Some(".call-native") => (22, call_width()),
            Some(".assign") => (15, 3),
            Some(".base-initialize") => (20, 2),
            _ => {
                return Err(fail(*at, "unknown IR directive"));
            }
        };
        if ((kind == 6 && width < 4)
            || (kind == 17 && width < 3)
            || (kind == 18 && width < 3)
            || (kind == 19 && width < 2)
            || (kind == 22 && width < 4))
            || (kind == 24 && width < 3)
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
        let operation = if kind == 40 {
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
        } else if kind == 41 {
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
        } else if kind == 38 {
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
        } else if kind == 39 {
            Operation::SelfFieldAssign {
                field: fields[1].clone(),
                value: parse_value(&fields[2])?,
            }
        } else if kind == 33 {
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
        } else if kind == 34 {
            Operation::NativeConstant {
                result: parse_value(&fields[1])?,
                module: fields[2].clone(),
                member: fields[3].clone(),
            }
        } else if kind == 35 {
            Operation::NativeCallStatement {
                receiver: parse_value(&fields[1])?,
                member: fields[2].clone(),
                arguments: fields[3..width]
                    .iter()
                    .map(|v| parse_value(v))
                    .collect::<Result<_, _>>()?,
            }
        } else if kind == 32 {
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
        } else if kind == 31 {
            Operation::NativeNew {
                result: parse_value(&fields[1])?,
                class: crate::NativeClass::from_name(&fields[2])
                    .ok_or_else(|| fail(*at, "unsupported native construction class"))?,
            }
        } else if kind == 61 {
            if width < 4 {
                return Err(fail(*at, "constant array requires a type and elements"));
            }
            Operation::ArrayConstant {
                result: parse_value(&fields[1])?,
                element_type: match fields[2].as_str() {
                    "number" => crate::ArrayElement::Number,
                    "double" => crate::ArrayElement::Double,
                    "nullable-double" => crate::ArrayElement::NullableDouble,
                    "float" => crate::ArrayElement::Float,
                    "nullable-float" => crate::ArrayElement::NullableFloat,
                    "long" => crate::ArrayElement::Long,
                    "nullable-long" => crate::ArrayElement::NullableLong,
                    "nullable-number" => crate::ArrayElement::NullableNumber,
                    "char" => crate::ArrayElement::Char,
                    "boolean" => crate::ArrayElement::Boolean,
                    "string" => crate::ArrayElement::String,
                    "nullable-string" => crate::ArrayElement::NullableString,
                    "nullable-char" => crate::ArrayElement::NullableChar,
                    "nullable-boolean" => crate::ArrayElement::NullableBoolean,
                    _ => {
                        return Err(fail(
                            *at,
                            "constant arrays require supported scalar elements",
                        ));
                    }
                },
                values: fields[3..width]
                    .iter()
                    .map(|s| {
                        if s == "null" {
                            return Ok(crate::ArrayConstantValue::Null);
                        }
                        if matches!(fields[2].as_str(), "boolean" | "nullable-boolean") {
                            return match s.as_str() {
                                "true" => Ok(crate::ArrayConstantValue::Boolean(true)),
                                "false" => Ok(crate::ArrayConstantValue::Boolean(false)),
                                _ => {
                                    Err(fail(*at, "Boolean array constants require true or false"))
                                }
                            };
                        }
                        if matches!(fields[2].as_str(), "string" | "nullable-string") {
                            return s
                                .strip_prefix('"')
                                .map(|s| crate::ArrayConstantValue::String(s.into()))
                                .ok_or_else(|| {
                                    fail(*at, "String array constants require quoted text")
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
                            let wide = !matches!(fields[2].as_str(), "float" | "nullable-float");
                            let hex = s
                                .strip_prefix("0x")
                                .filter(|s| s.len() == if wide { 16 } else { 8 })
                                .ok_or_else(|| {
                                    fail(*at, "numeric array constant has wrong bit width")
                                })?;
                            if !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
                                return Err(fail(*at, "invalid numeric array constant bits"));
                            }
                            let bits = u64::from_str_radix(hex, 16)
                                .map_err(|_| fail(*at, "invalid numeric array bits"))?;
                            return Ok(match fields[2].as_str() {
                                "long" | "nullable-long" => crate::ArrayConstantValue::Long(bits),
                                "double" | "nullable-double" => {
                                    crate::ArrayConstantValue::Double(bits)
                                }
                                _ => crate::ArrayConstantValue::Float(bits as u32),
                            });
                        }
                        let hex = s
                            .strip_prefix("0x")
                            .filter(|s| s.len() == 8 && s.bytes().all(|b| b.is_ascii_hexdigit()))
                            .ok_or_else(|| fail(*at, "array constants require eight hex digits"))?;
                        u32::from_str_radix(hex, 16)
                            .map(crate::ArrayConstantValue::Bits)
                            .map_err(|_| fail(*at, "invalid array constant"))
                    })
                    .collect::<Result<_, _>>()?,
            }
        } else if kind == 57 {
            Operation::ArrayBegin {
                builder: parse_builder(&fields[1])?,
                element_type: match fields[2].as_str() {
                    "char" => crate::ArrayElement::Char,
                    "boolean" => crate::ArrayElement::Boolean,
                    "number" => crate::ArrayElement::Number,
                    "double" => crate::ArrayElement::Double,
                    "nullable-double" => crate::ArrayElement::NullableDouble,
                    "float" => crate::ArrayElement::Float,
                    "nullable-float" => crate::ArrayElement::NullableFloat,
                    "long" => crate::ArrayElement::Long,
                    "nullable-long" => crate::ArrayElement::NullableLong,
                    "nullable-number" => crate::ArrayElement::NullableNumber,
                    "string" => crate::ArrayElement::String,
                    "nullable-string" => crate::ArrayElement::NullableString,
                    "nullable-char" => crate::ArrayElement::NullableChar,
                    "nullable-boolean" => crate::ArrayElement::NullableBoolean,
                    "view" => crate::ArrayElement::Object(crate::NativeClass::View),
                    _ => return Err(fail(*at, "unsupported builder element type")),
                },
                length: parse_count(&fields[3])?,
            }
        } else if kind == 60 {
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
        } else if kind == 58 {
            Operation::ArrayFill {
                builder: parse_builder(&fields[1])?,
                index: parse_count(&fields[2])?,
                value: parse_value(&fields[3])?,
            }
        } else if kind == 59 {
            Operation::ArrayFinish {
                result: parse_value(&fields[1])?,
                builder: parse_builder(&fields[2])?,
            }
        } else if kind == 30 {
            let element_type = match fields[2].as_str() {
                "char" => crate::ArrayElement::Char,
                "boolean" => crate::ArrayElement::Boolean,
                "number" => crate::ArrayElement::Number,
                "double" => crate::ArrayElement::Double,
                "nullable-double" => crate::ArrayElement::NullableDouble,
                "float" => crate::ArrayElement::Float,
                "nullable-float" => crate::ArrayElement::NullableFloat,
                "long" => crate::ArrayElement::Long,
                "nullable-long" => crate::ArrayElement::NullableLong,
                "nullable-number" => crate::ArrayElement::NullableNumber,
                "string" => crate::ArrayElement::String,
                "nullable-string" => crate::ArrayElement::NullableString,
                "nullable-char" => crate::ArrayElement::NullableChar,
                "nullable-boolean" => crate::ArrayElement::NullableBoolean,
                "view" => crate::ArrayElement::Object(crate::NativeClass::View),
                _ => return Err(fail(*at, "unsupported array element type")),
            };
            Operation::ArrayNew {
                result: parse_value(&fields[1])?,
                element_type,
            }
        } else if kind == 29 {
            Operation::Discard {
                value: parse_value(&fields[1])?,
            }
        } else if kind == 70 {
            Operation::DictionaryBegin {
                builder: parse_builder(&fields[1])?,
                dictionary_type: crate::DictionaryType::parse(&fields[2])
                    .ok_or_else(|| fail(*at, "unsupported Dictionary contract"))?,
                length: parse_count(&fields[3])?,
            }
        } else if kind == 71 {
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
        } else if kind == 72 {
            Operation::DictionaryFinish {
                result: parse_value(&fields[1])?,
                builder: parse_builder(&fields[2])?,
            }
        } else if kind == 68 {
            Operation::DictionaryRead {
                result: parse_value(&fields[1])?,
                dictionary: parse_value(&fields[2])?,
                key: parse_value(&fields[3])?,
            }
        } else if kind == 69 {
            Operation::DictionaryWrite {
                dictionary: parse_value(&fields[1])?,
                key: parse_value(&fields[2])?,
                value: parse_value(&fields[3])?,
            }
        } else if kind == 49 {
            Operation::ArrayWrite {
                array: parse_value(&fields[1])?,
                index: parse_value(&fields[2])?,
                value: parse_value(&fields[3])?,
            }
        } else if kind == 28 {
            Operation::ArrayRead {
                result: parse_value(&fields[1])?,
                array: parse_value(&fields[2])?,
                index: parse_value(&fields[3])?,
            }
        } else if kind == 27 {
            Operation::Not {
                result: parse_value(&fields[1])?,
                operand: parse_value(&fields[2])?,
            }
        } else if kind == 56 {
            Operation::Complement {
                result: parse_value(&fields[1])?,
                operand: parse_value(&fields[2])?,
            }
        } else if kind == 62 || kind == 63 {
            Operation::Step {
                result: parse_value(&fields[1])?,
                operand: parse_value(&fields[2])?,
                increment: kind == 62,
            }
        } else if kind == 55 {
            Operation::Positive {
                result: parse_value(&fields[1])?,
                operand: parse_value(&fields[2])?,
            }
        } else if kind == 26 {
            Operation::Negate {
                result: parse_value(&fields[1])?,
                operand: parse_value(&fields[2])?,
            }
        } else if kind == 25 {
            let digits = fields[2]
                .strip_prefix("0x")
                .filter(|digits| digits.len() == 8 && digits.bytes().all(|b| b.is_ascii_hexdigit()))
                .ok_or_else(|| {
                    fail(*at, "Char literal requires 0x followed by eight hex digits")
                })?;
            Operation::Char {
                result: parse_value(&fields[1])?,
                value: u32::from_str_radix(digits, 16)
                    .map_err(|_| fail(*at, "invalid Char bits"))?,
            }
        } else if kind == 24 {
            Operation::NativeModuleCallStatement {
                module: fields[1].clone(),
                member: fields[2].clone(),
                arguments: fields[3..width]
                    .iter()
                    .map(|f| parse_value(f))
                    .collect::<Result<_, _>>()?,
            }
        } else if kind == 23 {
            Operation::Boolean {
                result: parse_value(&fields[1])?,
                value: match fields[2].as_str() {
                    "true" => true,
                    "false" => false,
                    _ => return Err(fail(*at, "Boolean literal requires true or false")),
                },
            }
        } else if kind == 22 {
            Operation::NativeCall {
                result: parse_value(&fields[1])?,
                receiver: parse_value(&fields[2])?,
                member: fields[3].clone(),
                arguments: fields[4..width]
                    .iter()
                    .map(|f| parse_value(f))
                    .collect::<Result<_, _>>()?,
            }
        } else if kind == 21 {
            Operation::Null {
                result: parse_value(&fields[1])?,
            }
        } else if kind == 20 {
            Operation::NativeBaseInitialize {
                base: match fields[1].as_str() {
                    "Application.AppBase" => crate::ClassBase::ApplicationAppBase,
                    "WatchUi.View" => crate::ClassBase::WatchUiView,
                    "WatchUi.BehaviorDelegate" => crate::ClassBase::WatchUiBehaviorDelegate,
                    _ => return Err(fail(*at, "unsupported native initializer base")),
                },
            }
        } else if kind == 18 {
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
        } else if kind == 19 {
            Operation::SelfCallStatement {
                member: fields[1].clone(),
                arguments: fields[2..width]
                    .iter()
                    .map(|f| parse_value(f))
                    .collect::<Result<_, _>>()?,
            }
        } else if kind == 16 {
            Operation::ReturnVoid
        } else if kind == 17 {
            Operation::ModuleCallStatement {
                module: fields[1].clone(),
                member: fields[2].clone(),
                arguments: fields[3..width]
                    .iter()
                    .map(|f| parse_value(f))
                    .collect::<Result<_, _>>()?,
            }
        } else if kind == 15 {
            Operation::LocalAssign {
                local: parse_local(&fields[1])?,
                value: parse_value(&fields[2])?,
            }
        } else if kind == 67 {
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
        } else if kind == 66 {
            Operation::DictionaryNew {
                dictionary_type: if width == 3 {
                    Some(
                        crate::DictionaryType::parse(fields[2].strip_prefix("type=").unwrap())
                            .ok_or_else(|| fail(*at, "unsupported Dictionary contract"))?,
                    )
                } else {
                    None
                },
                result: parse_value(&fields[1])?,
            }
        } else if kind == 64 {
            Operation::BlockParameter {
                result: parse_value(&fields[1])?,
                value_type: crate::joins::parse_type(&fields[2])
                    .ok_or_else(|| fail(*at, "unsupported block value type"))?,
            }
        } else if kind == 65 {
            Operation::JumpValue {
                target: parse_block(&fields[1])?,
                value: parse_value(&fields[2])?,
            }
        } else if kind == 10 {
            Operation::Block {
                block: parse_block(&fields[1])?,
            }
        } else if kind == 11 {
            Operation::Branch {
                condition: parse_value(&fields[1])?,
                on_true: parse_block(&fields[2])?,
                on_false: parse_block(&fields[3])?,
            }
        } else if kind == 12 {
            Operation::Jump {
                target: parse_block(&fields[1])?,
            }
        } else if matches!(kind, 13 | 14 | 45..=48) {
            Operation::Compare {
                result: parse_value(&fields[1])?,
                kind: match kind {
                    13 => ComparisonOp::Equal,
                    14 => ComparisonOp::Greater,
                    45 => ComparisonOp::Less,
                    46 => ComparisonOp::LessEqual,
                    47 => ComparisonOp::GreaterEqual,
                    48 => ComparisonOp::NotEqual,
                    _ => unreachable!(),
                },
                left: parse_value(&fields[2])?,
                right: parse_value(&fields[3])?,
            }
        } else if kind == 8 {
            let value = fields[2]
                .strip_prefix('"')
                .ok_or_else(|| fail(*at, "string values require quotes"))?;
            Operation::String {
                result: parse_value(&fields[1])?,
                value: value.into(),
            }
        } else if kind == 9 {
            Operation::Println {
                value: parse_value(&fields[1])?,
            }
        } else if kind == 75 || kind == 76 {
            let digits = fields[2]
                .strip_prefix("0x")
                .ok_or_else(|| fail(*at, "floating bits require 0x hexadecimal"))?;
            let width = if kind == 75 { 8 } else { 16 };
            if digits.len() != width || !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err(fail(
                    *at,
                    "floating bits require exactly 8 or 16 hexadecimal digits for their width",
                ));
            }
            let bits =
                u64::from_str_radix(digits, 16).map_err(|_| fail(*at, "invalid floating bits"))?;
            let result = parse_value(&fields[1])?;
            if kind == 75 {
                Operation::Float {
                    result,
                    bits: bits as u32,
                }
            } else {
                Operation::Double { result, bits }
            }
        } else if kind == 74 {
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
        } else if kind == 92 {
            Operation::SelfValue {
                result: parse_value(&fields[1])?,
            }
        } else if kind == 91 {
            Operation::Symbol {
                result: parse_value(&fields[1])?,
                name: fields[2].clone(),
            }
        } else if kind == 0 {
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
        } else if kind == 1 {
            Operation::Return {
                value: parse_value(&fields[1])?,
            }
        } else if kind == 4 {
            Operation::LocalInitialize {
                local: parse_local(&fields[1])?,
                value: parse_value(&fields[2])?,
            }
        } else if kind == 5 {
            let result = parse_value(&fields[1])?;
            let local = parse_local(&fields[2])?;
            if fields[0] == ".take" {
                Operation::LocalTake { result, local }
            } else {
                Operation::LocalRead { result, local }
            }
        } else if kind == 6 {
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
        } else if kind == 7 {
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
        } else {
            Operation::Binary {
                result: parse_value(&fields[1])?,
                kind: match kind {
                    2 => BinaryOp::Add,
                    3 => BinaryOp::Subtract,
                    42 => BinaryOp::Multiply,
                    43 => BinaryOp::Divide,
                    44 => BinaryOp::Remainder,
                    50 => BinaryOp::BitAnd,
                    51 => BinaryOp::BitOr,
                    52 => BinaryOp::BitXor,
                    53 => BinaryOp::ShiftLeft,
                    54 => BinaryOp::ShiftRight,
                    _ => unreachable!("all non-binary directives parsed above"),
                },
                left: parse_value(&fields[2])?,
                right: parse_value(&fields[3])?,
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
        let result = match field {
            tag if tag.starts_with("dictionary(") => {
                crate::ReturnType::TypedDictionary(crate::DictionaryType::parse(tag).ok_or_else(
                    || Diagnostic::at(text, offset, "unsupported Dictionary contract"),
                )?)
            }
            "nullable-integer" => crate::ReturnType::NullableInteger,
            "nullable-long" => crate::ReturnType::NullableLong,
            "nullable-double" => crate::ReturnType::NullableDouble,
            "nullable-float" => crate::ReturnType::NullableFloat,
            "nullable-string" => crate::ReturnType::NullableString,
            "nullable-boolean" => crate::ReturnType::NullableBoolean,
            "nullable-char" => crate::ReturnType::NullableChar,
            "integer" => crate::ReturnType::Integer,
            "long" => crate::ReturnType::Long,
            "double" => crate::ReturnType::Double,
            "float" => crate::ReturnType::Float,
            "string" => crate::ReturnType::String,
            "boolean" => crate::ReturnType::Boolean,
            "char" => crate::ReturnType::Char,
            "dictionary" => crate::ReturnType::Dictionary,
            "object" => crate::ReturnType::LangObject,
            "nullable-object" => crate::ReturnType::NullableObject,
            "weak-reference" => crate::ReturnType::WeakReference,
            "method" => crate::ReturnType::Method,
            "symbol" => crate::ReturnType::Symbol,
            "char-array" => crate::ReturnType::Array(crate::ArrayElement::Char),
            "boolean-array" => crate::ReturnType::Array(crate::ArrayElement::Boolean),
            "number-array" => crate::ReturnType::Array(crate::ArrayElement::Number),
            "double-array" => crate::ReturnType::Array(crate::ArrayElement::Double),
            "nullable-double-array" => {
                crate::ReturnType::Array(crate::ArrayElement::NullableDouble)
            }
            "float-array" => crate::ReturnType::Array(crate::ArrayElement::Float),
            "nullable-float-array" => crate::ReturnType::Array(crate::ArrayElement::NullableFloat),
            "long-array" => crate::ReturnType::Array(crate::ArrayElement::Long),
            "nullable-long-array" => crate::ReturnType::Array(crate::ArrayElement::NullableLong),
            "nullable-char-array" => crate::ReturnType::Array(crate::ArrayElement::NullableChar),
            "nullable-boolean-array" => {
                crate::ReturnType::Array(crate::ArrayElement::NullableBoolean)
            }
            "nullable-number-array" => {
                crate::ReturnType::Array(crate::ArrayElement::NullableNumber)
            }
            "string-array" => crate::ReturnType::Array(crate::ArrayElement::String),
            "nullable-string-array" => {
                crate::ReturnType::Array(crate::ArrayElement::NullableString)
            }
            "view" => crate::ReturnType::Object(crate::NativeClass::View),
            "dc" => crate::ReturnType::Object(crate::NativeClass::Dc),
            "behavior-delegate" => crate::ReturnType::Object(crate::NativeClass::BehaviorDelegate),
            "view-delegate" => crate::ReturnType::Tuple(crate::TupleShape::ViewDelegate),
            "one-view" => crate::ReturnType::Tuple(crate::TupleShape::OneView),
            "view-array" => {
                crate::ReturnType::Array(crate::ArrayElement::Object(crate::NativeClass::View))
            }
            tag if tag.starts_with("source(") && tag.ends_with(')') => {
                crate::ReturnType::SourceObject(
                    crate::SourceClass::parse(&tag[7..tag.len() - 1]).ok_or_else(|| {
                        Diagnostic::at(text, offset, "invalid source reference contract")
                    })?,
                )
            }
            _ => return Err(Diagnostic::at(text, offset, "unsupported call result type")),
        };
        Ok((result, at + 1))
    } else {
        Ok((crate::ReturnType::Integer, at))
    }
}
