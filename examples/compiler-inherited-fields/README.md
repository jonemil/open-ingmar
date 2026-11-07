# Inherited instance fields

[app.mc](app.mc) covers multilevel field lookup, parent-before-child allocation
expressions, literal defaults, protected mutation, inherited constants, and
independent parent/child private fields. Ordinary parent initializers are empty.
Compare [expected.txt](expected.txt).

```sh
mkdir -p scratch/compiler-inherited-fields
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 3.0.0 \
  --app-id b67c1691c9a5422a9d3202a0c016870c --entry MinimalConsoleApp.onStart \
  examples/compiler-inherited-fields/app.mc scratch/compiler-inherited-fields/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-inherited-fields/unsigned.prg \
  scratch/compiler-inherited-fields/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-inherited-fields/generated.prg
```

SDK reference: 9.2.0, `fr265`, minimum API 3.0.0, `-O 0`.
Use the [minimal console scaffold](../minimal-console/README.md) with this source.
