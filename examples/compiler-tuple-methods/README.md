# Tuple copy, query, sorting, and Object methods

[app.mc](app.mc) checks reversed and sliced slot contracts, independent copies,
negative/Null/clipped bounds, dynamic bounds, source/nested slots, search,
default/custom sorting, typed method binding, weak handles, and effect order.
Dynamic slices retain the SDK's erased Array element contract.

From the repository root, using fresh output paths:

```sh
mkdir -p scratch/compiler-tuple-methods
cargo run -p ciq-compiler -- --unit --emit prg --target-api 5.2.0 --min-api 5.0.0 --app-id 96a7320a6f22400baf5abfd341875081 --entry MinimalConsoleApp.onStart examples/compiler-tuple-methods/app.mc scratch/compiler-tuple-methods/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-tuple-methods/unsigned.prg scratch/compiler-tuple-methods/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-tuple-methods/generated.prg
```

Compare the full console with [expected.txt](expected.txt). Reference environment:
SDK 9.2.0, fr265, minimum API 5.0.0 and target 5.2.0. See
[tuple contracts](../../docs/specs/frontend/README.md#positional-tuples).
