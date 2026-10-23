# VM lowering

`ciq-compiler::backend::lower` consumes validated semantic IR and constructs a
`ciq-assembly::Program` directly. `Function::lower` delegates to it. Printing
and reparsing IR or assembly is optional and must preserve the same result.

The frontend establishes source meaning; IR validation checks types, ownership,
block-local single uses, and an effect schedule that the backend can honor.
Lowering assigns VM storage and selects instructions without constant folding,
extra runtime guards, or substitutions for unsupported source features. Assembly
then resolves identities and placement; it does not select operations.

Native union contracts are erased during lowering. Operations use the same VM
instructions as their concrete alternatives; only explicit source type/Null tests
emit runtime tests. Union normalization and static narrowing add no conversions,
wrapper allocation, or implicit guards.

Slot zero is the context/receiver; parameters begin at one. Ordinary locals
follow parameters in declaration/initialization order, including locals on
mutually
exclusive paths. Private builders and hidden temporaries count against the same
255-slot budget. Slots are not reused. The prologue uses `argc` without extra
locals or `argcincsp` to allocate additional frame storage. These are structural
encoding bounds, not demonstrated maximum device capacities.

Hidden reference temporaries must release their ownership on their final use. A
`LocalTake` loads the value, pushes Null, and clears its frame slot while
retaining the value on the evaluation stack. Source locals keep ordinary
assignment/frame lifetimes; there is no general last-use destruction analysis.
Ordinary CFG edges have empty stacks; value edges carry their one join argument.

