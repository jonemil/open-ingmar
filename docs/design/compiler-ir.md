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

`ArrayWrite` is a resultless statement with receiver, index, and value operands.
It preserves the receiver's invariant Number/Char/View element contract and
existing alias identity. Source View instances project to the native element
type; array slots do not retain exact source-class identity. The backend emits each operand once, left to right, then
`aputv`; shared effect validation rejects reordered definitions. When source
operands need control-flow staging, the builder snapshots all three into
ordinary typed locals in that order. This retains the earlier receiver/index
across later branches while leaving plain writes free of temporary slots.
Bounds checks remain with the runtime, after operand effects.

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
inference placeholder.

Direct empty literals receive an element contract from a declared return,
user-call parameter, established array local assignment, or catalogue-resolved
native argument. The declaration pass collects user parameter/result contracts
before body construction, including cross-file and self calls. A native
receiver is evaluated and snapshotted before its generic argument types are
resolved. Context selects an allocation contract; it does not authorize a call
or coerce a nonempty literal. Unsupported or ambiguous context is diagnosed.
Empty construction emits only `ArrayNew`, without append or element locals.

Native object references retain an explicit `NativeClass` identity in values,
parameters, results, and Array elements. `NativeNew` means allocation followed
by initialization; it is order-sensitive even when its result is consumed later.
The backend preserves the instance across the initializer call and discards
only that call's result. Native construction supports View with a zero-argument
initializer. Dc references arrive through callback parameters and cannot be
constructed or put in arrays. Scalar operators and object methods remain
separately checked.

`SourceNew` retains a source class name and native base for unit resolution and
application binding. Its value uses the base's supported native projection,
currently View or BehaviorDelegate, with exact constructor identity retained
as a local refinement
for bounded instance dispatch. Source lowering resolves the base from collected
class declarations, including declarations in later files; module methods do
not establish a class. Function validation checks native construction support;
unit validation checks the claimed base against the class declaration and
requires a validated initializer with matching argument contracts. Saved IR
carries both identities so future native bases need not be inferred as View.

Exact source-class identity is reconstructed from IR by shared object tracking.
It follows construction, explicit source parameters/results, local initialization,
copies, and same-class reassignment. Function validation checks returns and an
ObjectCall's claimed class; unit validation resolves source declarations and
checks call arguments/results. Generic native references, native View parameters/
returns, and array reads erase exact identity. Named SourceObject contracts
remain restricted to View subclasses and carry an
owned class name while their native API projection remains View. Constructed
delegates retain their distinct base type and local identity; native delegate
signatures erase the exact source identity. This separates
exact source assignability from native compatibility without a runtime cast.
Source-reference fields retain the same exact contract through checked field
reads and writes. Their null metadata default is hidden behind mandatory source
initializer assignments; the initialization prefix prevents reads or self calls
before all reference fields are established.

Native constants retain module/member identity and catalogue type. FONT_MEDIUM
is a FontDefinition enum value, even though lowering emits its numeric value.
This prevents a plain Number from satisfying the drawing method's font contract.
Native Void member calls are effect-only operations with no semantic result.

Positional results use `TupleShape`, distinct from mutable Array element
contracts. `TupleNew` validates one View or a View/delegate pair and accepts only
already evaluated local/parameter reads. The source callback snapshots its slots
expressions first, preserving their effects before allocating and filling the
fixed-size runtime array. Temporaries share the normal frame budget. This
sealed shape has no generic Array mutation/read profile; stored copies preserve
the same shape. Saved IR cannot relabel an arbitrary Array as a tuple.

An AppBase `getInitialView` accepts the native View or View/delegate result
contract without a source annotation. Every return must construct that shape. Application
generation binds its DATA field directly to the source CODE target and omits
the generated blank-view body. The bootstrap supplies its default only when
there is no source override. Richer construction contexts remain separate work.

Validated IR is immutable. Parsing source, constructing typed IR, and reading
IR text share validation so no producer can bypass backend assumptions.

Explicit nullable user signatures preserve the declared union, even for bodies
that return only scalars or Null. Constructors, source, and text validate each
return against that contract. Unit validation checks argument assignability and
requires call result metadata to equal the callee's result; callers cannot hide
a nullable result behind scalar metadata. Inferred scalar/Void contracts remain
available for functions without an explicit result declaration.

Division and remainder remain Number operations selected as VM instructions.
Their runtime failures make evaluation order observable, so they activate the
same strict definition/effect schedule as calls and reads. Saved IR cannot move
a potentially failing computation ahead of an earlier computation merely by
changing its consumer order. Multiplication retains ordinary dependency-based
lowering when no ordered effects are present. No arithmetic folding or
exception-handling policy is introduced.

## Null-check facts

Nullable Number/String values preserve their union until control flow proves
a local or parameter non-null. `null_flow` tracks those storage identities
separately, sharing facts between typed construction and text resumption.
Direct null equality establishes a false-edge fact; null inequality establishes
a true-edge fact. Boolean negation reverses the edge for either comparison. Fresh
reads use the facts while declared types remain nullable. Joins retain only
facts provided by every path. Local writes invalidate that local's fact;
parameter slots are immutable in the supported IR. Copies are separate
identities and do not inherit a later proof of the original. No runtime conversion or assertion is emitted for narrowing.
Keeping this proof in IR validation prevents edited text from bypassing source
checks. [IR rules](../specs/compiler-ir-text.md#nullable-results) define
the supported guard forms and their limits.

## Layer responsibilities and naming

The frontend resolves source syntax, names, and scopes into semantic IR.
Arithmetic precedence uses an iterative operator stack so nested calls and
parentheses do not require an extra parser frame for each precedence level.
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

FieldKind separates Number metadata defaults from null source-reference slots
requiring initializer assignments. SelfFieldRead/Assign retain symbolic names
and an implicit current receiver. Unit validation checks the owning declaration,
read/result claims, and exact reference writes; function validation checks effect
order. Reads are observable across mutations and calls, so saved IR cannot defer
an earlier read past a later write. A local captures a needed snapshot.
Reference initialization precedes control flow, self calls, and known shutdown
effects; exit effects propagate through source call graphs. This bounded prefix
establishes normal initializer postconditions without interprocedural field-write
inference or claims about runtime failures and unknown native effects.

Owner-qualified labels distinguish same-name bodies. Application generation
binds methods to their AppBase or View DATA record and resolves native identity
to imports; IR never stores import indexes. Callback signatures are resolved
against the owner's base, preserving Dc parameters and Void results through
saved IR. Generated lifecycle bodies supply only missing source overrides.
[Class rules](../specs/compiler-classes.md) define the supported profile;
[application design](console-application.md) describes the intended extension.

[source]: ../specs/compiler-subset.md
[ir]: ../specs/compiler-ir-text.md
[unit]: ../specs/compiler-unit.md
[pipeline]: ../investigations/compiler-pipeline.md
