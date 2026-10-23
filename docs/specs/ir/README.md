# Semantic IR

[Grammar](grammar.md) defines accepted IR text syntax; this reference owns
meaning, validation, and implementation requirements.

Semantic IR retains computations, storage identities, types, and effect order
before VM instruction selection. Native construction and text resumption pass
through the same validation.

[Source semantics](../frontend/README.md) define the shared type and behavior
contracts; [lowering](../lowering/README.md) owns their VM implementation.

Contents:

- [Model and validation](#model-and-validation)
- [Compilation units and classes](#compilation-units-and-classes)
- [Values and storage](#values-and-storage)
- [Dynamic values](#dynamic-values)
- [Control flow](#control-flow)
- [Containers and private construction](#containers-and-private-construction)
- [Calls and objects](#calls-and-objects)

## Model and validation

This specification defines the semantic model and its optional text format. The
representation is provisional and independent of Garmin MIR. The in-memory
pipeline is source → validated semantic IR → symbolic VM assembly → CODE; text
readers reconstruct the same models rather than defining alternate paths. See
[architecture](../../architecture.md) and [pipeline
evidence](../../investigations/compiler-pipeline.md).

### Semantic model

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

[Source contracts](../frontend/README.md), [class contracts](../frontend/README.md#source-classes),
and [unit contracts](../frontend/README.md#declarations-and-name-resolution) own
accepted language behavior and resolution. [API
declarations](../sdk/README.md#api-declarations) own catalogue semantics. Shared
invariants and operation-family contracts are collected here;
[grammar](grammar.md) owns serialization, and [lowering](../lowering/README.md)
owns instruction selection.

### Native union contracts

`ParameterType::Union(Type)` and `ReturnType::Union(Type)` retain normalized
native API alternatives when no established named contract applies. Standalone
Null is a signature type too. Saved signatures, fixed local contracts, and
block-value metadata use the [union tags](grammar.md#type-tags); reload validates
and normalizes their alternatives. Top-level source-class identities and Void
are excluded from native union atoms; tuple atoms retain nested source slots.

The [frontend union rules](../frontend/README.md#native-unions) own assignment,
operation, and narrowing semantics. IR validation recomputes these checks from
operands and CFG facts; edited result/read metadata cannot invent a type proof.
Both branch edges carry direct class/Null refinements, and storage writes kill
those facts. Every possible native receiver/operand must support an operation;
result alternatives are normalized. Container reference unions retain invariant
content contracts. Fields and lexical casts retain their existing domains.

### Source interface contracts

`StructuralInterface` contains name-ordered fields with `ReturnType` contracts
and `StructuralMethod` signatures with `ParameterType` arguments and a
`ReturnType` result. `api::Type::Structural`, parameter/result contracts, joins,
and fixed storage retain these finite shapes; typedef names do not survive expansion.
Typed construction and text reload validate names, member/argument/depth limits,
and canonical supported types. Mutable fields are invariant and method
signatures match exactly. Exact source member types, including nullable/mixed
unions, remain distinct from their native projections. Unit validation resolves
nested class references against constructible declarations and native bases,
including references in otherwise unused field/local contracts.

`.member-read result receiver name` derives its value type from the receiver's
declared field. `.member-write receiver name value` checks the value against
every possible receiver alternative. Receiver and value IDs obey ordinary
use/order rules. Reads and writes participate in effect scheduling; declarations
or edited metadata cannot move a read across a write. Instance calls reuse
`.call-native`/`.call-native-discard` with the receiver's structural declaration
supplying arity, argument, and result contracts. The operation names cover
structural dispatch as well as catalogue-provided members.
Source provenance derives from the declared member contract, rather than its
native projection. Union receivers check each signature's arguments and join
results without discarding exact source alternatives. Reads remain snapshots;
mutation does not change an earlier read's contract or runtime value.

`ArrayElement::Structural`, `DictionaryKey::Structural`, and
`DictionaryValue::Structural` retain finite member maps instead of erasing them
to Object. Array elements and Dictionary values carry a separate nullable flag;
keys exclude Null. These owned contracts use Clone rather than Copy.
Container construction/mutation operations retain their ordinary forms and
derive full content types from the receiver or builder. Unit validation proves
source implementations at every ingress, including lookup keys. Native
signature projection alone cannot discharge that proof. Nested source member
references resolve within container contracts, and depth checks traverse them.
The [frontend rules](../frontend/README.md#source-interface-types) own invariance,
literal context, snapshot behavior, and applicable container operations.

Function checks may defer an exact source implementation to unit validation.
The unit resolves the required source/native member signatures at every checked
boundary before application generation. Opaque native base types cannot invent
structural witnesses. [Frontend rules](../frontend/README.md#source-interface-types)
own assignment, nullability, and observable behavior; [type tags](grammar.md#type-tags)
own saved spelling.

### Typed callback contracts

Parameter/result contracts, `api::Type::Callback`, and saved tags retain a
`StructuralMethod` signature with source-aware arguments/results. Exact signature
equality controls typed assignments; plain Method and Any retain gradual boundary
checks. Callback container contracts preserve signatures and content
nullability. Shared signature validation and source-reference resolution traverse
nested callbacks, interfaces, and containers.

`.bind-method result receiver selector callback-tag` binds a literal Symbol value
and records the resolved signature. Function validation checks operands, signature
shape, API types, and evaluation order; unit validation confirms the selector,
every exact receiver, target signature, and native member introduction level.
An edited tag cannot assert an arbitrary binding signature or bypass a source
override of Object.method. A standalone function
retains its unit-level binding obligation.

Invocation reuses `.call-native` and `.call-native-discard` with member `invoke`.
The callback signature supplies arguments and results, including exact source
provenance and Void. Receiver/argument effects execute once in order. The backend
binds through Object.method using the existing symbol and invocation instructions;
callback tags add no VM class, conversion, or PRG record. Signature checking does
not prove callback behavior or absence of shutdown effects. The
[frontend contract](../frontend/README.md#typed-callbacks) owns source syntax,
gradual erasure, inference limits, and observable behavior.

### Validation

Source typedefs are resolved before IR construction. Function signatures, field
contracts, and operation types retain the expanded types; neither typed IR nor
saved text needs alias declarations or a separate alias-resolution pass.

`Function::new` validates explicitly constructed operations;
`compile_function(source)` constructs an immutable Function through the same
validator. `parse_function_ir` resumes saved function text with those checks.
These are body-level entry points, not
complete-program validation: user module callees may remain external and require
later symbol binding. `compile_sources` and `parse_unit_ir` additionally require
declared callees and check their contracts. The unit text reader uses the same
function reader for its nested bodies. Public IR types are provisional. None of
these entry points may bypass their applicable semantic validation before
lowering.

Definitions are unique; operands refer to earlier definitions and every defined
expression value has exactly one use in its own block. Local initialization
consumes a value, while each read defines a fresh value. Unread locals are
legal: their initializers still run. Shared value graphs, dead definitions,
invalid names/spans, out-of-range constants, and malformed control flow are
rejected. Source compilation supplies optional spans; typed/text IR may omit
them. The function is limited to 1024 operations, including block markers and
returns; parameter metadata is not an operation.

An implicit block must end in one return. Explicit CFGs require the
[control-flow invariants](#control-flow). Locals and private builder state
cross edges only under their respective availability/progress checks; expression
values cross an edge only as a single typed join argument.

For pure expressions, definition order may differ from VM stack order: lowering
walks dependencies for each sink, left operand before right. Potential failures
such as division/remainder, allocations, calls, prints, reads/writes with
observable effects, and ownership transfers impose the checked expression and
statement schedule. Saved IR that would reorder effects is rejected rather than
silently rescheduled. [Call operations](#module-calls-and-operation-order) and
[joins](#expression-joins) specify the applicable boundaries.

`parse_function_ir` diagnoses input byte positions; typed construction reports the
operation index when available. Unit validation adds declaration ownership,
exact call signatures, field contracts, and callback restrictions after checking
each function. [API context](../sdk/README.md#api-context-inputs) adds
availability checks without changing serialization or authorizing unsupported
execution profiles.

### Native and text boundaries

`Function::to_ir_text` preserves value IDs, receiver metadata, and optional
spans. `parse_function_ir` reconstructs that semantic model. `Function::lower` constructs
shared assembly directly; `Function::assembly` lowers and prints it. Native and
saved-stage compilation must produce equivalent assembly/CODE under identical
bindings. Text is not mandatory between stages.

## Compilation units and classes

`ClassDeclaration` records a name, native `ClassBase`, ordered fields, and
ordered methods. Bases are identities rather than VM import indexes.
Methods include the frontend's synthesized Object default initializer when
source omits it; the IR reader and unit constructors do not synthesize bodies.
`Function::method` constructs receiver-bearing IR; ordinary function
constructors reject receiver operations. Unit validation requires instance
bodies in classes and ordinary bodies in global/module owners. A method with no
self operations still retains its receiver contract.

`SelfCall` defines a result under an explicit contract; `SelfCallStatement`
consumes arguments without a result. Both share single-use, block-local,
control-flow, type, and effect-schedule checks with other calls. The hidden
receiver is not an explicit parameter or semantic value. Source ownership is
checked separately from member Symbol IDs.

The [unit/class grammar](grammar.md#unit-and-class-framing) serializes global,
module, and class owners, including Lang.Object bases and every supported field
kind. [Source class contracts](../frontend/README.md#source-classes) define
constructibility, receiver provenance, and ordinary versus callback invocation;
[field contracts](../frontend/README.md#instance-fields) define types, defaults,
required initialization, and snapshot/null proofs. Unit reload rechecks these
same contracts. A result tag cannot forge a field or source-class identity
merely because a native projection matches.

### Unit IR text

Each owner record wraps exactly one complete function. Class containers wrap
fields followed by complete receiver-bearing method documents. Inner function
validation precedes ownership and cross-body call checking. Reject malformed
framing, duplicate owners/members, unresolved references, mismatched receiver
contracts, wrong arity, incompatible arguments/results, invalid fields, and
ordinary calls to native lifecycle callbacks.

The unit contains 1..=256 bodies including methods. Error body indexes follow
all global/module bodies, then class methods. [Unit
layout](../lowering/README.md#unit-layout) owns this grouped order and generated
CODE labels. Multiple supported helper classes may accompany the application
class; unit validity alone does not select an application entry or establish the
complete-app profile.

Spans remain optional per-function provenance. Filenames and source-file indexes
are not serialized. Native source compilation and saved unit IR produce the same
assembly/CODE with identical bindings. Standalone function IR remains supported,
with unresolved declaration checks retained as caller obligations. Examples and
record syntax are in the [IR grammar](grammar.md#examples).

## Values and storage

### Boolean and Number inversion

`.not` consumes one non-null Boolean, Number, or Long and defines the same type.
Integer inversion complements all 32 or 64 bits, as for `.complement`; nullable
operands need a non-null proof. Other types are rejected without coercion. The
operand must be an earlier definition in the same block and obey ordinary
one-use rules. Typed and text IR recompute the contract and include the operand
in effect traversal. Lowering evaluates it once and emits `invv`, without a
native binding. Negating a direct null comparison reverses its non-null proof
edge when used by a branch. See the [runtime
evidence](../../investigations/compiler-boolean-negation.md).

Short-circuit expressions use existing `.block`, `.branch`, and `.jump` forms,
not `.and`/`.or` directives. Each operand ends its own block; the right operand
is entered only on the required outcome. Negated source conditions may reverse
destinations without a `.not`. Every branch still requires Boolean; ordinary
block-local value, effect-order, and empty-stack edge rules apply. Text resumes
the explicit graph rather than reconstructing source operators. See the
[source profile](../frontend/README.md#short-circuit-expressions).

Value-producing logical expressions also use `.boolean`, `.local`, `.assign`,
and `.read`. A temporary Boolean local is initialized before branching, assigned
true on the true outcome, and read after both outcomes join. Surrounding
operands evaluated before a later branch are stored in distinct temporary locals
and read in the final block. Temporary locals obey the same type,
initialization, frame-budget, and null-proof rules as source locals; no
producer-only metadata or new directive grants special treatment. These
logical-result edges have an empty stack. See the [value-result
evidence](../../investigations/compiler-logical-values.md).

### Number negation

`.positive` consumes one non-null numeric value and defines an unchanged result
of the same type. Concrete widths and numeric aliases follow the [source numeric
contracts](../frontend/README.md#numbers-and-numeric-aliases). Nullable operands
require a checked fresh read under a non-null proof; other types are rejected.
Definitions, single-use rules, block boundaries, spans, and effect order apply
as for other unary operations. Lowering emits the operand exactly once and no
additional VM instruction. This preserves type checking through saved IR even
though executable code is identity. `.integer` spelling still rejects a leading
plus; source `+1` uses `.integer 1` followed by `.positive`. See [unary-plus
evidence](../../investigations/compiler-number-plus.md).

`.negate` consumes one Number Long, Float, Double, or a supported numeric alias
and defines the same type. String, Char, Boolean, Null, and unchecked nullable
values are rejected. Operands must be earlier definitions in the same block and
obey ordinary one-use rules. Source and typed/text IR recompute the contract;
fresh reads with a direct non-null proof may feed it. Effect traversal evaluates
the operand once and retains call order. Lowering emits zero, the operand, then
`subv`; it requires no native member or new opcode. Minimum Number negation
wraps to the same i32 minimum in the tested runtime. See the
[evidence](../../investigations/compiler-number-negation.md).

### Floating constants and values

`.float` and `.double` produce Float/Double values from exact IEEE 754 bits.
Width, prefix, and hexadecimal digits are mandatory. Native
`Operation::Float`/`Double` hold u32/u64 bits, preserving signed zero,
infinities, and NaN payloads through text, equality, and binary round trips.
Saved IR accepts every bit pattern even though source literals must be finite.

`float`/`double` are parameter, user-call result, and expression-join tags;
`nullable-float`/`nullable-double` additionally serve as explicit function
result contracts. Shared source/IR validation applies the
[floating contracts](../frontend/README.md#numbers-and-numeric-aliases), including numeric
promotion, null proofs, native calls, and invariant storage types.

Lower positive zero with `fpushz`/`dpushz`; all other bits, including negative
zero, use `fpush`/`dpush`. These carry four/eight big-endian operand bytes.
Arithmetic and comparisons use existing VM operations. Negation emits same-type
positive zero followed by the operand and `subv`. No DATA constant is needed.

### Long constants and values

`.long` produces a Long without Number narrowing. Decimal text permits an
optional leading minus; source suffixes, leading plus, hexadecimal text, and
values outside i64 are invalid. `Operation::Long` holds i64; text/native
construction share the same scalar type and value-use checks.

`long` and `nullable-long` are parameter and user-call result tags. Functions
may infer uniform Long returns; `.function result=long` is not a separate field.
Unit calls must match callee signatures exactly, including Long versus Number.
Locals and printing retain the Long type; unsupported scalar operators and
container forms remain invalid. API checking uses `Toybox.Lang.Long`, since
1.0.0. Explicit nullable returns use `.function result=nullable-long`; native
Long methods and String conversions use the shared `.call-native`
representation.

Lower zero as `lpushz`; other values use `lpush` with one big-endian eight-byte
operand containing the signed value's two's-complement bits. The semantic IR
holds no split words or DATA address. Both source and saved IR use this
lowering.

Existing arithmetic and comparison directives accept the bounded Number/Long
pairs from the [source contract](../frontend/README.md#long-values). They retain
semantic `Binary`/`Compare` operations, choose Long results where required, and
use the existing VM arithmetic/comparison instructions without conversion calls.
Long-aware `.positive` preserves its operand; `.negate` uses `lpushz`, the
operand, and `subv`. Validated value types drive zero selection. Number negation
keeps `ipushz`. No arithmetic results or promotions are trusted from text tags.

`.not` and `.complement` on Long retain Long and select `invv`. `.increment` and
`.decrement` preserve Long through ordinary read/add-or-subtract/write lowering,
using a Number one operand. Shifts use `shlv`/`shrv` and leave count
normalization to the runtime; they add no masking or conversion instructions.
Source and text validation recompute all these results, including a Long result
for a Number left operand with Long count. Statement-only update syntax and
storage invariance remain unchanged.

### Char constants

`.char` defines Char from raw u32 bits. Exactly eight digits follow lowercase
`0x`; canonical output uses lowercase digits. This is independent of the
narrower source literal syntax: surrogates and non-scalar bit patterns are
preserved. Validation gives the value the distinct Char type and requires API
1.3.0 when a context is supplied. Lowering emits `cpush` with the same bits,
without a DATA string or native conversion call. Invalid widths and malformed
operands are rejected. See the
[literal evidence](../../investigations/compiler-char-literals.md).

### Number and Char steps

`.increment` and `.decrement` add or subtract one from a non-null Number, Long,
Float, Double, or numeric alias. Increment also accepts Char (API 1.3.0); both
operations preserve the operand type. Typed construction uses `Operation::Step`
with an `increment` Boolean. Char decrement, other classes, and unproven
nullable operands are rejected. These are semantic value operations:
capture/read and target write-back remain explicit surrounding operations.
Source updates themselves are statements and expose no expression result.

[VM lowering](../lowering/README.md#number-and-char-steps).

### Numeric aliases

`numeric`, `integer-union`, and `decimal` denote the supported Numeric, Integer,
and Decimal aliases. `integer` continues to mean concrete Number. Their nullable
tags are `nullable-numeric`, `nullable-integer-union`, and `nullable-decimal`.
These are parameter, function-result, user-call-result, cast, and
conditional-edge tags; aliases introduce no runtime class or conversion.
Validation recomputes operator results, common native members, class
refinements, and current null proofs under the
[source rules](../frontend/README.md#numbers-and-numeric-aliases). A result annotation cannot
narrow an arithmetic union or establish a null proof. Lowering uses ordinary VM
operators; alias negation uses Number zero subtraction. Concrete scalar field
contracts use
[`.field-scalar`](../frontend/README.md#typed-scalar-fields), preserving the same
scalar/nullability tags. Numeric alias field declarations reuse their tags
through
[`.field-numeric`](../frontend/README.md#numeric-alias-fields); reads retain the
exact declaration and writes are checked against it after unit reload.

Array element tags also use `numeric`, `integer-union`, `decimal`, and their
nullable forms. Parameter/call-result tags append `-array`; conditional-edge
tags prepend `array-`. `.array-new`, `.array-begin`, `.array-allocate`, and
`.array-constant` retain the selected element contract. Dictionary contracts
reuse these scalar tags on either side of `=>`, excluding nullable keys. Native
methods and index operations recheck the
[source container rules](../frontend/README.md#numeric-alias-containers), including
invariance and null proofs, rather than trusting a saved result annotation.

### Dynamic values

`Any` is a dynamic value contract, distinct from Object and Void. It includes
Null and has no API introduction level. In-memory parameter/result contracts
and `api::Type` expose `Any`; saved parameter, result, local, and block-value
contracts spell it `any`. The source frontend uses these contracts for
[omitted annotations](../frontend/README.md#function-type-contracts).

Any may cross supported typed argument, assignment, and return boundaries
without a conversion or generated guard. Concrete mismatches remain errors
when neither side is Any. Container element contracts do not widen to Any;
computed Array literals may select Any elements when their first value is
dynamic. Their saved element tag is `any`, signature/call tag `any-array`, and
join tag `array-any`. Reads produce Any; fills and writes admit values including
Null. The element contract is invariant: an existing Array<Number> is not an
Array<Any>. Dynamic-element arrays use ordinary construction/fill operations;
the per-type scalar constant format does not admit `any` constants. This does
not introduce Dictionary Any parameters or new source annotation syntax.
Void calls still cannot produce values, and an Any result
requires a value return. Null is an ordinary admissible return value.

Source empty/bare-return bodies lower to `.null` and `.return`, with an explicit
Any result contract. `.return-void` remains a no-value return, not an implicit
Null. A declared concrete result returned from an Any expression must also be
serialized explicitly: inferring the saved function result from that expression
would lose the declared boundary. Unchanged concrete returns may still omit
redundant result metadata under the ordinary text rules.

Arithmetic and unary operations with dynamic operands use the existing
operators and produce Any; comparisons produce Boolean. Dynamic conditions
use the ordinary branch. Indexing an Any receiver uses existing indexed
operations with dynamic keys/values and an Any read result. A concrete Array
receiver retains its Number index and element contracts, admitting Any arguments
under the dynamic boundary rule. Runtime type, bounds, and operation failures
remain possible.

Calling a named member on Any accepts structurally valid names and up to 254
arguments, producing Any. Method existence, arity, and runtime argument types
are not statically known. On a concrete receiver, member resolution, supported
executable profiles, arity, and API availability still apply. An Any argument
may satisfy a known parameter without changing its declared result contract.
Names share pinned API IDs or allocated source-member IDs in the application;
there is no independent dynamic-member namespace.

Runtime class tests can narrow fresh Any storage reads through the shared
fact analysis. Any alone does not prove a source class; passing it through an
explicit source contract defers that boundary to runtime. Writes invalidate
old facts. Inferred-variable read metadata cannot replace an incoming Any view
with an invented concrete contract; narrowing must be independently established.
Structural checks remain mandatory: definition/use order, frame limits,
definite assignment, block-local values, and effect scheduling.

The [saved-IR example](../../../examples/ir-any/README.md) and
[runtime evidence](../../investigations/compiler-arguments.md#dynamic-ir-contracts)
cover these operations through assembly, PRG reconstruction, and the simulator.

### Local operations

`.local` initializes a local from an earlier value. `.read` defines a fresh
value from an earlier initialized local. Both accept the same optional source
span. Local IDs use `$` followed by decimal `u32`; value IDs still use `%`.
Initializations must have unique IDs; reads before initialization,
reinitialization, and parameters plus locals exceeding 255 are rejected. Unread
locals are accepted: their initializer still executes. Source names are resolved
by the frontend and are not stored in this provisional IR.

`.declare-local $id <contract>` reserves typed storage without a value. Its
contract uses the existing API/source spellings, including opaque/native/tuple
domains excluded from conditional joins. It counts toward the frame and
operation limits but emits no default initialization. Declaration IDs share the
same uniqueness domain as `.local`. A plain `.assign` may establish its first
value; reads/takes require assignment on every executable path. The declaration
must dominate each assignment, independently of value availability. Source
contracts retain exact identity, and interface writes retain unit-level
signature checks. Re-executing a declaration clears assignment status for its
new iteration. This is a checking rule, not a runtime slot clear: earlier
references retain ordinary frame/write lifetimes until replaced or the frame
exits.

`.variable $id [%initial]` reserves inferred storage, optionally initialized.
Unlike `.local` and `.declare-local`, its assignments may change type. It obeys
the same declaration uniqueness, dominance, frame budget, and definite
assignment rules. `.read-variable %result $id <contract>` defines a fresh value
with a checked read contract; ordinary `.read` and `.take` cannot read inferred
storage. Hidden ownership-transfer temporaries retain fixed storage contracts.

Validation reconstructs possible writes at each CFG point to a fixed point,
replacing a slot's alternatives on assignment and combining them at incoming
edges. Read contracts must accept the resulting view, refined only by shared
runtime-class/non-null facts. Metadata cannot assert a type unsupported by the
incoming values. Exact source identities, supported numeric aliases, comparison
interfaces, and nullable joins retain their contracts; other supported native
alternatives retain normalized unions. Exact source alternatives retain source
unions, with native alternatives retained separately from exact identities.
A write invalidates old storage proofs;
previously captured values retain their own contracts and identity.

Source construction selects inferred storage when writes change the local's
contract and recomputes reads and conditional result contracts after loop
backedges are known. Fixed storage remains useful for unchanged source locals,
hidden temporaries, and explicit IR contracts. Saved IR validation checks both
forms independently of source inference.

`.take` defines the same typed value as `.read` but transfers ownership out of
the local. The local becomes unavailable on that CFG path; later reads and takes
fail until its initialization executes again (for example, on another loop
iteration). Assignment can refill explicitly declared storage; ordinary `.local`
storage retains its initialization requirement. Joins require availability on
every incoming edge. A take is ordered with other effects and cannot move across
writes or calls. Each resulting value still has exactly one use.

[VM lowering](../lowering/README.md#local-operations).

Local IDs are semantic identities, independent of their numeric value or VM
slot. The backend assigns local slots after parameters in initialization order
and executes every initializer before proceeding. Repeated reads may refer to
one local, while each resulting value still has one use. Pure-only functions use
dependency traversal while locals retain their initialized value. Effectful
functions also obey the [shared schedule](#validation). `.assign` writes a local
without repeating initialization.

```text
ciq-ir
.function probe
.integer %10 42
.local $99 %10
.read %20 $99
.read %30 $99
.subtract %40 %20 %30
.return %40
.end
```

### Parameter metadata and reads

The optional `parameters=<nonnegative-decimal-count>` field on `.function`
defaults to zero, with a maximum of 254. `Function::with_parameters(name, count,
operations)` uses the shared validator; `Function::new` retains zero-parameter
behavior. `Function::with_parameter_types` supplies explicit contracts.
`.parameter` defines a fresh value with the signature position's type, narrowed
under the [null-check proof](#nullable-results) when available. Index must be
below the parameter count; repeated reads and unused declared parameters are
allowed. Source parameter names are resolved by the frontend, not stored in IR.
Position is independent of value/local IDs and VM slots.

`.assign-parameter <index> <value>` replaces the signature position's stored
value and defines no result. Typed IR uses `ParameterAssign { index, value }`.
The index must be within the signature; the operand must satisfy its declared
type, exact source identity, and structural interface requirements. Source and
saved IR share these checks. The write invalidates null/class facts for that
parameter and participates in ordinary effect scheduling. It does not change
the signature or count as a local. Earlier value definitions remain snapshots;
fresh `.parameter` reads observe the replacement under current CFG facts.

Call argument IDs refer to earlier value definitions and count as uses; their
order is significant. The call-containing evaluation-order validator includes
argument trees and nested calls. Up to 254 arguments are encodable; this is a
structural bound, not a verified device maximum. Old zero-argument `.call` and
literal/locals IR remain compatible. Parameters and locals together cannot
exceed 255, preserving u8 slot indexes.

```text
ciq-ir
.function difference parameters=2
.parameter %0 0
.integer %1 3
.add %2 %0 %1
.local $0 %2
.read %4 $0
.parameter %5 1
.subtract %6 %4 %5
.return %6
.end
```

### String values and print statements

`.string` defines a string value; `.println` consumes a Number, Long, Float,
Double, String, Boolean, or Char without defining a result. Both accept optional
spans. Quotes preserve spaces, `#`, and text resembling a span. Escapes follow
[IR quoting rules](grammar.md#text-and-value-definitions); commas outside quotes are rejected.
The semantic string limit is 65535 UTF-8 bytes. String types propagate through
locals, function parameters, user calls, and returns. Arithmetic follows the
documented operand contracts. Value returns must have the same supported type;
Concrete scalar results are inferred from compatible return values, unless a
nullable result is declared as specified below. Void returns have no value. All
existing use/dependency/ order rules apply.

```text
ciq-ir
.function probe
.string %0 "Hello # from Rust"
.println %0
.integer %2 43
.return %2
.end
```

The IR string directive names a semantic value. Lowered assembly `.string`
directives name DATA requirements, deduplicated by content within the function:

```text
.string __ciq_string_probe_0 "Hello # from Rust"
probe:
argc 0x01
getmv @__ciq_sdk_System @__ciq_member_println
frpush
news &__ciq_string_probe_0
invokem 0x02
popv
ipush1 0x2b
return
```

For complete section-project assembly, put literal declarations in the
**authoritative DATA source**, removing them from CODE. Existing DATA records
can be reused by changing CODE aliases explicitly; otherwise append unplaced
strings to DATA and update the manifest length. CODE and IR/text paths consume
the same DATA bindings. See
[reproduction](../../investigations/compiler-strings.md) for source → IR → assembly
→ signed PRG and native CODE parity.

### Local assignment

Source compounds use ordinary `.read`/`.parameter`, a binary directive, and
`.assign`/`.assign-parameter` for local/parameter targets respectively.
There is no compound IR directive. Shared validation checks operand contracts,
result assignability, null proofs, and read-before-RHS/write ordering. Staged
RHS expressions use the existing typed local/snapshot representation. See
[source rules](../frontend/README.md#local-and-parameter-compound-assignment) and
[evidence](../../investigations/compiler-compound-locals.md).

`.assign` consumes an earlier value and replaces an initialized local. No
explicit blocks are required for a straight-line assignment body.

```text
ciq-ir
.function probe
.integer %0 40
.local $0 %0
.read %1 $0
.integer %2 2
.add %3 %1 %2
.assign $0 %3
.read %4 $0
.return %4
.end
```

The target must be declared on every incoming path. An assignment may establish
the first value of `.declare-local` or `.variable`; ordinary `.local` requires
its initialization. `.assign` does not introduce an identity or count as another
local. Repeated declarations for one ID remain invalid. Fixed storage retains
its initializer's or declared contract; inferred variables use the flow rules
above. Undefined
values/locals, wrong-path targets, shared values, and cross-block RHS values are
rejected. The RHS contributes one use; assignment defines no result. Each write
counts toward the 1024-operation limit and reuses the existing slot.

If a function contains any assignment, its entire operation sequence must match
left-to-right expression evaluation and ordered statement sinks, as for calls
and prints. A read defined before a write and used after it is rejected, since
dependency lowering would move the read past the write. Store a snapshot in a
separate initialized local before the write when that behavior is needed. This
rule applies to native typed construction and text resumption equally.
Assignment-free pure functions permit flexible definition order.

[The source subset](../frontend/README.md#local-reassignment)
specifies name resolution and supported statement forms; source names/scopes are
not part of this IR.

### Expression joins

Conditional expressions use a single typed block parameter, independently of
mutable locals. Native IR uses `BlockParameter { result, value_type }`
immediately after `Block`; incoming edges use `JumpValue { target, value }`.
`value_type` is `JoinType::Api(Type)` for ordinary API contracts or
`JoinType::Source { class, nullable }` for one exact source reference, or
`JoinType::SourceUnion(SourceUnion)` for multiple exact alternatives. Source
contracts keep identities separate from their API projections.
Text syntax is:

```text
.block ^3
.block-value %7 integer
.return %7
```

Each predecessor must end in `.jump-value`, consuming one assignable value in
that predecessor. Plain jumps and branch targets cannot enter a parameterized
block. The entry block cannot have a parameter; a block accepts at most one.
Missing targets, malformed placement, type/arity mismatches, duplicate
definitions, and unused/reused values are errors in native and text
construction. The parameter defines a fresh block-local value; the predecessor's
value ID does not cross the edge.

The [join grammar](grammar.md#type-tags) defines admitted contracts and tags.
Array tags prefix the element tag with `array-`, such as `array-string` or
`array-view`. Source contracts use `source(Class@Base)` or
`nullable-source(Class@Base)`.
Incoming Arrays must match the invariant element contract exactly; non-null and
nullable elements do not widen to a common Array type. A union instead retains
each distinct container contract as an alternative. Array tags describe element
nullability; nullable references use union tags. Availability checks apply to the
parameter and its incoming values, including Char's API 1.3.0 requirement.

[VM lowering](../lowering/README.md#expression-joins).

Source joins use normalized native alternatives or exact source-class sets
with optional Null; see
[conditional expressions](../frontend/README.md#conditional-expressions) for
source inference and restrictions. Source construction resolves each join from
checked incoming operand types, including full-graph null proofs and local
writes. It stores the resolved type explicitly in the immutable function and its
text output. Native/text IR retain their declared parameter types and check
incoming values against them; parsing does not infer or silently narrow them.
Native IR may declare a compatible nullable parameter explicitly. Each source
edge must preserve the declared class, native base, and nullability. Unit checking
resolves the join's source declaration, including a nullable join fed only Null.
A source join result retains its contract through copies, calls, and field
assignment; an API projection alone cannot authorize source dispatch. Nullable edge
proofs still concern source storage and intersect at joins; an incoming value
does not create a proof about that storage. See [SDK
evidence](../../investigations/compiler-conditionals.md).

### Nullable results

`.call-native` for String `find`, `toNumber`, and `toNumberWithBase` defines
Number-or-Null according to its API contract; String `substring` defines
String-or-Null. String `toLong` and `toLongWithBase` define Long-or-Null;
`toFloat`/`toDouble` define their corresponding scalar-or-Null. Index API
requirements are the same as the [source
contract](../frontend/README.md#nullable-results). The existing `.local`,
`.read`, `.null`, `.equal`, `.not-equal`, and `.branch` forms carry this
behavior; no assertion or unchecked cast operation is introduced. Equality with
Null is accepted in either order.

Shared IR validation computes non-null local and parameter facts from control
flow. A branch whose condition is a direct `.equal` between `.read` or
`.parameter` and `.null` establishes that storage identity's fact on its false
edge. A direct `.not-equal` check establishes it on the true edge instead. A
chain of `.not` definitions around either comparison reverses the proof edge
once per operation. Every definition in the guard chain must precede its
consumer in the branch's block. The checked read must be in that block; a
checked local must have no intervening write. Both outcomes reaching the same
target provide no new proof. Entry has no external facts; joins intersect
incoming facts and every initialization/assignment kills that local's proof. The
fixed-point analysis handles backedges without treating a later guard as proof
for a loop's first entry.

A fresh read of a nullable scalar local or parameter under this proof defines
its non-null type; otherwise it defines the original union. Storage remains
nullable. Local assignment can supply Null, the non-null type, or the same
union, but always requires another proof before non-null use. Existing
effect-order, one-use, and block-local value rules apply. Stored Boolean guards
and comparisons against null-valued locals are not proof forms. Nullable values
may use matching user parameter/result contracts, but cannot use non-null
consumers without proof. Nullable Numbers may be passed as substring indices,
which explicitly accept that union. Readers and typed constructors recompute all
facts; IR text carries no trusted narrowing metadata. See
[source examples](../frontend/README.md#nullable-results).

### Nullable user signatures

A function declaration may carry `result=nullable-integer`,
`result=nullable-long`, `result=nullable-float`, `result=nullable-double`,
`result=nullable-string`, `result=nullable-boolean`, `result=nullable-char`, or
`result=nullable-symbol`. Every value return must be Null, the corresponding
scalar type, or the same union. A void return is invalid. The declared union is
preserved even when all returns are scalar or all are Null; canonical text
always writes it. Omitted result metadata infers a uniform scalar/Void contract
and rejects nullable/Null returns. Reference result tags follow the
[native value contracts](#native-value-calls). Unsupported result tags are errors.
Parameter lists independently accept the same nullable type names.

```text
ciq-ir
.function identity parameters=1 parameter-types=nullable-string result=nullable-string
.parameter %0 0
.return %0
.end
```

`Function::with_signature` and `method_with_signature` supply explicit result
contracts to the shared validator. Parameter-only constructors infer scalar/Void
results. Unit validation accepts Null, the corresponding scalar, or the same
union for nullable arguments; scalar arguments still require exact types. Call
result contracts must equal the callee's declared result, even when its body
always returns a scalar. This prevents edited call metadata from hiding nullable
results. Standalone IR has no callee signature environment.

Parameter reads retain their declared union until a direct branch proves the
parameter non-null. Later reads under that proof define the scalar alternative.
Local and parameter identities are distinct; neither a copied value nor another
parameter is narrowed by the original's proof. Parameters have no write
operation, so unrelated local writes preserve parameter facts. No casts or new
VM instructions are introduced. See the
[complete example](../../../examples/compiler-nullable-contracts/README.md).

### Runtime object predicates

[Directive syntax](grammar.md#operation-syntax) is defined in the grammar.

Both operations define Boolean and accept optional source spans. `InstanceOf`
records a canonical concrete runtime class independently of symbol IDs. Function
validation checks the supported class and API level. `Has` requires a non-null
receiver and a Symbol selector. Each operand has one semantic use; native and
text input share these checks. A direct `.instanceof` of a `.read` or
`.parameter` can establish a branch proof for fresh reads, including through
`.not` and ordinary control-flow edges. The read, test, and branch must be in
the same block with no intervening local write. Entry, join, write invalidation,
and projection rules follow the [source
profile](../frontend/README.md#runtime-object-predicates). `.has` establishes no
member-signature or API-availability proof.

Both typed construction and text parsing recompute the complete CFG fixed point;
branch proofs carry no trusted text annotations. Source selection uses candidate
facts from the prefix built so far, then submits the complete function to the
same validation. Backedges and joins can invalidate those candidates. No cast or
assertion operation is inserted. Narrowed value types guide existing native call
checks and numeric instruction selection.

[VM lowering](../lowering/README.md#runtime-object-predicates).

ByteArray tests use `Toybox.Lang.ByteArray` with API 3.0.0. Positive Object
refinement yields the concrete ByteArray contract, enabling indexed operations
as well as native methods under the shared proof rules.

### Lexical casts

[Directive syntax](grammar.md#operation-syntax) is defined in the grammar.

`Cast { result, operand, target }` consumes one value and defines a value with
an explicitly asserted API type. Native construction and text parsing check the
supported target and its API availability; they do not require assignability
from the operand or a branch proof. This explicit assertion is separate from
recomputed control-flow facts.

Target tags are `integer`, `long`, `float`, `double`, `boolean`, `char`,
`string`, `object`, `symbol`, `method`, `weak-reference`, `byte-array`,
`numeric`, `number-format`, `endian`, and `dictionary`. Supported nullable
targets use `nullable-` tags. Types and their source obligations follow the
[source cast contract](../frontend/README.md#lexical-casts).

[VM lowering](../lowering/README.md#lexical-casts).

### Source-class tests and casts

[Directive syntax](grammar.md#operation-syntax) is defined in the grammar.

`SourceInstanceOf` and `SourceCast` carry a typed `SourceClass` target with
separate name and base fields. Function validation checks target syntax, native
projection, and API requirements. Unit validation resolves the declaration and
requires both identity and base to match. Forward and cross-file targets work.
Standalone parsing retains that unit obligation; text cannot provide trusted
branch facts or bypass declaration/base checks.

Positive source-class tests participate in the same CFG fixed point as native
class tests. Fresh reads retain both the native value type and exact source
provenance. Casts establish the explicitly asserted source identity without a
proof. The [source contract](../frontend/README.md#source-class-targets) owns
scope, assignability, and caller obligations. Class facts stay typed in memory;
text is only the save/resume representation.

`SourceCast` also carries a `nullable` flag. Its saved target is
`nullable-source(ClassName@Native.Base)` when true, otherwise
`source(ClassName@Native.Base)`. The nullable form retains the exact identity
with a nullable native projection; source dispatch requires a non-null proof
on a later read. It does not narrow other storage or change the runtime value.
`SourceInstanceOf` has no nullable target flag: Null does not match the class.

[VM lowering](../lowering/README.md#source-class-tests-and-casts).
### Arithmetic definitions

Constants and unary/binary operations use the [grammar's value
directives](grammar.md#values-and-storage). Operands follow the [source operator
contracts](../frontend/README.md#expressions-and-operators) and
[numeric-width/alias rules](../frontend/README.md#numbers-and-numeric-aliases); saved result claims
cannot bypass promotion or non-null requirements. Concrete floating and integer
classes remain distinct. Binary operations define a new single-use value and
consume two earlier block-local values; unary operations consume one.

Division/remainder remain runtime operations, including literal zero divisors;
their possible failure requires evaluation-order checking. No constant folding,
coercion, or inserted exception guard changes that behavior. Integer complement
retains a distinct unary operation, including repeated complements, rather than
being rewritten as binary XOR. Its result preserves Number/Long width; floating,
Char, Boolean, and unproved nullable operands are rejected. Numeric aliases
require every alternative to satisfy the operator. See
[lowering](../lowering/README.md#values-and-storage) for concrete instruction selection.

```text
ciq-ir
.function probe
.integer %0 40
.integer %1 8
.integer %2 5
.subtract %3 %1 %2
.add %4 %0 %3
.return %4
.end
```

Boolean constants use exactly `true` or `false`, producing Boolean rather than
Number zero/one or Null. Typed locals, calls, returns, branches, equality, and
printing preserve that identity. Booleans are not integer operands. Raw scalar
constant widths and source-literal restrictions remain separate; see the
[constant grammar](grammar.md#values-and-storage).

## Control flow

Explicit control flow uses block declarations and terminators:

```text
ciq-ir
.function choose parameters=1
.block ^0
.parameter %0 0
.integer %1 0
.equal %2 %0 %1
.branch %2 ^1 ^2
.block ^1
.integer %3 47
.return %3
.block ^2
.integer %4 49
.return %4
.end
```

- `.block` starts a block; IDs are decimal `u32`, distinct from `%` values
  and `$` locals. Block IDs must be unique; the first block is entry.
- `.equal`, `.not-equal`, `.less`, `.less-equal`, `.greater`, and `.greater-equal`
  each take `%<result> %<left> %<right>`. They follow the [source comparison domains](../frontend/README.md#operator-contracts) and
  define Boolean. Equality/inequality also accept the String identity profile,
  Boolean pairs, and supported dictionary/null checks. Ordering rejects String,
  Boolean, null, and unproved nullable operands. Lowering selects `eq`, `ne`,
  `lt`, `lte`, `gt`, and `gte` respectively; operand evaluation remains left to
  right. See [comparison evidence](../../investigations/compiler-number-comparisons.md).
- `.branch` consumes a Boolean and
  terminates the block. The two targets may be the same block.
- `.jump` terminates the block unconditionally.

All new directives accept the existing optional source span. A body containing
block/branch/jump operations must begin with `.block`. Every block must end in
`.return`, `.return-void`, `.branch`, `.jump`, or `.jump-value`, with no
operations after its terminator until the next block. Targets may name earlier
or later blocks. Every target must be declared; missing targets, duplicate
blocks, and implicit
fallthrough are rejected. The final block returns. A body without these
operations uses an implicit single block.

Disconnected blocks retain validation for source headers that cannot execute,
such as an update after an unconditionally breaking body. Walk successors from
entry first, then each unvisited block in textual order to form checking
components. Edges into an earlier component do not contribute initialization,
refinement, or builder-state facts there. Each component starts with no branch
refinements or private builders; unfinished builders cannot leave it. Name,
type, operand-use, target, and call checks still apply to every block. A
disconnected `.return-void` epilogue does not affect result inference or conflict
with a value result. Value returns retain their ordinary result checks.

Expression definitions still precede their uses, are globally unique, and have
exactly one use, restricted to their defining block. Locals can cross edges, but
entry-reachable reads and assignments require the unique initializer to execute
on every incoming path. Its declaration must also precede the textual use.
Availability is the fixed-point intersection described below. A local initialized
in only one arm cannot be read or assigned in a sibling or join. Typed [expression
joins](#expression-joins) carry incoming values through fresh block parameters.
Other cross-block value IDs remain unsupported. Local slots are allocated across
the entire function, including mutually exclusive paths.

The 1024-operation limit includes block markers and terminators. Effectful
functions retain left-to-right expression and statement order within each block;
block markers and jumps participate in schedule validation. Branches consume
their comparisons. Ordinary edges carry no evaluation-stack values; value edges
carry exactly their parameter. Source parsing, typed construction, and text
resumption use these same checks. Source conditions use these same forward joins
with `.jump` for continuing arms, including nested statements and early returns.
Source local names/scopes are resolved before serialization; the IR only retains
globally unique local identities. See [the source
subset](../frontend/README.md).

### Cyclic control flow

Backward branch/jump targets are permitted. Entry reachability is determined by
walking from the first block, including cycles. Having a predecessor alone does
not make a block executable. The final declared block must still end in a return;
this does not prove every runtime path terminates. When no return establishes a
result type, retain an explicit signature or default to Void. A disconnected
void epilogue does not change that signature. Saved IR preserves non-Void
contracts in [function metadata](grammar.md#text-and-value-definitions), even
when an ordinary scalar result would otherwise be inferred from returns.

Definite-local sets are computed to a greatest fixed point. Start incoming and
outgoing sets at the universe of declared local IDs. For every block, intersect
its predecessor outgoing sets to obtain its incoming set; its outgoing set adds
its `.local` initializers and `.assign` writes, and removes `.declare-local` and
`.take` storage. Entry additionally has an empty external predecessor, so its
incoming set is always empty. Repeat until sets stop changing. Scan each block's
operations from its final incoming set, adding initializers/writes only when
encountered. Reads require membership at their position. A separate declaration
analysis admits `.declare-local` and `.local` and does not remove declarations
on takes; assignments require a declaration on every incoming path and an
earlier textual definition. Disconnected component roots instead start with
declared locals available, permitting references in unexecuted headers; earlier
textual declaration and type checks still apply. Intersections use only
predecessors within the component, so dead edges cannot affect executable
initialization.

This accepts common initialized locals carried around a loop and rejects values
initialized only on its backedge. Local initializer IDs remain globally unique;
one declaration may execute repeatedly, resetting its existing slot each visit.
Types retain their initializer's supported classification. Expression values
still refer to earlier definitions in the same block and have one static use;
runtime execution may visit that definition/use repeatedly. No expression values
or evaluation-stack state cross an edge.

Source loops lower to these existing graph operations. Test comparisons/calls
execute again on every visit; body statements retain evaluation order. Native
construction, text resumption, and source compilation use the same validator.
See [loop source rules](../frontend/README.md#while-loops) and
[the runtime experiment](../../investigations/compiler-loops.md).

## Containers and private construction

### Dictionary allocation

`.dictionary-new` defines a fresh empty non-null Dictionary. Native construction
uses `Operation::DictionaryNew` with optional `dictionary_type`; omission means
an opaque Dictionary. Validation requires Dictionary's API 1.0.0 class, single
use, block-local dependencies, and ordered allocation. Lowering emits `ipushz;
newd`; no DATA record or compiler capacity operand is involved. Allocation
failures remain runtime obligations.

`dictionary` is the opaque parameter-type and value-call result tag.
`dictionary(<key>=><value>)` also tags `.block-value` joins of matching typed
Dictionaries. Both incoming contracts must match exactly; this neither enables
Dictionary/Null joins nor parameterized Dictionary casts. Reference ownership
and empty-stack receiver setup follow the ordinary join/staging rules.

`dictionary(<key>=><value>)` preserves typed contracts in `parameter-types=`,
function `result=`, call `result=`, and allocation `type=` fields. Keys are
`integer`, `long`, `float`, `double`, `string`, `boolean`, `char`, `symbol`,
`comparable`, `comparator`, or `method`. Values use those tags,
`weak-reference`, `byte-array`, or their `nullable-` counterparts. Keys and
values also accept `numeric`, `integer-union`, and `decimal`; only values accept
their nullable forms. Values also accept `object` and `nullable-object`, with
the
[source erasure rules](../frontend/README.md#object-containers).
Interface value contracts require API 5.0.0; WeakReference requires 1.2.0 and
ByteArray requires 3.0.0. Other tags, nested ordinary containers, and arbitrary
heterogeneous unions are rejected. For example:

```text
ciq-ir
.function make result=dictionary(string=>nullable-integer)
.dictionary-new %0 type=dictionary(string=>nullable-integer)
.return %0
.end
```

Typed and opaque Dictionary result headers remain explicit so an erased or
differently typed allocation cannot change the contract on reload.
`DictionaryType` backs the `TypedDictionary` parameter/result variants. Locals
and calls preserve reference identity and invariant contracts; erasure to opaque
Dictionary is one-way. `.call-native` accepts the [source method
profile](../frontend/README.md#dictionaries); Void `put`/`remove` calls use
`.call-native-discard`. Existing nullable callback reads narrow to opaque
Dictionary under a non-null proof. Conditional Dictionary joins remain
unsupported. See the
[source profile](../frontend/README.md#dictionaries).

### Dictionary access

`.dictionary-read` reads a typed non-null Dictionary and yields V or Null.
Native construction uses `Operation::DictionaryRead`; lowering evaluates
receiver then key and emits `agetv`. Missing keys yield Null rather than an
Array bounds failure.

`.dictionary-write` inserts or replaces an entry without producing a result.
`Operation::DictionaryWrite` evaluates receiver, key, and value, then emits
`aputv`. It does not use an Array index or require an existing key. Both
operations validate against the generic `get`/`put` contracts, including
key/value types and API availability.

Native `get`/`hasKey` use ordinary `.call-native`; `put`/`remove` use the Void
statement form. Explicit method calls preserve member dispatch. Saved IR infers
lookup nullability from the validated receiver, never from key presence. Opaque
receivers, wrong keys/values, unproved nullable operands, and invalid value uses
of Void methods fail validation. See the
[source rules](../frontend/README.md#dictionaries).

Reads and writes participate in the effect schedule. A read consumed before a
write cannot move after it. Branching operands use the existing staging rules;
incoming join parameters cannot bypass the stack-use checks. Source candidate
types select the operation, while full IR validation proves its safety.

### Dictionary constants

`.dictionary-constant` defines a fresh non-null Dictionary from nonempty flat
scalar entries. Native construction uses `Operation::DictionaryConstant`,
`DictionaryType`, and pairs of `DictionaryScalar`. Scalar syntax is `number
0x<bits>`, `char 0x<bits>`, `boolean true|false`, `string "text"`, or `null`.
Number/Char bits must fit u32; Strings require quotes and use the existing
escape rules. For example:

```text
.dictionary-constant %0 dictionary(string=>nullable-integer) string "one" number 0x00000001 string "missing" null
```

An interface value contract can accept compatible scalar witnesses while each
token retains its VM tag. Source/interface references use private construction
instead; they cannot be represented as scalar constant tokens. Private fills,
indexed writes, and native `put` share the
[source/unit interface proof](../frontend/README.md#comparison-interface-dictionaries).
Interface keys require runtime construction, even with scalar contents; this
constant operation rejects those key contracts. Private fills, indexed reads and
writes, and native key arguments retain the
[key witness obligation](../frontend/README.md#interface-dictionary-keys).

The [source contracts](../frontend/README.md#dictionaries) govern supported
key/value kinds, nullability, and entry typing. Missing pairs, empty entry
lists, incompatible scalar types, and unencodable pool strings fail before
lowering. The operation retains source entry order and duplicate keys for
inspection; materialization retains the last value for equal scalar keys and
shares equal pool strings. No computed entries are folded into this operation.

[VM lowering](../lowering/README.md#dictionary-constants).

Long/Double scalar payloads use u64 hexadecimal bits; Float uses u32, alongside
the existing Number/Char/Boolean/String/Null payloads. Exact class/bits survive
text and DATA binding. Constant-key coalescing follows the
[source rules](../frontend/README.md#dictionaries), including distinct signed
zeros and non-coalescing Double NaNs.

### Computed Dictionary construction

`DictionaryBegin`, `DictionaryFill`, and `DictionaryFinish` represent a private
nonempty literal with the [source key/value
contract](../frontend/README.md#dictionaries). Their text forms are:

[Directive syntax](grammar.md#operation-syntax) is defined in the grammar.

Each accepts optional `span=<start>:<end>`. Length is the positive source entry
count, including duplicates, and must fit a nonnegative Number. A fill supplies
one or more pairs, beginning at the next source-entry ordinal and fitting the
remaining count. Entries check against K/V using the same put contract as
indexed writes. Finish consumes the completed builder and produces its typed
Dictionary.

[VM lowering](../lowering/README.md#computed-dictionary-construction).

### Private container checks

Array and Dictionary builders share one identity namespace and lifetime checker.
Begin declarations are unique within a function; operations cannot substitute
one builder kind for the other. Every CFG join requires identical active-builder
progress on incoming paths, including backedges. Returns cannot leave builders
active. Unknown/inactive builders, early publication,
skipped/repeated/out-of-order fills, and incompatible values fail validation. A
completed identity can begin again only by re-executing its original
declaration, such as in a loop.

Builders reserve frame slots alongside parameters and ordinary locals under the
shared 255-slot limit. They are not semantic values or ordinary local IDs; only
finish exposes a usable reference. Allocation remains ordered before entry
effects and can fail there. Finishing either builder kind clears its private
strong reference; a completed container is owned by the produced value and its
subsequent consumers.

### ByteArray values and allocation

`byte-array` denotes a non-null `Toybox.Lang.ByteArray`; `nullable-byte-array`
adds Null. Both are parameter, function-result, call-result, and
conditional-edge tags. ByteArray is distinct from `number-array`; no conversion
or element-nullability inference connects them. Local assignment and matching
conditional arms retain the ByteArray reference. ByteArray/Null joins retain
either runtime value without conversion. Reuse the
[source null-proof rules](../frontend/README.md#bytearray) in native and saved IR;
annotations do not assert a proof. API 3.0.0 also applies to nullable contracts
without an allocation. Class fields use
[`.field-byte-array`](../frontend/README.md#bytearray-fields). Array elements reuse
these tags under the [allocation grammar](#array-allocation). Dictionary value
contracts use `dictionary(<key>=>byte-array)` or
`dictionary(<key>=>nullable-byte-array)` with an existing supported key
contract. Lookups yield nullable ByteArray; `values()` preserves the declared
value nullability in its Array element tag. Availability applies to unused types
and Null-only constants as well as construction and reads. ByteArray references
use private Dictionary fills, never scalar constant tokens; nullable constants
can contain Null under existing payload validation.
`dictionary(byte-array=><value>)` enables non-null ByteArray keys, with matching
`byte-array-array` results from `keys()`. Reference-bearing entries remain
runtime construction rather than scalar constants.

`decodeNumber` and `encodeNumber` use ordinary native calls, API 3.1.0, and the
[conversion contracts](../frontend/README.md#bytearray). `number-format` and
`endian` are distinct parameter/result/cast/join tags, not `integer`.
`.constant` retains enum identity and introduction level; lowering pushes its
integer value. Decode retains its Numeric result after reload.

`.byte-options-begin` allocates a private Dictionary for zero to two fields.
Existing `.dictionary-fill` operations require direct literal Symbol keys:
`offset` with Number, or `endianness` with Endian. Values are staged before
key/value setup when needed. `.dictionary-finish` exposes a sealed
`byte-options` record after complete ordered fills, clearing builder ownership.
The `byte-options` tag also supports matching conditional edges; lexical casts
cannot manufacture this proof. Native and saved IR check field names/types,
counts, order, block/value uses, and the shared frame limit. CODE uses ordinary
Dictionary construction and calls; no codec opcode or new DATA framing is added.

`.byte-array-new` allocates a fresh zero-filled ByteArray. Length must be an
earlier same-block non-null Number value. Validate API 3.0.0, single-use/block
locality, and allocation effect order; negative/oversized lengths and allocation
failure remain VM obligations. Lower the length once followed by `newba`, with
no DATA binding or initializer call. A direct incoming block parameter can
supply the length without staging.

Existing `.array-read` and `.array-write` operations also accept ByteArrays.
Reads produce Number; writes accept Number/Char. Preserve evaluation order,
runtime index bounds and byte range checks, and unsigned readback. No masking,
clamping, or expansion is emitted by the compiler. Native calls reuse the
catalogue and ordinary value-call lowering under the
[source method profile](../frontend/README.md#bytearray).

`.byte-array-constant` contains exact unsigned bytes; bare `0x` denotes an empty
payload. Hex digits must occur in pairs. API and ordered-effect checks still
apply. Lower to a ByteArray literal requirement and `bpush`, using a separate
DATA binding namespace. Source signed byte normalization happens before this IR
operation; readers never truncate wider numbers or reinterpret the payload as
scalar tags.

`.byte-array-begin` allocates a private ByteArray of a positive constant Number
length with `newba`. It shares `.array-fill`, `.array-fill-many`, and
`.array-finish` with indexed Array construction, retaining a distinct ByteArray
contract. Values must be non-null Number/Char; bounds remain runtime checks.
Existing builder validation requires complete, ordered, exactly-once
initialization on every CFG path, forbids early escape/restart, and counts
private slots toward the common frame limit. Grouped fills use retaining stores;
branching elements stage incoming values before builder/index setup. Finish
produces ByteArray and clears the builder's hidden ownership. No append loop or
intermediate Array is emitted.

### Array allocation

`.array-new` allocates a fresh empty non-null array. Scalar element tags are
`number`, `long`, `float`, `double`, `char`, `boolean`, `string`, and `symbol`,
each also accepting a `nullable-` prefix. `view` represents the supported native
View contract. `comparable`, `comparator`, `nullable-comparable`, and
`nullable-comparator` retain comparison-interface element contracts. `method`,
`weak-reference`, and their `nullable-` forms retain callable/weak-handle
contracts. `byte-array` and `nullable-byte-array` retain strong ByteArray
references. Numeric alias tags follow the [alias grammar](#numeric-aliases).
`object` and `nullable-object` retain erased Object elements; their parameter
and call-result tags are `object-array` and `nullable-object-array`, and their
conditional-edge tags are `array-object` and `array-nullable-object`.
Construction uses `Operation::ArrayNew` with the corresponding `ArrayElement`
variant. Other parsed element contracts are rejected. Definition/use, block, and
effect checks apply even when the allocation result is consumed later. API
checking requires Array and the element type: 5.0.0 for comparison interfaces,
1.3.0 for Char, 1.2.0 for WeakReference, 3.0.0 for ByteArray, otherwise 1.0.0
within this profile.

Parameter/user-call Array categories append `-array` to these element tags, for
example `long-array`, `nullable-double-array`, or `view-array`. Nullability
belongs to the element, not the Array reference. Function Array results are
inferred from typed returns rather than a separate `.function result=` field.

[VM lowering](../lowering/README.md#array-allocation).

`.array-allocate` allocates a fresh Null-filled Array. Length is an earlier
same-block non-null Number value, consumed once before `newa`. Only nullable
element tags from the allocation profile above are accepted. Lengths are not
clamped or validated against a compile-time size limit; VM failures remain
observable. Validation rechecks type availability, definition/use rules, and
allocation effect order. An incoming block parameter can supply length directly:
`newa` consumes it without inserting stack setup. The result retains its
invariant nullable element type even for a constant zero length.

### Constant arrays

```text
.array-constant %0 number 0x0000000a 0x00000014
.return %0
```

`.array-constant` defines a fresh mutable array. Element tags follow [Array
allocation](#array-allocation), excluding View; at least one element is
required. Number/Char/Float values use `0x` followed by exactly eight
hexadecimal digits; Long/Double require sixteen. Values retain their raw bits,
including signed floating zero and NaN payloads. String values are quoted text
using the ordinary IR String escapes, for example `.array-constant %0 string
"one" ""`. Boolean payloads use `true`/`false` or `ArrayConstantValue::Boolean`;
numeric 0/1 do not substitute for them; Null requires a nullable element
contract. Typed IR uses distinct `ArrayConstantValue::Number` and `::Char`
variants, `::Long`, `::Float`, or `::Double` for those exact bits, or `::String`
for text; nullable contracts also permit `null` / `ArrayConstantValue::Null`.
Every payload must match its element contract; other mixed types and nested
containers are rejected. API checks require Array and each element class, as for
allocation. Definition/use and ordered-effect checks include this allocation.

Object element tags use the tagged scalar tokens below for supported scalar
constants; `nullable-object` also permits Null. Validation checks each payload's
API requirements independently of its Object contract. Reference values use
private construction, not scalar DATA tokens. Unit/native calls preserve the
same [Object storage contracts](../frontend/README.md#object-containers).

Numeric alias element tags use tagged scalar tokens to retain each alternative,
with the same syntax as [Dictionary constants](#dictionary-constants):

```text
.array-constant %0 numeric number 0x00000001 long 0x0000000000000001 float 0x3f800000 double 0x3ff0000000000000
.array-constant %1 nullable-integer-union number 0x00000001 null
```

Each scalar must satisfy the alias/nullability contract. In particular, Char
cannot substitute for a numeric token, and integer payloads cannot satisfy
Decimal. Numeric aliases reuse the supported mixed-scalar DATA framing.

Interface element tags use tagged scalar tokens with the same syntax as
[Dictionary constants](#dictionary-constants), retaining each runtime class:

```text
.array-constant %0 comparable number 0x00000061 char 0x00000061 string "a"
.array-constant %1 nullable-comparable null boolean true
.array-constant %2 nullable-comparator null
```

Every token must satisfy the declared interface/nullability contract; no source
reference or arbitrary Object token is supported. Comparable admits the seven
supported native scalar witnesses. Comparator has no scalar witness, so only
Null fits its nullable constant profile. Interface contracts require API 5.0.0.
Typed IR shares the tagged scalar enum with `DictionaryScalar`; Number and Char
remain distinct even outside a scalar element contract. Nested containers and
empty constant payloads remain rejected. Computed/source-object elements use
private construction. Concrete scalar-element Array text retains the compact
untagged payload syntax above.

`symbol` and `nullable-symbol` are Array element tags; their parameter/call
result categories are `symbol-array` and `nullable-symbol-array`. Dictionary
keys and values also accept `symbol`, with `nullable-symbol` available only for
values. These contracts require API 1.0.0 and use the
[Symbol container rules](../frontend/README.md#symbol-containers). Symbol-bearing
literals emit `.symbol` values and private fills. No constant token accepts an
embedded numeric Symbol ID; Null-only nullable payloads remain valid constants.

Method Array elements use `method` or `nullable-method`; parameter/call result
categories use `method-array` or `nullable-method-array`. Runtime literals use
private Array builders and ordinary Method values; constants permit only Null
under the nullable element contract. No Method payload token or DATA tag is
added. See [Method Arrays](../frontend/README.md#method-arrays). Dictionary tags
also accept `method` keys and `method`/`nullable-method` values, for example
`dictionary(method=>nullable-method)`. Method entries use private runtime
construction; [Method Dictionary
rules](../frontend/README.md#method-dictionaries) also permit existing
scalar-key/Null-only nullable constants.

WeakReference Array elements use `weak-reference`/`nullable-weak-reference`;
parameter/call results use
`weak-reference-array`/`nullable-weak-reference-array`. Dictionary values use
those element tags, for example `dictionary(string=>nullable-weak-reference)`;
weak keys are unsupported. Weak-bearing literals use private builders. Existing
Null-only constants need no new DATA tag. See [WeakReference
containers](../frontend/README.md#weakreference-containers).

[VM lowering](../lowering/README.md#constant-arrays).

### Private array construction

Builders are not language values or ordinary locals. Their IDs use `&<u32>`;
they cannot be returned, copied, passed to calls, or read with `.read`.

```text
.array-begin &0 number 2
.integer %0 10
.integer %1 20
.array-fill-many &0 0 %0 %1
.array-finish %2 &0
.return %2
```

- `.array-begin` allocates the final length.
  Element contracts follow [Array allocation](#array-allocation).
  Length is decimal, positive, and fits a nonnegative Number. Builder
  declarations are unique within a function. Allocation remains ordered and may fail before element effects.
- `.array-fill` initializes the next slot.
  The decimal index must equal the number of slots already filled and be less
  than the declared length. The value must be assignable to the element contract. Nullable contracts
  accept the base scalar, Null, or their union without weakening null checks
  for non-null builders.
- `.array-fill-many` initializes a nonempty group
  of consecutive remaining slots, evaluating and storing each value before
  the next. Start and group length must fit the remaining construction range.
  Values obey ordinary definition, single-use, block, and effect-order checks.
- `.array-finish` requires all slots initialized. It consumes
  the builder and exposes a non-null typed Array; reuse requires execution of
  the original begin again, such as in a loop.

Optional `span=<start>:<end>` follows each operation. Builders follow the
[shared private construction checks](#private-container-checks).

Each builder reserves a VM frame slot alongside parameters and ordinary locals,
under the shared 255-slot limit. Lowering selects a Number push, `newa`, and a
private store at begin; fills use `aputv`, or `aputvdup` followed by final
`aputv` for grouped values. Finish loads the completed array. No native `add`
binding or availability requirement is imposed by construction. API checks still
apply to Array, element types, and any calls within elements. Empty construction
keeps the separate `.array-new` operation. See
[lowering evidence](../../investigations/natural-lowering.md#array-construction).

### Array element reads

The read result follows the receiver's element type, including native View.

`.array-read` consumes a non-null supported array and non-null Number index, and
defines the element type, including nullable scalar elements. Shared validation
recomputes operand types, null proofs, block-local definitions, one-use rules,
and evaluation order. No element/result type annotation is trusted. Array
evaluates before index, once each; lowering emits those operands then `agetv`.
Reads are order-sensitive because bounds failures are observable, even when the
function has no calls, prints, or writes.

Index values are not range-checked, clamped, or adjusted by the compiler.
Negative and other out-of-range Numbers remain accepted IR inputs whose reads
may fail at runtime. Supported Array element domains follow the frontend
container contracts. ByteArray and typed Dictionary indexing share this IR
operation with their separately checked key/result rules. Union receivers must
satisfy every alternative; nullable references require a non-null proof. String
indexing remains unsupported. See the
[source profile](../frontend/README.md#char-arrays) and
[runtime evidence](../../investigations/compiler-array-reads.md).

### Array element writes

`.array-write` is a statement with no result. It consumes a non-null supported
array, non-null Number index, and a value matching its invariant element
contract. ByteArray and typed Dictionary writes use their checked indexed
contracts; a union receiver requires a valid write for every alternative. Source View instances project to native View; array
slots retain no exact source-class identity. Shared validation recomputes these
types and null proofs, checks defined block-local single-use operands, and
preserves their left-to-right evaluation order. Writes activate the effect-order
check even without calls or printing; reads cannot move across a write.

[VM lowering](../lowering/README.md#array-element-writes).

Source indexed compounds capture array/index with ordinary typed locals, perform
`.array-read` before RHS effects, apply the binary directive, and finish with
`.array-write`. Plain RHS expressions inline the old read into the binary value.
A branching RHS stores that old read and the computed result before creating the
final write operands, preserving block-local IDs and effect order. No
indexed-compound directive or result-type annotation is introduced. See
[source rules](../frontend/README.md#indexed-compound-assignment) and
[evidence](../../investigations/compiler-compound-targets.md).

## Calls and objects

### Native object construction

`.new-native` allocates and initializes a non-null View with no arguments. Typed
IR uses `NativeNew { result, class: NativeClass::View }`.

`Toybox.WatchUi.BehaviorDelegate` is also accepted, using the corresponding
NativeClass, `behavior-delegate` parameter/result tag, and initializer API
1.0.0. WeakReference construction follows its separate contract below; other
native class paths and extra arguments are rejected. View parameter text is
`view`; user value calls use `result=view`. Function results are inferred from
returns; `.function result=view` is unsupported.

Construction requires API 2.1.0 for View.initialize, while passing an existing
View needs only the class's 1.0.0 availability. It participates in single-use,
same-block, and effect-order checking as an allocation/call. Discarding its
result still executes allocation and initialization.

[VM lowering](../lowering/README.md#native-object-construction).

`.new-native` uses `WeakReferenceNew { result }` and produces a non-null
WeakReference. It takes no operands, retains allocation effect order, and checks
class availability from API 1.2.0. Lowering resolves `__ciq_sdk_Lang` /
`__ciq_type_WeakReference`, then uses the existing `newc`, `initialize` lookup,
and zero-argument invocation sequence. It does not invoke `weak` or introduce a
new VM opcode/DATA record. The [source
contract](../frontend/README.md#weakreference-construction) scopes the
SDK-observed constructor separately from target binding.

### Positional result construction

`.tuple-new` constructs a fresh positional Array. Typed IR uses
`TupleNew { result, elements }`; validation infers a `TupleType` from the
evaluated operands, including exact source identities. Parameters, results,
locals, fields, callbacks, and unions use the same `tuple(tag;tag;...)`
contract; `tuple()` represents an empty positional value, including a literal
append argument. Source annotation syntax still requires at least one slot.
Nested tuple tags retain their slot contracts. Void slots and nesting
beyond the shared 64-level signature limit are rejected.

Every operand must be a local/parameter read evaluated earlier. Store effectful
slot expressions left to right before constructing the tuple. Single-use,
same-block, initialization, and effect-order rules remain in force. Locals may
retain/copy a shape. Explicit tuple and Array result headers preserve declared
contracts even when a returned value has narrower positional information.
User-call result metadata must match the callee's contract.

`ArrayRead` also reads tuples. Validation selects a slot for a direct Integer
operand or joins the possible slot contracts otherwise, using the
[frontend rules](../frontend/README.md#positional-tuples). Source provenance is
reconstructed on reload; result metadata cannot invent a source slot identity.
Unit checks resolve nested class/native-base claims and retain native callback
slot/cardinality checks.

`.tuple-write %result %array %index %value` replaces one slot and returns the
same runtime Array reference. A direct Integer operand must be in bounds for
every tuple alternative and replaces that slot's contract, retaining exact
source identities. A dynamic Number index loses positional shape and derives
an Array element union from the receiver and replacement value. Existing
inferred Array receivers retain an element domain instead of gaining slots.
No result-type assertion supplies or overrides this proof.
Store the result back to the named receiver's inferred local to make later reads
use the new view. Branch joins use ordinary inferred-local domains. Tuple
parameters requiring tuple mutations have entry reads stored into inferred locals
without changing their calling convention or the capacity for read-only
parameters. Aliased locals retain their own static views.
Named tuple-valued local initializers and assignments use inferred-variable
storage from declaration, including nullable tuples. This lets an early guard
constrain a later mutation without treating the declaration as a fixed slot view.

`.tuple-add %result %array %value` and
`.tuple-add-all %result %array %tail` invoke the corresponding Array methods.
Typed IR uses `TupleAppend { result, array, value, spread, positional }`.
The tuple forms set `positional=true`: known shapes concatenate slots, including
an empty `tuple()` tail, retaining exact source identities. An unknown-length
Array tail loses positional shape and joins the native element contracts.
`.array-add` and `.array-add-all` have the same operands but set
`positional=false`, deriving an element contract even for known tuple inputs.
The frontend uses these forms for loop-carried tuple storage; their result is
the same runtime receiver reference. Generalized elements must fit a supported
Array profile; unsupported source/nested/Null-only domains are rejected.
Validation checks API 1.3.0 and the actual operands on reload, without trusting
an asserted result or tail length. Store the result into the receiver's inferred
local before emitting subsequent reads. Derived result signatures have a shared
64 KiB budget across the possible shapes, checked before concatenating slots;
this bounds metadata growth rather than runtime Array capacity.
Inferred-local joins merge an initial tuple and Array outcomes into the common
Array element view. This is specific to tuple-derived storage; ordinary Array
assignment/call contracts remain invariant. Non-null edge proofs narrow tuple
and Array storage domains before forwarding them into loops. They do not
introduce a bottom type for unrelated source-class paths. Saved reads are
checked against these reconstructed domains, not accepted as slot assertions.

[VM lowering](../lowering/README.md#positional-result-construction).

### Module calls and operation order

`.call` defines the result of a positional-argument call to a top-level user
module. Scalar result types are `integer`, `long`, `float`, `double`, `string`,
`boolean`, and `char`, plus `nullable-integer`, `nullable-long`,
`nullable-float`, `nullable-double`, `nullable-string`, `nullable-boolean`, and
`nullable-char`. Symbol calls use `symbol` or `nullable-symbol`. Method calls
use `method` or `nullable-method`. Array results use the [element
categories](#array-allocation). Omission means integer. Canonical text omits the
integer default and writes noninteger contracts. Void is invalid in a value
call. Names use ASCII identifier rules and remain symbolic. The optional span
has the same meaning as for other operations. The module's expected stored
symbol name is `globals/<module>` and the member's is `<member>`; this is not
numeric symbol allocation or method metadata generation.

Calls, prints, and writes are effectful. If a function contains any `.call`,
`.println`, `.call-native`, or `.assign`, all its operations must match the
backend's left-to-right expression traversal and ordered
initialization/assignment/print/branch/return sinks. The shared validator
rejects incompatible schedules, including moving a call across an initialization
or reversing call operands. Pure-only functions permit flexible definition
order. This conservative policy keeps runtime/IR operation order consistent
until a general scheduler is implemented.

```text
ciq-ir
.function probe
.call %0 Numbers left
.local $0 %0
.read %2 $0
.call %3 Numbers right
.add %4 %2 %3
.integer %5 39
.subtract %6 %4 %5
.return %6
.end
```

Names and result expectations are not checked against a whole-program type
environment in standalone mode: the scaffold must supply matching callees. Unit
mode checks names, exact arity, argument types, and call result contracts.
`.call-global` defines a positional call to a global user function, with the
same argument/result contracts and effect ordering as `.call`. It carries a
member name without a module or instance receiver. Unit validation resolves it
only against global declarations, even inside a module or class body. Its
symbolic owner is the global namespace `globals`; the member symbol retains its
ordinary identifier. Typed IR uses `GlobalCall { result, result_type, member,
arguments }`. Saved text preserves this ownership explicitly rather than
depending on the caller's scope. Source-instance calls follow the
[receiver profile](#instance-receivers).

### Void returns and statement calls

Void returns and effect-only calls use:

- `.return-void`: terminate without a semantic result.
- `.call-discard`:
  invoke a qualified user function for effects without defining a value.
- `.call-global-discard`:
  invoke a global user function for effects without defining a value.

Both discard forms may target any supported user result, including Void; the
caller discards it. Argument IDs name earlier supported scalar, nullable, or Null
values in the same block and each counts as a use. Calls use identifier rules,
argument limits, and left-to-right effect scheduling already required by
`.call`. Names, arity, and callee result contracts are checked in unit mode;
standalone functions retain their external-callee obligations.

```text
ciq-ir
.function startup
.integer %0 41
.call-discard Startup greet %0
.return-void
.end
```

Every return in a function must agree: `.return` yields its value's supported
type, `.return-void` yields void. The shared validator infers and exposes that
result contract. Nullable result declarations instead require assignability to
that union on every return. A block may terminate with either form. The final
declared block still returns, and the existing reachability, definite-local, and
empty-edge-stack rules apply. A single `.return-void` is a valid function body.
Implicit text fallthrough is rejected; only source compilation inserts a final
void return.

Void is not an expression value, and arbitrary SDK calls remain unsupported.
Object, nullable, and container values follow their separately checked profiles.
See [source rules](../frontend/README.md#void-functions-and-statement-calls) and
[runtime evidence](../../investigations/compiler-void.md).

### Native superclass initialization

`.base-initialize` accepts `Application.AppBase`, `WatchUi.View`, or
`WatchUi.BehaviorDelegate` and invokes the identified native superclass
initializer on the current instance, with no explicit arguments or value result.
It accepts the usual optional source span. Other base names or additional fields
are rejected. Function validation requires an instance receiver and preserves
the call's effect order; unit validation additionally enforces the
[source initializer contract](../frontend/README.md#source-initialization).
Source, typed, and text paths use the same operation and VM lowering.

### Comparison interface contracts

`comparable` and `comparator` are non-null parameter/result tags for the
[source interface profile](../frontend/README.md#structural-interfaces).
`nullable-comparable` and `nullable-comparator` add Null to those contracts.
They retain API 5.0.0 availability and require non-null branch proofs for member
calls. Nullable result contracts must be explicit, as for other nullable types.
They occur in `parameter-types=`, function `result=`, user-call `result=`, and
interface block-parameter tags. Calls use `.call-native` with `compareTo` or
`compare`; Comparator values also supply the ordinary native `sort` call. No
interface class allocation, cast, or new VM instruction is implied.

```text
.function forward parameters=1 parameter-types=comparable result=comparable
.parameter %0 0
.return %0
```

Function validation checks interface call arity/types and retained value
contracts, including scalar Comparable witnesses at returns, local assignments,
and incoming block values. User-call arguments use the same witness rules. No
coercion operation is serialized; the declared storage/edge/call contract
retains the interface while the scalar keeps its runtime representation. Source
witnesses at arguments, returns, local/field assignments, Array fills, indexed
writes, and native element arguments remain unit obligations; unit validation
resolves their exact classes and checks required members after text reload. A
forged call result or a changed implementation signature cannot bypass that
check. Erased calls retain conservative constructor effects. The backend selects
the existing comparison symbols and invocation instructions. Class declarations
retain interface fields through
[`.field-interface`](../frontend/README.md#comparison-interface-fields). Reads
must claim the declared result tag; writes require unit-level compatibility. No
new VM opcode or DATA value tag is introduced for interface storage. Ordinary
Null values, branches, and frame slots retain their existing encodings. Sorting
accepts a nullable Comparator argument without a receiver refinement; compare
member calls still require one.

### Nullable callback state

Instance-function metadata may include `parameter-types=...` with
colon-separated scalar/Array contracts. Array categories follow [Array
allocation](#array-allocation). Ordinary signatures also accept `long`,
`nullable-long`, `float`, `double`, `nullable-float`, `nullable-double`,
`nullable-integer`, `nullable-string`, `nullable-boolean`, and `nullable-char`.
Symbol signatures use `symbol` or `nullable-symbol`. Method signatures use
`method` or `nullable-method`. Separate entries with colons; the list must match
`parameters=<count>`. Omitted types are integers. This field applies to ordinary
functions and instance methods. `nullable-dictionary` is also accepted; unit
validation restricts it to native AppBase.onStart/onStop contracts. Unsupported
names, duplicate lists, and mismatched counts are errors. For example:

```text
.function onStart parameters=1 receiver=instance parameter-types=nullable-dictionary
```

`.null` defines null. `.equal`/`.not-equal` permit supported scalar/reference
equality domains and nullable/null checks; it also accepts the [String identity
profile](../frontend/README.md#operator-contracts). Ordering directives follow
the [numeric comparison
domains](../frontend/README.md#numbers-and-numeric-aliases) and [Char
restrictions](../frontend/README.md#operator-contracts). Callback-state
parameter reads and fixed locals retain their declared nullable type; proven
reads yield a non-null Dictionary. Printing and arithmetic reject Null and
Dictionary values. Null may
be used in nullable scalar user contracts. Non-null Dictionary references
support the [allocation/call profile](#containers-and-private-construction). A
nullable callback Dictionary local may be assigned null. Inferred storage follows
the [local flow rules](#local-operations). Lowering uses `npush` for null and the
existing local-slot and equality instructions.

### Instance receivers

The optional declaration field `receiver=instance` marks an instance method;
ordinary bodies omit it. It may appear before or after `parameters=<count>`;
duplicate or unknown fields are rejected. Every instance method requires
receiver metadata, even without self calls. The receiver is the hidden context,
not an explicit parameter/value ID.

`.self` defines the current receiver as opaque Object. It requires
`receiver=instance` and lowers to `getself`. It does not claim an exact source
class. Reflective `self.method(selector)` uses this value with ordinary
`.call-native` for erased lookup or `.bind-method` for a resolved typed binding.

Two call directives also require an instance method:

- `.call-self`:
  define a value under the same result contract rules as `.call`.
- `.call-self-discard`:
  invoke a current-receiver member for effects and discard its result.

Arguments are ordered, earlier-defined supported scalar, nullable, or Null
values in the same block. The existing use/effect/control-flow checks apply.
Unit validation binds calls only to declared methods in the enclosing class,
checks arity/result contracts, and rejects instance bodies in global/module
owners. Standalone method IR retains its required receiver and unbound
class/callee obligations.

```text
ciq-ir
.function start receiver=instance
.integer %0 50
.integer %1 3
.call-self %2 answer %0 %1
.println %2
.return-void
.end
```

Canonical method output keeps `receiver=instance`. Native base-class identity
and enclosing class name belong to [unit class
containers](#compilation-units-and-classes), not function IR; neither carries a
VM import index. Source-class construction and fields follow their unit contracts.

### Native module values

`.call-native-module` invokes a runtime-provided module function and defines its
catalogue-declared result. Module names are canonical API paths; optional spans
use the ordinary syntax. The executable profile accepts `Toybox.Lang format`:
String and erased Array arguments, String result, API 1.0.0. Validation checks
arity, assignability, availability, supported lowering, and ordered single-use
operands. Neither the consumer nor saved text supplies a result-type override.
Void members require the [statement form](#native-module-statements); ignoring a
value uses `.discard`.

[VM lowering](../lowering/README.md#native-module-values).

### Native module statements

`.call-native-module-discard` invokes a runtime-provided module function without
defining a value. Module names are canonical API paths, distinct from user
module owners; arguments follow the ordinary definition/use and effect-order
rules. Optional spans use the usual syntax. The profile accepts `Toybox.System
exit` and `Toybox.WatchUi requestUpdate`, each with no arguments and a catalogue
Void result. Both require API 1.0.0. API context checks match source
compilation.

[VM lowering](../lowering/README.md#native-module-statements).

### Discarded values and native Void calls

`.discard` evaluates and consumes one earlier non-Void value without defining a
result or changing the function return type. It accepts every supported value
category, including Null, nullable values, and Char arrays. Definition/use,
same-block, and evaluation-order rules apply. The backend evaluates the operand
through the usual expression path and emits one `popv`; nested call results
remain available to their consumers.

Native source statements use an ordinary `.call-native` followed by `.discard`:

```text
ciq-ir
.function f parameters=1
.parameter %0 0
.call-native %1 %0 abs
.discard %1
.return-void
.end
```

The call retains its argument/result and API checks. A dropped nullable result
does not prove its receiver non-null. User Void calls and native module Void
statements keep their existing forms; no Void expression value is created. See
[runtime evidence](../../investigations/compiler-native-statements.md).

For a Void native member, use `.call-native-discard`. It defines no value.
Validation requires the resolved method result to be Void; receiver/argument
types, API availability, and ordered evaluation still apply. The bounded Dc
clear/setColor/drawText profile uses this form.

### Source construction and native constants

Source instance calls use:

[Directive syntax](grammar.md#operation-syntax) is defined in the grammar.

Optional spans follow the existing syntax; omitted result types mean Integer.
`ObjectCall` and `ObjectCallStatement` carry a `classes: Vec<String>` target
set, serialized as one whitespace-free `A|B` field (a singleton remains `A`).
Construction and parsing sort/deduplicate the set and require 1..=254 valid
names. Receiver provenance must prove exactly that non-null source-only set,
independently of the claimed names; omitting a possible class is invalid.

Unit validation resolves every target and checks its arity, parameters, and
lifecycle-callback restrictions. Arguments must satisfy all signatures. A value
call's result metadata must equal the shared join of every declared result;
Void and unsupported joins are rejected. Statement calls define no semantic
result and permit different result types, including Void/value mixtures.
Contextual literals use a common concrete parameter contract when all targets
accept it, including Any parameters. Every possible target contributes shutdown
effects, including through recursive calls and initializer checks. The target
set adds static obligations, not runtime branching. Both forms preserve one
receiver evaluation before arguments in source order. Standalone function parsing retains
declaration-resolution obligations; it does not authorize arbitrary receiver
casts. See the [source-call
contract](../frontend/README.md#calls-on-constructed-source-objects).

`source(ClassName@Native.Base)` is an exact non-null source-class contract in
`parameter-types=...` and function/call `result=...` fields. It occupies one
parameter-list entry without a colon inside the tag. Source function results
always serialize this tag; omission erases the exact class at the return
boundary. Class names must be valid identifiers; native bases are `Lang.Object`,
`WatchUi.View`, or `WatchUi.BehaviorDelegate`. Unit validation checks both
against the actual source declaration. Return provenance and call
arguments/results are checked through the [source signature
rules](../frontend/README.md#source-class-signatures); the tag does not cast an
erased native reference.

`nullable-source(ClassName@Native.Base)` carries the same exact class requirement
while admitting Null. It is valid in parameter and function/call result metadata.
Null branches refine fresh parameter/local reads without erasing source
identity. Fixed storage preserves its class/nullability contract on assignment;
inferred variables recompute their read contracts from incoming writes.
Function validation checks return provenance and non-null receivers; unit
validation resolves declarations and checks call contracts against their bodies.
Its runtime representation is the ordinary reference or Null, with no added
conversion instruction. Nullable source tags also describe field declarations
and reads, and selects a nullable source assertion in `.cast`. Source tags are
also exact join contracts, but are not native API cast tags.

`SourceUnion` stores normalized `SourceClass` identities, native alternatives,
and nullability. At least one exact class is required on construction;
`with_native` adds supported native types without turning their projections into
exact witnesses. Constructors reject malformed names, nonconstructible bases,
unsupported native types, and sets exceeding 254 alternatives including Null.
The native-only result of narrowing uses the ordinary API contract. Parameters and
results use `ParameterType::SourceUnion` and `ReturnType::SourceUnion`; joins and
fixed/inferred local read contracts use `JoinType::SourceUnion`. Singleton sets
use the existing exact/nullable variants. Typed construction canonicalizes
singleton signature, call-result, and local/join metadata just as parsing does.
Saved text uses `source-union(...)`
from the [type grammar](grammar.md#type-tags).

Function validation tracks the complete exact set through values and storage,
checks subset assignability, and refines both class-test edges. API projections
are derived from all retained bases, including after negative-edge narrowing;
they do not establish exact identity. Unit validation resolves every class/base
against declarations and every call/result claim against its signature.
Mixed assignability checks each actual exact class against exact alternatives
or a valid native alternative, with structural witnesses resolved by the unit.
Actual native alternatives can satisfy only native contracts. Fixed local and
block-edge checks resolve deferred witnesses too; inferred read metadata cannot
claim a witness by changing its type tag. Structural interface consumers check
both retained classes and native alternatives. Unknown multiple callback bodies
retain conservative initialization/shutdown effects. Direct common source calls
instead track every exact body as specified above. Field contracts retain these
same alternatives; general union casts remain unsupported.

`FieldKind::Value { value_type: ReturnType }` carries field contracts without an
existing specialized storage form, including general unions and nullable
container references. `.field-value name tag` uses the ordinary saved value tag.
Typed construction and parsing normalize aliases/unions and reuse specialized
field forms where applicable. Tuple fields retain their slots. Void and
Graphics.Dc alternatives are rejected. Unit declaration validation resolves
every exact class/native-base pair, including unused union fields; API checks
visit every retained type.

Field-read result metadata normalizes in both typed and parsed IR and must equal
the declaration's full contract. It does
not record a last-written subtype or grant a proof for another field read.
Writes check assignability and every required structural interface witness.
Generic field metadata contains Null; non-null contracts participate in the
existing required-initialization prefix checks. The
[frontend field rules](../frontend/README.md#union-fields) own defaults, snapshot
semantics, and contextual construction. No union discriminator is stored.

Instance field operations are `.read-self` and `.assign-self`, with optional
source spans. Reads define Number by default;
`result=source(ClassName@Native.Base)` selects a non-null source-reference read;
`result=nullable-source(ClassName@Native.Base)` selects its nullable snapshot.
`.field-reference` uses these same tags for the storage declaration. Writes
consume a value without a result. Both require an instance
receiver and preserve observable order. Their symbolic field identities resolve
through the enclosing unit class's
[field declarations](../frontend/README.md#instance-fields).
Reference declarations, result claims, and initializer safety follow the
[reference-field contract](../frontend/README.md#source-reference-fields).
Symbol fields use [`.field-symbol`](../frontend/README.md#symbol-fields),
retaining literal default names and explicit Symbol/nullability read contracts.
Standalone method parsing leaves that unit obligation unresolved.

`.new-source` allocates and initializes a source object. Supported bases are
`Lang.Object`, `WatchUi.View`, and `WatchUi.BehaviorDelegate`; for example,
`.new-source %0 HelloView WatchUi.View %1` passes one argument. The base is
required and defines the native value projection: Object, View, or
BehaviorDelegate. Source class identity remains separate from that projection.
Construction is order-sensitive. Function validation checks the identifier,
supported construction profile, and API availability (Object: 1.0.0; native
View.initialize: 2.1.0; BehaviorDelegate.initialize: 1.0.0). Object has no
native base-initialization operation. Unit text records `.class Name
Lang.Object`; its explicit IR initializer is Void and contains no native
base call. Unit validation requires the named class's declared base to equal the
operation's base, with a matching explicit initializer whose parameter contracts
match the arguments. A standalone function retains this unresolved declaration
obligation and cannot supply complete application metadata by itself. AppBase
construction is reserved for application startup; it is not accepted by this
operation.

`.constant` defines a FontDefinition enum value. Validation checks the catalogue
identity, type, and API level; lowering emits its numeric value 3. It is not an
Integer operation and cannot be replaced by one when used as a font. No other
constants are accepted.

`dc` identifies a non-null Graphics.Dc reference in parameter/result contracts.
It is not constructible and `dc-array` is rejected. View.onUpdate requires one
Dc parameter and Void; onShow/onHide require zero parameters and Void. Callback
ownership and constructor resolution follow the [class
rules](../frontend/README.md#source-classes).

### Native value calls

Comparison and equality calls retain the catalogue's Object argument contracts,
including nullable equality arguments and non-null comparison arguments.
Validation does not replace these with a narrower runtime-safe class list; see
the [shared source contract](../frontend/README.md#comparison-object-arguments).

Method parameters, call results, and matching conditional-edge values use the
`method` or `nullable-method` tag, retained by explicit function result
contracts. `nullable-method` means Method or Null; reads after a proven null
test use the existing storage refinement rules. No new runtime representation is
introduced. Creation uses ordinary `.call-native`; invocation uses
`.call-native`. No separate callback instruction or target-signature annotation
is inferred. Result validation recomputes Object-or-Null for invocation,
including when all observed results were non-null. Native API declarations carry
a repeated argument type for `invoke`; ordinary fixed-arity methods remain
fixed. The existing 254-argument VM limit applies. See [source Method
contracts](../frontend/README.md#method-values-and-invocation). Class storage
uses [`.field-method`](../frontend/README.md#method-fields), retaining the
declared Method/nullability contract through field reads and writes.

`.symbol` defines a non-null Symbol by ASCII identifier, without a numeric ID.
Parameter, call-result, and matching conditional-edge contracts use `symbol` or
`nullable-symbol`; explicit function result contracts retain either tag.
Validation applies the
[source Symbol profile](../frontend/README.md#symbol-values), including API checks.

[VM lowering](../lowering/README.md#native-value-calls).

`.call-native` invokes a runtime-provided method and defines its semantic
result. The receiver and arguments must be earlier values with the usual
single-use/block restrictions. Optional source spans use the ordinary syntax.

Receiver, argument, result, generic-element, and API contracts are the same as
[the supported native methods](../frontend/README.md#native-value-calls). Validation
recomputes native results from catalogue/receiver contracts rather than trusting
frontend metadata. Unknown or unsupported members, receivers, argument counts,
and types are rejected. Runtime formatting, conversion ranges, and case mapping
are not compile-time evaluations.

Scalar and Array values use the same parameter/user-call categories described in
the [type grammar](grammar.md#type-tags), with `result=<category>` for valued
calls. Function result categories are inferred from returns; those categories
are not `.function result=` fields. Each chained call checks the preceding
result's type. Nullable Array receivers and unsupported element types cannot be
introduced by text edits.

Array `sort` uses a native call with one Null, exact source-comparator, or typed
Comparator(-or-Null) argument and a supported scalar/comparison-interface Array
receiver. Comparator elements require a non-null callback; nullable elements
remain unsupported. Its Void result is consumed by `.discard`, or the call uses
`.call-native-discard`. Validation reapplies the [sorting
contract](../frontend/README.md#source-comparators), including API 5.0.0 and
element restrictions. Source identity is reconstructed from construction, typed
source parameters/calls/fields, and locals; no text claim can turn an erased
View/Object into a Comparator. Standalone function IR retains a class/member
resolution obligation, which unit parsing must discharge against the recorded
`compare` declaration. Source and parsed units share this check. Lowering uses
existing member lookup and `invokem`, followed by `popv`; application DATA binds
`compare` to the runtime callback symbol.

Native `equals` uses the shared
[Object argument profile](../frontend/README.md#object-equality-and-container-observers),
including supported scalar, Array, Dictionary, and nullable/Null arguments. An
Array receiver's `equals` defines Boolean and `hashCode` defines Number; neither
changes its invariant element contract. They use existing native call
directives, including ordinary discard for unused results.

Arithmetic and comparison directives enforce the same
[operator contracts](../frontend/README.md#operator-contracts) as source, including
mixed Char addition, String identity equality, and nullable restrictions. Native
`weak` remains a call whose exact immutable receiver determines its specialized
result; it is not erased or rewritten as a copy.

Ordinary WeakReference values use `weak-reference` or `nullable-weak-reference`
for parameter, call-result, and conditional-edge tags. Class fields use
[`.field-weak-reference`](../frontend/README.md#weakreference-fields).
Opaque Object values use `object` and `nullable-object`. Explicit function
`result=` contracts retain these tags, including nullable Object results even if
all returns happen to be non-null. `get` and `stillAlive` use existing
`.call-native` operations, with result types recomputed as Object-or-Null and
Boolean. Availability, erasure, joins, and null proofs follow the [source
reference contracts](../frontend/README.md#ordinary-weak-references).

Native calls are effectful: receiver evaluation precedes arguments and
invocation, and their operation order must agree with dependency traversal at
statement sinks. The backend emits receiver evaluation and `getsv
@__ciq_native_<member>`. A zero-argument call uses `invokemz`; calls with
arguments add `frpush`, evaluate arguments in order, and use `invokem` with
argument count plus one (the receiver). Substring therefore uses `invokem 3`.
Binding requires exactly one stored symbol with the method's stored name in an
explicit source, or the supported complete application's native profile. IR
contains no symbol ID or import index.

```text
ciq-ir
.function show parameters=1
.parameter %0 0
.call-native %1 %0 toString
.println %1
.return-void
.end
```
