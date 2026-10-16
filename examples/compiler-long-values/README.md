# Long literals and value transport

[app.mc](app.mc) checks decimal Long literals with `l`/`L`, zero, signed
boundaries, values above 32 bits, local copies/reassignment, module/instance
arguments and results, discarded calls, repeated loads, and printing. SDK and
generated `fr265` programs match [expected output](expected.txt).

From the repository root, using fresh output paths:

```sh
mkdir -p scratch/compiler-long-values
cargo run -p ciq-compiler -- --unit --emit prg --target-api 5.2.0 --min-api 1.0.0 --app-id 7b9aef856dca4c458cf6879ea717883e --entry MinimalConsoleApp.onStart examples/compiler-long-values/app.mc scratch/compiler-long-values/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-long-values/unsigned.prg scratch/compiler-long-values/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-long-values/generated.prg
```

Arithmetic, conversions, native Long methods, nullability, and Long container
elements remain deferred. See the
[source contract](../../docs/specs/compiler-subset.md#long-values).
