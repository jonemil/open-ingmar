# ciq-compiler

Library and CLI for the restricted Monkey C frontend, validated semantic IR,
VM lowering, and complete unsigned console application generation.

From the workspace root (input paths are placeholders):

```sh
cargo run -p ciq-compiler -- input.mc scratch/output.mcasm
cargo run -p ciq-compiler -- --emit ir input.mc scratch/function.ciqir
cargo run -p ciq-compiler -- --from ir scratch/function.ciqir scratch/resumed.mcasm
```

Output files must be new. `--unit` collects multiple source files; complete PRG
output also requires `--emit prg` and `--app-id`. See the
[class application example](../../examples/compiler-class/README.md) for
compiling, signing, and running a complete application.

`api::core()` exposes a bounded catalogue of core Lang types and native
contracts with inherited lookup and introduction-level checks. It does not
enable arbitrary API calls or perform complete target availability validation.
See [API declarations](../../docs/specs/sdk/README.md#api-declarations).

Supply `--target-api 1.0.0 --min-api 1.0.0` for explicit unguarded API-level
checks on source or IR inputs. Library entry points with `_with_api` accept an
`api::ApiContext`; typed models expose `validate_api`. See
[input rules](../../docs/specs/sdk/README.md#api-context-inputs).

Start with `compile_sources` and `CompilationUnit` in the [library API](src/lib.rs)
for one or more source files with checked user calls. `compile_function` and
`parse_function_ir` handle isolated `Function` fragments with externally bound
module calls; they do
not check unit ownership or callee signatures. Both source entries share the
declaration parser. See [compiler IR specification](../../docs/specs/ir/README.md#model-and-validation),
[source/CLI rules](../../docs/specs/frontend/README.md),
[unit rules](../../docs/specs/frontend/README.md#declarations-and-name-resolution), and
[IR syntax](../../docs/specs/ir/grammar.md).
