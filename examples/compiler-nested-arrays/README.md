# Nested Array elements

[app.mc](app.mc) checks Array/Dictionary/tuple element contracts, recursive
literal context, shared inner references, independent outer reverse/slice copies,
mutation/search/removal, custom sorting, nullable elements, allocation, ordered
fills, and tuple shape loss. Compare [expected output](expected.txt).

From the repository root, using fresh output paths:

```sh
mkdir -p scratch/compiler-nested-arrays
cargo run -p ciq-compiler -- --unit --emit prg --target-api 5.2.0 --min-api 5.0.0 --app-id e3c9f5e3cf004c9b95ce32649b2a7c28 --entry MinimalConsoleApp.onStart examples/compiler-nested-arrays/app.mc scratch/compiler-nested-arrays/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-nested-arrays/unsigned.prg scratch/compiler-nested-arrays/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-nested-arrays/generated.prg
```

Reference: SDK 9.2.0, strict checking, `fr265`, minimum API 5.0.0 for Comparator.
See [nested element contracts](../../docs/specs/frontend/README.md#nested-array-elements).
