# Wider numeric Dictionaries

[app.mc](app.mc) checks Long/Float/Double keys and values, nullable values,
constant/computed construction, empty dictionaries, duplicate keys, mutation,
collections, conversion, identity, and operand order. Floating signed-zero keys
remain distinct. SDK/generated `fr265` programs match
[expected output](expected.txt).

From the repository root, using fresh output paths:

```sh
mkdir -p scratch/compiler-numeric-dictionaries
cargo run -p ciq-compiler -- --unit --emit prg --target-api 5.2.0 --min-api 1.3.0 --app-id 4cb174d4aa1e44d7b0f7f70c39274316 --entry MinimalConsoleApp.onStart examples/compiler-numeric-dictionaries/app.mc scratch/compiler-numeric-dictionaries/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-numeric-dictionaries/unsigned.prg scratch/compiler-numeric-dictionaries/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-numeric-dictionaries/generated.prg
```

Dictionary members require API 1.0.0, except `toString` since 1.0.1; the example's
Array `indexOf` checks require 1.3.0. Nested/heterogeneous values and Null keys
remain deferred. See the
[source contracts](../../docs/specs/compiler-subset.md#dictionary-values) and
[separate NaN probe](../../docs/investigations/compiler-numeric-dictionaries.md).
