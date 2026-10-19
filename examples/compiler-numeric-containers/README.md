# Numeric alias containers

[app.mc](app.mc) exercises Numeric, Integer, and Decimal Array elements and
Dictionary keys/values. It checks mixed numeric constants, computed literals,
empty/sized allocation, nullable elements/values, updates, native methods,
sorting, snapshots, and Dictionary enumeration. Equal-valued keys of different
numeric classes remain distinct.

API minimum: 5.0.0; target: 5.2.0; SDK/compiler/simulator: 9.2.0 / fr265.

From the repository root, using unused output paths:

```sh
mkdir -p scratch/compiler-numeric-containers
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 5.0.0 \
  --app-id 71d3fbe13b6d4a72bd84add390a9be1a --entry MinimalConsoleApp.onStart \
  examples/compiler-numeric-containers/app.mc scratch/compiler-numeric-containers/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-numeric-containers/unsigned.prg \
  scratch/compiler-numeric-containers/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh \
  --prg scratch/compiler-numeric-containers/generated.prg
```

Compare the complete console with [expected.txt](expected.txt). See the
[numeric contracts](../../docs/specs/compiler-subset.md#numeric-values) and
[evidence](../../docs/investigations/compiler-numeric-unions.md).
