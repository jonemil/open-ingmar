# Exact source-class Array elements

[app.mc](app.mc) checks typed source construction, indexed reads/writes,
append/search/removal, shallow copies, custom sorting, nullable source elements,
class unions, nested Arrays, and tuple shape loss. Compare
[expected output](expected.txt).

From the repository root, using fresh output paths:

```sh
mkdir -p scratch/compiler-source-arrays
cargo run -p ciq-compiler -- --unit --emit prg --target-api 5.2.0 --min-api 5.0.0 --app-id f46cb5d483fa4b1c96405e92c348f6da --entry MinimalConsoleApp.onStart examples/compiler-source-arrays/app.mc scratch/compiler-source-arrays/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-source-arrays/unsigned.prg scratch/compiler-source-arrays/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-source-arrays/generated.prg
```

Reference: SDK 9.2.0, strict checking, `fr265`, minimum API 5.0.0 for Comparator.
See [source element contracts](../../docs/specs/frontend/README.md#source-class-array-elements).
