//! Source name resolution and structured statement lowering to semantic blocks.
use crate::{
    BlockId, Diagnostic, Function, LocalId, LocatedOperation, Operation, Span, ValueId, parser,
};
use std::collections::BTreeMap;

/// Extract a supported concrete Array element contract from an expected type.
fn array_element(ty: &crate::api::Type) -> Option<crate::ArrayElement> {
    match ty {
        crate::api::Type::Array(element) => crate::ArrayElement::from_api_type(element),
        _ => None,
    }
}

fn emit(
    source: &str,
    expression: parser::Expression,
    locals: &std::collections::BTreeMap<String, LocalId>,
    parameters: &std::collections::BTreeMap<String, usize>,
    operations: &mut Vec<LocatedOperation>,
    calls: &CallContext,
) -> Result<ValueId, Diagnostic> {
    let operation = match expression.kind {
        parser::ExpressionKind::SelfField(field) => Operation::SelfFieldRead {
            result: ValueId(operations.len() as u32),
            result_type: calls
                .field_types
                .get(&field)
                .cloned()
                .unwrap_or(crate::ReturnType::Integer),
            field,
        },
        parser::ExpressionKind::SourceNew { class, arguments } => {
            let base = calls.class_bases.get(&class).copied().ok_or_else(|| {
                Diagnostic::at(
                    source,
                    expression.span.start,
                    "source constructor requires a declared class",
                )
            })?;
            if locals.contains_key(&class) || parameters.contains_key(&class) {
                return Err(Diagnostic::at(
                    source,
                    expression.span.start,
                    "source construction class is shadowed",
                ));
            }
            let arguments = arguments
                .into_iter()
                .map(|argument| emit(source, argument, locals, parameters, operations, calls))
                .collect::<Result<Vec<_>, _>>()?;
            Operation::SourceNew {
                result: ValueId(operations.len() as u32),
                class,
                base,
                arguments,
            }
        }
        parser::ExpressionKind::NativeConstant {
            module,
            member,
            qualifier,
        } => {
            if locals.contains_key(&qualifier) || parameters.contains_key(&qualifier) {
                return Err(Diagnostic::at(
                    source,
                    expression.span.start,
                    "native constant qualifier is shadowed",
                ));
            }
            Operation::NativeConstant {
                result: ValueId(operations.len() as u32),
                module,
                member,
            }
        }
        parser::ExpressionKind::NativeNew { class, qualifier } => {
            if locals.contains_key(&qualifier) || parameters.contains_key(&qualifier) {
                return Err(Diagnostic::at(
                    source,
                    expression.span.start,
                    "native constructor qualifier is shadowed by a local or parameter",
                ));
            }
            Operation::NativeNew {
                result: ValueId(operations.len() as u32),
                class,
            }
        }
        parser::ExpressionKind::ArrayLiteral(_) => {
            return Err(Diagnostic::at(
                source,
                expression.span.start,
                "array literal was not materialized before value emission",
            ));
        }
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
        parser::ExpressionKind::Positive(operand) => {
            let operand = emit(source, *operand, locals, parameters, operations, calls)?;
            Operation::Positive {
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
        kind @ parser::ExpressionKind::Binary { .. } => {
            return emit_binary_chain(
                source,
                parser::Expression {
                    kind,
                    span: expression.span,
                },
                locals,
                parameters,
                operations,
                calls,
            );
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
/// Emit a left-associated arithmetic chain without one Rust frame per term.
/// Preserve operand order and each operation's original source span; grouped
/// right operands use the parser's bounded nesting depth.
fn emit_binary_chain(
    source: &str,
    mut expression: parser::Expression,
    locals: &BTreeMap<String, LocalId>,
    parameters: &BTreeMap<String, usize>,
    operations: &mut Vec<LocatedOperation>,
    calls: &CallContext,
) -> Result<ValueId, Diagnostic> {
    let mut chain = Vec::new();
    while let parser::ExpressionKind::Binary { kind, left, right } = expression.kind {
        chain.push((kind, *right, expression.span));
        expression = *left;
    }
    let mut left = emit(source, expression, locals, parameters, operations, calls)?;
    for (kind, right, span) in chain.into_iter().rev() {
        let right = emit(source, right, locals, parameters, operations, calls)?;
        let result = ValueId(operations.len() as u32);
        operations.push(LocatedOperation {
            operation: Operation::Binary {
                result,
                kind,
                left,
                right,
            },
            span: Some(span),
        });
        left = result;
    }
    Ok(left)
}

fn emit_array_read(
    source: &str,
    kind: parser::ExpressionKind,
    locals: &BTreeMap<String, LocalId>,
    parameters: &BTreeMap<String, usize>,
    operations: &mut Vec<LocatedOperation>,
    calls: &CallContext,
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
    calls: &CallContext,
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
    let class = receiver.and_then(|value| {
        crate::source_objects::class(operations, value, calls.parameter_types.clone())
    });
    let arguments = arguments
        .into_iter()
        .map(|argument| emit(source, argument, locals, parameters, operations, calls))
        .collect::<Result<Vec<_>, _>>()?;
    let result = ValueId(operations.len() as u32);
    Ok(if let Some(receiver) = receiver {
        if let Some(class) = class {
            Operation::ObjectCall {
                result,
                result_type: calls
                    .get(&(class.clone(), member.clone()))
                    .map(|signature| signature.result.clone())
                    .unwrap_or(crate::ReturnType::Integer),
                receiver,
                class,
                member,
                arguments,
            }
        } else {
            Operation::NativeCall {
                result,
                receiver,
                member,
                arguments,
            }
        }
    } else if module.as_deref() == Some("self") {
        Operation::SelfCall {
            result,
            result_type: calls
                .get(&("self".into(), member.clone()))
                .map(|signature| signature.result.clone())
                .unwrap_or(crate::ReturnType::Integer),
            member,
            arguments,
        }
    } else {
        Operation::ModuleCall {
            result,
            result_type: calls
                .get(&(module.clone().unwrap(), member.clone()))
                .map(|signature| signature.result.clone())
                .unwrap_or(crate::ReturnType::Integer),
            module: module.unwrap(),
            member,
            arguments,
        }
    })
}

struct Builder<'a> {
    source: &'a str,
    calls: &'a CallContext,
    result_type: crate::ReturnType,
    parameters: BTreeMap<String, usize>,
    operations: Vec<LocatedOperation>,
    types: crate::source_types::Types,
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
        self.snapshot_with_context(expression, locals, None)
    }

    /// Save an operand, passing an expected array contract to a direct empty literal.
    fn snapshot_with_context(
        &mut self,
        expression: parser::Expression,
        locals: &BTreeMap<String, LocalId>,
        element: Option<crate::ArrayElement>,
    ) -> Result<parser::Expression, Diagnostic> {
        let span = expression.span;
        let value = self.value_with_context(expression, locals, element)?;
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

    /// Construct a typed array literal before evaluating its elements in order.
    /// Two temporary locals retain the array and current element across joins;
    /// native append preserves the element contract without uninitialized slots.
    fn array_literal(
        &mut self,
        elements: Vec<parser::Expression>,
        span: Span,
        locals: &BTreeMap<String, LocalId>,
    ) -> Result<parser::Expression, Diagnostic> {
        if elements.is_empty() {
            return Err(Diagnostic::at(
                self.source,
                span.start,
                "empty array literal requires a concrete expected Array element type",
            ));
        }
        let array = self.local_id(span)?;
        let element = self.local_id(span)?;
        let result = ValueId(self.operations.len() as u32);
        self.push(
            Operation::ArrayNew {
                result,
                element_type: crate::ArrayElement::Char,
            },
            Some(span),
        );
        self.push(
            Operation::LocalInitialize {
                local: array,
                value: result,
            },
            Some(span),
        );
        for (index, expression) in elements.into_iter().enumerate() {
            let element_span = expression.span;
            let value = self.value(expression, locals)?;
            if index == 0 {
                let element_type = self
                    .types
                    .element(value)
                    .unwrap_or(crate::ArrayElement::Char);
                // Materializing a logical element can insert an entry block.
                // Locate the provisional allocation by identity, not position.
                let allocation = self.operations.iter_mut().find(|op| {
                    matches!(op.operation, Operation::ArrayNew { result: id, .. } if id == result)
                }).expect("literal allocation was emitted before its elements");
                allocation.operation = Operation::ArrayNew {
                    result,
                    element_type,
                };
                self.types.set_array(result, array, element_type);
            }
            let store = if index == 0 {
                Operation::LocalInitialize {
                    local: element,
                    value,
                }
            } else {
                Operation::LocalAssign {
                    local: element,
                    value,
                }
            };
            self.push(store, Some(element_span));
            let receiver = ValueId(self.operations.len() as u32);
            self.push(
                Operation::LocalRead {
                    result: receiver,
                    local: array,
                },
                Some(span),
            );
            let argument = ValueId(self.operations.len() as u32);
            self.push(
                Operation::LocalRead {
                    result: argument,
                    local: element,
                },
                Some(element_span),
            );
            let result = ValueId(self.operations.len() as u32);
            self.push(
                Operation::NativeCall {
                    result,
                    receiver,
                    member: "add".into(),
                    arguments: vec![argument],
                },
                Some(element_span),
            );
            self.push(Operation::Discard { value: result }, Some(element_span));
        }
        Ok(parser::Expression {
            kind: parser::ExpressionKind::Temporary(array),
            span,
        })
    }

    /// Prepare children needing flow or construction in source order, snapshotting
    /// operands before emitting their parent in the final block.
    fn prepare(
        &mut self,
        expression: parser::Expression,
        locals: &BTreeMap<String, LocalId>,
    ) -> Result<parser::Expression, Diagnostic> {
        use parser::ExpressionKind as E;
        let span = expression.span;
        let kind = match expression.kind {
            E::ArrayLiteral(elements) => return self.array_literal(elements, span, locals),
            kind @ E::Logical { .. } => {
                return self.logical_value(parser::Expression { kind, span }, locals);
            }
            E::Not(operand) => E::Not(Box::new(self.prepare(*operand, locals)?)),
            E::Positive(operand) => E::Positive(Box::new(self.prepare(*operand, locals)?)),
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
            E::SourceNew { class, arguments } => {
                let expected = self.user_argument_context(&class, "initialize");
                E::SourceNew {
                    class,
                    arguments: self.snapshot_arguments(arguments, locals, &expected)?,
                }
            }
            E::ValueCall {
                receiver,
                member,
                arguments,
            } => {
                let receiver = self.snapshot(*receiver, locals)?;
                let expected = self.receiver_argument_context(&receiver, &member);
                E::ValueCall {
                    receiver: Box::new(receiver),
                    member,
                    arguments: self.snapshot_arguments(arguments, locals, &expected)?,
                }
            }
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
                let expected = if let Some(receiver) = &receiver {
                    self.receiver_argument_context(receiver, &member)
                } else {
                    self.user_argument_context(&module, &member)
                };
                let arguments = self.snapshot_arguments(arguments, locals, &expected)?;
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
        expected: &[Option<crate::ArrayElement>],
    ) -> Result<Vec<ValueId>, Diagnostic> {
        let arguments = if arguments
            .iter()
            .any(parser::Expression::needs_materialization)
        {
            self.snapshot_arguments(arguments, locals, expected)?
        } else {
            arguments
        };
        arguments
            .into_iter()
            .map(|arg| self.value(arg, locals))
            .collect()
    }
    /// Snapshot call operands in order, supplying declared array argument types.
    fn snapshot_arguments(
        &mut self,
        arguments: Vec<parser::Expression>,
        locals: &BTreeMap<String, LocalId>,
        expected: &[Option<crate::ArrayElement>],
    ) -> Result<Vec<parser::Expression>, Diagnostic> {
        arguments
            .into_iter()
            .enumerate()
            .map(|(index, argument)| {
                self.snapshot_with_context(argument, locals, expected.get(index).copied().flatten())
            })
            .collect()
    }

    /// Read array argument expectations from predeclared user call signatures.
    fn user_argument_context(
        &self,
        module: &str,
        member: &str,
    ) -> Vec<Option<crate::ArrayElement>> {
        self.calls
            .get(&(module.into(), member.into()))
            .map(|signature| {
                signature
                    .parameters
                    .iter()
                    .map(|ty| array_element(&ty.api_type()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Resolve Array argument expectations from a source or native receiver.
    /// This supplies construction context; final validation checks the call.
    fn receiver_argument_context(
        &self,
        receiver: &parser::Expression,
        member: &str,
    ) -> Vec<Option<crate::ArrayElement>> {
        let parser::ExpressionKind::Temporary(local) = receiver.kind else {
            return Vec::new();
        };
        if let Some(class) = self.types.local_class(local) {
            return self.user_argument_context(class, member);
        }
        self.types
            .local_type(local)
            .and_then(|ty| crate::native::value_parameters(ty, member))
            .map(|parameters| parameters.iter().map(array_element).collect())
            .unwrap_or_default()
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
        self.types.observe(&operation);
        self.operations.push(LocatedOperation { operation, span });
    }
    fn block_id(&mut self) -> BlockId {
        let id = BlockId(self.next_block);
        self.next_block += 1;
        id
    }
    /// Emit a call statement, retaining Void as a statement-only native contract.
    fn value_statement(
        &mut self,
        expression: parser::Expression,
        locals: &BTreeMap<String, LocalId>,
        span: Span,
    ) -> Result<(), Diagnostic> {
        let value = self.value(expression, locals)?;
        if self.types.value_type(value) == Some(&crate::api::Type::Void) {
            let last = self
                .operations
                .last_mut()
                .expect("call emitted an operation");
            if let Operation::ObjectCall {
                receiver,
                class,
                member,
                arguments,
                ..
            } = &last.operation
            {
                last.operation = Operation::ObjectCallStatement {
                    receiver: *receiver,
                    class: class.clone(),
                    member: member.clone(),
                    arguments: arguments.clone(),
                };
                return Ok(());
            }
            if let Operation::NativeCall {
                receiver,
                member,
                arguments,
                ..
            } = &last.operation
            {
                last.operation = Operation::NativeCallStatement {
                    receiver: *receiver,
                    member: member.clone(),
                    arguments: arguments.clone(),
                };
                return Ok(());
            }
        }
        self.push(Operation::Discard { value }, Some(span));
        Ok(())
    }

    /// Build a return value under a callback's positional or ordinary contract.
    fn returned_value(
        &mut self,
        expression: parser::Expression,
        locals: &BTreeMap<String, LocalId>,
    ) -> Result<ValueId, Diagnostic> {
        if self.result_type != crate::ReturnType::Tuple(crate::TupleShape::OneView) {
            return self.value_with_context(
                expression,
                locals,
                array_element(&self.result_type.api_type()),
            );
        }
        let span = expression.span;
        let parser::ExpressionKind::ArrayLiteral(elements) = expression.kind else {
            return Err(Diagnostic::at(
                self.source,
                span.start,
                "initial View result requires a direct View or View/delegate literal",
            ));
        };
        if !(1..=2).contains(&elements.len()) {
            return Err(Diagnostic::at(
                self.source,
                span.start,
                "initial View result requires one or two slots",
            ));
        }
        let snapshots = elements
            .into_iter()
            .map(|element| self.snapshot(element, locals))
            .collect::<Result<Vec<_>, _>>()?;
        let elements = snapshots
            .into_iter()
            .map(|snapshot| self.value(snapshot, locals))
            .collect::<Result<Vec<_>, _>>()?;
        let result = ValueId(self.operations.len() as u32);
        self.push(Operation::TupleNew { result, elements }, Some(span));
        Ok(result)
    }

    fn value(
        &mut self,
        expression: parser::Expression,
        locals: &BTreeMap<String, LocalId>,
    ) -> Result<ValueId, Diagnostic> {
        self.value_with_context(expression, locals, None)
    }

    /// Build a value, using an expected element contract only for direct `[]`.
    /// Nonempty literals keep their inferred contract and undergo IR validation.
    fn value_with_context(
        &mut self,
        expression: parser::Expression,
        locals: &BTreeMap<String, LocalId>,
        expected: Option<crate::ArrayElement>,
    ) -> Result<ValueId, Diagnostic> {
        if matches!(&expression.kind, parser::ExpressionKind::ArrayLiteral(elements) if elements.is_empty())
        {
            let element_type = expected.ok_or_else(|| {
                Diagnostic::at(
                    self.source,
                    expression.span.start,
                    "empty array literal requires a concrete expected Array element type",
                )
            })?;
            let result = ValueId(self.operations.len() as u32);
            self.push(
                Operation::ArrayNew {
                    result,
                    element_type,
                },
                Some(expression.span),
            );
            return Ok(result);
        }
        let expression = if expression.needs_materialization() {
            self.prepare(expression, locals)?
        } else {
            expression
        };
        let first = self.operations.len();
        let result = emit(
            self.source,
            expression,
            locals,
            &self.parameters,
            &mut self.operations,
            self.calls,
        )?;
        for operation in &self.operations[first..] {
            self.types.observe(&operation.operation);
        }
        Ok(result)
    }

    /// Evaluate an indexed compound target once, then read, combine, and write.
    /// Keep captured array/index in typed locals; stage a branching RHS before
    /// defining the final write operands so their IDs remain block-local.
    fn array_compound(
        &mut self,
        target: parser::Expression,
        kind: crate::BinaryOp,
        value: parser::Expression,
        span: Span,
        locals: &BTreeMap<String, LocalId>,
    ) -> Result<(), Diagnostic> {
        use parser::{Expression, ExpressionKind as E};
        let target_span = target.span;
        let E::ArrayRead { array, index } = target.kind else {
            unreachable!("parsed indexed compound target")
        };
        let array = self.snapshot(*array, locals)?;
        let index = self.snapshot(*index, locals)?;
        let temporary = |expression: &Expression| {
            let E::Temporary(local) = expression.kind else {
                unreachable!("captured target operand")
            };
            Expression {
                kind: E::Temporary(local),
                span: expression.span,
            }
        };
        let old = Expression {
            kind: E::ArrayRead {
                array: Box::new(temporary(&array)),
                index: Box::new(temporary(&index)),
            },
            span: target_span,
        };
        let value_span = Span {
            start: target_span.start,
            end: value.span.end,
        };
        let mut value = Expression {
            kind: E::Binary {
                kind,
                left: Box::new(old),
                right: Box::new(value),
            },
            span: value_span,
        };
        if value.needs_materialization() {
            value = self.snapshot(value, locals)?;
        }
        let array = self.value(array, locals)?;
        let index = self.value(index, locals)?;
        let value = self.value(value, locals)?;
        self.push(
            Operation::ArrayWrite {
                array,
                index,
                value,
            },
            Some(span),
        );
        Ok(())
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
                parser::Statement::ArrayCompound {
                    target,
                    kind,
                    value,
                    ..
                } => {
                    self.array_compound(target, kind, value, span, locals)?;
                }
                parser::Statement::ArrayAssign {
                    array,
                    index,
                    value,
                    ..
                } => {
                    // Keep earlier operands stable across control-flow staging
                    // in any later expression; plain operands need no temporaries.
                    let (array, index, value) = if array.needs_materialization()
                        || index.needs_materialization()
                        || value.needs_materialization()
                    {
                        (
                            self.snapshot(array, locals)?,
                            self.snapshot(index, locals)?,
                            self.snapshot(value, locals)?,
                        )
                    } else {
                        (array, index, value)
                    };
                    let array = self.value(array, locals)?;
                    let index = self.value(index, locals)?;
                    let value = self.value(value, locals)?;
                    self.push(
                        Operation::ArrayWrite {
                            array,
                            index,
                            value,
                        },
                        Some(span),
                    );
                }
                parser::Statement::FieldAssign { name, value, .. } => {
                    let value = self.value(value, locals)?;
                    self.push(
                        Operation::SelfFieldAssign { field: name, value },
                        Some(span),
                    );
                }
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
                    let expected = self.types.local_type(local).and_then(array_element);
                    let value = self.value_with_context(value, locals, expected)?;
                    self.push(Operation::LocalAssign { local, value }, Some(span));
                }
                parser::Statement::BaseInitialize { span, base } => {
                    self.push(Operation::NativeBaseInitialize { base }, Some(span));
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
                    let arguments = self.arguments(arguments, locals, &[])?;
                    self.push(
                        Operation::NativeModuleCallStatement {
                            module,
                            member,
                            arguments,
                        },
                        Some(span),
                    );
                }
                parser::Statement::ValueCall { expression, .. } => {
                    self.value_statement(expression, locals, span)?;
                }
                parser::Statement::Call {
                    module,
                    member,
                    arguments,
                    ..
                } => {
                    if locals.contains_key(&module) || self.parameters.contains_key(&module) {
                        self.value_statement(
                            parser::Expression {
                                kind: parser::ExpressionKind::ModuleCall {
                                    module,
                                    member,
                                    arguments,
                                },
                                span,
                            },
                            locals,
                            span,
                        )?;
                    } else {
                        let expected = self.user_argument_context(&module, &member);
                        let arguments = self.arguments(arguments, locals, &expected)?;
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
                }
                parser::Statement::Return { value, .. } => {
                    let operation = if let Some(value) = value {
                        Operation::Return {
                            value: self.returned_value(value, locals)?,
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
    build_body(source, ast, None, context, &CallContext::new())
}

#[derive(Clone)]
pub(crate) struct CallSignature {
    pub result: crate::ReturnType,
    pub parameters: Vec<crate::ParameterType>,
}

/// Collected callee contracts and the current body's parameter contracts.
#[derive(Clone, Default)]
pub(crate) struct CallContext {
    signatures: BTreeMap<(String, String), CallSignature>,
    pub(crate) class_bases: BTreeMap<String, crate::ClassBase>,
    parameter_types: Vec<crate::ParameterType>,
    pub(crate) field_types: BTreeMap<String, crate::ReturnType>,
}
impl CallContext {
    pub(crate) fn new() -> Self {
        Self::default()
    }
    pub(crate) fn insert(&mut self, key: (String, String), signature: CallSignature) {
        self.signatures.insert(key, signature);
    }
    fn get(&self, key: &(String, String)) -> Option<&CallSignature> {
        self.signatures.get(key)
    }
}

pub(crate) fn build_with_calls(
    source: &str,
    ast: parser::ParsedFunction,
    context: Option<crate::api::ApiContext>,
    method: Option<crate::ClassBase>,
    calls: &CallContext,
) -> Result<Function, Diagnostic> {
    build_body(source, ast, method, context, calls)
}

fn build_body(
    source: &str,
    ast: parser::ParsedFunction,
    method: Option<crate::ClassBase>,
    context: Option<crate::api::ApiContext>,
    calls: &CallContext,
) -> Result<Function, Diagnostic> {
    let callback = method.and_then(|base| crate::native::callback(base, &ast.name));
    let effective_result = callback
        .as_ref()
        .map(|signature| signature.result.clone())
        .unwrap_or_else(|| declared_result(&ast));
    if callback.as_ref().is_some_and(|signature| {
        ast.result_type
            .as_ref()
            .is_some_and(|ty| *ty != signature.result)
    }) {
        return Err(Diagnostic::at(
            source,
            ast.body_span.start,
            "result annotation conflicts with native callback contract",
        ));
    }
    let value_result = effective_result != crate::ReturnType::Void;
    let result_type = ast.result_type;
    let annotations = ast.parameter_types;
    let candidate_parameters = callback
        .as_ref()
        .map(|signature| signature.parameters.clone())
        .unwrap_or_else(|| {
            annotations
                .iter()
                .map(|ty| ty.clone().unwrap_or(crate::ParameterType::Integer))
                .collect()
        });
    let mut body_calls = calls.clone();
    body_calls.parameter_types = candidate_parameters.clone();
    let mut builder = Builder {
        source,
        calls: &body_calls,
        result_type: effective_result.clone(),
        parameters: ast
            .parameters
            .into_iter()
            .enumerate()
            .map(|(i, n)| (n, i))
            .collect(),
        operations: Vec::new(),
        types: crate::source_types::Types::new(candidate_parameters),
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
    let parameter_types = if let Some(signature) = &callback {
        if builder.parameters.len() != signature.parameters.len() {
            return Err(Diagnostic::at(
                source,
                ast.body_span.start,
                format!(
                    "{} parameter count does not match its native callback contract",
                    ast.name
                ),
            ));
        }
        if annotations
            .iter()
            .zip(&signature.parameters)
            .any(|(annotation, expected)| annotation.as_ref().is_some_and(|ty| ty != expected))
        {
            return Err(Diagnostic::at(
                source,
                ast.body_span.start,
                "parameter annotation conflicts with native callback contract",
            ));
        }
        signature.parameters.clone()
    } else {
        annotations
            .into_iter()
            .map(|ty| ty.clone().unwrap_or(crate::ParameterType::Integer))
            .collect()
    };
    let result = Function::construct_for_api(
        ast.name,
        parameter_types,
        builder.operations,
        method.is_some(),
        context,
        callback
            .as_ref()
            .filter(|signature| !matches!(signature.result, crate::ReturnType::Tuple(_)))
            .map(|signature| signature.result.clone())
            .or(result_type),
    );
    let function = result.map_err(|error| {
        let offset = error
            .operation
            .and_then(|index| spans.get(index).copied().flatten())
            .map_or(0, |span| span.start);
        Diagnostic::at(source, offset, error.message)
    })?;
    let expected = effective_result;
    if !callback.as_ref().map_or_else(
        || function.return_type() == expected,
        |signature| signature.accepts_result(&function.return_type()),
    ) {
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
        .clone()
        .unwrap_or(if has_value_return(&ast.statements) {
            crate::ReturnType::Integer
        } else {
            crate::ReturnType::Void
        })
}

/// Read user-declared call contracts before constructing any function body.
pub(crate) fn declared_signature(ast: &parser::ParsedFunction) -> CallSignature {
    CallSignature {
        result: declared_result(ast),
        parameters: ast
            .parameter_types
            .iter()
            .map(|ty| ty.clone().unwrap_or(crate::ParameterType::Integer))
            .collect(),
    }
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
