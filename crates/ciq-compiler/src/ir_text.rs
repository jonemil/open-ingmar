use crate::{BinaryOp, Diagnostic, Function, LocalId, LocatedOperation, Operation, Span, ValueId};

/// Parse the provisional IR format and apply the shared semantic validator.
pub fn parse_ir(source: &str) -> Result<Function, Diagnostic> {
    let mut rows = Vec::new();
    let mut offset = 0;
    for line in source.split_inclusive('\n') {
        let content = line.split('#').next().unwrap();
        let fields: Vec<_> = content.split_whitespace().collect();
        if !fields.is_empty() {
            rows.push((offset, fields));
        }
        offset += line.len();
    }
    let fail = |offset, message: &str| Diagnostic::at(source, offset, message);
    if rows.first().map(|(_, fields)| fields.as_slice()) != Some(&["ciq-ir", "1"][..]) {
        return Err(fail(
            rows.first().map_or(0, |row| row.0),
            "expected 'ciq-ir 1'",
        ));
    }
    let Some((name_at, declaration)) = rows.get(1) else {
        return Err(fail(source.len(), "expected .function <name>"));
    };
    if declaration.len() != 2 || declaration[0] != ".function" {
        return Err(fail(*name_at, "expected .function <name>"));
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
        let (kind, width) = match fields.first().copied() {
            Some(".integer") => (0, 3),
            Some(".return") => (1, 2),
            Some(".add") => (2, 4),
            Some(".subtract") => (3, 4),
            Some(".local") => (4, 3),
            Some(".read") => (5, 3),
            Some(".call") => (6, 4),
            _ => {
                return Err(fail(
                    *at,
                    "expected .integer, .add, .subtract, .local, .read, .call, .return, or .end",
                ));
            }
        };
        if fields.len() < width || fields.len() > width + 1 {
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
        let operation = if kind == 0 {
            let digits = fields[2];
            if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
                return Err(fail(*at, "integer must be nonnegative decimal"));
            }
            Operation::Integer {
                result: parse_value(fields[1])?,
                value: digits
                    .parse()
                    .map_err(|_| fail(*at, "integer exceeds i32"))?,
            }
        } else if kind == 1 {
            Operation::Return {
                value: parse_value(fields[1])?,
            }
        } else if kind == 4 {
            Operation::LocalInitialize {
                local: parse_local(fields[1])?,
                value: parse_value(fields[2])?,
            }
        } else if kind == 5 {
            Operation::LocalRead {
                result: parse_value(fields[1])?,
                local: parse_local(fields[2])?,
            }
        } else if kind == 6 {
            Operation::ModuleCall {
                result: parse_value(fields[1])?,
                module: fields[2].into(),
                member: fields[3].into(),
            }
        } else {
            Operation::Binary {
                result: parse_value(fields[1])?,
                kind: if kind == 2 {
                    BinaryOp::Add
                } else {
                    BinaryOp::Subtract
                },
                left: parse_value(fields[2])?,
                right: parse_value(fields[3])?,
            }
        };
        operations.push(LocatedOperation { operation, span });
        operation_offsets.push(*at);
    }
    if !ended {
        return Err(fail(source.len(), "expected .end"));
    }
    Function::new(declaration[1].to_string(), operations).map_err(|error| {
        Diagnostic::at(
            source,
            error
                .operation
                .map_or(*name_at, |index| operation_offsets[index]),
            error.message,
        )
    })
}
