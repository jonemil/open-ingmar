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

Start with `compile`, `compile_sources`, `Function`, and `CompilationUnit` in
the [library API](src/lib.rs). See [compiler design](../../docs/design/compiler-ir.md),
[source/CLI rules](../../docs/specs/compiler-subset.md),
[unit rules](../../docs/specs/compiler-unit.md), and
[IR syntax](../../docs/specs/compiler-ir-text.md).
