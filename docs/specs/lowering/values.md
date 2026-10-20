# Value and storage lowering

## Parameters and positional arguments

Backend prologue counts parameters plus one context slot; parameter index 0
loads slot 1, and local slots start after all parameters. Nonempty calls select
the callable, emit `frpush`, evaluate arguments left to right, then `invokem
<argument-count + 1>`. Empty calls keep `invokemz`. Function/call parameter
counts are limited to 254 and parameters plus locals to 255, so existing u8
fields/slot indexes fit. The 1024-operation and 64-level expression nesting
limits remain. These encoding limits do not claim runtime validation at maximum
counts.
[SDK and runtime evidence](../../investigations/compiler-arguments.md) verifies two
parameters, a following local, nested calls, and generated caller/callee
execution with observable argument order.

## Native name declarations

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
integration are described in [compiler commands](../build/compiler-cli.md) and
[the runtime experiment](../../investigations/compiler-strings.md). [Application generation](../build/application.md) allocates DATA and method
metadata; raw CODE consumers must supply them explicitly.

## Array element writes

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
frame budget. See [source
rules](../frontend/statements.md#array-element-writes),
[Number evidence](../../investigations/compiler-array-writes.md) and
[Char/View evidence](../../investigations/compiler-typed-array-writes.md).

## Dictionary constants

Lowering declares an unplaced `Item::Dictionary` requirement and emits `hpush
&alias`. Complete applications materialize it in DATA; native/text assembly uses
the same model. `dictionary_bindings()` binds requirements against provided DATA
by matching alias and ordered scalar contents, or otherwise by a unique content
match. Local pool aliases resolve to text for comparison. Missing, ambiguous,
incompatible, or conflicting bindings fail. See
[DATA constants](../formats/data-dictionaries.md) and [assembly](../assembly/model.md).

## Array allocation

Lowering emits `ipushz; newa`, with no symbol or DATA binding. Nonempty source
literals use constant loads or the [private construction
operations](../ir/containers.md#private-array-construction). The explicit IR
element type preserves empty allocation after source contextual typing. See
[nonempty construction](../../investigations/compiler-array-literals.md) and
[empty construction](../../investigations/compiler-empty-arrays.md).

## Constant arrays

Lowering emits an unplaced `.array` requirement and `apush &alias`. Complete
applications materialize its [DATA record](../formats/data-arrays.md), using a
shared local pool entry for equal String text, and bind the outer tag offset.
Raw CODE output requires an explicit matching DATA source; it does not allocate
DATA. A new mutable array is loaded on every execution, even when the same
record is referenced repeatedly. Source selection is limited to literal tokens
matching the element contract; computed/effectful elements use private builders.

## Number and Char steps

Lowering emits the operand, `ipush1 0x01`, and `addv` or `subv`. Existing single
use, block-local dependency, type, and effect-order checks apply. No extra
native call or new VM opcode is needed. See [source
updates](../frontend/statements.md#increment-and-decrement).

## Local operations

Lowering uses `lgetv <slot>; npush; lputv <slot>`: the value remains on the
expression stack while the slot releases its reference. Source-generated hidden
reference temporaries use this transfer on their final read; source locals keep
ordinary assignment/frame lifetimes. This is ownership scheduling, not early
destruction of named locals based on last use.

## Runtime object predicates

Lowering evaluates the tested value, selects the runtime class with `getmv`,
then emits `isa`. Lang class lookup binds the `Toybox_Lang` module identity,
independently of PRG import-table placement. `Has` evaluates receiver then
selector and emits `hasv`. No section offsets or native symbol IDs are stored in
semantic IR.

## Lexical casts

Lowering emits only the operand. There is no cast opcode, runtime class test,
conversion, or new PRG record. Assertions preserve one-use and effect-order
rules and may consume an incoming stack value directly without setup. Their
result type participates in subsequent native calls and instruction selection;
source-class provenance does not pass through an opaque native assertion.

## Source-class tests and casts

Lowering a source test evaluates its operand, uses `getmv` with the unit's
global class binding, and emits `isa`. The target is the emitted source class
object, not its native base or a symbol ID alone. Source casts emit only their
operand, like native lexical casts. No new VM instruction or DATA record is
required.

## Scalar instruction selection

Number constants use `ipushz` for zero, otherwise the smallest signed
`ipush1`/`ipush2`/`ipush3`/`ipush4` immediate. Long uses `lpushz`/`lpush`; Float
uses `fpushz`/`fpush`; Double uses `dpushz`/`dpush`. Signed floating zero and
bit-pattern preservation follow the [IR constant contracts](../ir/values.md).
Char uses `cpush`, Boolean `btpush`/`bfpush`, Null `npush`, and String `news`.
The assembly encoder preserves explicit widths independently of these choices.

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
schedule](../ir/model.md#validation). Context-independent bodies may omit SDK
global-resolution sequences; that does not establish valid frame setup for
arbitrary bytecode.
