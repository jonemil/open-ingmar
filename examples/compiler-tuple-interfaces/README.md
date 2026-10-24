# Tuple interface witnesses

[app.mc](app.mc) checks source fields/methods and core interfaces in tuple slots,
including inferred tuples, nullable slots, nested containers, source calls,
callback invocation, and mutable interface fields.
Compare [expected output](expected.txt).

From the repository root, using fresh output paths:

```sh
mkdir -p scratch/compiler-tuple-interfaces
cargo run -p ciq-compiler -- --unit --emit prg --target-api 5.2.0 --min-api 5.0.0 --app-id 2bf86c6ec47c44089467296308bf1285 --entry MinimalConsoleApp.onStart examples/compiler-tuple-interfaces/app.mc scratch/compiler-tuple-interfaces/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-tuple-interfaces/unsigned.prg scratch/compiler-tuple-interfaces/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-tuple-interfaces/generated.prg
```

Reference: SDK 9.2.0, strict checking, `fr265`, minimum API 5.0.0 for Comparator.
See [tuple contracts](../../docs/specs/frontend/README.md#positional-tuples).
