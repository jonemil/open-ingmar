# Static methods through instances

[app.mc](app.mc) checks static method declarations, lexical/self calls, shared
storage, receiver identity, native Method bindings, and nested-module lookup.
A method-only class exercises static metadata without static fields. Compare
[expected.txt](expected.txt).

```sh
mkdir -p scratch/compiler-static-methods
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 3.0.0 \
  --app-id b67c1691c9a5422a9d3202a0c016870c --entry MinimalConsoleApp.onStart \
  examples/compiler-static-methods/app.mc scratch/compiler-static-methods/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-static-methods/unsigned.prg \
  scratch/compiler-static-methods/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-static-methods/generated.prg
```

SDK reference: 9.2.0, `fr265`, minimum API 3.0.0, `-O 0`.
Use the [minimal console scaffold](../minimal-console/README.md) with this source.
See [class calls](../compiler-class-calls/README.md) for direct static dispatch.
