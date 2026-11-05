# Class-member visibility

[app.mc](app.mc) exercises public/default, protected/hidden, and private instance
and static members. Private members have class-specific runtime selectors;
ordinary Symbol literals and reflection keep their source names.
Compare [expected.txt](expected.txt).

```sh
mkdir -p scratch/compiler-visibility
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 3.0.0 \
  --app-id b67c1691c9a5422a9d3202a0c016870c --entry MinimalConsoleApp.onStart \
  examples/compiler-visibility/app.mc scratch/compiler-visibility/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-visibility/unsigned.prg \
  scratch/compiler-visibility/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-visibility/generated.prg
```

SDK reference: 9.2.0, `fr265`, minimum API 3.0.0, `-O 0`.
Use the [minimal console scaffold](../minimal-console/README.md) with this source.
