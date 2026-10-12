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
                "nullable-integer" => crate::ReturnType::NullableInteger,
                "nullable-string" => crate::ReturnType::NullableString,
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
                        "nullable-integer" => Ok(crate::ParameterType::NullableInteger),
                        "nullable-string" => Ok(crate::ParameterType::NullableString),
                        "integer" => Ok(crate::ParameterType::Integer),
                        "string" => Ok(crate::ParameterType::String),
                        "boolean" => Ok(crate::ParameterType::Boolean),
                        "char" => Ok(crate::ParameterType::Char),
                        "char-array" => Ok(crate::ParameterType::CharArray),
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
            Some(".integer") => (0, 3),
            Some(".boolean") => (23, 3),
            Some(".char") => (25, 3),
            Some(".negate") => (26, 3),
            Some(".not") => (27, 3),
            Some(".call-native-module-discard") => (24, call_width()),
            Some(".return") => (1, 2),
            Some(".add") => (2, 4),
            Some(".subtract") => (3, 4),
            Some(".local") => (4, 3),
            Some(".read") => (5, 3),
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
            Some(".block") => (10, 2),
            Some(".branch") => (11, 4),
            Some(".jump") => (12, 2),
            Some(".equal") => (13, 4),
            Some(".greater") => (14, 4),
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
        let operation = if kind == 27 {
            Operation::Not {
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
            if fields[1] != "Application.AppBase" {
                return Err(fail(*at, "unsupported native initializer base"));
            }
            Operation::NativeBaseInitialize {
                base: crate::ClassBase::ApplicationAppBase,
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
        } else if kind == 13 || kind == 14 {
            Operation::Compare {
                result: parse_value(&fields[1])?,
                kind: if kind == 13 {
                    ComparisonOp::Equal
                } else {
                    ComparisonOp::Greater
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
            Operation::LocalRead {
                result: parse_value(&fields[1])?,
                local: parse_local(&fields[2])?,
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
                kind: if kind == 2 {
                    BinaryOp::Add
                } else {
                    BinaryOp::Subtract
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
            "nullable-integer" => crate::ReturnType::NullableInteger,
            "nullable-string" => crate::ReturnType::NullableString,
            "integer" => crate::ReturnType::Integer,
            "string" => crate::ReturnType::String,
            "boolean" => crate::ReturnType::Boolean,
            "char" => crate::ReturnType::Char,
            "char-array" => crate::ReturnType::CharArray,
            _ => return Err(Diagnostic::at(text, offset, "unsupported call result type")),
        };
        Ok((result, at + 1))
    } else {
        Ok((crate::ReturnType::Integer, at))
    }
}
