# String arrays

[app.mc](app.mc) checks typed String-array parameters/results, pooled constants,
shared references, fresh repeated loads, Unicode/empty strings, indexed reads
and writes, contextual empty construction, mutation/search/transforms, guarded
nullable elements, and ordered computed/branching construction. Markers check
receiver/index/value order and reads before a later element mutates its source.
SDK and generated `fr265` programs match [expected output](expected.txt).

From the repository root, using fresh output paths:

```sh
mkdir -p scratch/compiler-string-arrays
cargo run -p ciq-compiler -- --unit --emit prg --target-api 5.2.0 --min-api 1.3.0 --app-id 739aef856dca4c458cf6879ea717883e --entry MinimalConsoleApp.onStart examples/compiler-string-arrays/app.mc scratch/compiler-string-arrays/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-string-arrays/unsigned.prg scratch/compiler-string-arrays/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-string-arrays/generated.prg
```

See the [source contracts](../../docs/specs/compiler-subset.md#string-arrays).
