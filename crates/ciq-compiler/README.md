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

Start with `compile`, `compile_sources`, `Function`, and `CompilationUnit` in
the [library API](src/lib.rs). See [compiler IR specification](../../docs/specs/ir/README.md#semantic-model),
[source/CLI rules](../../docs/specs/frontend/README.md),
[unit rules](../../docs/specs/frontend/README.md#declarations-and-name-resolution), and
[IR syntax](../../docs/specs/ir/grammar.md).
