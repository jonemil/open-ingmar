# Static constants

[app.mc](app.mc) checks readonly class storage, lazy once-only initialization,
conditional initializers, instance access, mutable contents of a constant Array,
and a retained Method. Reading a class value alone does not activate statics.
Compare [expected.txt](expected.txt).

```sh
mkdir -p scratch/compiler-static-constants
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 3.0.0 \
  --app-id b67c1691c9a5422a9d3202a0c016870c --entry MinimalConsoleApp.onStart \
  examples/compiler-static-constants/app.mc scratch/compiler-static-constants/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-static-constants/unsigned.prg \
  scratch/compiler-static-constants/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-static-constants/generated.prg
```

SDK reference: 9.2.0, `fr265`, minimum API 3.0.0, `-O 0`.
Use the [minimal console scaffold](../minimal-console/README.md) with this source.
