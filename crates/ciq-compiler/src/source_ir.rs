//! Source name resolution and structured statement lowering to semantic blocks.
use crate::{
    BlockId, Diagnostic, Function, LocalId, LocatedOperation, Operation, Span, ValueId, parser,
};
use std::collections::BTreeMap;

fn emit(
    source: &str,
    expression: parser::Expression,
    locals: &std::collections::BTreeMap<String, LocalId>,
    parameters: &std::collections::BTreeMap<String, usize>,
    operations: &mut Vec<LocatedOperation>,
) -> Result<ValueId, Diagnostic> {
    let operation = match expression.kind {
        parser::ExpressionKind::String(value) => Operation::String {
            result: ValueId(operations.len() as u32),
            value,
        },
        parser::ExpressionKind::Integer(value) => Operation::Integer {
            result: ValueId(operations.len() as u32),
            value,
        },
        parser::ExpressionKind::Local(name) => {
            let result = ValueId(operations.len() as u32);
            if let Some(local) = locals.get(&name) {
                Operation::LocalRead {
                    result,
                    local: *local,
                }
            } else if let Some(index) = parameters.get(&name) {
                Operation::ParameterRead {
                    result,
                    index: *index,
                }
            } else {
                return Err(Diagnostic::at(
                    source,
                    expression.span.start,
                    format!("unknown or not yet initialized local {name:?}"),
                ));
            }
        }
        parser::ExpressionKind::ModuleCall {
            module,
            member,
            arguments,
        } => {
            if locals.contains_key(&module) || parameters.contains_key(&module) {
                return Err(Diagnostic::at(
                    source,
                    expression.span.start,
                    "calls on local receivers are unsupported",
                ));
            }
            let arguments = arguments
                .into_iter()
                .map(|argument| emit(source, argument, locals, parameters, operations))
                .collect::<Result<Vec<_>, _>>()?;
            Operation::ModuleCall {
                result: ValueId(operations.len() as u32),
                module,
                member,
                arguments,
            }
        }
        parser::ExpressionKind::Binary { kind, left, right } => {
            let left = emit(source, *left, locals, parameters, operations)?;
            let right = emit(source, *right, locals, parameters, operations)?;
            Operation::Binary {
                result: ValueId(operations.len() as u32),
                kind,
                left,
                right,
            }
        }
        parser::ExpressionKind::Compare { kind, left, right } => {
            let left = emit(source, *left, locals, parameters, operations)?;
            let right = emit(source, *right, locals, parameters, operations)?;
            Operation::Compare {
                result: ValueId(operations.len() as u32),
                kind,
                left,
                right,
            }
        }
    };
    let result = ValueId(operations.len() as u32);
    operations.push(LocatedOperation {
        operation,
        span: Some(expression.span),
    });
    Ok(result)
}

struct Builder<'a> {
    source: &'a str,
    parameters: BTreeMap<String, usize>,
    operations: Vec<LocatedOperation>,
    next_local: u32,
    next_block: u32,
}

