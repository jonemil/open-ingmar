use crate::{
    BinaryOp, BlockId, ComparisonOp, Diagnostic, Function, LocalId, LocatedOperation, Operation,
    Span, ValueId,
};

/// Parse provisional `ciq-ir` text into a validated semantic function.
///
/// Applies the same validation as typed [`Function`] construction. Invalid
/// syntax or semantic operations produce diagnostics at input byte offsets.
pub fn parse_ir(source: &str) -> Result<Function, Diagnostic> {
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
    if !(2..=4).contains(&declaration.len()) || declaration[0] != ".function" {
        return Err(fail(*name_at, "expected .function <name>"));
    }
    let mut parameter_count = 0;
    let mut parameters = false;
    let mut instance_method = false;
    for field in declaration.iter().skip(2) {
        if field == "receiver=instance" && !instance_method {
            instance_method = true;
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
            Some(".assign") => (15, 3),
            Some(".base-initialize") => (20, 2),
            _ => {
                return Err(fail(*at, "unknown IR directive"));
            }
        };
        if ((kind == 6 && width < 4)
            || (kind == 17 && width < 3)
            || (kind == 18 && width < 3)
            || (kind == 19 && width < 2))
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
        let operation = if kind == 20 {
            if fields[1] != "Application.AppBase" {
                return Err(fail(*at, "unsupported native initializer base"));
            }
            Operation::NativeBaseInitialize {
                base: crate::ClassBase::ApplicationAppBase,
            }
        } else if kind == 18 {
            Operation::SelfCall {
                result: parse_value(&fields[1])?,
                member: fields[2].clone(),
                arguments: fields[3..width]
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
            if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
                return Err(fail(*at, "integer must be nonnegative decimal"));
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
            Operation::ModuleCall {
                result: parse_value(&fields[1])?,
                module: fields[2].clone(),
                member: fields[3].clone(),
                arguments: fields[4..width]
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
    let construct = if instance_method {
        Function::method
    } else {
        Function::with_parameters
    };
    construct(declaration[1].to_string(), parameter_count, operations).map_err(|error| {
        Diagnostic::at(
            source,
            error
                .operation
                .map_or(*name_at, |index| operation_offsets[index]),
            error.message,
        )
    })
}
