# Literal inference

[app.mc](app.mc) checks empty, homogeneous, computed, Symbol, and source-class
literal inference, evolving slots, assignment, loops, sorting, copies, and
ordinary typed Array consumers. Compare [expected output](expected.txt).
Nested tuple-to-Array matching and ByteArray bulk append exercise consumers of
the inferred contracts.

From the repository root, using fresh output paths:

```sh
mkdir -p scratch/compiler-literal-inference
cargo run -p ciq-compiler -- --unit --emit prg --target-api 5.2.0 --min-api 5.0.0 --app-id a143276ae46d4dffb075657591c6d403 --entry MinimalConsoleApp.onStart examples/compiler-literal-inference/app.mc scratch/compiler-literal-inference/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-literal-inference/unsigned.prg scratch/compiler-literal-inference/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-literal-inference/generated.prg
```

Reference: SDK 9.2.0, strict checking, `fr265`, minimum API 5.0.0 for sorting.
See [positional tuples](../../docs/specs/frontend/README.md#positional-tuples).
