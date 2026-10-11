# Supported instruction reference

This reference covers each of the 39 opcodes currently encoded and decoded by
`ciq-mc`. It describes behavior in our own words, with the
[instruction investigation](../investigations/instruction-semantics.md)
recording source revisions, observed bytes, and limitations. The
[encoding table and assembly grammar](bytecode-subset.md) remain the compact
reference.

## Scope and notation

Encodings below occur in SDK 9.2.0, `fr265`, `-O 0` debug builds,
including the [control-flow specimen](../investigations/control-flow.md),
except `bt`, tested through a controlled opcode replacement.
Instruction presence and operand widths are **experimentally observed**.
Execution descriptions and abstract stack effects are **source-derived**, with
compiled source/bytecode supporting their interpretation. We have not traced
individual instructions inside Garmin's VM. The original Hello World runs in the
simulator; this does not validate every opcode's edge cases or every generated
function's execution. Compatibility with other SDK/device versions is unverified
for every entry.

An opcode occupies one byte. `u8` occupies one byte; `u32be` occupies four
bytes, most significant first. Length includes the opcode. Numeric assembly
operands currently preserve bit patterns; declared `@alias` operands are
supported in symbol-ID fields as described in
[symbol assembly](symbol-assembly.md), and `news` accepts `&alias` references as
described in [DATA strings](data-strings.md). Stack expressions in these
descriptions remain explanatory.

Stack notation puts the top at the right. `...` means unchanged lower values.
“Local” denotes a frame slot, separate from the abstract evaluation stack. Stack
effects summarize successful execution, excluding exceptions and physical frame
mechanics. Member IDs refer to [symbols](symbols.md); string data addresses are
a different reference kind. Relative branch and subroutine operands identify
CODE instruction targets, independently of symbols and DATA addresses.

## Index

