# Instance field metadata defaults

[app.mc](app.mc) compares literal field metadata with allocation-time writes,
class-qualified reads, readonly String defaults, and inherited initialization.
A fresh instance retains its own defaults after another instance is changed.
[expected.txt](expected.txt) records the console output.

```sh
mkdir -p scratch/compiler-field-metadata
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 3.0.0 \
  --app-id b67c1691c9a5422a9d3202a0c016870c --entry MinimalConsoleApp.onStart \
  examples/compiler-field-metadata/app.mc scratch/compiler-field-metadata/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-field-metadata/unsigned.prg \
  scratch/compiler-field-metadata/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-field-metadata/generated.prg
```

SDK reference: 9.2.0, `fr265`, minimum API 3.0.0, `-O 0`.
Use the [minimal console scaffold](../minimal-console/README.md) with this source.
