# First Monkey C compiler subset

Initial restricted frontend, 2026-10-10. Design:
[small semantic IR](../design/compiler-ir.md). Runtime evidence:
[constant returns](../investigations/first-compiler.md),
[arithmetic](../investigations/compiler-arithmetic.md),
[locals](../investigations/compiler-locals.md),
[module calls](../investigations/compiler-calls.md), and
[parameters/arguments](../investigations/compiler-arguments.md).

Input is exactly one top-level function:

```text
function <identifier>(<parameter-list>?) { <local-declaration>* return <expression>; }
parameter-list = <identifier> ("," <identifier>)*
argument-list = expression ("," expression)*
local-declaration = "var" <identifier> "=" <expression> ";"
expression = primary (("+" | "-") primary)*
primary = <decimal-integer> | <parameter-or-local-identifier> | <module> "." <member> "(" argument-list? ")" | "(" expression ")"
```

Identifiers use ASCII letters/underscore followed by ASCII
letters/digits/underscore. `function`, `return`, and `var` are reserved.
Whitespace, `//` comments, and non-nested `/* ... */` comments are supported.
The accepted literal domain is `0..=127`; computed results may be negative or
exceed 127. Unary signs, larger literals, other literal forms, typed parameters,
types, modules/classes, other statements/additional functions, and other
operators are rejected. This is a frontend limit, not a Monkey C language or VM
integer limit. The domain deliberately avoids unresolved high-bit one-byte
immediate behavior.

Diagnostics identify the input byte offset and one-based line/column (columns
count Unicode scalar values). The caller supplies the filename. Unsupported
input never silently generates a partial function. Name resolution currently
covers parameters and earlier initialized function locals. There is no general
type inference, global/member resolver, whole-program analysis, or optimizer
yet.

The IR contains integer constants, binary add/subtract definitions, parameter
reads, local initialization/read and module-call operations, and a final return
using a value ID, with optional source spans. Source compilation supplies them;
IR text may omit them. Without locals, lowering emits a label from the function
name, `argc <parameter-count + 1>`, `ipushz` for zero or `ipush1` for a positive
supported literal, left/right operands followed by `addv` or `subv` for binary
definitions, then `return`. The frame count follows the observed hidden context
slot; omitting SDK global-resolution instructions is limited to this
context-independent body. It does not establish the frame layout or valid setup
for arbitrary functions.

```sh
cargo run -p ciq-compiler -- examples/compiler-probe/probe.mc scratch/probe.mcasm
```

CLI output must be a new file; existing output is never overwritten. Output is
ordinary assembly, not a complete PRG. Library `compile(source)` returns an
inspectable immutable `Function` IR; `Function::assembly()` lowers/prints it.
Public IR types are preliminary. `Function::new` validates explicitly
constructed operations, `parse_ir` resumes [IR text](compiler-ir-text.md), and
`Function::lower()` constructs the
[shared assembly model](../design/assembly-model.md) directly. Existing
assembler/signing commands consume the output after explicitly integrating it
into an application section project.

