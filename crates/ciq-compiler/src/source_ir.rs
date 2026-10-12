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
    calls: &CallResults,
) -> Result<ValueId, Diagnostic> {
    let operation = match expression.kind {
        kind @ parser::ExpressionKind::ArrayRead { .. } => {
            emit_array_read(source, kind, locals, parameters, operations, calls)?
        }
        parser::ExpressionKind::Temporary(local) => Operation::LocalRead {
            result: ValueId(operations.len() as u32),
            local,
        },
        parser::ExpressionKind::Logical { .. } => {
            return Err(Diagnostic::at(
                source,
                expression.span.start,
                "logical expression was not materialized before value emission",
            ));
        }
        parser::ExpressionKind::Not(operand) => {
            let operand = emit(source, *operand, locals, parameters, operations, calls)?;
            Operation::Not {
                result: ValueId(operations.len() as u32),
                operand,
            }
        }
        parser::ExpressionKind::Negate(operand) => {
            let operand = emit(source, *operand, locals, parameters, operations, calls)?;
            Operation::Negate {
                result: ValueId(operations.len() as u32),
                operand,
            }
        }
        parser::ExpressionKind::Char(value) => Operation::Char {
            result: ValueId(operations.len() as u32),
            value,
        },
        parser::ExpressionKind::Boolean(value) => Operation::Boolean {
            result: ValueId(operations.len() as u32),
            value,
        },
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
        kind @ (parser::ExpressionKind::ValueCall { .. }
        | parser::ExpressionKind::ModuleCall { .. }) => emit_call(
            source,
            kind,
            expression.span,
            locals,
            parameters,
            operations,
            calls,
        )?,
        parser::ExpressionKind::Binary { kind, left, right } => {
            let left = emit(source, *left, locals, parameters, operations, calls)?;
            let right = emit(source, *right, locals, parameters, operations, calls)?;
            Operation::Binary {
                result: ValueId(operations.len() as u32),
                kind,
                left,
                right,
            }
        }
        parser::ExpressionKind::Compare { kind, left, right } => {
            let left = emit(source, *left, locals, parameters, operations, calls)?;
            let right = emit(source, *right, locals, parameters, operations, calls)?;
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

/// Emit array/index operands in order and return their read operation. Keep this
/// frame separate so long arithmetic trees retain their existing stack budget.
fn emit_array_read(
    source: &str,
    kind: parser::ExpressionKind,
    locals: &BTreeMap<String, LocalId>,
    parameters: &BTreeMap<String, usize>,
    operations: &mut Vec<LocatedOperation>,
    calls: &CallResults,
) -> Result<Operation, Diagnostic> {
    let parser::ExpressionKind::ArrayRead { array, index } = kind else {
        unreachable!()
    };
    let array = emit(source, *array, locals, parameters, operations, calls)?;
    let index = emit(source, *index, locals, parameters, operations, calls)?;
    Ok(Operation::ArrayRead {
        result: ValueId(operations.len() as u32),
        array,
        index,
    })
}

/// Resolve call ownership and emit receiver/arguments in source order. Keep call
/// construction separate so long arithmetic trees retain small recursive frames.
fn emit_call(
    source: &str,
    kind: parser::ExpressionKind,
    span: Span,
    locals: &BTreeMap<String, LocalId>,
    parameters: &BTreeMap<String, usize>,
    operations: &mut Vec<LocatedOperation>,
    calls: &CallResults,
) -> Result<Operation, Diagnostic> {
    let (receiver, module, member, arguments) = match kind {
        parser::ExpressionKind::ValueCall {
            receiver,
            member,
            arguments,
        } => (Some(*receiver), None, member, arguments),
        parser::ExpressionKind::ModuleCall {
            module,
            member,
            arguments,
        } => {
            if module == "Application.AppBase" {
                return Err(Diagnostic::at(
                    source,
                    span.start,
                    "native base initialization has no expression result",
                ));
            }
            if locals.contains_key(&module) || parameters.contains_key(&module) {
                (
                    Some(parser::Expression {
                        kind: parser::ExpressionKind::Local(module),
                        span,
                    }),
                    None,
                    member,
                    arguments,
                )
            } else {
                (None, Some(module), member, arguments)
            }
        }
        _ => unreachable!("only calls enter call construction"),
    };
    let receiver = receiver
        .map(|value| emit(source, value, locals, parameters, operations, calls))
        .transpose()?;
    let arguments = arguments
        .into_iter()
        .map(|argument| emit(source, argument, locals, parameters, operations, calls))
        .collect::<Result<Vec<_>, _>>()?;
    let result = ValueId(operations.len() as u32);
    Ok(if let Some(receiver) = receiver {
        Operation::NativeCall {
            result,
            receiver,
            member,
            arguments,
        }
    } else if module.as_deref() == Some("self") {
        Operation::SelfCall {
            result,
            result_type: calls
                .get(&("self".into(), member.clone()))
                .copied()
                .unwrap_or(crate::ReturnType::Integer),
            member,
            arguments,
        }
    } else {
        Operation::ModuleCall {
            result,
            result_type: calls
                .get(&(module.clone().unwrap(), member.clone()))
                .copied()
                .unwrap_or(crate::ReturnType::Integer),
            module: module.unwrap(),
            member,
            arguments,
        }
    })
}

struct Builder<'a> {
    source: &'a str,
    calls: &'a CallResults,
    parameters: BTreeMap<String, usize>,
    operations: Vec<LocatedOperation>,
    next_local: u32,
    next_block: u32,
    loops: Vec<(BlockId, BlockId)>,
}

impl Builder<'_> {
    /// Allocate source and temporary locals under the shared frame-slot limit.
    fn local_id(&mut self, span: Span) -> Result<LocalId, Diagnostic> {
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
        let id = LocalId(self.next_local);
        self.next_local += 1;
        Ok(id)
    }

    /// Evaluate an operand now and save its result before later operands branch.
    /// The returned builder-only expression emits a fresh read at the use site.
    fn snapshot(
        &mut self,
        expression: parser::Expression,
        locals: &BTreeMap<String, LocalId>,
    ) -> Result<parser::Expression, Diagnostic> {
        let span = expression.span;
        let value = self.value(expression, locals)?;
        let local = self.local_id(span)?;
        self.push(Operation::LocalInitialize { local, value }, Some(span));
        Ok(parser::Expression {
            kind: parser::ExpressionKind::Temporary(local),
            span,
        })
    }

    /// Materialize a logical result with a Boolean local and empty-stack edges.
    /// Both outcomes reach a join; this stored result carries no null proof.
    fn logical_value(
        &mut self,
        expression: parser::Expression,
        locals: &BTreeMap<String, LocalId>,
    ) -> Result<parser::Expression, Diagnostic> {
        let span = expression.span;
        if !matches!(
            self.operations.first().map(|op| &op.operation),
            Some(Operation::Block { .. })
        ) {
            self.operations.insert(
                0,
                LocatedOperation {
                    operation: Operation::Block { block: BlockId(0) },
                    span: None,
                },
            );
        }
        let local = self.local_id(span)?;
        let value = ValueId(self.operations.len() as u32);
        self.push(
            Operation::Boolean {
                result: value,
                value: false,
            },
            Some(span),
        );
        self.push(Operation::LocalInitialize { local, value }, Some(span));
        let on_true = self.block_id();
        let on_false = self.block_id();
        let join = self.block_id();
        self.condition(expression, locals, on_true, on_false)?;
        self.push(Operation::Block { block: on_true }, None);
        let value = ValueId(self.operations.len() as u32);
        self.push(
            Operation::Boolean {
                result: value,
                value: true,
            },
            Some(span),
        );
        self.push(Operation::LocalAssign { local, value }, Some(span));
        self.push(Operation::Jump { target: join }, Some(span));
        self.push(Operation::Block { block: on_false }, None);
        self.push(Operation::Jump { target: join }, Some(span));
        self.push(Operation::Block { block: join }, None);
        Ok(parser::Expression {
            kind: parser::ExpressionKind::Temporary(local),
            span,
        })
    }

    /// Prepare flow-producing children in source order, snapshotting operands
    /// before emitting their parent in the final block. Ordinary trees bypass it.
    fn prepare(
        &mut self,
        expression: parser::Expression,
        locals: &BTreeMap<String, LocalId>,
    ) -> Result<parser::Expression, Diagnostic> {
        use parser::ExpressionKind as E;
        let span = expression.span;
        let kind = match expression.kind {
            kind @ E::Logical { .. } => {
                return self.logical_value(parser::Expression { kind, span }, locals);
            }
            E::Not(operand) => E::Not(Box::new(self.prepare(*operand, locals)?)),
            E::Negate(operand) => E::Negate(Box::new(self.prepare(*operand, locals)?)),
            E::ArrayRead { array, index } => E::ArrayRead {
                array: Box::new(self.snapshot(*array, locals)?),
                index: Box::new(self.snapshot(*index, locals)?),
            },
            E::Binary { kind, left, right } => E::Binary {
                kind,
                left: Box::new(self.snapshot(*left, locals)?),
                right: Box::new(self.snapshot(*right, locals)?),
            },
            E::Compare { kind, left, right } => E::Compare {
                kind,
                left: Box::new(self.snapshot(*left, locals)?),
                right: Box::new(self.snapshot(*right, locals)?),
            },
            E::ValueCall {
                receiver,
                member,
                arguments,
            } => E::ValueCall {
                receiver: Box::new(self.snapshot(*receiver, locals)?),
                member,
                arguments: arguments
                    .into_iter()
                    .map(|arg| self.snapshot(arg, locals))
                    .collect::<Result<_, _>>()?,
            },
            E::ModuleCall {
                module,
                member,
                arguments,
            } => {
                let receiver =
                    if locals.contains_key(&module) || self.parameters.contains_key(&module) {
                        Some(Box::new(self.snapshot(
                            parser::Expression {
                                kind: E::Local(module.clone()),
                                span,
                            },
                            locals,
                        )?))
                    } else {
                        None
                    };
                let arguments = arguments
                    .into_iter()
                    .map(|arg| self.snapshot(arg, locals))
                    .collect::<Result<_, _>>()?;
                if let Some(receiver) = receiver {
                    E::ValueCall {
                        receiver,
                        member,
                        arguments,
                    }
                } else {
                    E::ModuleCall {
                        module,
                        member,
                        arguments,
                    }
                }
            }
            kind => kind,
        };
        Ok(parser::Expression { kind, span })
    }

    /// Evaluate statement arguments in source order. If any argument branches,
    /// save all results and read them only after the final argument has completed.
    fn arguments(
        &mut self,
        arguments: Vec<parser::Expression>,
        locals: &BTreeMap<String, LocalId>,
    ) -> Result<Vec<ValueId>, Diagnostic> {
        let arguments = if arguments.iter().any(parser::Expression::has_logical) {
            arguments
                .into_iter()
                .map(|arg| self.snapshot(arg, locals))
                .collect::<Result<Vec<_>, _>>()?
        } else {
            arguments
        };
        arguments
            .into_iter()
            .map(|arg| self.value(arg, locals))
            .collect()
    }
    /// Emit a Boolean condition as explicit edges. Evaluate logical operands
    /// left to right and enter the right operand only when the left requires it.
    /// Grouping and negation preserve this flow without cross-block value IDs.
    fn condition(
        &mut self,
        expression: parser::Expression,
        locals: &BTreeMap<String, LocalId>,
        on_true: BlockId,
        on_false: BlockId,
    ) -> Result<(), Diagnostic> {
        let span = expression.span;
        match expression.kind {
            parser::ExpressionKind::Logical { and, left, right } => {
                let next = self.block_id();
                if and {
                    self.condition(*left, locals, next, on_false)?;
                } else {
                    self.condition(*left, locals, on_true, next)?;
                }
                self.push(Operation::Block { block: next }, None);
                self.condition(*right, locals, on_true, on_false)?;
            }
            parser::ExpressionKind::Not(operand) => {
                self.condition(*operand, locals, on_false, on_true)?;
            }
            kind => {
                let condition = self.value(parser::Expression { kind, span }, locals)?;
                self.push(
                    Operation::Branch {
                        condition,
                        on_true,
                        on_false,
                    },
                    Some(span),
                );
            }
        }
        Ok(())
    }
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
        let expression = if expression.has_logical() {
            self.prepare(expression, locals)?
        } else {
            expression
        };
        emit(
            self.source,
            expression,
            locals,
            &self.parameters,
            &mut self.operations,
            self.calls,
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
                    let value = self.value(local.initializer, locals)?;
                    let id = self.local_id(span)?;
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
                parser::Statement::NativeModuleCall {
                    module,
                    member,
                    arguments,
                    ..
                } => {
                    if locals.contains_key("System") || self.parameters.contains_key("System") {
                        return Err(Diagnostic::at(
                            self.source,
                            span.start,
                            "a local/parameter cannot be used as the System module receiver",
                        ));
                    }
                    let arguments = self.arguments(arguments, locals)?;
                    self.push(
                        Operation::NativeModuleCallStatement {
                            module,
                            member,
                            arguments,
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
                    let arguments = self.arguments(arguments, locals)?;
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
                    self.condition(condition, locals, body_block, exit)?;
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
                    let true_block = self.block_id();
                    let false_block = self.block_id();
                    let join = self.block_id();
                    self.condition(condition, locals, true_block, false_block)?;
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

/// Resolve source names and emit function IR under declared type contracts.
/// Value functions require a result on every path. Void fallthrough gains a
/// return terminator; mixed returns and unreachable statements are rejected.
pub(crate) fn build(
    source: &str,
    ast: parser::ParsedFunction,
    context: Option<crate::api::ApiContext>,
) -> Result<Function, Diagnostic> {
    build_body(source, ast, false, context, &CallResults::new())
}

pub(crate) type CallResults = BTreeMap<(String, String), crate::ReturnType>;

pub(crate) fn build_with_calls(
    source: &str,
    ast: parser::ParsedFunction,
    context: Option<crate::api::ApiContext>,
    method: bool,
    calls: &CallResults,
) -> Result<Function, Diagnostic> {
    build_body(source, ast, method, context, calls)
}

fn build_body(
    source: &str,
    ast: parser::ParsedFunction,
    method: bool,
    context: Option<crate::api::ApiContext>,
    calls: &CallResults,
) -> Result<Function, Diagnostic> {
    let value_result = declared_result(&ast) != crate::ReturnType::Void;
    let result_type = ast.result_type;
    let annotations = ast.parameter_types;
    let mut builder = Builder {
        source,
        calls,
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
    // Logical value expressions add an entry block when they first need flow.
    if ast.statements.iter().any(|s| {
        matches!(
            s,
            parser::Statement::If { .. } | parser::Statement::While { .. }
        )
    }) {
        builder.push(Operation::Block { block: BlockId(0) }, None);
    }
    if !builder.sequence(ast.statements, &mut BTreeMap::new())? {
        if value_result {
            return Err(Diagnostic::at(
                source,
                ast.body_span.end - 1,
                "function requires a value return on every path",
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
    let parameter_types = if let Some(signature) = method
        .then(|| crate::native::callback(crate::ClassBase::ApplicationAppBase, &ast.name))
        .flatten()
    {
        if builder.parameters.len() != signature.parameters.len() {
            return Err(Diagnostic::at(
                source,
                ast.body_span.start,
                format!("{} requires one nullable-dictionary parameter", ast.name),
            ));
        }
        if annotations
            .iter()
            .zip(&signature.parameters)
            .any(|(annotation, expected)| annotation.is_some_and(|ty| ty != *expected))
        {
            return Err(Diagnostic::at(
                source,
                ast.body_span.start,
                "parameter annotation conflicts with native callback contract",
            ));
        }
        signature.parameters
    } else {
        annotations
            .into_iter()
            .map(|ty| ty.unwrap_or(crate::ParameterType::Integer))
            .collect()
    };
    let result = Function::construct_for_api(
        ast.name,
        parameter_types,
        builder.operations,
        method,
        context,
        result_type,
    );
    let function = result.map_err(|error| {
        let offset = error
            .operation
            .and_then(|index| spans.get(index).copied().flatten())
            .map_or(0, |span| span.start);
        Diagnostic::at(source, offset, error.message)
    })?;
    let expected = result_type.unwrap_or(if value_result {
        crate::ReturnType::Integer
    } else {
        crate::ReturnType::Void
    });
    if function.return_type() != expected {
        return Err(Diagnostic::at(
            source,
            function
                .operations()
                .iter()
                .find_map(|op| {
                    matches!(
                        op.operation,
                        Operation::Return { .. } | Operation::ReturnVoid
                    )
                    .then_some(op.span)
                    .flatten()
                    .map(|span| span.start)
                })
                .unwrap_or(ast.body_span.start),
            "return type does not match function contract; non-Number value results require an explicit annotation",
        ));
    }
    Ok(function)
}

pub(crate) fn declared_result(ast: &parser::ParsedFunction) -> crate::ReturnType {
    ast.result_type
        .unwrap_or(if has_value_return(&ast.statements) {
            crate::ReturnType::Integer
        } else {
            crate::ReturnType::Void
        })
}

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