[VM encodings](../vm/README.md#encoding) and [instruction semantics](../vm/README.md#instruction-reference)
are distinct from this compiler's instruction choices. Native API declarations
supply identities and availability, not VM slots, imports, or implementations.
[Application generation](../build/README.md#complete-application-generation) materializes DATA, symbols,
imports, and container sections from the lowered unit.

Contents:

- [Values and storage](#values-and-storage)
- [Control flow](#control-flow)
- [Calls and objects](#calls-and-objects)
- [Unit layout](#unit-layout)

## Values and storage

### Parameters and positional arguments

Backend prologue counts parameters plus one context slot; parameter index 0
loads slot 1, and local slots start after all parameters. Nonempty calls select
the callable, emit `frpush`, evaluate arguments left to right, then `invokem
<argument-count + 1>`. Empty calls keep `invokemz`. Function/call parameter
counts are limited to 254 and parameters plus locals to 255, so existing u8
fields/slot indexes fit. The 1024-operation and 64-level expression nesting
limits remain. These encoding limits do not claim runtime validation at maximum
counts.

Parameter writes evaluate the RHS, then emit `lputv <index + 1>`. They reuse the
existing parameter slot, leave the hidden receiver in slot 0 intact, and
allocate no local copy. Non-null tuple parameters used as mutation receivers
start with a read into an inferred local, which counts toward the same frame
budget; their mutations use that local view. Read-only tuple parameters keep
their ABI slots. A write affects this frame's value; earlier evaluation
stack values and caller variables retain their own copies/references.
[SDK and runtime evidence](../../investigations/compiler-arguments.md) verifies two
parameters, a following local, nested calls, and generated caller/callee
execution with observable argument order.

### Native name declarations

`String { result, value }` defines a semantic string; `Println { value }` is an
effectful sink without a result. Printing participates in the same conservative
IR scheduling check as user calls. Lowering uses `news` for literals and `getmv;
frpush; <argument>; invokem 2; popv` for printing. The dedicated
`__ciq_sdk_System` alias binds only to `Toybox_System`; the member alias binds
to `println`. This does not establish a general SDK naming convention.

Lowering deduplicates literal content within each function, naming declarations
`__ciq_string_<function>_<first-occurrence-index>`. These CODE declarations omit
`at`; they describe required content without assigning DATA placement. Raw CODE
needs `--data <data-source.mcasm>` in addition to `--symbols`. Matching and
integration are described in [compiler
commands](../build/README.md#compiler-commands-and-saved-stages) and
[the runtime experiment](../../investigations/compiler-strings.md). [Application generation](../build/README.md#complete-application-generation) allocates DATA and method
metadata; raw CODE consumers must supply them explicitly.

### Array element writes

Lowering evaluates the three operands and emits `aputv`. Logical expressions or
array construction in any operand stage all three operands through hidden locals
in order; these locals use the ordinary frame budget. Plain operands need no
hidden slots. Bounds are a runtime obligation, with no clamping or static range
rejection. The tested VM reports a fatal bounds error after all operand effects,
including for an index equal to the length.

Lowering emits receiver, index, value, then `aputv`, without a discarded result.
Negative and out-of-range indices remain accepted inputs with runtime bounds
obligations. The operation does not resize the array. API availability follows
Array/Number/View (1.0.0), or Char (1.3.0); source literal construction or
native operand calls may require a later API independently. Source staging
across joins appears as ordinary typed locals in saved IR and uses the existing
frame budget. See [source rules](../frontend/README.md#array-element-writes),
[Number evidence](../../investigations/compiler-array-writes.md) and
[Char/View evidence](../../investigations/compiler-typed-array-writes.md).

### Dictionary constants

Lowering declares an unplaced `Item::Dictionary` requirement and emits `hpush
&alias`. Complete applications materialize it in DATA; native/text assembly uses
the same model. `dictionary_bindings()` binds requirements against provided DATA
by matching alias and ordered scalar contents, or otherwise by a unique content
match. Local pool aliases resolve to text for comparison. Missing, ambiguous,
incompatible, or conflicting bindings fail. See
[DATA constants](../formats/data.md#dictionary-constants) and [assembly](../assembly/README.md#model-and-layout).

### Array allocation

Lowering emits `ipushz; newa`, with no symbol or DATA binding. Nonempty source
literals use constant loads or the [private construction
operations](../ir/README.md#private-array-construction). The explicit IR element
type preserves empty allocation after source contextual typing. See
[nonempty construction](../../investigations/compiler-array-literals.md) and
[empty construction](../../investigations/compiler-empty-arrays.md).

### Constant arrays

Lowering emits an unplaced `.array` requirement and `apush &alias`. Complete
applications materialize its [DATA record](../formats/data.md#array-constants),
using a shared local pool entry for equal String text, and bind the outer tag
offset. Raw CODE output requires an explicit matching DATA source; it does not
allocate DATA. A new mutable array is loaded on every execution, even when the
same record is referenced repeatedly. Source selection is limited to literal
tokens matching the element contract; computed/effectful elements use private
builders.

### Number and Char steps

Lowering emits the operand, `ipush1 0x01`, and `addv` or `subv`. Existing single
use, block-local dependency, type, and effect-order checks apply. No extra
native call or new VM opcode is needed. See [source
updates](../frontend/README.md#increment-and-decrement).

### Local operations

Any is erased during instruction selection. Dynamic arithmetic, conditions,
calls, and indexed access use the same VM operations as their concrete forms;
no boxing, type conversion, or runtime-check sequence is introduced. Dynamic
member symbols share the application's ordinary API/source name bindings.
An empty/bare source return under Any emits `npush; return`; explicit Void
retains a return without a semantic value. Array Any elements use ordinary
`newa`, fills, and reads with no element tags stored in the runtime Array.
See [dynamic IR contracts](../ir/README.md#dynamic-values).

Declaration lists become sequential declarations/initializers in the existing
scope. `.declare-local` reserves a frame slot without emitting a write; the first
assignment uses ordinary `lputv` after evaluating its RHS. No synthetic Null or
placeholder initialization is emitted.
`.variable` uses the same slot allocation, with `lputv` only when an initializer
is present. `.read-variable` emits ordinary `lgetv`; its flow-checked contract
does not insert a cast, conversion, or runtime guard. Assignments reuse the slot
even when the inferred value type changes.
Additional statement terminators are consumed during source parsing; they do
not introduce IR operations, VM instructions, or extra frame slots.

Lowering uses `lgetv <slot>; npush; lputv <slot>`: the value remains on the
expression stack while the slot releases its reference. Source-generated hidden
reference temporaries use this transfer on their final read; source locals keep
ordinary assignment/frame lifetimes. This is ownership scheduling, not early
destruction of named locals based on last use.

### Runtime object predicates

Lowering evaluates the tested value, selects the runtime class with `getmv`,
then emits `isa`. Lang class lookup binds the `Toybox_Lang` module identity,
independently of PRG import-table placement. `Has` evaluates receiver then
selector and emits `hasv`. No section offsets or native symbol IDs are stored in
semantic IR.

### Lexical casts

Lowering emits only the operand. There is no cast opcode, runtime class test,
conversion, or new PRG record. Assertions preserve one-use and effect-order
rules and may consume an incoming stack value directly without setup. Their
result type participates in subsequent native calls and instruction selection;
source-class provenance does not pass through an opaque native assertion.

### Source-class tests and casts

Lowering a source test evaluates its operand, uses `getmv` with the unit's
global class binding, and emits `isa`. The target is the emitted source class
object, not its native base or a symbol ID alone. Source casts emit only their
operand, like native lexical casts, including nullable source assertions. Null
and falsely asserted runtime identities remain unchanged. No new VM instruction
or DATA record is required.

### Scalar instruction selection

Number constants use `ipushz` for zero, otherwise the smallest signed
`ipush1`/`ipush2`/`ipush3`/`ipush4` immediate. Long uses `lpushz`/`lpush`; Float
uses `fpushz`/`fpush`; Double uses `dpushz`/`dpush`. Signed floating zero and
bit-pattern preservation follow the [IR constant
contracts](../ir/README.md#values-and-storage). Char uses `cpush`, Boolean
`btpush`/`bfpush`, Null `npush`, and String `news`. The assembly encoder
preserves explicit widths independently of these choices.

Binary trees emit left operand, right operand, then the selected instruction:
`addv`, `subv`, `mulv`, `divv`, `modv`, `andv`, `orv`, `xorv`, `shlv`, or
`shrv`. Comparisons select `eq`, `ne`, `lt`, `lte`, `gt`, or `gte`; typed IR has
already checked the operand/result domains. No folding or guards are added.
Unary plus emits only its operand. Integer inversion/complement emits `invv`.
Negation subtracts the operand from the appropriate typed positive zero; numeric
alias negation uses Number zero. This differs from a negative-zero literal.

Pure dependency traversal may differ from definition order, but initialization,
returns, and other sinks remain ordered. Potentially failing division/remainder
and effectful operations activate the [shared
schedule](../ir/README.md#validation). Context-independent bodies may omit SDK
global-resolution sequences; that does not establish valid frame setup for
arbitrary bytecode.

### Instance fields

A read selects the current instance field with `getselfv`. A write emits
`getself`, the field symbol with `spush`, its RHS, then `putv`, preserving the
existing expression staging rules. Class member binding resolves the same field
identity for every supported contract. Union types and snapshots add static
checks; lowering emits no conversion, discriminator, or implicit runtime guard.
Generic value-field metadata starts as Null, with required source constructor
writes supplying non-null initial values.

### Structural members

Structural interface contracts carry no runtime class or conversion. Calls use
the ordinary receiver/member lookup and invocation sequence. `.member-read`
evaluates the receiver and emits `getsv` with the field symbol;
`.member-write` evaluates the receiver, pushes the field symbol, evaluates the
new value, and emits `putv`. Member references bind by their stored short names,
including fields without a directly visible source-class receiver.

Compound writes and steps capture the receiver once before reading the old
field. Existing typed temporaries stage branching RHS values without moving the
read across effects or keeping values live across block edges. Direct scalar
assignments retain ordinary expression lowering. Interface shapes and nullable
snapshots add static checks without discriminator bytes or new opcodes.

## Control flow

### Terminal conditions

Lowering emits an entry block, condition evaluation, and branch, then separate
arm blocks. Both arms may read parameters and enclosing locals. The
[IR rules](../ir/README.md#control-flow) validate
block-local values and initialization on every path. See
[conditions.mc](../../../examples/compiler-probe/conditions.mc) and
[initial simulator evidence](../../investigations/compiler-conditions.md).

An `else if` uses the same branch layout inside the preceding false arm, so its
condition executes only when earlier tests fail. Standalone source blocks only
scope names: they add no IR/VM blocks or instructions, and propagate their
body's termination and enclosing loop targets. Control flow inside such a scope
still establishes the function's explicit entry block.

### While loops

Lowering uses explicit test/body/exit blocks and a symbolic backward jump from
continuing body paths. Conditions are evaluated in the test block each visit;
body effects stay in the body. Every edge has an empty evaluation stack. These
use branch/jump directives; cyclic initialization checks are specified in [IR
text](../ir/README.md#cyclic-control-flow).

`for` uses the same layout, preceded by its initializer list and followed by an
update block when present. Each list emits statements in source order using the
shared expression/assignment lowering; expression joins can introduce their
usual internal blocks. Continuing body paths, including `continue`, target the
whole update list before returning to the test. An absent condition uses an
unconditional jump to the body, without a placeholder Boolean. `do/while`
enters the body first; its continuing paths and `continue` target the post-test.
`break` always targets the innermost loop's exit.

Unexecuted update/post-test blocks remain in IR and assembly for semantic
checking. An unreachable final exit has a void-return epilogue. These forms use
existing jumps, branches, and returns; no flag local or new VM opcode is needed.

### Switch dispatch

Emit ordered `equals` calls or class tests and ordinary branches to case bodies.
Each continuing body jumps to the next textual group; the final group exits.
Default is the unmatched destination regardless of its textual position. Break
uses the nearest structured exit; continue searches through switches for a loop
latch. No new IR directive, flag local, or VM switch opcode is introduced.

Local/parameter selectors are stable during dispatch under the supported
no-closure/no-assignment-expression profile, so their storage can be read for
each test. This preserves class proofs for source reads. Computed selectors are
evaluated once into a private local before tests. A default-only switch still
evaluates and discards its selector. The reference SDK keeps a selector on its
stack with `dup`; our empty-stack control-flow edges use frame storage instead.
Calls use the existing native/source method selection and argument staging.

### Expression joins

Lower a value edge to its operand and `goto`; the parameter emits no load, copy,
placeholder initializer, or native call. Exactly one value arrives on the VM
stack. Consume it in a return, discard, local initialization/assignment, branch,
or another value jump. A prefix-free chain of unary operations or left operands
of binary/comparison operations may precede that consumption. Operations that
push call, field, or array setup must first stage the result in a local. Both
stack-use validation and declaration/evaluation-order validation enforce this;
no unrelated root may run before the parameter is consumed. Ordinary edges keep
an empty stack. Surrounding operand staging shares the existing frame budget; a
direct conditional return needs no additional local.

Exact source joins use the same stack transfer. Their class/nullability contract
adds validation metadata, not a conversion, class lookup, or additional load.
Null remains Null, and only the selected source reference reaches the join.

## Calls and objects

### Calls on constructed source objects

Lowering evaluates the receiver, selects its method with `getsv`, prepares the
argument context when needed, invokes it, and discards statement results. Native
and text paths use identical lowering and ordinary member-symbol binding.
Multiple proven source targets use this same sequence: `getsv` selects the
implementation from the actual receiver. Static class sets and result unions
emit no class tests, conversions, or additional receiver evaluations. See
the [object-call example](../../../examples/compiler-object-calls/README.md) and
[runtime evidence](../../investigations/compiler-object-calls.md).

### User module calls

Global calls use `getmv @__ciq_source_globals @__ciq_member_<member>` to select
the global receiver and member. Bind the owner alias to `globals`. Their
argument, invocation, and discarded-result sequences match module calls below;
they do not use the current instance or module receiver. Complete application
generation already provides global method records from unit declarations.

Backend lowering selects `getmv @__ciq_module_<module> @__ciq_member_<member>;
invokemz`, for zero arguments. Nonempty calls additionally use `frpush`,
argument expressions in order, and `invokem <argument-count + 1>`. Module
bindings must identify `globals/<module>`; the short `<module>` symbol is a
distinct field identity. The compiler CLI supports `--symbols
<symbol-source.mcasm>` for CODE emission: resolve generated aliases by unique
stored names in the provided `.symbol` declarations, reject absent/ambiguous
targets, and preserve ordinary aliases for assembly inputs. Library consumers
can supply bindings through the assembly encoder. [Runtime
evidence](../../investigations/compiler-calls.md) confirms return values and
observable call order. Complete application generation creates module/method
DATA from the unit; raw CODE consumers must provide those records themselves.

### Native object construction

Lowering emits `getmv @__ciq_sdk_WatchUi @__ciq_native_View; newc; dup 0; getsv
@__ciq_native_initialize; dup 1; invokem 1; popv`. The instance remains on the
stack after the initializer result is popped. Symbol sources bind those aliases
to exactly one `Toybox_WatchUi`, class, and `initialize` declaration
respectively; the class alias is `__ciq_native_BehaviorDelegate` for delegate
construction. See [constructor
evidence](../../investigations/compiler-native-views.md).

### Positional result construction

Lowering pushes the tuple length, allocates with `newa`, then emits
`dup 0; <index>; <stored slot read>; aputv` for each slot. Lengths and indexes
use the ordinary Number literal encoder, including wider pushes when needed.
The result is published after all slots are filled. Indexed reads use the
existing Array read instructions. Tuple slot replacement emits the receiver,
`dup 0`, index, replacement value, and `aputv`, leaving the same receiver on the
stack for storage under its updated IR contract. Compound targets capture the
receiver before reading/updating the slot; immediate indexes retain their
positional proof across staging. No new VM opcode or DATA record is required.
Tuple `add`/`addAll` use ordinary native method calls: read the receiver,
`getsv` with `__ciq_native_add`/`__ciq_native_addAll`, `frpush`, read the saved
argument, and `invokem 2`. Store the returned reference under its updated
contract. Empty positional values allocate with length zero and `newa`;
`addAll([])` still invokes `addAll` rather than dropping the call.
The `.array-add`/`.array-add-all` IR forms select those same instructions;
losing tuple shape changes the checked contract, not the runtime call.
Dynamic-index tuple writes use the same receiver duplication and `aputv` path
as known-slot writes, leaving runtime bounds checks in place.
Computed receiver calls reuse the staged native-call path. Capture the receiver
once, evaluate arguments once in order, invoke the method, and save its returned
reference if consumed later. Nested stores use `aputv` on the captured inner
Array; they neither copy nor replace the outer container. Discarding a tuple
write's result does not discard the store itself.
Array availability and each slot's native types are checked separately; View
construction requires its initializer's 2.1.0 level. No allocation failure
timing or memory equivalence with SDK lowering is implied. See the
[tuple example](../../../examples/compiler-tuples/README.md) and
[input evidence](../../investigations/compiler-input-counter.md).

### Native module values

Lowering selects `getmv @__ciq_sdk_Lang @__ciq_native_format`, `frpush`, the
arguments in order, and `invokem` with receiver-inclusive arity. Explicit symbol
sources bind `Toybox_Lang` and `format`; application generation supplies their
native IDs. Concrete Array types survive the erased parameter boundary. See the
[source contract](../sdk/README.md#native-module-and-ui-profiles) for literal context and
callback effects.

### Native module statements

Lowering selects `getmv @__ciq_sdk_<module> @__ciq_native_<member>`, `invokemz`,
and `popv`. Explicit symbol sources must contain one matching module/member
identity; application generation supplies the observed native IDs. The operation
is an ordered statement, not a CFG terminator. IR still requires its ordinary
returns and branches; exit has no expression value. Wrong arity, unknown
declarations, and unsupported native statement contracts are errors.

### Native value calls

Lowering emits `spush @__ciq_symbol_<name>`. Raw CODE binding resolves that
alias by a unique stored name in the supplied symbol source. Complete
application generation allocates the name in the shared identity map and retains
its development name in SYMBOLS. Known native member names reuse their pinned
IDs. These are existing assembly/SYMBOLS formats, not a new bytecode encoding.

### Computed Dictionary construction

Begin lowers to a Number push, `newd`, and private frame storage before entry
effects. A fill evaluates/stores each pair before the next; non-final pairs use
`aputvdup`, and the last uses `aputv`. Finish transfers the private reference
and clears its frame slot using `lgetv; npush; lputv`. Source control flow
stages key/value operands into locals and emits separate fill groups, preserving
empty-stack ordinary edges and incoming-value consumption. Candidate inference
is provisional; final IR checking validates each entry and its API/null proofs.
Shared checks follow the [builder
rules](../ir/README.md#private-container-checks).

## Unit layout

### Identity and CODE layout

Semantic function names remain unqualified; declaration ownership is separate.
Lowering emits all bodies into one assembly model in declaration order, using:

```text
global: __ciq_fn_g_<name-byte-length>_<name>
member: __ciq_fn_m_<module-byte-length>_<module>_<name-byte-length>_<name>
```

Lengths are decimal; source names are ASCII. For example, global `probe` maps to
`__ciq_fn_g_5_probe`, and `Numbers.difference` maps to
`__ciq_fn_m_7_Numbers_10_difference`. Internal block and string aliases use that
label as their prefix, preventing collisions across owners. Method metadata must
bind to the declaration's CODE label, not just its source name.

Calls still select symbolic module/member identities through `getmv` and the
existing context/argument convention. CODE labels identify bodies; they do not
replace module DATA method records. Symbol IDs and DATA literal placements
remain explicit encoder inputs. Strings are deduplicated within each function;
unit lowering does not merge literals across functions.

### Class bodies

Global/module bodies precede class bodies; each group preserves declaration
order, and methods retain class order. Class labels are
`__ciq_fn_c_<class-length>_<class>_<member-length>_<member>` with decimal byte
lengths and ASCII names. Internal aliases use that prefix. Body indexes used in
validation diagnostics follow this same order. All groups together have the
unit's 1..=256-body limit. Serialized spans remain provenance only.