- [`popv`](#popv): discard the top value.
- [`npush`](#npush): push null.
- [`addv`](#addv): add two values.
- [`subv`](#subv): subtract two values.
- [`divv`](#divv): divide two values.
- [`orv`](#orv): combine values with bitwise or.
- [`putv`](#putv): store an object member.
- [`invokem`](#invokem): invoke a callable with counted call slots.
- [`aputv`](#aputv): store an array element.
- [`lgetv`](#lgetv): load a local-frame slot.
- [`lputv`](#lputv): store a local-frame slot.
- [`newa`](#newa): create an array.
- [`newc`](#newc): create a class instance.
- [`return`](#return): return from a function.
- [`news`](#news): load a string from serialized data.
- [`spush`](#spush): push a symbol value.
- [`frpush`](#frpush): prepare a function-call context value.
- [`dup`](#dup): duplicate a value at stack depth.
- [`getm`](#getm): resolve a module symbol.
- [`argc`](#argc): set up the function argument frame.
- [`ipushz`](#ipushz): push integer zero.
- [`ipush1`](#ipush1): push a one-byte integer constant.
- [`getselfv`](#getselfv): load a member of the current receiver.
- [`getself`](#getself): load the current receiver/context.
- [`getmv`](#getmv): load a member of a named module.
- [`getlocalv`](#getlocalv): load a member through a local-frame slot.
- [`getsv`](#getsv): load a member of a stacked receiver.
- [`invokemz`](#invokemz): invoke a callable with no explicit arguments.
- [`argcincsp`](#argcincsp): set up arguments and reserve local-frame space.
- [`goto`](#goto): jump unconditionally.
- [`bt`](#bt): branch on a true condition.
- [`bf`](#bf): branch on a false condition.
- [`eq`](#eq): compare two values for equality.
- [`gt`](#gt): compare two values for greater-than.
- [`jsr`](#jsr): call a local subroutine.
- [`ret`](#ret): return from a local subroutine.
- [`throw`](#throw): raise an exception.
- [`btpush`](#btpush): push Boolean true.
- [`bfpush`](#bfpush): push Boolean false.

## popv

**Discard the top value.** Opcode `0x02`; length **1 byte**. Operands: none.

Abstract stack: `[..., value] → [...]`.

Removes one evaluation-stack value. Generated initialization and call sites use
it to discard results they do not need.

Observed encoding: `02` at CODE+`0x000022` in **minimal-console**, Rez/<init>.

**Limits:** This establishes neither object destruction nor garbage-collection
timing.

## addv

**Add two values.** Opcode `0x03`; length **1 byte**. Operands: none.

Abstract stack: `[..., a, b] → [..., a + b]`.

Consumes the two top values and pushes their sum. In HelloWorldView/onUpdate it
participates in preparing display content.

Observed encoding: `03` at CODE+`0x0000c0` in **hello-world**,
HelloWorldView/onUpdate.

**Limits:** Numeric promotion, overflow, string concatenation, and invalid-type
behavior have not been independently characterized.

## subv

**Subtract two values.** Opcode `0x04`; length **1 byte**. Operands: none.

Abstract stack: `[..., a, b] → [..., a - b]`.

Subtracts the top value from the value below it. HelloWorldView/onUpdate uses it
in display-coordinate calculations.

Observed encoding: `04` at CODE+`0x000098` in **hello-world**,
HelloWorldView/onUpdate.

**Limits:** Operand order matters. Numeric promotion and overflow behavior
remain unverified.

## divv

**Divide two values.** Opcode `0x06`; length **1 byte**. Operands: none.

Abstract stack: `[..., a, b] → [..., a / b]`.

Divides the value below the top by the top value. The reference view uses it
while calculating display coordinates.

Observed encoding: `06` at CODE+`0x00007c` in **hello-world**,
HelloWorldView/onUpdate.

**Limits:** Integer rounding, mixed numeric types, division by zero, and
exceptional values need controlled experiments.

## orv

**Combine values with bitwise OR.** Opcode `0x08`; length **1 byte**. Operands:
none.

Abstract stack: `[..., a, b] → [..., a | b]`.

Source documentation identifies a bitwise OR. Generated font/style
initialization combines values before a field assignment.

Observed encoding: `08` at CODE+`0x000092` in **minimal-console**,
prompt_font__title/<init>.

**Limits:** Do not infer short-circuit Boolean behavior. Supported types and
conversion rules remain unverified.

## putv

**Store an object member.** Opcode `0x0e`; length **1 byte**. Operands: none.

Abstract stack: `[..., object, symbol, value] → [...]`.

Assigns value to the member selected by a Symbol value on the stack. All three
inputs are consumed; no assignment result is pushed. The instruction itself has
no immediate symbol operand.

Observed encoding: `0e` at CODE+`0x000093` in **minimal-console**,
prompt_font__title/<init>.

**Limits:** Member lookup, inheritance, read-only members, and invalid receivers
are not characterized.

## invokem

**Invoke a callable with counted call slots.** Opcode `0x0f`; length **2
bytes**. Operands: `n: u8` — counted call slots.

Abstract stack: `[..., callable, slot_0, …, slot_(n-1)] → [..., result]`.

The u8 operand n counts call slots; the optimizer models consumption of n + 1
values, including the callable, and production of one result. In hello(), getmv
selects println, frpush prepares a context slot, news supplies the explicit
string argument, and invokem 0x02 performs the call.

Observed encoding: `0f 03` at CODE+`0x000020` in **minimal-console**,
Rez/<init>.

**Limits:** The count is not simply the number of explicit source arguments:
println has one but uses 2. Exact context/receiver binding, dispatch, variadic
calls, and absent return values remain unresolved.

## aputv

**Store an array element.** Opcode `0x11`; length **1 byte**. Operands: none.

Abstract stack: `[..., array, index, value] → [...]`.

Stores value at index in array and consumes all three values. getInitialView
uses it to fill the one-element array containing the new view.

Observed encoding: `11` at CODE+`0x000069` in **minimal-console**,
MinimalConsoleApp/getInitialView.

**Limits:** Bounds checks, index conversion, array kinds, and error behavior
remain unverified.

## lgetv

**Load a local-frame slot.** Opcode `0x12`; length **2 bytes**. Operands:
`index: u8` — local-frame slot.

Abstract stack: `[...] → [..., local[index]]`.

The u8 operand selects a slot in the current local frame. Loading it does not
consume an evaluation-stack input. The reference view reads slot 2 during
coordinate calculations.

Observed encoding: `12 02` at CODE+`0x000092` in **hello-world**,
HelloWorldView/onUpdate.

**Limits:** The local frame includes context/argument/local storage; its
physical layout is unknown. Do not assume slot 0 is always the application
object.

## lputv

**Store a local-frame slot.** Opcode `0x13`; length **2 bytes**. Operands:
`index: u8` — local-frame slot.

Abstract stack: `[..., value] → [...]`.

Stores the top value into the local-frame slot selected by the u8 operand.
hello() resolves globals and stores it in slot 0 with lputv 0x00.

Observed encoding: `13 00` at CODE+`0x00002c` in **minimal-console**, /hello.

**Limits:** The observed slot-0 write constrains simplistic “local 0 is always
this” descriptions. Frame ownership and lifetime remain unresolved.

## newa

**Create an array.** Opcode `0x14`; length **1 byte**. Operands: none.

Abstract stack: `[..., length] → [..., array]`.

Consumes a length and produces an array reference. getInitialView pushes 1,
allocates an array, fills index 0, and returns it.

Observed encoding: `14` at CODE+`0x00004f` in **minimal-console**,
MinimalConsoleApp/getInitialView.

**Limits:** Initial element values, allowable sizes, allocation failures, and
length conversion remain unverified.

## newc

**Create a class instance.** Opcode `0x15`; length **1 byte**. Operands: none.

Abstract stack: `[..., class] → [..., instance]`.

Consumes a class reference and produces an instance. The generated view
construction subsequently retrieves and invokes initialize explicitly. The
optimizer treats allocation as potentially affecting other state.

Observed encoding: `15` at CODE+`0x00005c` in **minimal-console**,
MinimalConsoleApp/getInitialView.

**Limits:** The optimizer attributes generated <init> effects to newc. This does
not establish that newc itself calls the public initialize method; allocation,
generated initialization, and the explicit method call must be distinguished.

## return

**Return from a function.** Opcode `0x16`; length **1 byte**. Operands: none.

Abstract stack: `optimizer model: [..., result] → caller`.

Ends the current function. The optimizer accounts for one consumed return value;
Hello World’s probe explicitly pushes 41 before returning. A caller’s call
instruction is modeled as producing the result.

Observed encoding: `16` at CODE+`0x000002` in **minimal-console**, a generated
stub without a debug function mapping.

**Limits:** Compiler-generated empty stubs contain argc followed by return
without an explicit value push. onStart can also discard a call result before
return. Hidden/default return storage and frame unwinding remain unresolved; the
abstract pop count is not a proven physical-stack rule.

## news

**Load a string from serialized data.** Opcode `0x18`; length **5 bytes**.
Operands: `address: u32be` — serialized string data address.

Abstract stack: `[...] → [..., string]`.

The big-endian u32 operand is a data address, not a symbol ID or a symbol-name
offset. hello() uses news 0x00003279. At data-section offset 0x3279 the bytes 01
00 06 48 65 6c 6c 6f 21 00 encode the observed string Hello!.

Observed encoding: `18 00 00 32 79` at CODE+`0x000038` in **minimal-console**,
/hello.

**Limits:** Initial specimens have a zero address prefix and a DATA-relative
offset. A [larger-offset simulator test][ref-larger-string-offset]
also loads a literal at DATA+`0x01000000` with a raised app memory allowance.
[String records and pinned assembly](data-strings.md) currently impose a narrower
authoring profile. General address-space rules, runtime allocation versus
interning, and other string representations remain unresolved.

## spush

**Push a Symbol value.** Opcode `0x27`; length **5 bytes**. Operands:
`id: u32be` — symbol ID.

Abstract stack: `[...] → [..., Symbol(id)]`.

Pushes a Symbol value identified by its big-endian u32 ID. It does not itself
load the named field. hello() pushes the globals symbol, then getm resolves the
module.

Observed encoding: `27 00 80 00 a1` at CODE+`0x00001b` in **minimal-console**,
Rez/<init>.

**Limits:** An ID is neither a table index nor a byte offset to its printable
name. Symbol identity and table layout are documented separately.

## frpush

**Prepare a function-call context value.** Opcode `0x2a`; length **1 byte**.
Operands: none.

Abstract stack: `[...] → [..., context]`.

Adds one value without consuming existing evaluation-stack values. Source
documentation describes a function reference/context; hello() places it between
the println callable and the explicit string argument.

Observed encoding: `2a` at CODE+`0x000019` in **minimal-console**, Rez/<init>.

**Limits:** The exact value, whether it carries receiver binding, and its
relationship to the current function/frame are not independently established.
“context” is an explanatory placeholder, not a specified runtime type.

## dup

**Duplicate a value at stack depth.** Opcode `0x2e`; length **2 bytes**.
Operands: `depth: u8` — zero-based stack depth.

Abstract stack:
`[..., value_at_depth, …] → [..., value_at_depth, …, value_at_depth]`.

The u8 operand is a zero-based depth from the top: dup 0x00 duplicates the top;
dup 0x01 duplicates the next value down. getInitialView duplicates its array so
it can populate an element while retaining the array for return.

Observed encoding: `2e 00` at CODE+`0x000050` in **minimal-console**,
MinimalConsoleApp/getInitialView.

**Limits:** The stack model adds one value without removing the original. For
reference values this means another reference, not a deep copy; object
representation and invalid-depth errors remain unverified.

## getm

**Resolve a module Symbol.** Opcode `0x30`; length **1 byte**. Operands: none.

Abstract stack: `[..., symbol] → [..., module]`.

Consumes a Symbol and produces the corresponding module value. hello() uses
spush 0x008002e6 followed by getm to obtain globals.

Observed encoding: `30` at CODE+`0x00002b` in **minimal-console**, /hello.

**Limits:** Resolution of unknown IDs, namespace rules, and module
initialization timing are not characterized.

## argc

**Set up the function argument frame.** Opcode `0x35`; length **2 bytes**.
Operands: `count: u8` — argument/context-slot count.

Abstract stack: `logical evaluation stack unchanged`.

The u8 operand describes argument/context slots at function entry. hello()
begins with argc 0x01 even though it has no explicit source parameters;
onStart(state) uses argc 0x02. This supports a hidden context slot in the count.

Observed encoding: `35 01` at CODE+`0x000000` in **minimal-console**, a
generated stub without a debug function mapping.

**Limits:** The source-derived prologue role is clear, but exact
stack/frame-pointer changes, validation of passed counts, and frame-slot mapping
remain unresolved. Logical pop 0/push 0 does not mean no physical frame changes.

## ipushz

**Push integer zero.** Opcode `0x37`; length **1 byte**. Operands: none.

Abstract stack: `[...] → [..., 0]`.

Pushes an integer zero with no operand bytes. getInitialView uses it as the
array index when populating its result.

Observed encoding: `37` at CODE+`0x000052` in **minimal-console**,
MinimalConsoleApp/getInitialView.

**Limits:** This is an integer constant instruction; Boolean false and null are
separate concepts. Exact runtime number representation is not established.

## ipush1

**Push a one-byte integer constant.** Opcode `0x38`; length **2 bytes**.
Operands: `bits: u8` — integer immediate.

Abstract stack: `[...] → [..., integer]`.

Carries a one-byte immediate. The research decoder reads it as an unsigned u8.
HelloWorldApp/probe uses ipush1 0x29 to produce the experimentally observed 41;
getInitialView uses 0x01 for its array length.

Observed encoding: `38 01` at CODE+`0x00004d` in **minimal-console**,
MinimalConsoleApp/getInitialView.

**Limits:** The encoding is preserved as raw bits in our Rust model. Values at
0x80 and 0xff, extension rules, and negative-number encodings have not been
independently tested; do not silently interpret the byte as signed i8.

## getselfv

**Load a member of the current receiver.** Opcode `0x43`; length **5 bytes**.
Operands: `member_id: u32be` — symbol ID.

Abstract stack: `[...] → [..., self.member(id)]`.

The big-endian u32 operand is a member symbol ID. Loads a member using the
implicit current receiver/context, consuming no evaluation-stack input. onStart
selects hello with getselfv 0x00000080 before invokemz.

Observed encoding: `43 00 80 02 9c` at CODE+`0x000004` in **minimal-console**,
MinimalConsoleApp/<init>.

Current-instance method selection is also
[SDK-observed and simulator-verified](../investigations/compiler-class.md),
including nested positional calls.

**Limits:** Global-function resolution through the current context is observed
in compiled code. Receiver representation, general inherited lookup, and
missing-member behavior remain unverified.

## getself

**Load the current receiver/context.** Opcode `0x44`; length **1 byte**.
Operands: none.

Abstract stack: `[...] → [..., self]`.

Pushes the current receiver/context without an operand. It occurs in generated
Rez initialization before another function is invoked.

Observed encoding: `44` at CODE+`0x00001a` in **minimal-console**, Rez/<init>.

**Limits:** “self” denotes the current execution context, not necessarily the
application object. Exact frame binding is still unknown.

## getmv

**Load a member of a named module.** Opcode `0x45`; length **9 bytes**.
Operands: `module_id: u32be`, `member_id: u32be` — symbol IDs.

Abstract stack: `[...] → [..., module(module_id).member(member_id)]`.

Two big-endian u32 symbol IDs select a module and then its member. The
instruction pushes the member value without consuming prior stack inputs or
invoking it. hello() uses getmv 0x00800005 0x00800067 to load
Toybox_System.println.

Observed encoding: `45 00 80 00 15 00 80 03 11` at CODE+`0x000010` in
**minimal-console**, Rez/<init>.

**Limits:** A loaded member may be a callable or another field value. Method
binding and resolution failures remain unverified. Both operands are IDs, not
string bytes or symbol-name offsets.

## getlocalv

**Load a member through a local-frame slot.** Opcode `0x46`; length **6 bytes**.
Operands: `index: u8`, `member_id: u32be` — local-frame slot and symbol ID.

Abstract stack: `[...] → [..., local[index].member(id)]`.

Combines a u8 local-frame index with a big-endian u32 member symbol ID. The
optimizer models a direct load with no evaluation-stack input. onUpdate uses
getlocalv 0x01 0x0080020b while accessing its drawing context.

Observed encoding: `46 01 00 80 02 0b` at CODE+`0x00004f` in **hello-world**,
HelloWorldView/onUpdate.

**Limits:** This combines local lookup and member lookup; it does not consume a
receiver from the evaluation stack. Frame mapping and lookup failures remain
unverified.

## getsv

**Load a member of a stacked receiver.** Opcode `0x47`; length **5 bytes**.
Operands: `member_id: u32be` — symbol ID.

Abstract stack: `[..., receiver] → [..., receiver.member(id)]`.

The big-endian u32 operand is a member symbol ID. Consumes the receiver from the
stack and replaces it with the member value. It occurs in generated application
initialization.

Observed encoding: `47 00 80 00 18` at CODE+`0x000009` in **minimal-console**,
MinimalConsoleApp/<init>.

**Limits:** Unlike getselfv, this instruction takes an explicit stacked
receiver. Inheritance, callable binding, and invalid receiver handling remain
unverified.

## invokemz

**Invoke a callable with no explicit arguments.** Opcode `0x48`; length **1
byte**. Operands: none.

Abstract stack: `[..., callable] → [..., result]`.

The optimizer models consumption of the callable and production of one result.
onStart loads hello and invokes it with invokemz. The plugin describes this as
the compact equivalent of frpush followed by invokem 0x01.

Observed encoding: `48` at CODE+`0x00000e` in **minimal-console**,
MinimalConsoleApp/<init>.

**Limits:** That expansion is source-derived, not independently proven for every
callable. Context selection and default/void return conventions remain
unresolved; the z does not imply an absence of a hidden context.

## argcincsp

**Set up arguments and reserve local-frame space.** Opcode `0x4a`; length **3
bytes**. Operands: `count: u8`, `locals: u8` — argument/context slots and
additional local space.

Abstract stack: `logical evaluation stack unchanged`.

Two u8 operands describe argument/context-slot count and additional local space,
respectively. It is documented as a combined argc and incsp prologue. onUpdate
begins with argcincsp 0x02 0x02, consistent with its drawing-context parameter
plus hidden context and two source locals.

Observed encoding: `4a 02 02` at CODE+`0x00004c` in **hello-world**,
HelloWorldView/onUpdate.

**Limits:** The exact physical stack-pointer/frame-pointer changes and storage
unit are unverified. These operands are counts, not symbol IDs or code
addresses.


## goto

**Jump unconditionally.** Opcode `0x19`; length **3 bytes**. Operand:
signed i16 big-endian displacement from the end of the instruction. The raw
operand preserves its two-byte bit pattern; `code:label` resolves during
assembly. Evaluation-stack effect is source-derived as unchanged.
Observed `19 ff e8` at CODE+`0xa1` targets the loop test at `0x8c`.
Changing the displacement to zero runs the body once, returning `2`.

## bt

**Branch if true.** Opcode `0x28`; length **3 bytes**. Operand: the same signed
relative displacement as `goto`. Source-derived effect consumes one condition
value. Replacing `bf` at CODE+`0xb5` with `bt`, retaining `0006`, swaps the
observed return values for `choose(0)` and `choose(1)`. General truth conversion
for non-Boolean values remains unverified.

## bf

**Branch if false.** Opcode `0x29`; length **3 bytes**. Operand: signed relative
displacement as `goto`. Source-derived effect consumes one condition value.
Observed `29 00 06` at CODE+`0xb5` jumps to `0xbe` when `value == 0` is false;
otherwise execution falls through. General truth conversion is unverified.

## eq

**Compare equality.** Opcode `0x1a`; length **1 byte**; no operands.
Source-derived stack effect: `[..., left, right] → [..., Boolean(left ==
right)]`. Observed at CODE+`0xb4`, after an integer parameter and zero are
loaded. Both comparison outcomes execute in the simulator. Object/string
equality and mixed-type behavior remain unverified.

## gt

**Compare greater-than.** Opcode `0x1d`; length **1 byte**; no operands.
Source-derived stack effect: `[..., left, right] → [..., Boolean(left >
right)]`. Observed at CODE+`0x8f`, testing the countdown parameter against zero.
The loop executes three times and exits at zero. Other operand types remain
unverified.

## jsr

**Call a local subroutine.** Opcode `0x23`; length **3 bytes**. Operand: signed
relative displacement as `goto`. Observed `23 00 1d` at CODE+`0x30` targets
`ret` at `0x50` in the try/catch function. Source-derived behavior transfers
control and retains a continuation; its physical storage and stack/frame
interaction are unresolved. This is distinct from `invokem` and `invokemz`.

## ret

**Return from a local subroutine.** Opcode `0x17`; length **1 byte**; no
operands. Observed at CODE+`0x50`, shared by several `jsr` sites. Source-derived
behavior resumes their continuation, rather than returning the containing
function. Exact continuation storage and invalid-call behavior remain
unresolved.

## throw

**Raise an exception.** Opcode `0x33`; length **1 byte**; no operands.
Source-derived effect consumes the thrown value and transfers control through
exception handling. Observed at CODE+`0x2a` after constructing `Lang.Exception`;
the selected handler returns `43`. CODE+`0x4f` also contains a rethrow path.
Handler stack setup, unwinding, and uncaught behavior remain unresolved.

## btpush

**Push Boolean true.** Opcode `0x3e`; length **1 byte**; no operands.
Source-derived stack effect: `[...] → [..., true]`. Observed at CODE+`0x206`
as the argument to `caught(true)`, which takes the explicit throw path.
Physical Boolean representation is unverified.

## bfpush

**Push Boolean false.** Opcode `0x3f`; length **1 byte**; no operands.
Source-derived stack effect: `[...] → [..., false]`. Observed at CODE+`0x1f0`
as the argument to `caught(false)`, which returns without throwing.
Physical Boolean representation is unverified.

## npush

**Push null.** Opcode `0x2c`; length **1 byte**; no operands.
Stack effect: `[...] → [..., null]`. SDK 9.2.0 / fr265 / `-O 0` emits it for
`var missing = null`. Our generated startup callback executes it with local
storage and equality in the simulator. Null's physical representation remains
unknown; it is distinct from an integer zero in semantic IR. See the
[startup investigation](../investigations/compiler-class.md#source-startup-and-nullable-state).

## Runtime evidence and limits

Encoding and stack descriptions above apply to the supported observations.
Physical frame layout, larger local/argument counts, overflow, and general type
promotion remain incompletely characterized. Do not infer universal VM limits
from the small bodies used for validation.

Procedures and runtime results belong in the investigations:
[function replacement](../investigations/function-replacement.md),
[arithmetic](../investigations/compiler-arithmetic.md),
[locals](../investigations/compiler-locals.md),
[calls](../investigations/compiler-calls.md), and
[arguments](../investigations/compiler-arguments.md).

[ref-larger-string-offset]: ../investigations/string-address-boundary.md
