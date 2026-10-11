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
        parser::ExpressionKind::Null => Operation::Null {
            result: ValueId(operations.len() as u32),
        },
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
            if module == "Application.AppBase" {
                return Err(Diagnostic::at(
                    source,
                    expression.span.start,
                    "native base initialization has no expression result",
                ));
            }
            if locals.contains_key(&module) || parameters.contains_key(&module) {
                let receiver = emit(
                    source,
                    parser::Expression {
                        kind: parser::ExpressionKind::Local(module),
                        span: expression.span,
                    },
                    locals,
                    parameters,
                    operations,
                )?;
                let arguments = arguments
                    .into_iter()
                    .map(|argument| emit(source, argument, locals, parameters, operations))
                    .collect::<Result<Vec<_>, _>>()?;
                let result = ValueId(operations.len() as u32);
                operations.push(LocatedOperation {
                    operation: Operation::NativeCall {
                        result,
                        receiver,
                        member,
                        arguments,
                    },
                    span: Some(expression.span),
                });
                return Ok(result);
            }
            let arguments = arguments
                .into_iter()
                .map(|argument| emit(source, argument, locals, parameters, operations))
                .collect::<Result<Vec<_>, _>>()?;
            if module == "self" {
                Operation::SelfCall {
                    result: ValueId(operations.len() as u32),
                    member,
                    arguments,
                }
            } else {
                Operation::ModuleCall {
                    result: ValueId(operations.len() as u32),
                    module,
                    member,
                    arguments,
                }
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
    loops: Vec<(BlockId, BlockId)>,
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

    /// Emit a statement sequence and report whether all paths terminate the
    /// current block with return, break, or continue. Declarations stay scoped
    /// to the sequence; loop jumps select the innermost test or exit.
    fn sequence(
        &mut self,
        statements: Vec<parser::Statement>,
        locals: &mut BTreeMap<String, LocalId>,
    ) -> Result<bool, Diagnostic> {
        let mut terminated = false;
        for statement in statements {
            let span = statement.span();
            if terminated {
                return Err(Diagnostic::at(
                    self.source,
                    span.start,
                    "unreachable statement after all paths terminate",
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
                parser::Statement::BaseInitialize { span } => {
                    self.push(
                        Operation::NativeBaseInitialize {
                            base: crate::ClassBase::ApplicationAppBase,
                        },
                        Some(span),
                    );
                }
                parser::Statement::Call {
                    module,
                    member,
                    arguments,
                    ..
                } => {
                    if locals.contains_key(&module) || self.parameters.contains_key(&module) {
                        return Err(Diagnostic::at(
                            self.source,
                            span.start,
                            "calls on local receivers are unsupported",
                        ));
                    }
                    let arguments = arguments
                        .into_iter()
                        .map(|arg| self.value(arg, locals))
                        .collect::<Result<Vec<_>, _>>()?;
                    let operation = if module == "self" {
                        Operation::SelfCallStatement { member, arguments }
                    } else {
                        Operation::ModuleCallStatement {
                            module,
                            member,
                            arguments,
                        }
                    };
                    self.push(operation, Some(span));
                }
                parser::Statement::Return { value, .. } => {
                    let operation = if let Some(value) = value {
                        Operation::Return {
                            value: self.value(value, locals)?,
                        }
                    } else {
                        Operation::ReturnVoid
                    };
                    self.push(operation, Some(span));
                    terminated = true;
                }
                parser::Statement::Break { .. } | parser::Statement::Continue { .. } => {
                    let &(test, exit) = self.loops.last().ok_or_else(|| {
                        Diagnostic::at(
                            self.source,
                            span.start,
                            "break/continue requires an enclosing loop",
                        )
                    })?;
                    let target = if matches!(statement, parser::Statement::Break { .. }) {
                        exit
                    } else {
                        test
                    };
                    self.push(Operation::Jump { target }, Some(span));
                    terminated = true;
                }
                parser::Statement::While {
                    condition, body, ..
                } => {
                    let test = self.block_id();
                    let body_block = self.block_id();
                    let exit = self.block_id();
                    self.push(Operation::Jump { target: test }, Some(span));
                    self.push(Operation::Block { block: test }, None);
                    let condition = self.value(condition, locals)?;
                    self.push(
                        Operation::Branch {
                            condition,
                            on_true: body_block,
                            on_false: exit,
                        },
                        Some(span),
                    );
                    self.push(Operation::Block { block: body_block }, None);
                    self.loops.push((test, exit));
                    let body_terminates = self.sequence(body, &mut locals.clone())?;
                    self.loops.pop();
                    if !body_terminates {
                        self.push(Operation::Jump { target: test }, Some(span));
                    }
                    self.push(Operation::Block { block: exit }, None);
                    // The condition may initially be false, even when the body returns.
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
                    let true_terminates = self.sequence(on_true, &mut locals.clone())?;
                    if !true_terminates {
                        self.push(Operation::Jump { target: join }, Some(span));
                    }
                    self.push(Operation::Block { block: false_block }, None);
                    let false_terminates = self.sequence(on_false, &mut locals.clone())?;
                    if !false_terminates {
                        self.push(Operation::Jump { target: join }, Some(span));
                    }
                    terminated = true_terminates && false_terminates;
                    if !terminated {
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
        Ok(terminated)
    }
}

/// Resolve source names and emit validated integer-return or void function IR.
/// Integer functions require a value on every path. Void fallthrough gains a
/// return terminator; mixed returns and unreachable statements are rejected.
pub(crate) fn build(source: &str, ast: parser::ParsedFunction) -> Result<Function, Diagnostic> {
    build_body(source, ast, false)
}

pub(crate) fn build_method(
    source: &str,
    ast: parser::ParsedFunction,
) -> Result<Function, Diagnostic> {
    build_body(source, ast, true)
}

fn build_body(
    source: &str,
    ast: parser::ParsedFunction,
    method: bool,
) -> Result<Function, Diagnostic> {
    fn has_value_return(statements: &[parser::Statement]) -> bool {
        statements.iter().any(|s| match s {
            parser::Statement::Return { value: Some(_), .. } => true,
            parser::Statement::If {
                on_true, on_false, ..
            } => has_value_return(on_true) || has_value_return(on_false),
            parser::Statement::While { body, .. } => has_value_return(body),
            _ => false,
        })
    }
    let integer_result = has_value_return(&ast.statements);
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
        loops: Vec::new(),
    };
    // Nested control flow necessarily occurs beneath a top-level if or while.
    if ast.statements.iter().any(|s| {
        matches!(
            s,
            parser::Statement::If { .. } | parser::Statement::While { .. }
        )
    }) {
        builder.push(Operation::Block { block: BlockId(0) }, None);
    }
    if !builder.sequence(ast.statements, &mut BTreeMap::new())? {
        if integer_result {
            return Err(Diagnostic::at(
                source,
                ast.body_span.end - 1,
                "function requires an integer return on every path",
            ));
        }
        builder.push(
            Operation::ReturnVoid,
            Some(Span {
                start: ast.body_span.end - 1,
                end: ast.body_span.end,
            }),
        );
    }
    let spans: Vec<_> = builder.operations.iter().map(|op| op.span).collect();
    let construct = if method {
        Function::method
    } else {
        Function::with_parameters
    };
    let result = if let Some(signature) = method
        .then(|| crate::native::callback(crate::ClassBase::ApplicationAppBase, &ast.name))
        .flatten()
    {
        if builder.parameters.len() != signature.parameters.len() {
            return Err(Diagnostic::at(
                source,
                ast.body_span.start,
                "onStart requires one nullable-dictionary parameter",
            ));
        }
        Function::method_with_parameter_types(
            ast.name,
            signature.parameters.to_vec(),
            builder.operations,
        )
    } else {
        construct(ast.name, builder.parameters.len(), builder.operations)
    };
    result.map_err(|error| {
        let offset = error
            .operation
            .and_then(|index| spans.get(index).copied().flatten())
            .map_or(0, |span| span.start);
        Diagnostic::at(source, offset, error.message)
    })
}
