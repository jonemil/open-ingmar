# Provisional compiler IR text, version 1

Implemented 2026-10-10 to exercise the compiler stage boundaries while the language subset is small. This is our semantic IR format, separate from Garmin MIR and VM assembly. Syntax/API can change as operations are introduced; it is not a stable interchange contract. Evidence: [pipeline experiment](../investigations/compiler-pipeline.md) and [arithmetic extension](../investigations/compiler-arithmetic.md).

```text
ciq-ir 1
.function probe
.integer %0 41 span=120:122
.return %0 span=113:123
.end
```

One nonempty directive per line. Blank lines and `#` comments are accepted. Header version must be exactly `1`. Exactly one `.function <identifier>` and trailing `.end` are required; input after `.end` is rejected. Identifiers use the assembly label identifier rules. The supported semantic body is one pure integer expression tree followed by a final `.return <value-id>`. Definitions are `.integer <result-id> <value>`, `.add <result-id> <left-id> <right-id>`, and `.subtract <result-id> <left-id> <right-id>`. Value IDs are `%` plus decimal `u32`; any such ID is supported, not just `%0`. Integer values are nonnegative decimal `0..=127`, matching the backend's currently verified literal encoding subset; computed results are not restricted to that range.

Each operation optionally has `span=<start>:<end>` with decimal `usize` byte offsets and `start <= end`. These offsets refer to the original source, not positions in the IR text. Spans are optional metadata; they do not affect generated instructions. The current format does not record a filename or embed the original source, and cannot verify annotations against that source. A text edit may require updating or omitting its span. This is not generated PRG debug information.

The text parser and typed `Function::new(name, operations)` use the same semantic validator. It requires unique definitions, operands referring to earlier definitions, exactly one use of every defined value, and a final return. The function is limited to 1024 operations including return. Invalid names/spans, out-of-range literals, shared values, dead definitions, forward/undefined uses, multiple returns, and blocks are rejected. Definition order need not match VM stack order: the backend walks dependencies from the returned value and emits each binary left operand before its right operand. `parse_ir` reports positions in the IR input; typed validation reports the offending operation index when available. Source compilation also constructs its IR through this validator.

`Function::to_ir_text()` prints the semantic operations and optional spans, preserving value IDs. `parse_ir` reconstructs them. `backend::lower()` (also exposed through `Function::lower()`) creates the shared assembly `Program` directly; it does not print/reparse text. `Function::assembly()` is convenience lowering plus assembly printing. This gives a native source → IR → assembly model → CODE encoder path and text stop/resume points at both IR and assembly.

The arithmetic directives extend provisional version 1; existing literal-only input remains accepted. For example:

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

Defaults: `--from source`, `--emit assembly`; the old two-path command still works. `--from` accepts `source`, `ir`, or `assembly`; `--emit` accepts `ir`, `assembly`, or `code`. Source/IR can emit all three. Assembly can emit canonical assembly or CODE, but cannot be raised to semantic IR. Duplicate options/unknown stages are errors. Validate and encode before creating a new output file; existing files are never overwritten.

`--emit code` emits an ordinary CODE payload, **not a complete PRG or executable**. PRG metadata/resources and signing still use the explicitly documented scaffold workflow. This CLI cannot yet supply external symbol/string bindings; self-contained assembly declarations are supported, and library callers can provide external bindings to the shared assembly encoder.
