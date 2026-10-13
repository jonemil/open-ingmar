# Compiler representation design

Use a value-based semantic IR between source parsing and VM assembly. It makes
language meaning, dependencies, storage identity, and effect order inspectable
before selecting instructions. The representation remains provisional; it does
not require Garmin MIR compatibility or LLVM/SSA infrastructure.

The [source subset][source], [function IR][ir], and [unit IR][unit] specify
exact accepted constructs and validation rules. This page explains the choices
behind those contracts.

## Representation and invariants

Value IDs identify expression results; local IDs identify stored locals;
parameter indexes identify positions. None chooses a VM slot, symbol ID, import
index, or section offset. The backend accounts for hidden context separately.
Source spans provide diagnostic provenance rather than executable meaning.

Expression values have one use within their defining block. Repeated local
reads produce fresh values from stored state, avoiding recomputation of
initializers. This permits tree-based lowering without a general shared-value
scheduler. Calls, prints, and assignments preserve statement and left-to-right
expression order. Array reads also retain order because bounds failures are
observable; shared validation traverses array before index. Pure expressions
may be scheduled by dependencies.

`Discard` is an expression root that consumes a value for effects. Native source
call statements use it with an ordinary native value call, keeping contract
resolution, operand scheduling, and invocation in one path. The backend emits
one final `popv`; intermediate results remain available for chained calls.
Void calls have separate statement contracts and do not become expression
values. The IR accepts discarded pure values even though the source subset
restricts expression statements to calls and assignments.

Array literals use staged construction alongside logical expressions.
`ArrayNew` creates an empty typed array before element effects, and native
append maintains its invariant element contract. The builder keeps the array
and current element in two temporary locals, so branch joins have empty stacks
and no expression value crosses a block. The element slot is reused across the
literal. This avoids exposing a partly filled Array with Null slots as
the declared element type. Direct allocation/fill needs initialized-element
rules before replacing this strategy; the present lowering does not promise SDK
memory use.

The source builder selects Number or Char from the first element's candidate
type and records that concrete contract on `ArrayNew`. Candidate tracking uses
shared operator/native contracts, parameter annotations, fixed local types,
and user-call result declarations. It can select a nullable value's possible
non-null class, but does not establish a branch proof. Whole-function validation
checks every element and use, including null safety; saved IR carries no
inference placeholder. Empty source literals still need contextual inference.

Validated IR is immutable. Parsing source, constructing typed IR, and reading
IR text share validation so no producer can bypass backend assumptions.

Explicit nullable user signatures preserve the declared union, even for bodies
that return only scalars or Null. Constructors, source, and text validate each
return against that contract. Unit validation checks argument assignability and
requires call result metadata to equal the callee's result; callers cannot hide
a nullable result behind scalar metadata. Inferred scalar/Void contracts remain
available for functions without an explicit result declaration.

## Null-check facts

