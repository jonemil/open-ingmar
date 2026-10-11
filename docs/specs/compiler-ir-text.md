# Provisional compiler IR text

This semantic IR format supports inspection and resumption of compiler stages.
It is separate from Garmin MIR and VM assembly. Syntax/API can change as
operations are introduced; it is not a stable interchange contract. Evidence:
[pipeline experiment](../investigations/compiler-pipeline.md),
[arithmetic extension](../investigations/compiler-arithmetic.md),
[local declarations](../investigations/compiler-locals.md),
[module calls](../investigations/compiler-calls.md),
[parameters/arguments](../investigations/compiler-arguments.md), and
[strings/printing](../investigations/compiler-strings.md).

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
Definitions are `.integer <result-id> <value>`, `.add <result-id>
<left-id> <right-id>`, and `.subtract <result-id> <left-id> <right-id>`. Value
IDs are `%` plus decimal `u32`; any such ID is supported, not just `%0`. Integer
values are nonnegative decimal `0..=127`, matching the backend's currently
verified literal encoding subset; computed results are not restricted to that
range.

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

`.call %<result-id> <module> <member> [%<argument-id> ...]` defines the integer
result of a positional-argument call to an externally supplied top-level user
module. Names use ASCII identifier rules and remain symbolic. The optional span
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
environment in standalone mode: the scaffold must supply integer-return
callees. Unit mode checks names, arity, and integer/void result contracts.
Object calls and global unqualified calls remain unsupported.

## Parameter metadata and reads

The optional `parameters=<nonnegative-decimal-count>` field on `.function`
defaults to zero, with a maximum of 254. `Function::with_parameters(name, count,
operations)` uses the shared validator; `Function::new` retains zero-parameter
behavior. `.parameter %<result-id> <zero-based-index>` defines a fresh integer
value from a signature position. Index must be below the parameter count;
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
consumes an integer or string without defining a result. Both accept optional
spans. Quotes preserve spaces, `#`, and text resembling a span. Escapes follow
[assembly string syntax](symbol-assembly.md); commas outside quotes are rejected.
The semantic string limit is 65535 UTF-8 bytes. String types propagate through
local initialization/assignment/reads; arithmetic, parameters, user-call
arguments and expression-call results remain integer-only; valued returns
require integers. Void returns have no value. All existing use/dependency/
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
- `.equal %<result> %<left> %<right>` and `.greater` compare integers and define
  a Boolean result. Only `.branch` may consume it.
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
initializer's integer/string type; assignments must match it. Boolean values,
undefined values/locals, wrong-path targets, shared values, and cross-block RHS
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
Types retain their initializer's integer/string classification. Expression
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

`.call-discard` may target an integer-return or void function; the caller
discards its result. Argument IDs name earlier integer
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

Every return in a function must agree: `.return` yields an integer,
`.return-void` yields void. The shared validator infers and exposes that result
contract; no separate result annotation is stored. A block may terminate with
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

`.base-initialize Application.AppBase` invokes the supported native superclass
initializer on the current instance, with no explicit arguments or value result.
It accepts the usual optional source span. Other base names or additional fields
are rejected. Function validation requires an instance receiver and preserves
the call's effect order; unit validation additionally enforces the
[source initializer contract](compiler-classes.md#source-initialization).
Source, typed, and text paths use the same operation and VM lowering.

## Nullable callback state

Instance-function metadata may include
`parameter-types=integer,nullable-dictionary,...`. The list must match
`parameters=<count>`; omitted types are integers. Unsupported names, duplicate
type lists, mismatched counts, or lists without an instance receiver are errors.
Unit validation currently permits noninteger parameters only for the native
AppBase.onStart contract. For example:

```text
.function onStart parameters=1 receiver=instance parameter-types=nullable-dictionary
```

`.null %<result-id> [span=<start>:<end>]` defines null. `.equal` permits integer
pairs, null pairs, and nullable Dictionary/null pairs; `.greater` remains
integer-only. Callback-state reads/locals preserve their opaque nullable type.
Printing, arithmetic, integer returns, and user-call arguments reject null and
Dictionary values. A nullable Dictionary local may be assigned null; other
assignments preserve the initializer type. Lowering uses `npush` for null and
the existing local-slot and equality instructions.

## Instance receivers

The optional declaration field `receiver=instance` marks an instance method;
ordinary bodies omit it. It may
appear before or after `parameters=<count>`; duplicate or unknown fields are
rejected. Every instance method requires receiver metadata, even without self
calls. The receiver is the hidden context, not an explicit parameter/value ID.

Two additional directives require an instance method:

- `.call-self %<result> <member> [%<argument> ...] [span=<start>:<end>]`:
  define an integer result from a current-receiver member call.
- `.call-self-discard <member> [%<argument> ...] [span=<start>:<end>]`:
  invoke a current-receiver member for effects and discard its result.

Arguments remain integer, ordered, earlier-defined values in the same block.
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

## Native value calls

`.call-native %<result-id> %<receiver-id> <member> [%<argument-id> ...]` invokes a
runtime-provided method and defines its semantic result. The receiver and
arguments must be earlier values with the usual single-use/block restrictions.
Optional source spans use the ordinary syntax.

Current validation accepts only a Number receiver, member `toString`, and no
arguments. Resolve its inherited Object contract through the API catalogue;
its result is String. A typed/text producer may supply any validated Number
expression as receiver; the source frontend currently accepts named locals and
parameters only. Unsupported members, receivers, arity, and result contracts
are rejected before lowering.

Native calls are effectful: receiver evaluation precedes invocation, and their
operation order must agree with dependency traversal at statement sinks. The
backend emits receiver evaluation, `getsv @__ciq_native_toString`, and
`invokemz`. Binding requires exactly one stored symbol named `toString` in an
explicit symbol source, or the supported complete application's native profile.
IR contains no symbol ID or import index.

```text
ciq-ir
.function show parameters=1
.parameter %0 0
.call-native %1 %0 toString
.println %1
.return-void
.end
```
