# Compiler CLI and saved stages

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

Defaults: `--from source`, `--emit assembly`. `--from` accepts `source`, `ir`,
or `assembly`; `--emit` accepts `ir`, `assembly`, `code`, or `prg`. PRG requires
unit mode and the
[application inputs](application.md). Source/IR can emit IR, assembly, or CODE. Assembly can emit canonical
assembly or CODE, but cannot be raised to semantic IR. Duplicate options/unknown
stages are errors. Validate and encode before creating a new output file;
existing files are never overwritten.

`--emit code` emits an ordinary CODE payload, **not a complete PRG or
executable**. Use unit-mode `--emit prg` for an independently generated complete
application; signing follows generation. For CODE emission, `--symbols
<symbol-source.mcasm>` supplies a metadata-only `.symbol` file, including an
exported PRG symbol section. Its declared aliases resolve ordinary assembly
references. Compiler aliases `__ciq_module_<name>` / `__ciq_member_<name>`
resolve by unique stored names `globals/<name>` / `<name>`, independently of the
file's aliases. Missing/ambiguous names, duplicate aliases/IDs, conflicting
generated bindings, and non-symbol items are errors before output creation.
Literal aliases `__ciq_symbol_<name>` likewise resolve by the exact stored name.
Native aliases `__ciq_native_<name>` follow that same short-name rule; method
authorization belongs to IR validation rather than a second binding whitelist.
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

## CLI

`--unit` selects compilation-unit source or IR mode. With source input it
accepts multiple input paths, with the final path always the new output:

```sh
cargo run -p ciq-compiler -- --unit examples/compiler-unit/entry.mc examples/compiler-unit/numbers.mc scratch/unit.mcasm
cargo run -p ciq-compiler -- --unit --emit ir examples/compiler-unit/entry.mc examples/compiler-unit/numbers.mc scratch/unit.ciqir
cargo run -p ciq-compiler -- --unit --from ir scratch/unit.ciqir scratch/resumed.mcasm
```

`--from ir` and `--from assembly` require one input. Assembly is the existing
shared format, so `--unit` is unnecessary when resuming it. `--emit code`
accepts the existing explicit `--symbols`/`--data` bindings and emits raw CODE,
not a PRG. All outputs must be new files. Without `--unit`, the original
single-function CLI behavior is unchanged.

See [the example](../../../examples/compiler-unit/README.md) and
[simulator evidence](../../investigations/compiler-unit.md).

For complete unsigned application output, use `--emit prg --app-id <id>` with
source or saved unit IR. The [console application profile](application.md)
specifies the generated lifecycle, metadata, entry selection, and restrictions.

Unit signatures include explicit parameter types and the validated result
contract. Expression `.call` carries the expected supported value or nullable
result; unit validation requires it to match the callee. Statement
`.call-discard` permits any supported result, including Void, and discards it.
Both check argument types and exact arity. Inner functions permit [void-return
IR](../ir/calls.md#void-returns-and-statement-calls). See [void source
rules](../frontend/statements.md#void-functions-and-statement-calls).