Nullable Number/String values preserve their union until control flow proves
a local or parameter non-null. `null_flow` tracks those storage identities
separately, sharing facts between typed construction and text resumption. Fresh
reads use the facts while declared types remain nullable. Joins retain only
facts provided by every path. Local writes invalidate that local's fact;
parameter slots are immutable in the supported IR. Copies are separate
identities and do not inherit a later proof of the original. No runtime conversion or assertion is emitted for narrowing.
Keeping this proof in IR validation prevents edited text from bypassing source
checks. [IR rules](../specs/compiler-ir-text.md#nullable-results) define
the supported guard forms and their limits.

## Layer responsibilities and naming

The frontend resolves source syntax, names, and scopes into semantic IR.
`ciq-compiler::backend` selects VM instructions and frame/local storage;
`Function::lower()` delegates to `backend::lower()`. Assembly owns selected
instructions, symbolic operands, layout, binding, and encoding through
`ciq-mc`. [Architecture](../architecture.md) defines crate boundaries.

Application generation consumes unit identities and CODE labels to build DATA,
imports, and container metadata. Configuration, signing, and packaging stay
outside the frontend. Extract `ciq-ir` or `ciq-codegen` only when independent
consumers or dependencies justify it; a future IR crate must not depend on its
VM backend. Keep `ciq-asm` and `ciq-dis` as the paired tools.

## API declarations and built-in values

The [API declaration model](api-declarations.md) supplies a bounded catalogue of
class/member contracts and introduction levels independently of executable
bindings. Startup callback validation reads that model. Supported parameter and
result contracts and internal value categories map to core `Lang` identities.
A separate operator-rule layer checks API operand types and supplies result
types before VM instruction selection. It retains the restricted executable
profile. Value-producing native calls retain an explicit receiver and member;
validation resolves the API contract and argument types before checking
executable support. Native receivers are evaluated before their arguments;
text resumption and effect-order checks preserve the same traversal.
General API member lookup remains separate work. Object semantics do
not require every IR value to denote a heap allocation.

## Native and textual pipelines

Source → semantic IR → assembly model → CODE is the in-memory path. IR and
`.mcasm` text are optional inspection/resumption boundaries. Text readers
reestablish the same model invariants and reuse native lowering and encoding.
Every extension updates validation, lowering, text I/O, and parity tests
together.

The [shared assembly model](assembly-model.md) serves handwritten source,
compiler output, and disassembly. Retain readable assembly for debugging without
making temporary text files mandatory in the compiler pipeline.

## Control flow and joins

Explicit blocks and terminators describe control flow independently of textual
order. Every block is reachable from entry and has a return, branch, or jump;
there is no implicit fallthrough. Boolean literals, local reads, and comparison
results feed branches. Boolean locals use ordinary initialization/assignment
rules; printing and Boolean equality also consume these values.

Expression values remain block-local. Lower each expression into a statement or
terminator sink, leaving an empty evaluation stack on every edge. Stored locals
carry state across edges, with fixed slots across the function. This avoids phi
values and general stack merging while supporting branches, joins, and loops.

Short-circuit conditions build that graph directly: `&&` reaches the right
operand on true, `||` on false, and `!` reverses destinations. No eager logical
operation is serialized. Null proofs follow the same edges and intersect at
joins, so guarded right operands and selected arms use fresh non-null reads.
Value-producing logical expressions materialize a Boolean in a temporary local
before the join. A parent whose children include control flow snapshots its
receiver/operands in source order and uses fresh reads in the final block.
Statement-call arguments follow the same rule. Thus earlier calls execute
before later conditional operands, and no expression value crosses an edge.
Temporary identities are separate from source scope and share the ordinary
local validator and frame budget. A stored Boolean result carries no guard
relation; a snapshot of a proved non-null value retains that value's scalar
type. Slot reuse and snapshot reduction remain potential optimizations, not
requirements for correctness.

The source builder resolves lexical scopes and constructs the graph separately
from parsing and instruction selection. Only paths that continue contribute
join edges. Arm/body declarations disappear from the enclosing source scope;
IR retains their distinct identities. [Source rules][source] define visibility,
shadowing, unreachable statements, and return requirements.

## Mutable locals

Initialization establishes a stored identity and its fixed supported type.
Assignment replaces content in the existing slot and cannot establish initial
availability. These restrictions belong to our subset, not all Monkey C.

Reads observe the most recent write on the executed path. Evaluate the RHS
before replacing the slot. Writes make read timing observable, so effectful IR
must preserve operation order even when no calls occur. In particular, a read
cannot be defined before a write but dependency-lowered after it: that would
change the value observed. A broader scheduler would need snapshot temporaries
rather than relaxing this validation without corresponding lowering support.

## Loops and initialization analysis

A loop reevaluates its condition at the test block. Continuing body paths return
there; `continue` selects the innermost test and `break` its exit. Body-local
initializers run on every iteration using fixed slots. Source analysis keeps the
exit reachable because the first condition may be false; it does not prove
termination or impose an iteration cap.

Check graph reachability from entry to reject disconnected cycles. Definite
initialization computes a greatest fixed point: incoming availability is the
intersection of predecessor outgoing sets, and outgoing availability adds local
initializers. An empty external predecessor for entry prevents backedges from
inventing first-entry initialization. Assignments do not add availability.
Within each block, reads/writes require initialization at their operation.

This handles zero iterations and cyclic paths without a general SSA framework.
[IR rules][ir] specify declaration ordering, iteration, and validation precisely.

## Rationale

Direct AST-to-bytecode emission couples source traversal to VM stack and frame
choices. A semantic boundary makes effect order and storage requirements
reviewable before lowering, and supports the same pipeline in memory and text.

The frontend/IR/machine/encoding separation is useful without importing register
allocation, relocatable objects, or a linker. External terminology references
and parity evidence are in [the pipeline investigation][pipeline].

## Compilation units

Collect declarations and signatures before checking calls so forward and
recursive references work across files. Separate ownership from file boundaries.
Source collection resolves declared result contracts before building bodies.
Value calls carry an expected result type so function validation can check their
consumers without assuming Number. Validated functions retain their value types
for unit argument checks. Unit validation compares argument and result contracts
against callees; standalone functions retain external-binding obligations.
Unannotated ordinary parameters default to Number; noninteger source results
require annotations. General dynamic inference remains outside this subset.

Lower all bodies into one assembly layout with owner-qualified labels. Metadata
uses those same identities and labels. Unit text preserves ownership alongside
function IR; native and saved paths share checks and bindings. [Unit
rules][unit] define declaration order, call resolution, labels, and text syntax.

## Literal content and placement

Semantic strings carry UTF-8 content rather than DATA addresses. Lowering
deduplicates content per function and emits unplaced declarations with symbolic
`news` operands. Explicit DATA binding or the application builder supplies
placement, keeping function IR independent of section layout. PRG reconstruction
completes method bindings after layout.

`System.println` is an explicit intrinsic with an established SDK binding;
arbitrary SDK identities and return types are not inferred from spelling.
[String evidence](../investigations/compiler-strings.md) supports this boundary.

## Void results and effect-only calls

Void returns have no value dependency. Statement calls preserve argument order
and define no semantic result, whether their callee returns a value or Void.
This keeps effect-only calls outside the expression graph. Unit validation
rejects void callees in value contexts; lowering discards statement-call runtime
results. The application wrapper follows the entry's result contract.

## Class ownership and instance receivers

Classes own instance-method bodies and a native base identity, separately from
global/module declarations. Function IR records its receiver requirement; unit
validation associates it with an owner and resolves explicit self calls within
that class. The receiver is hidden context, not a first-class semantic value.

Owner-qualified labels distinguish same-name bodies. Application generation
binds methods to the source AppBase DATA record and resolves native identity to
imports; IR never stores import indexes. Generated callbacks and View/base setup
remain separate until source native calls and object/null/array values are
supported. [Class rules](../specs/compiler-classes.md) define the supported profile;
[application design](console-application.md) describes the intended extension.

[source]: ../specs/compiler-subset.md
[ir]: ../specs/compiler-ir-text.md
[unit]: ../specs/compiler-unit.md
[pipeline]: ../investigations/compiler-pipeline.md
