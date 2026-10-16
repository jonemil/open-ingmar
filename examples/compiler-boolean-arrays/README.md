# Boolean arrays

[app.mc](app.mc) checks typed Boolean-array arguments/results, literal constants,
shared mutation, fresh loads, indexing in Boolean conditions, contextual empty
arrays, native methods, and ordered computed/conditional/short-circuit elements.
SDK and generated `fr265` programs match [expected output](expected.txt).

From the repository root, using fresh output paths:

```sh
mkdir -p scratch/compiler-boolean-arrays
cargo run -p ciq-compiler -- --unit --emit prg --target-api 5.2.0 --min-api 1.3.0 --app-id 539aef856dca4c458cf6879ea717883e --entry MinimalConsoleApp.onStart examples/compiler-boolean-arrays/app.mc scratch/compiler-boolean-arrays/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-boolean-arrays/unsigned.prg scratch/compiler-boolean-arrays/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-boolean-arrays/generated.prg
```

See the [source contracts](../../docs/specs/compiler-subset.md#boolean-arrays).
