# Nullable Array elements

[app.mc](app.mc) checks explicit Number-or-Null and String-or-Null element
contracts, literal/computed construction, guarded reads, Null writes, native
mutation/search/transforms, shared references, fresh constant loads, and empty
or Null-only contextual literals. Markers check conditional element effects.
SDK and generated `fr265` programs match [expected output](expected.txt).

From the repository root, using fresh output paths:

```sh
mkdir -p scratch/compiler-nullable-arrays
cargo run -p ciq-compiler -- --unit --emit prg --target-api 5.2.0 --min-api 1.3.0 --app-id 639aef856dca4c458cf6879ea717883e --entry MinimalConsoleApp.onStart examples/compiler-nullable-arrays/app.mc scratch/compiler-nullable-arrays/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-nullable-arrays/unsigned.prg scratch/compiler-nullable-arrays/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-nullable-arrays/generated.prg
```

See the [source contracts](../../docs/specs/compiler-subset.md#nullable-array-elements).
