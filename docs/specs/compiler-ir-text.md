# Semantic compiler IR and text format

This specification defines the semantic model and its optional text format.
The representation is provisional and independent of Garmin MIR. The in-memory
pipeline is source → validated semantic IR → symbolic VM assembly → CODE;
text readers reconstruct the same models rather than defining alternate paths.
See [architecture](../architecture.md) and [pipeline evidence](../investigations/compiler-pipeline.md).

## Semantic model

Value IDs identify computations; local IDs identify stored state; parameters
identify positions; private array builders identify incomplete construction.
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

[Source contracts](compiler-subset.md), [class contracts](compiler-classes.md),
and [unit contracts](compiler-unit.md) own accepted language behavior and
resolution. [API declarations](api-declarations.md) own catalogue semantics.
This page specifies IR operations, model invariants, serialization, and lowering.

## Text and value definitions

```text
ciq-ir
.function probe
.integer %0 41 span=120:122
.return %0 span=113:123
.end
```

One nonempty directive per line. Blank lines and `#` comments are accepted.
The header is `ciq-ir`, with no version field. Exactly one
`.function <identifier>` with optional parameter/receiver/type metadata and
trailing `.end` are required; input after `.end` is
rejected. Identifiers use the assembly label identifier rules. An implicit
single-block body contains ordered local initializations, print
statements, calls, and expression trees, ending with `.return <value-id>` or
`.return-void`. Explicit blocks support branches and loops as specified below.
Definitions are `.integer <result-id> <value>` and binary directives
`.add`, `.subtract`, `.multiply`, `.divide`, `.remainder`, `.bit-and`,
`.bit-or`, `.bit-xor`, `.shift-left`, and `.shift-right`, each with
`<result-id> <left-id> <right-id>`. Value
IDs are `%` plus decimal `u32`; any such ID is supported, not just `%0`. Integer
values are decimal i32 values in `-2147483648..=2147483647`, with an optional
leading minus and no plus/hex/suffix/underscore syntax. Canonical output uses
ordinary decimal spelling; negative zero becomes zero. Typed `Operation::Integer`
already holds i32 and has the same value domain. Lowering uses `ipushz` for zero,
then the smallest signed 8-, 16-, 24-, or 32-bit immediate otherwise. Assembly
preserves encoded widths and raw operand bits independently of this compiler
choice. See the [literal evidence](../investigations/compiler-number-literals.md).

Each operation optionally has `span=<start>:<end>` with decimal `usize` byte
offsets and `start <= end`. These offsets refer to the original source, not
positions in the IR text. Spans are optional metadata; they do not affect
generated instructions. The current format does not record a filename or embed
the original source, and cannot verify annotations against that source. A text
edit may require updating or omitting its span. This is not generated PRG debug
information.

The text parser and typed `Function::new(name, operations)` use the same
semantic validator. It requires unique value definitions, operands referring to
earlier definitions, exactly one use of every defined value, and a final return.
Local initialization consumes an expression value; each local read defines a
fresh expression value. The function is limited to 1024 operations including
return; parameter metadata is not an operation. Invalid names/spans,
out-of-range literals, shared values, dead definitions, forward/undefined uses,
and malformed control flow are rejected. An implicit single-block body must
end with exactly one return; explicit blocks follow the control-flow rules below.
For pure-only functions,
definition order need not match VM stack order: the backend walks dependencies
for each initialization/return in statement order and emits each binary left
operand before its right operand. `parse_ir` reports positions in the IR input;
typed validation reports the offending operation index when available. Source
compilation also constructs its IR through this validator.

`Function::to_ir_text()` prints the semantic operations and optional spans,
preserving value IDs and receiver metadata. `parse_ir` reconstructs them.
`backend::lower()` (also
exposed through `Function::lower()`) creates the shared assembly `Program`
directly; it does not print/reparse text. `Function::assembly()` is convenience
lowering plus assembly printing. This gives a native source → IR → assembly
model → CODE encoder path and text stop/resume points at both IR and assembly.

`.multiply`, `.divide`, and `.remainder` require two non-null Number operands
and produce Number. Other types, including nullable Number without a proof,
are rejected. Lowering selects `mulv`, `divv`, and `modv`; no constant folding
or guard is emitted. Division/remainder may fail at runtime and therefore
require expression evaluation order in IR, even without calls or assignments.
A saved graph that would reorder them is rejected. Literal zero divisors remain
runtime operations. Rounding, signs, overflow evidence, and the simulator's
minimum-Number/-1 failure follow the [source arithmetic contract][products].

`.bit-and`, `.bit-or`, `.bit-xor`, `.shift-left`, and `.shift-right` each
consume two non-null Numbers and produce Number. Shared validation rejects
other types and missing null proofs. Lowering selects `andv`, `orv`, `xorv`,
`shlv`, or `shrv` after emitting left and right operands once in that order.
Shift counts pass unchanged to the VM; no range check, masking, or constant
folding is emitted. All ordinary block,
single-use, staging, and effect-order rules apply. See
[bitwise evidence](../investigations/compiler-number-bits.md).

