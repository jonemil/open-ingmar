# Provisional compiler IR text, versions 1 and 2

Implemented 2026-10-10 to exercise the compiler stage boundaries while the
language subset is small. This is our semantic IR format, separate from Garmin
MIR and VM assembly. Syntax/API can change as operations are introduced; it is
not a stable interchange contract. Evidence:
[pipeline experiment](../investigations/compiler-pipeline.md),
[arithmetic extension](../investigations/compiler-arithmetic.md),
[local declarations](../investigations/compiler-locals.md),
[module calls](../investigations/compiler-calls.md),
[parameters/arguments](../investigations/compiler-arguments.md), and
[strings/printing](../investigations/compiler-strings.md).

```text
ciq-ir 1
.function probe
.integer %0 41 span=120:122
.return %0 span=113:123
.end
```

One nonempty directive per line. Blank lines and `#` comments are accepted.
Header version must be `1` or `2`. Version 1 describes a single-block body;
version 2 adds the control-flow directives below. Exactly one `.function <identifier>
[parameters=<count>]` and trailing `.end` are required; input after `.end` is
rejected. Identifiers use the assembly label identifier rules. The supported
semantic body is ordered local initializations, print statements, and expression
trees (which may include module calls) ending with a final integer `.return
<value-id>`. Definitions are `.integer <result-id> <value>`, `.add <result-id>
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
multiple returns, and blocks are rejected in version 1. For pure-only functions,
definition order need not match VM stack order: the backend walks dependencies
for each initialization/return in statement order and emits each binary left operand
before its right operand. `parse_ir` reports positions in the IR input; typed
validation reports the offending operation index when available. Source
compilation also constructs its IR through this validator.

`Function::to_ir_text()` prints the semantic operations and optional spans,
preserving value IDs. It emits version 2 when blocks or comparisons are present,
otherwise version 1, even if the input header was version 2. `parse_ir`
reconstructs them. `backend::lower()` (also
exposed through `Function::lower()`) creates the shared assembly `Program`
directly; it does not print/reparse text. `Function::assembly()` is convenience
lowering plus assembly printing. This gives a native source → IR → assembly
model → CODE encoder path and text stop/resume points at both IR and assembly.

The arithmetic directives extend provisional version 1; existing literal-only
input remains accepted. For example:

```text
ciq-ir 1
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
Call-containing functions add the ordering restriction below; reassignment still
requires a revised policy.

```text
ciq-ir 1
.function probe
.integer %10 42
.local $99 %10
.read %20 $99
.read %30 $99
.subtract %40 %20 %30
.return %40
.end
```

These are additive version-1 extensions; literal/arithmetic input stays
accepted.

## Module calls and operation order

`.call %<result-id> <module> <member> [%<argument-id> ...]` defines the integer
result of a positional-argument call to an externally supplied top-level user
module. Names use ASCII identifier rules and remain symbolic. The optional span
has the same meaning as for other operations. The module's expected stored
symbol name is `globals/<module>` and the member's is `<member>`; this is not
numeric symbol allocation or method metadata generation.

Calls are effectful. If a function contains any `.call` or `.println`, all its
operations must match the backend's left-to-right expression traversal and
ordered initialization/print/return sinks. The shared validator rejects
incompatible schedules, including moving a call across an initialization or
reversing call operands. Pure-only functions retain the earlier definition-order
flexibility. This conservative policy keeps runtime/IR operation order
consistent until a general scheduler is implemented.

```text
ciq-ir 1
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
environment yet: the scaffold must supply integer-return callees. Object
calls and global unqualified calls remain unsupported. Argument counts must
match the supplied callee signature; whole-program arity checking is not
implemented.

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
ciq-ir 1
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
local initialization/reads; arithmetic, parameters, user-call arguments/results,
and returns remain integer-only. All existing use/dependency/order rules apply.

```text
ciq-ir 1
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

## Version 2 control flow

Version 2 accepts all version-1 directives and additionally:

```text
ciq-ir 2
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
the next block. All targets must be declared later in operation order. Every
block must be reachable from entry; backward edges, missing targets, duplicate
blocks, and implicit fallthrough are rejected. The final block returns.
A version-2 body without these operations retains the implicit single block.

Expression definitions still precede their uses, are globally unique, and have
exactly one use, now restricted to their defining block. Locals can cross edges,
but a read requires its unique initializer to execute on every incoming path;
at a join the available set is the intersection of predecessor sets. A local
initialized in only one arm cannot be read in a sibling or join. No reassignment
or cross-block expression/phi values are supported. Local slots are allocated
across the entire function, including mutually exclusive paths.

The 1024-operation limit includes block markers and terminators. Effectful
functions retain left-to-right expression and statement order within each
block; block markers and jumps participate in schedule validation. Branches
consume their comparisons, and no evaluation-stack values are carried over an
edge. Source parsing, typed construction, and text resumption use these same
checks. Typed/text IR supports forward joins with `.jump`; the source subset
currently only generates a terminal conditional with one return per arm.
