# Mutable static fields

[app.mc](app.mc) checks first-write initialization order, captured compound
operands, shared instance/class access, branching writes, dynamic and nullable
fields, reference release, and nested-module lookup. Compare
[expected.txt](expected.txt).

```sh
mkdir -p scratch/compiler-static-fields
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 3.0.0 \
  --app-id b67c1691c9a5422a9d3202a0c016870c --entry MinimalConsoleApp.onStart \
  examples/compiler-static-fields/app.mc scratch/compiler-static-fields/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-static-fields/unsigned.prg \
  scratch/compiler-static-fields/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-static-fields/generated.prg
```

SDK reference: 9.2.0, `fr265`, minimum API 3.0.0, `-O 0`.
Use the [minimal console scaffold](../minimal-console/README.md) with this source.