`.complement %<result-id> %<operand-id> [span=<start>:<end>]` consumes one
non-null Number and produces its 32-bit complement as Number. Nullable operands
require a non-null proof; other types are rejected. It retains source `~` as a
unary operation and selects `invv`, evaluating the operand once. Single-use,
block, staging, and effect-order rules apply. Repeated complements remain
separate operations; ordinary `.bit-xor` still selects `xorv`. See the
[lowering evidence](../investigations/natural-lowering.md#number-complement).

Arithmetic directives use expression value IDs. For example:

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

Boolean literals use `.boolean %<result-id> true|false`, with exactly these
lowercase spellings. They produce `Toybox.Lang.Boolean`, distinct from integer
zero/one and Null. Boolean values may initialize/read/assign Boolean locals,
feed branches, compare for equality with another Boolean, or be printed. They
cannot be integer operands; typed user calls and returns accept them. Lowering
uses `btpush`/`bfpush`, with ordinary local slots and `eq` for Boolean equality.

## Boolean negation

`.not %<result-id> %<operand-id> [span=<start>:<end>]` consumes one non-null
Boolean and defines Boolean. Other types are rejected without coercion. The
operand must be an earlier definition in the same block and obey ordinary
one-use rules. Typed and text IR recompute the contract and include the operand
in effect traversal. Lowering evaluates it once and emits `invv`, without a
native binding. Negating a
direct null comparison reverses its non-null proof edge when used by a branch.
See the [runtime evidence](../investigations/compiler-boolean-negation.md).

Short-circuit expressions use existing `.block`, `.branch`, and `.jump` forms,
not `.and`/`.or` directives. Each operand ends its own block; the right operand
is entered only on the required outcome. Negated source conditions may reverse
destinations without a `.not`. Every branch still requires Boolean; ordinary
block-local value, effect-order, and empty-stack edge rules apply. Text resumes
the explicit graph rather than reconstructing source operators. See the
[source profile](compiler-subset.md#short-circuit-expressions).

Value-producing logical expressions also use `.boolean`, `.local`, `.assign`,
and `.read`. A temporary Boolean local is initialized before branching,
assigned true on the true outcome, and read after both outcomes join. Surrounding
operands evaluated before a later branch are stored in distinct temporary locals
and read in the final block. Temporary locals obey the same type, initialization,
frame-budget, and null-proof rules as source locals; no producer-only metadata
or new directive grants special treatment. Each CODE edge has an empty stack.
See the [value-result evidence](../investigations/compiler-logical-values.md).

## Number negation

`.positive %<result-id> %<operand-id> [span=<start>:<end>]` consumes one
non-null Number and defines an unchanged Number result. Nullable operands
require a checked fresh read under a non-null proof; other types are rejected.
Definitions, single-use rules, block boundaries, spans, and effect order apply
as for other unary operations. Lowering emits the operand exactly once and no
additional VM instruction. This preserves type checking through saved IR even
though executable code is identity. `.integer` spelling still rejects a
leading plus; source `+1` uses `.integer 1` followed by `.positive`.
See [unary-plus evidence](../investigations/compiler-number-plus.md).

`.negate %<result-id> %<operand-id> [span=<start>:<end>]` consumes one Number
and defines Number. String, Char, Boolean, Null, and unchecked nullable values
are rejected. Operands must be earlier definitions in the same block and obey
ordinary one-use rules. Source and typed/text IR recompute the contract; fresh
reads with a direct non-null proof may feed it. Effect traversal evaluates the
operand once and retains call order. Lowering emits zero, the operand, then
`subv`; it requires no native member or new opcode. Minimum Number negation
wraps to the same i32 minimum in the tested runtime. See the
[evidence](../investigations/compiler-number-negation.md).

## Char constants

`.char %<result-id> 0x<eight-hex-digits> [span=<start>:<end>]` defines Char from
raw u32 bits. Exactly eight digits follow lowercase `0x`; canonical output uses
lowercase digits. This is independent of the narrower source literal syntax:
surrogates and non-scalar bit patterns are preserved. Validation gives the value
the distinct Char type and requires API 1.3.0 when a context is supplied.
Lowering emits `cpush` with the same bits, without a DATA string or native
conversion call. Invalid widths and malformed operands are rejected. See the
[literal evidence](../investigations/compiler-char-literals.md).

## Native object construction

`.new-native %<result> Toybox.WatchUi.View [span=<start>:<end>]` allocates and
initializes a non-null View with no arguments. Typed IR uses
`NativeNew { result, class: NativeClass::View }`.
`Toybox.WatchUi.BehaviorDelegate` is also accepted, using the corresponding
NativeClass, `behavior-delegate` parameter/result tag, and initializer API 1.0.0.
Other class paths and extra arguments are rejected. View parameter text is
`view`; user value calls use
`result=view`. Function results are inferred from returns; `.function
result=view` is unsupported.

Construction requires API 2.1.0 for View.initialize, while passing an existing
View needs only the class's 1.0.0 availability. It participates in single-use,
same-block, and effect-order checking as an allocation/call. Discarding its
result still executes allocation and initialization.

Lowering emits `getmv @__ciq_sdk_WatchUi @__ciq_native_View; newc; dup 0;
getsv @__ciq_native_initialize; dup 1; invokem 1; popv`. The instance remains
on the stack after the initializer result is popped. Symbol sources bind those
aliases to exactly one `Toybox_WatchUi`, class, and `initialize` declaration
respectively; the class alias is `__ciq_native_BehaviorDelegate` for delegate
construction. See [constructor evidence](../investigations/compiler-native-views.md).

## Positional result construction

`.tuple-new %<result> %<view> [%<delegate>] [span=<start>:<end>]`
constructs a fresh positional result. One slot has `TupleShape::OneView`; two
have `TupleShape::ViewDelegate`, with View then InputDelegate API contracts.
The supported second-slot value is a non-null BehaviorDelegate. Wrong slot
count, ordering, types, or unsupported fields are rejected. Typed IR uses
`TupleNew { result, elements }` with the same checks.

Every operand must be a local/parameter read evaluated earlier. Store effectful
slot expressions left to right before constructing the tuple. Single-use,
same-block, initialization, and effect-order rules remain in force. Tuple shapes
are distinct from homogeneous Arrays and have no Array mutation/call profile.
Locals may retain/copy a shape. Function results are inferred from tuple returns;
explicit tuple function headers are unsupported. User-call metadata may use
`result=one-view` or `result=view-delegate`; unit validation checks the actual
callee result and rejects ordinary calls to native callbacks.

Lowering allocates length one or two with `ipush1 <length>; newa`, then emits
`dup 0; ipush1 <index>; <stored slot read>; aputv` for each slot. The result is
published after all slots are filled. Array/View/InputDelegate exist at API
1.0.0; View construction requires its initializer's 2.1.0 level. No allocation
failure timing or memory equivalence with SDK lowering is implied. See
[input evidence](../investigations/compiler-input-counter.md).

## Number and Char steps

`.increment %<result> %<operand> [span=<start>:<end>]` and `.decrement` add or
subtract one from a non-null Number. Increment also accepts Char (API 1.3.0);
both operations preserve the operand type. Typed construction uses
`Operation::Step` with an `increment` Boolean. Char decrement, other classes,
and unproven nullable operands are rejected. These are semantic value operations:
capture/read and target write-back remain explicit surrounding operations.
Source updates themselves
are statements and expose no expression result.

Lowering emits the operand, `ipush1 0x01`, and `addv` or `subv`. Existing single
use, block-local dependency, type, and effect-order checks apply. No extra native
call or new VM opcode is needed. See [source updates](compiler-subset.md#increment-and-decrement).

## Array allocation

`.array-new %<result> <element> [span=<start>:<end>]` allocates a fresh empty
non-null array. `element` is `char`, `number`, or `view`; typed construction uses
`Operation::ArrayNew` with `ArrayElement::Char` or `ArrayElement::Number`.
View uses `ArrayElement::Object(NativeClass::View)`. Other element contracts are
rejected. Definition/use, block, and evaluation order checks apply to allocation
even if its result is consumed later.
API context requires 1.3.0 for Char, or 1.0.0 for Number, View, and Array.

Lowering emits `ipushz; newa`, with no symbol or DATA binding. No nonzero length
or uninitialized element contract is implied. Nonempty source literals use
constant loads or the private construction operations below.
The explicit IR element type preserves empty allocation after source contextual
typing. See [nonempty construction](../investigations/compiler-array-literals.md)
and [empty construction](../investigations/compiler-empty-arrays.md).

## Constant arrays

```text
.array-constant %0 number 0x0000000a 0x00000014
.return %0
```

`.array-constant %<result> <element> <bits>... [span=<start>:<end>]` defines a
fresh mutable array.
The element type is `number` or `char`; at least one element is required. Each
value is `0x` followed by exactly eight hexadecimal digits, preserving signed
Number or raw Char bits. Mixed types, nested containers, and other element
contracts are rejected. API checks require Number/Array 1.0.0 or Char 1.3.0.
Definition/use and ordered-effect checks include this allocation.

Lowering emits an unplaced `.array` requirement and `apush &alias`. Complete
applications materialize its [DATA record](data-arrays.md) and bind the outer
tag offset. Raw CODE output requires an explicit matching DATA source; it does
not allocate DATA. A new mutable array is loaded on every execution, even when
the same record is referenced repeatedly. Source selection is limited to
homogeneous literal tokens; computed/effectful elements use private builders.

## Private array construction

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

- `.array-begin &<builder> <element> <length>` allocates the final length.
  Element contracts are `char`, `number`, or `view`; length is decimal, positive,
  and fits a nonnegative Number. Builder declarations are unique within a
  function. Allocation remains ordered and may fail before element effects.
- `.array-fill &<builder> <index> %<value>` initializes the next slot.
  The decimal index must equal the number of slots already filled and be less
  than the declared length. The value must match the element contract.
- `.array-fill-many &<builder> <start> %<value>...` initializes a nonempty group
  of consecutive remaining slots, evaluating and storing each value before
  the next. Start and group length must fit the remaining construction range.
  Values obey ordinary definition, single-use, block, and effect-order checks.
- `.array-finish %<result> &<builder>` requires all slots initialized. It consumes
  the builder and exposes a non-null typed Array; reuse requires execution of
  the original begin again, such as in a loop.

Optional `span=<start>:<end>` follows each operation. Every CFG join requires
identical active-builder progress on its incoming paths. Returns cannot leave
builders active. Premature publication, skipped/repeated/out-of-order fills,
unknown/inactive builders, and mismatched element types are errors. The finite
CFG check includes backedges; it rejects paths with inconsistent progress.

Each builder reserves a VM frame slot alongside parameters and ordinary locals,
under the shared 255-slot limit. Lowering selects a Number push, `newa`, and a
private store at begin; fills use `aputv`, or `aputvdup` followed by final `aputv`
for grouped values. Finish loads the completed array. No native `add` binding
or availability requirement is imposed by construction. API checks still apply
to Array, element types, and any calls within elements. Empty construction keeps
the separate `.array-new` operation. See
[lowering evidence](../investigations/natural-lowering.md#array-construction).

## Array element reads

The read result follows the receiver's element type, including native View.

`.array-read %<result> %<array> %<index> [span=<start>:<end>]` consumes a
non-null supported array and non-null Number index, and defines the element
type: Char or Number. Shared validation
recomputes operand types, null proofs, block-local definitions, one-use rules,
and evaluation order. No element/result type annotation is trusted. Array
evaluates before index, once each; lowering emits those operands then `agetv`.
Reads are order-sensitive because bounds failures are observable, even when
the function has no calls, prints, or writes.

Index values are not range-checked, clamped, or adjusted by the compiler.
Negative and other out-of-range Numbers remain accepted IR inputs whose reads
may fail at runtime. Other element types, String/dictionary receivers, and
nullable arrays remain unsupported. See the
[source profile](compiler-subset.md#char-arrays) and
[runtime evidence](../investigations/compiler-array-reads.md).

## Array element writes

`.array-write %<array> %<index> %<value> [span=<start>:<end>]` is a statement
with no result. It consumes a non-null supported array, non-null Number index,
and a value matching the invariant Number, Char, or native View element type.
Source View instances project to native View; array slots retain no exact
source-class identity. Shared validation recomputes these types and null
proofs, checks defined block-local single-use operands, and preserves their
left-to-right evaluation order. Writes activate the effect-order check even
without calls or printing; reads cannot move across a write.

Lowering emits receiver, index, value, then `aputv`, without a discarded result.
Negative and out-of-range indices remain accepted inputs with runtime bounds
obligations. The operation does not resize the array. API availability follows
Array/Number/View (1.0.0), or Char (1.3.0); source literal construction or native
operand calls may require a later API independently. Source staging across
joins appears as ordinary typed locals in saved IR and uses the existing frame budget.
See [source rules](compiler-subset.md#array-element-writes),
[Number evidence](../investigations/compiler-array-writes.md) and
[Char/View evidence](../investigations/compiler-typed-array-writes.md).

Source indexed compounds capture array/index with ordinary typed locals,
perform `.array-read` before RHS effects, apply the binary directive, and finish
with `.array-write`. Plain RHS expressions inline the old read into the binary
value. A branching RHS stores that old read and the computed result before
creating the final write operands, preserving block-local IDs and effect order.
No indexed-compound directive or result-type annotation is introduced. See
[source rules](compiler-subset.md#indexed-compound-assignment) and
[evidence](../investigations/compiler-compound-targets.md).

## API context inputs

`--target-api <major.minor.patch>` and `--min-api <major.minor.patch>` enable
introduction-level checks for source or IR input, in function and unit modes.
Supply both or neither. Each component is decimal `u16`; both levels must be at
least 1.0.0, and the minimum must not exceed the target. Malformed, duplicate,
partial, or inconsistent inputs fail before output creation. Assembly input
rejects these options because semantic API references are no longer available.

Known unguarded references must be available on the target and introduced no
later than the application's minimum API. Errors distinguish unknown members,
target unavailability, an insufficient minimum, and unsupported lowering.
For example, Number's `toChar` requires 1.3.0: target 1.0.0 reports unavailable;
target 1.3.0 with minimum 1.0.0 reports an insufficient minimum; both at 1.3.0
permit the conversion. The current subset has no guarded `has` calls.

```sh
cargo run -p ciq-compiler -- --target-api 1.0.0 --min-api 1.0.0 \
  --emit ir input.mc scratch/function.ciqir
cargo run -p ciq-compiler -- --from ir --target-api 1.0.0 --min-api 1.0.0 \
  scratch/function.ciqir scratch/function.mcasm
```

Levels are external build inputs, absent from `ciq-ir`/`ciq-unit` and PRG
deployment metadata. Resupply them when resuming text. Omitting both performs
the existing bounded contract/lowering checks without an API-level claim.
These inputs do not select a device, derive a level from the SDK, or check
permissions, app kind, execution scope, or VM capabilities. See the
[API declaration design](api-declarations.md) for coverage.

## Local operations

`.local $<local-id> %<value-id>` initializes a local from an earlier value.
`.read %<result-id> $<local-id>` defines a fresh value from an earlier
initialized local. Both accept the same optional source span. Local IDs use `$`
followed by decimal `u32`; value IDs still use `%`. Initializations must have
unique IDs; reads before initialization, reinitialization, and parameters plus
locals exceeding 255 are rejected. Unread locals are accepted: their initializer
still executes. Source names are resolved by the frontend and are not stored in
this provisional IR.

Local IDs are semantic identities, independent of their numeric value or VM
slot. The backend assigns local slots after parameters in initialization order
and executes every initializer before proceeding. Repeated reads may refer to
one local, while each resulting value still has one use. Pure-only functions use
dependency traversal while locals retain their initialized value.
Functions with calls, prints, or assignments add the ordering restriction below.
`.assign` writes a local without repeating initialization.

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

## Module calls and operation order

`.call %<result-id> <module> <member> [result=<type>] [%<argument-id> ...]`
defines the result of a positional-argument call to a top-level user module.
The result type is `integer`, `string`, `boolean`, `char`, `char-array`,
`number-array`,
`nullable-integer`, or
`nullable-string`; omission means integer.
Canonical text omits the integer default and writes noninteger contracts.
Void is invalid in a value call. Names use ASCII identifier rules and remain
symbolic. The optional span
has the same meaning as for other operations. The module's expected stored
symbol name is `globals/<module>` and the member's is `<member>`; this is not
numeric symbol allocation or method metadata generation.

Calls, prints, and writes are effectful. If a function contains any `.call`,
`.println`, `.call-native`, or `.assign`, all its operations must match the backend's
left-to-right expression traversal and ordered
initialization/assignment/print/branch/return sinks. The shared validator
rejects incompatible schedules, including moving a call across an initialization
or reversing call operands. Pure-only functions permit flexible definition order.
This conservative policy keeps runtime/IR
operation order consistent until a general scheduler is implemented.

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
environment in standalone mode: the scaffold must supply matching callees.
Unit mode checks names, exact arity, argument types, and call result contracts.
Object calls and global unqualified calls remain unsupported.

## Parameter metadata and reads

The optional `parameters=<nonnegative-decimal-count>` field on `.function`
defaults to zero, with a maximum of 254. `Function::with_parameters(name, count,
operations)` uses the shared validator; `Function::new` retains zero-parameter
behavior. `Function::with_parameter_types` supplies explicit contracts.
`.parameter %<result-id> <zero-based-index>` defines a fresh value with the
signature position's type, narrowed under the [null-check proof](#nullable-results)
when available. Index must be below the parameter count;
repeated reads and unused declared parameters are allowed. Source parameter
names are resolved by the frontend, not stored in IR. Position is independent of
value/local IDs and VM slots.

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

## Command-line stages

```sh
mkdir -p scratch/pipeline-example
# Source → saved IR
cargo run -p ciq-compiler -- --emit ir examples/compiler-probe/probe.mc scratch/pipeline-example/probe.ciqir
# Saved IR → saved assembly
cargo run -p ciq-compiler -- --from ir scratch/pipeline-example/probe.ciqir scratch/pipeline-example/probe.mcasm
# Saved assembly → raw CODE bytes
cargo run -p ciq-compiler -- --from assembly --emit code scratch/pipeline-example/probe.mcasm scratch/pipeline-example/text.bin
# Source → raw CODE bytes through the native models
cargo run -p ciq-compiler -- --emit code examples/compiler-probe/probe.mc scratch/pipeline-example/native.bin
cmp scratch/pipeline-example/text.bin scratch/pipeline-example/native.bin
```

Defaults: `--from source`, `--emit assembly`; the old two-path command still
works. `--from` accepts `source`, `ir`, or `assembly`; `--emit` accepts `ir`,
`assembly`, or `code`. Source/IR can emit all three. Assembly can emit canonical
assembly or CODE, but cannot be raised to semantic IR. Duplicate options/unknown
stages are errors. Validate and encode before creating a new output file;
existing files are never overwritten.

`--emit code` emits an ordinary CODE payload, **not a complete PRG or
executable**. PRG metadata/resources and signing still use the explicitly
documented scaffold workflow. For CODE emission, `--symbols
<symbol-source.mcasm>` supplies a metadata-only `.symbol` file, including an
exported PRG symbol section. Its declared aliases resolve ordinary assembly
references. Compiler aliases `__ciq_module_<name>` / `__ciq_member_<name>`
resolve by unique stored names `globals/<name>` / `<name>`, independently of the
file's aliases. Missing/ambiguous names, duplicate aliases/IDs, conflicting
generated bindings, and non-symbol items are errors before output creation.
`--symbols` with IR/assembly emission is rejected; those stages remain symbolic.
No DATA/SYMBOLS section is written by raw CODE encoding.

```sh
cargo run -p ciq-compiler -- --emit code --symbols scratch/call-project/010-symbols-5717b015.mcasm examples/compiler-probe/calls.mc scratch/calls.bin
```

`symbol_bindings(&assembly, &symbols)` exposes the same resolution to library
consumers. `Program::encode_with_references` consumes the resulting map; callers
may also supply explicit maps. `--data <data-source.mcasm>` supplies DATA string
bindings for CODE emission only. `Source::parse_layout` validates placement
while retaining unresolved class symbol/method fields; final PRG assembly must
resolve those fields. `string_bindings(&assembly, &data)` binds each unplaced
literal declaration by unique content, independently of the DATA alias.
Missing/ambiguous content or a conflicting alias is an error. Pinned
declarations must match both content and offset; ordinary undeclared `&alias`
operands use DATA aliases directly. Neither option writes or allocates DATA.
Both options are rejected for IR/assembly emission. Pinned self-contained
assembly still encodes without `--data`; symbolic stages can be generated
without either option.

## String values and print statements

`.string %<result-id> "value"` defines a string value; `.println %<value-id>`
consumes a Number, String, Boolean, or Char without defining a result. Both accept
optional spans. Quotes preserve spaces, `#`, and text resembling a span. Escapes follow
[assembly string syntax](symbol-assembly.md); commas outside quotes are rejected.
The semantic string limit is 65535 UTF-8 bytes. String types propagate through
locals, function parameters, user calls, and returns. Arithmetic follows the
documented operand contracts. Value returns must have the same supported type;
Number, String, Boolean, and Char results are inferred from uniform return values,
unless a nullable result is declared as specified below.
Void returns have no value. All existing use/dependency/
order rules apply.

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
[reproduction](../investigations/compiler-strings.md) for source → IR → assembly
→ signed PRG and native CODE parity.

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

- `.block ^<id>` starts a block; IDs are decimal `u32`, distinct from `%` values
  and `$` locals. Block IDs must be unique; the first block is entry.
- `.equal`, `.not-equal`, `.less`, `.less-equal`, `.greater`, and `.greater-equal`
  each take `%<result> %<left> %<right>`. They compare Number/Char pairs and
  define Boolean. Equality/inequality also accept the String identity profile,
  Boolean pairs, and supported dictionary/null checks. Ordering rejects String,
  Boolean, null, and unproved nullable operands. Lowering selects `eq`, `ne`,
  `lt`, `lte`, `gt`, and `gte` respectively; operand evaluation remains left to
  right. See [comparison evidence](../investigations/compiler-number-comparisons.md).
- `.branch %<condition> ^<true-target> ^<false-target>` consumes a Boolean and
  terminates the block. The two targets may be the same block.
- `.jump ^<target>` terminates the block unconditionally.

All new directives accept the existing optional source span. A body containing
block/branch/jump operations must begin with `.block`. Every block must end in
`.return`, `.branch`, or `.jump`, with no operations after its terminator until
the next block. Targets may name earlier or later blocks. Every block must be
reachable from entry; missing targets, duplicate blocks, disconnected cycles,
and implicit
fallthrough are rejected. The final block returns.
A body without these operations uses an implicit single block.

Expression definitions still precede their uses, are globally unique, and have
exactly one use, restricted to their defining block. Locals can cross edges,
but reads and assignments require the unique initializer to execute on every
incoming path. Its declaration must also precede the textual use. Availability
is the fixed-point intersection described below. A local initialized in only
one arm cannot be read or assigned in a sibling or join.
Cross-block expression/phi values remain unsupported. Local slots are allocated
across the entire function, including mutually exclusive paths.

The 1024-operation limit includes block markers and terminators. Effectful
functions retain left-to-right expression and statement order within each
block; block markers and jumps participate in schedule validation. Branches
consume their comparisons, and no evaluation-stack values are carried over an
edge. Source parsing, typed construction, and text resumption use these same
checks. Source conditions use these same forward joins with `.jump` for
continuing arms, including nested statements and early returns. Source local
names/scopes are resolved before serialization; the IR only retains globally
unique local identities. See [the source subset](compiler-subset.md).

## Local assignment

Source local compounds use ordinary `.read`, a binary directive, and `.assign`.
There is no compound IR directive. Shared validation checks operand contracts,
result assignability, null proofs, and read-before-RHS/write ordering. Staged
RHS expressions use the existing typed local/snapshot representation. See
[source rules](compiler-subset.md#local-compound-assignment) and
[evidence](../investigations/compiler-compound-locals.md).

`.assign $<local-id> %<value-id> [span=<start>:<end>]` consumes an earlier value
and replaces an initialized local. It is accepted with `ciq-ir` or `3`; no
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

The target must be initialized on every incoming path. `.assign` does not
introduce an identity, count as another local, or establish initialization.
Repeated `.local` declarations for one ID remain invalid. Local IDs retain their
initializer's supported type; assignments must match it. Undefined
values/locals, wrong-path targets, shared values, and cross-block RHS
values are rejected. The RHS contributes one use; assignment defines no result.
Each write counts toward the 1024-operation limit and reuses the existing slot.

If a function contains any assignment, its entire operation sequence must match
left-to-right expression evaluation and ordered statement sinks, as for calls
and prints. A read defined before a write and used after it is rejected, since
dependency lowering would move the read past the write. Store a snapshot in a
separate initialized local before the write when that behavior is needed. This
rule applies to native typed construction and text resumption equally.
Assignment-free pure functions permit flexible definition order.

[The source subset][assignment-source]
specifies name resolution and supported statement forms; source names/scopes
are not part of this IR.

[assignment-source]: compiler-subset.md#local-reassignment

## Cyclic control flow

Backward branch/jump targets are permitted. The graph is checked for
reachability by walking from the first block, including cycles. Having a
predecessor alone does not make a block reachable. The final declared block must
still end in `.return`; this does not prove every runtime path terminates.

Definite-local sets are computed to a greatest fixed point. Start incoming and
outgoing sets at the universe of declared local IDs. For every block, intersect
its predecessor outgoing sets to obtain its incoming set; its outgoing set adds
all its `.local` initializers. Entry additionally has an empty external
predecessor, so its incoming set is always empty. Repeat until sets stop
changing. Scan each block's operations from its final incoming set, adding
initializers only when encountered. Reads/assignments require membership at
their position and an earlier textual declaration; assignments do not add
initialization.

This accepts common initialized locals carried around a loop and rejects values
initialized only on its backedge. Local initializer IDs remain globally unique;
one declaration may execute repeatedly, resetting its existing slot each visit.
Types retain their initializer's supported classification. Expression
values still refer to earlier definitions in the same block and have one static
use; runtime execution may visit that definition/use repeatedly. No expression
values or evaluation-stack state cross an edge.

Source loops lower to these existing graph operations. Test comparisons/calls
execute again on every visit; body statements retain evaluation order. Native
construction, text resumption, and source compilation use the same validator.
See [loop source rules](compiler-subset.md#while-loops) and
[the runtime experiment](../investigations/compiler-loops.md).

## Void returns and statement calls

Void returns and effect-only calls use:

- `.return-void [span=<start>:<end>]`: terminate without a semantic result.
- `.call-discard <module> <member> [%<argument> ...] [span=<start>:<end>]`:
  invoke a qualified user function for effects without defining a value.

`.call-discard` may target any supported user result, including Void; the caller
discards it. Argument IDs name earlier supported scalar, nullable, or Null values in the
same block and each counts as a use. Calls use identifier rules,
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
type,
`.return-void` yields void. The shared validator infers and exposes that result
contract. Nullable result declarations instead require assignability to that
union on every return. A block may terminate with
either form. The final declared block still returns, and the existing
reachability, definite-local, and empty-edge-stack rules apply. A single
`.return-void` is a valid function body. Implicit text fallthrough is rejected;
only source compilation inserts a final void return.

Void values,
general object/null/array operations, and arbitrary SDK calls are unsupported.
See [source rules][void-source] and [runtime
evidence][void-evidence].

[void-source]: compiler-subset.md#void-functions-and-statement-calls
[void-evidence]: ../investigations/compiler-void.md

## Native superclass initialization

`.base-initialize <native-base>` accepts `Application.AppBase`, `WatchUi.View`,
or `WatchUi.BehaviorDelegate` and invokes
the identified native superclass
initializer on the current instance, with no explicit arguments or value result.
It accepts the usual optional source span. Other base names or additional fields
are rejected. Function validation requires an instance receiver and preserves
the call's effect order; unit validation additionally enforces the
[source initializer contract](compiler-classes.md#source-initialization).
Source, typed, and text paths use the same operation and VM lowering.

## Nullable callback state

Instance-function metadata may include
`parameter-types=integer:string:boolean:char:view:dc:char-array:number-array:view-array:...`.
Ordinary signatures also accept
`nullable-integer` and `nullable-string`. Separate entries with colons;
the list must match `parameters=<count>`. Omitted types are integers. This field
applies to ordinary functions and instance methods. `nullable-dictionary` is
also accepted; unit validation restricts it to native AppBase.onStart/onStop
contracts. Unsupported names, duplicate lists, and mismatched counts are
errors. For example:

```text
.function onStart parameters=1 receiver=instance parameter-types=nullable-dictionary
```

`.null %<result-id> [span=<start>:<end>]` defines null. `.equal`/`.not-equal` permit
Number/Char pairs, Boolean pairs, null pairs, and nullable Dictionary/null pairs;
it also accepts the String identity profile below.
Ordering directives permit Number/Char pairs. Callback-state reads/locals preserve their
opaque nullable type.
Printing and arithmetic reject Null and Dictionary values. Null may be used
in nullable Number/String user contracts; Dictionary remains callback-only. A nullable Dictionary local may be assigned null; other
assignments preserve the initializer type. Lowering uses `npush` for null and
the existing local-slot and equality instructions.

## Instance receivers

The optional declaration field `receiver=instance` marks an instance method;
ordinary bodies omit it. It may
appear before or after `parameters=<count>`; duplicate or unknown fields are
rejected. Every instance method requires receiver metadata, even without self
calls. The receiver is the hidden context, not an explicit parameter/value ID.

Two additional directives require an instance method:

- `.call-self %<result> <member> [result=<type>] [%<argument> ...] [span=<start>:<end>]`:
  define a value under the same result contract rules as `.call`.
- `.call-self-discard <member> [%<argument> ...] [span=<start>:<end>]`:
  invoke a current-receiver member for effects and discard its result.

Arguments are ordered, earlier-defined supported scalar, nullable, or Null
values in the same block.
The existing use/effect/control-flow checks apply. Unit validation binds calls
only to declared methods in the enclosing class, checks arity/result contracts,
and rejects instance bodies in global/module owners. Standalone method IR
retains its required receiver and unbound class/callee obligations.

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

Canonical method output keeps `receiver=instance`. Native base-class
identity and enclosing class name belong to [unit class containers][class-ir],
not function IR; neither carries a VM import index. General receiver values,
object construction, fields, native self calls, and
source inheritance are unsupported.

[class-ir]: compiler-classes.md#native-and-textual-ir

## Native module statements

`.call-native-module-discard <module> <member> [%<argument-id> ...]` invokes a
runtime-provided module function without defining a value. Module names are
canonical API paths, distinct from user module owners; arguments follow the
ordinary definition/use and effect-order rules. Optional spans use the usual
syntax. The profile accepts `Toybox.System exit` and `Toybox.WatchUi
requestUpdate`, each with no arguments and a catalogue Void result. Both require
API 1.0.0. API context checks match source compilation.

Lowering selects `getmv @__ciq_sdk_<module> @__ciq_native_<member>`, `invokemz`,
and `popv`. Explicit symbol sources must contain one matching module/member
identity; application generation supplies the observed native IDs.
The operation is an ordered statement, not a CFG terminator. IR still requires
its ordinary returns and branches; exit has no expression value. Wrong arity,
unknown declarations, and unsupported native statement contracts are errors.

## Discarded values and native Void calls

`.discard %<value-id> [span=<start>:<end>]` evaluates and consumes one earlier
non-Void value without defining a result or changing the function return type.
It accepts every supported value category, including Null, nullable values, and
Char arrays. Definition/use, same-block, and evaluation-order rules apply.
The backend evaluates the operand through the usual expression path and emits
one `popv`; nested call results remain available to their consumers.

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
statements keep their existing forms; no Void expression value is created.
See [runtime evidence](../investigations/compiler-native-statements.md).

For a Void native member, use
`.call-native-discard %<receiver-id> <member> [%<argument-id> ...]`.
It defines no value. Validation requires the resolved method result to be Void;
receiver/argument types, API availability, and ordered evaluation still apply.
The bounded Dc clear/setColor/drawText profile uses this form.

## Source construction and native constants

Source instance calls use:

```text
.call-object %<result> %<receiver> <class> <member> [result=<type>] [%<arg> ...]
.call-object-discard %<receiver> <class> <member> [%<arg> ...]
```

Optional spans follow the existing syntax; omitted result types mean Integer.
The receiver's exact source class must be proven through construction, typed source
parameters/results, or local operations, independently of the class written in
the call. Unit validation
checks the selected class method's arity, parameter/result types, and callback
restrictions. Both forms preserve receiver-before-argument evaluation order.
Statement calls define no semantic result. Standalone function parsing retains
declaration-resolution obligations; it does not authorize arbitrary receiver
casts. See the [source-call contract](compiler-classes.md#calls-on-constructed-source-objects).

`source(ClassName)` is an exact non-null source View contract in
`parameter-types=...` and function/call `result=...` fields. It occupies one
parameter-list entry without a colon inside the tag. Source function results
always serialize this tag; omission erases the exact class at the return boundary.
Class names must be valid identifiers and resolve to source View declarations
during unit validation. Return provenance and call arguments/results are checked
through the [source signature rules](compiler-classes.md#source-class-signatures);
the tag does not cast an arbitrary native View.

Instance field operations are `.read-self %<result-id> <field-name>` and
`.assign-self <field-name> %<value-id>`, with optional source spans. Reads define
Number by default; `result=source(ClassName)` selects a source-reference read.
Writes consume Number or a source reference without a result. Both require an
instance receiver
and preserve observable order. Their symbolic field identities resolve through
the enclosing unit class's
[field declarations](compiler-classes.md#number-instance-fields).
Reference declarations, result claims, and initializer safety follow the
[reference-field contract](compiler-classes.md#source-reference-fields).
Standalone method parsing leaves that unit obligation unresolved.

`.new-source %<result-id> <class-name> <native-base> [%<argument-id> ...]`
allocates and initializes a source object. Supported bases are `WatchUi.View` and
`WatchUi.BehaviorDelegate`;
for example, `.new-source %0 HelloView WatchUi.View %1` passes one argument.
The base is required and defines the native value projection, currently a
non-null View or BehaviorDelegate. Construction is order-sensitive. Function validation checks the
identifier, supported construction profile, and native initializer availability
(View: API 2.1.0; BehaviorDelegate: API 1.0.0). Unit validation requires the named
class's declared base to
equal the operation's base, with a matching explicit
initializer whose parameter contracts match the arguments. A standalone function
retains this unresolved declaration obligation and cannot supply complete
application metadata by itself. AppBase construction is reserved for application
startup; it is not accepted by this operation.

`.constant %<result-id> Toybox.Graphics FONT_MEDIUM` defines a FontDefinition
enum value. Validation checks the catalogue identity, type, and API level;
lowering emits its numeric value 3. It is not an Integer operation and cannot
be replaced by one when used as a font. No other constants are accepted.

`dc` identifies a non-null Graphics.Dc reference in parameter/result contracts.
It is not constructible and `dc-array` is rejected. View.onUpdate requires one
Dc parameter and Void; onShow/onHide require zero parameters and Void. Callback
ownership and constructor resolution follow the [class rules](compiler-classes.md).

## Native value calls

`.call-native %<result-id> %<receiver-id> <member> [%<argument-id> ...]` invokes a
runtime-provided method and defines its semantic result. The receiver and
arguments must be earlier values with the usual single-use/block restrictions.
Optional source spans use the ordinary syntax.

Receiver, argument, result, generic-element, and API contracts are the same as
[the supported native methods](compiler-subset.md#native-value-calls). Validation
recomputes native results from catalogue/receiver contracts rather than trusting
frontend metadata. Unknown or unsupported members, receivers, argument counts,
and types are rejected. Runtime formatting, conversion ranges, and case mapping
are not compile-time evaluations.

Char and Array values use `char`, `char-array`, `number-array`, or `view-array`
in parameter/user-call metadata, with `result=<category>` for valued calls.
Function result categories are inferred from returns; those categories are not
`.function result=` fields. Each chained call checks the preceding result's type.
Nullable arrays/Char and unsupported elements cannot be introduced by text edits.

Arithmetic and comparison directives enforce the same
[operator contracts](compiler-subset.md#operator-contracts) as source, including
mixed Char addition, String identity equality, and nullable restrictions.
Native `weak` remains a call whose exact immutable receiver determines its
specialized result; it is not erased or rewritten as a copy.

Native calls are effectful: receiver evaluation precedes arguments and
invocation, and their operation order must agree with dependency traversal at
statement sinks. The
backend emits receiver evaluation and `getsv @__ciq_native_<member>`. A
zero-argument call uses `invokemz`; calls with arguments add `frpush`, evaluate
arguments in order, and use `invokem` with argument count plus one (the
receiver). Substring therefore uses `invokem 3`. Binding requires exactly one
stored symbol with the method's stored name in an explicit source, or the
supported complete
application's native profile. IR contains no symbol ID or import index.

```text
ciq-ir
.function show parameters=1
.parameter %0 0
.call-native %1 %0 toString
.println %1
.return-void
.end
```

## Nullable results

`.call-native` for String `find`, `toNumber`, and `toNumberWithBase` defines
Number-or-Null according to its API contract; String `substring` defines
String-or-Null. Index API
requirements are the same as the [source contract][nullable-source]. The existing
`.local`, `.read`, `.null`, `.equal`, `.not-equal`, and
`.branch` forms carry this behavior; no assertion or unchecked cast operation
is introduced. Equality with Null is accepted in either order.

Shared IR validation computes non-null local and parameter facts from control
flow. A branch whose condition is a direct `.equal` between `.read` or
`.parameter` and `.null` establishes that storage identity's fact on its false
edge. A direct `.not-equal` check establishes it on the true edge instead.
A chain of `.not` definitions around either comparison reverses the proof
edge once per operation. Every definition in the guard chain must precede its
consumer in the branch's block. The checked read must be in that block; a checked
local must have
no intervening write. Both outcomes reaching the
same target provide no new proof. Entry has no external facts; joins intersect
incoming facts and every initialization/assignment kills that local's proof.
The fixed-point analysis handles backedges without treating a later guard as
proof for a loop's first entry.

A fresh read of a nullable Number/String local or parameter under this proof
defines its non-null type; otherwise it defines the original union. Storage
remains nullable. Local assignment can supply Null, the non-null type, or the same union, but always
requires another proof before non-null use. Existing effect-order, one-use,
and block-local value rules apply.
Stored Boolean guards and comparisons against null-valued locals are not proof
forms. Nullable values may use matching user parameter/result contracts,
but cannot use non-null consumers without proof. Nullable Numbers may be passed
as substring indices,
which explicitly accept that union. Readers and typed constructors recompute
all facts; IR text carries no trusted narrowing metadata. See
[source examples][nullable-source].

[nullable-source]: compiler-subset.md#nullable-results

## Nullable user signatures

A function declaration may carry `result=nullable-integer` or
`result=nullable-string`. Every value return must be Null, the corresponding
scalar type, or the same union. A void return is invalid. The declared union is
preserved even when all returns are scalar or all are Null; canonical text
always writes it. Omitted result metadata infers a uniform scalar/Void contract
and rejects nullable/Null returns. Other function result fields are unsupported.
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
union for nullable arguments; scalar arguments still require exact types.
Call result contracts must equal the callee's declared result, even when its
body always returns a scalar. This prevents edited call metadata from hiding
nullable results. Standalone IR has no callee signature environment.

Parameter reads retain their declared union until a direct branch proves the
parameter non-null. Later reads under that proof define the scalar alternative.
Local and parameter identities are distinct; neither a copied value nor another
parameter is narrowed by the original's proof. Parameters have no write
operation, so unrelated local writes preserve parameter facts.
No casts or new VM instructions are introduced. See the
[complete example](../../examples/compiler-nullable-contracts/README.md).

[products]: compiler-subset.md#arithmetic-and-lowering-contract