The compiler CLI now supports `--from source|ir|assembly` and
`--emit ir|assembly|code`. The default two-path invocation still emits assembly
from source. CODE output is a raw payload, not a PRG. See
[pipeline commands and limits](compiler-ir-text.md#command-line-stages).

## Arithmetic and lowering contract

The source subset supports binary `+` and `-` with equal precedence and left
associativity, and parentheses for grouping. Literals remain decimal `0..=127`;
subtraction results may be negative. Unary signs, other operators, reassignment,
object calls, and other statement forms remain unsupported. Parse nesting is
bounded at 64; the semantic function is limited to 1024 operations including
return. These are implementation limits.

Semantic IR contains pure integer add/subtract definitions with explicit
left/right value IDs. Each initializer/return consumes one expression tree:
definitions are unique, operands refer to earlier definitions, every defined
value has exactly one use, and initialization statements or the final return
consume expression results. General shared-value graphs and unused definitions
remain rejected until their scheduling policy is implemented. For pure-only
functions, definition order need not be VM stack order: the backend traverses
each initializer/return expression and emits left operand, right operand, then
`addv`/`subv`, preserving subtraction order. No constant folding is performed.
VM selection lives in `ciq_compiler::backend::lower`; `Function::lower()`
delegates to it. Source parsing, IR validation/text I/O, and assembly encoding
share the existing stage boundaries.

## Local declarations and reads

The frontend accepts zero or more `var <identifier> = <expression>;` statements
before the final return. Expressions can read earlier initialized locals
repeatedly; duplicate declarations, self/forward/unknown reads, uninitialized
declarations, reassignment, nested scopes, and other call forms are rejected.
Locals are function-scoped and retain their initializer value. This restricted
subset does not imply Monkey C `var` is immutable generally.

IR distinguishes semantic `LocalId` from `ValueId` and physical VM slots.
`LocalInitialize` consumes an expression value and declares a unique local;
`LocalRead` defines a fresh value from a previously initialized local. Every
expression value still has exactly one use; repeated source reads create
separate read definitions. Initializations execute in source/IR order, including
unused locals. The backend assigns local slots after parameters in
initialization order, preserving slot 0, uses
`argcincsp <parameter-count + 1> <local-count>`, and emits `lputv`/`lgetv`.
Without locals it uses `argc <parameter-count + 1>`. Parameters plus locals are
limited to 255 and functions to 1024 total operations. [Controlled SDK output
and simulator results][ref-controlled-sdk-output-and-simulator-resu] confirm
two/three-local bodies; the 255-local limit is checked structurally in tests,
not established as a device runtime limit.

Provisional version-1 text gains `.local $<decimal-u32-id> %<value-id>` and
`.read %<result-id> $<local-id>`, each with the existing optional span. Semantic
IDs need not be contiguous; their numeric values never select physical slots.

## User module calls

The supported call form is `<module>.<member>(<arguments>?)` in integer
expressions/initializers. The module is a top-level user module provided by the
surrounding PRG scaffold, and the member is an existing integer-return function
with matching positional arity. Global unqualified calls, local/object
receivers, nested modules, void/string calls, and standalone call statements
remain unsupported. Module/member names stay symbolic until encoding; neither
IDs nor DATA method records are generated here.

IR contains `ModuleCall { result, module, member, arguments }` and provisional
`.call %<result> <module> <member> [%<argument> ...]` text. Calls may have
effects. For any function containing a call, IR operations must appear in the
backend's left-to-right expression evaluation order, with initialization
statements in order. Reject incompatible schedules rather than silently
reordering effects; pure-only IR keeps its existing ordering flexibility. This
conservative restriction can change when a more general scheduling policy is
implemented.

Backend lowering selects
`getmv @__ciq_module_<module> @__ciq_member_<member>; invokemz`, for zero
arguments. Nonempty calls additionally use `frpush`, argument expressions in
order, and `invokem <argument-count + 1>`. Module bindings must identify
`globals/<module>`; the short `<module>` symbol is a distinct field identity.
The compiler CLI supports `--symbols <symbol-source.mcasm>` for CODE emission:
resolve generated aliases by unique stored names in the provided `.symbol`
declarations, reject absent/ambiguous targets, and preserve ordinary aliases for
assembly inputs. Library consumers can supply bindings through the assembly
encoder. [Runtime evidence](../investigations/compiler-calls.md) confirms return
values and observable call order; the helper functions and their DATA
module/method records are still supplied by the SDK scaffold.

## Parameters and positional arguments

The frontend accepts comma-separated untyped integer parameters and positional
integer expressions in user-module calls. Parameter identities are zero-based
positions, distinct from locals/value IDs; repeated reads define fresh values.
Duplicate names and local/parameter shadowing are rejected. Caller arity and
callee types are still obligations of the supplied scaffold.

Function IR records a parameter count (default zero); `ParameterRead` defines a
value by positional index. `ModuleCall` carries an ordered argument-value list.
Existing `.function <name>` and zero-argument `.call` text remain accepted;
optional `parameters=<count>` on `.function`, `.parameter %<result> <index>`,
and argument IDs after `.call %<result> <module> <member>` represent the
extension. Argument dependencies and nested calls participate in the existing
evaluation-order validation.

Backend prologue counts parameters plus one context slot; parameter index 0
loads slot 1, and local slots start after all parameters. Nonempty calls select
the callable, emit `frpush`, evaluate arguments left to right, then
`invokem <argument-count + 1>`. Empty calls keep `invokemz`. Function/call
parameter counts are limited to 254 and parameters plus locals to 255, so
existing u8 fields/slot indexes fit. The 1024-operation and 64-level expression
nesting limits remain. These encoding limits do not claim runtime validation at
maximum counts.
[SDK and runtime evidence](../investigations/compiler-arguments.md) verifies two
parameters, a following local, nested calls, and generated caller/callee
execution with observable argument order.

[ref-controlled-sdk-output-and-simulator-resu]: ../investigations/compiler-locals.md