impl Builder<'_> {
    fn push(&mut self, operation: Operation, span: Option<Span>) {
        self.operations.push(LocatedOperation { operation, span });
    }
    fn block_id(&mut self) -> BlockId {
        let id = BlockId(self.next_block);
        self.next_block += 1;
        id
    }
    fn value(
        &mut self,
        expression: parser::Expression,
        locals: &BTreeMap<String, LocalId>,
    ) -> Result<ValueId, Diagnostic> {
        emit(
            self.source,
            expression,
            locals,
            &self.parameters,
            &mut self.operations,
        )
    }

    /// Emit a sequence into the current block, returning whether every path
    /// returns. Each arm inherits names but its declarations stay in that arm.
    fn sequence(
        &mut self,
        statements: Vec<parser::Statement>,
        locals: &mut BTreeMap<String, LocalId>,
    ) -> Result<bool, Diagnostic> {
        let mut returned = false;
        for statement in statements {
            let span = statement.span();
            if returned {
                return Err(Diagnostic::at(
                    self.source,
                    span.start,
                    "unreachable statement after all paths return",
                ));
            }
            match statement {
                parser::Statement::Local(local) => {
                    if self.parameters.contains_key(&local.name) {
                        return Err(Diagnostic::at(
                            self.source,
                            span.start,
                            "local declaration shadows a parameter",
                        ));
                    }
                    if locals.contains_key(&local.name) {
                        return Err(Diagnostic::at(
                            self.source,
                            span.start,
                            "duplicate local declaration or shadowing an outer local",
                        ));
                    }
                    if self.next_local as usize >= crate::MAX_LOCALS {
                        return Err(Diagnostic::at(
                            self.source,
                            span.start,
                            "function exceeds 255-local limit",
                        ));
                    }
                    if self.parameters.len() + self.next_local as usize >= crate::MAX_LOCALS {
                        return Err(Diagnostic::at(
                            self.source,
                            span.start,
                            "parameters plus locals exceed 255-slot limit",
                        ));
                    }
                    let value = self.value(local.initializer, locals)?;
                    let id = LocalId(self.next_local);
                    self.next_local += 1;
                    self.push(Operation::LocalInitialize { local: id, value }, Some(span));
                    locals.insert(local.name, id);
                }
                parser::Statement::Println { argument, .. } => {
                    if locals.contains_key("System") || self.parameters.contains_key("System") {
                        return Err(Diagnostic::at(
                            self.source,
                            span.start,
                            "System receiver is shadowed",
                        ));
                    }
                    let value = self.value(argument, locals)?;
                    self.push(Operation::Println { value }, Some(span));
                }
                parser::Statement::Assign { name, value, .. } => {
                    let local = locals.get(&name).copied().ok_or_else(|| {
                        Diagnostic::at(
                            self.source,
                            span.start,
                            if self.parameters.contains_key(&name) {
                                "parameter assignment is not supported".into()
                            } else {
                                format!(
                                    "assignment requires an in-scope initialized local {name:?}"
                                )
                            },
                        )
                    })?;
                    let value = self.value(value, locals)?;
                    self.push(Operation::LocalAssign { local, value }, Some(span));
                }
                parser::Statement::Return { value, .. } => {
                    let value = self.value(value, locals)?;
                    self.push(Operation::Return { value }, Some(span));
                    returned = true;
                }
                parser::Statement::If {
                    condition,
                    on_true,
                    on_false,
                    ..
                } => {
                    let condition = self.value(condition, locals)?;
                    let true_block = self.block_id();
                    let false_block = self.block_id();
                    let join = self.block_id();
                    self.push(
                        Operation::Branch {
                            condition,
                            on_true: true_block,
                            on_false: false_block,
                        },
                        Some(span),
                    );
                    self.push(Operation::Block { block: true_block }, None);
                    let true_returns = self.sequence(on_true, &mut locals.clone())?;
                    if !true_returns {
                        self.push(Operation::Jump { target: join }, Some(span));
                    }
                    self.push(Operation::Block { block: false_block }, None);
                    let false_returns = self.sequence(on_false, &mut locals.clone())?;
                    if !false_returns {
                        self.push(Operation::Jump { target: join }, Some(span));
                    }
                    returned = true_returns && false_returns;
                    if !returned {
                        self.push(Operation::Block { block: join }, None);
                    }
                }
            }
            if self.operations.len() > crate::MAX_OPERATIONS {
                return Err(Diagnostic::at(
                    self.source,
                    span.start,
                    "function exceeds 1024-operation limit",
                ));
            }
        }
        Ok(returned)
    }
}

/// Resolve source names and emit validated semantic IR, requiring an integer
/// return on every path and rejecting unreachable statements.
pub(crate) fn build(source: &str, ast: parser::ParsedFunction) -> Result<Function, Diagnostic> {
    let mut builder = Builder {
        source,
        parameters: ast
            .parameters
            .into_iter()
            .enumerate()
            .map(|(i, n)| (n, i))
            .collect(),
        operations: Vec::new(),
        next_local: 0,
        next_block: 1,
    };
    // Nested conditions necessarily occur beneath a top-level conditional.
    if ast
        .statements
        .iter()
        .any(|s| matches!(s, parser::Statement::If { .. }))
    {
        builder.push(Operation::Block { block: BlockId(0) }, None);
    }
    if !builder.sequence(ast.statements, &mut BTreeMap::new())? {
        return Err(Diagnostic::at(
            source,
            ast.body_span.end - 1,
            "function requires an integer return on every path",
        ));
    }
    let spans: Vec<_> = builder.operations.iter().map(|op| op.span).collect();
    Function::with_parameters(ast.name, builder.parameters.len(), builder.operations).map_err(
        |error| {
            let offset = error
                .operation
                .and_then(|index| spans.get(index).copied().flatten())
                .map_or(0, |span| span.start);
            Diagnostic::at(source, offset, error.message)
        },
    )
}
