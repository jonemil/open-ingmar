# Semantic IR model and validation

This specification defines the semantic model and its optional text format. The
representation is provisional and independent of Garmin MIR. The in-memory
pipeline is source → validated semantic IR → symbolic VM assembly → CODE; text
readers reconstruct the same models rather than defining alternate paths. See
[architecture](../../architecture.md) and [pipeline
evidence](../../investigations/compiler-pipeline.md).

## Semantic model

Value IDs identify computations; local IDs identify stored state; parameters
identify positions; private container builders identify incomplete construction.
None selects a VM slot, native symbol/import ID, or section address. Source
spans carry diagnostic provenance. Immutable validated models are the boundary
between source checking and VM instruction selection.

Expression values are single-use and block-local. Stored locals carry values
across graph edges, whose evaluation stacks are empty. Staging preserves earlier
operands before a later operand branches. Typed construction, source lowering,
and text resumption share type, initialization, null-proof, source-identity, and
effect-order checks; no producer may bypass backend assumptions. Keeping this
semantic boundary makes dependencies inspectable without a general SSA/register
allocator, mandatory text intermediates, or a separate linker.

[Source contracts](../frontend/README.md), [class contracts](../frontend/classes.md),
and [unit contracts](../frontend/declarations.md) own accepted language behavior
and resolution. [API declarations](../sdk/declarations.md) own catalogue
semantics. This page owns shared model invariants. [Grammar](grammar.md),
operation-family pages, and [lowering](../lowering/README.md) own their separate
contracts.

## Validation

`Function::new` validates explicitly constructed operations; `compile(source)`
constructs an immutable Function through the same validator. `parse_ir` resumes
saved text with those checks. Public IR types are provisional. None of these
entry points may bypass semantic validation before lowering.

Definitions are unique; operands refer to earlier definitions and every defined
expression value has exactly one use in its own block. Local initialization
consumes a value, while each read defines a fresh value. Unread locals are
legal: their initializers still run. Shared value graphs, dead definitions,
invalid names/spans, out-of-range constants, and malformed control flow are
rejected. Source compilation supplies optional spans; typed/text IR may omit
them. The function is limited to 1024 operations, including block markers and
returns; parameter metadata is not an operation.

An implicit block must end in one return. Explicit CFGs require the
[control-flow invariants](control-flow.md). Locals and private builder state
cross edges only under their respective availability/progress checks; expression
values cross an edge only as a single typed join argument.

For pure expressions, definition order may differ from VM stack order: lowering
walks dependencies for each sink, left operand before right. Potential failures
such as division/remainder, allocations, calls, prints, reads/writes with
observable effects, and ownership transfers impose the checked expression and
statement schedule. Saved IR that would reorder effects is rejected rather than
silently rescheduled. [Call
operations](calls.md#module-calls-and-operation-order) and
[joins](values.md#expression-joins) specify the applicable boundaries.

`parse_ir` diagnoses input byte positions; typed construction reports the
operation index when available. Unit validation adds declaration ownership,
exact call signatures, field contracts, and callback restrictions after checking
each function. [API context](../sdk/context.md) adds availability checks without
changing serialization or authorizing unsupported execution profiles.

## Native and text boundaries

`Function::to_ir_text` preserves value IDs, receiver metadata, and optional
spans. `parse_ir` reconstructs that semantic model. `Function::lower` constructs
shared assembly directly; `Function::assembly` lowers and prints it. Native and
saved-stage compilation must produce equivalent assembly/CODE under identical
bindings. Text is not mandatory between stages.
