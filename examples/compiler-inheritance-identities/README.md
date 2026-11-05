# Source superclass identities

[app.mc](app.mc) exercises forward superclass declarations, module aliases,
root paths, subtype arguments/results, runtime class tests, guarded narrowing,
and contextual Array literals. Parent instance state and initializer effects
are outside this construction profile. Compare [expected.txt](expected.txt).

```sh
mkdir -p scratch/compiler-inheritance-identities
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 3.0.0 \
  --app-id b67c1691c9a5422a9d3202a0c016870c --entry MinimalConsoleApp.onStart \
  examples/compiler-inheritance-identities/app.mc scratch/compiler-inheritance-identities/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-inheritance-identities/unsigned.prg \
  scratch/compiler-inheritance-identities/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-inheritance-identities/generated.prg
```

SDK reference: 9.2.0, `fr265`, minimum API 3.0.0, `-O 0`.
Use the [minimal console scaffold](../minimal-console/README.md) with this source.
