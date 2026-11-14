# Static and instance method shadowing

[app.mc](app.mc) covers both receiver kinds, separate static redeclarations,
independent signatures behind static parents, private static binding, own-class
qualification, and lazy initialization. See [expected.txt](expected.txt).

```sh
mkdir -p scratch/compiler-static-method-shadowing
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 3.0.0 \
  --app-id b67c1691c9a5422a9d3202a0c016870c --entry MinimalConsoleApp.onStart \
  examples/compiler-static-method-shadowing/app.mc scratch/compiler-static-method-shadowing/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-static-method-shadowing/unsigned.prg \
  scratch/compiler-static-method-shadowing/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-static-method-shadowing/generated.prg
```

SDK reference: 9.2.0, `fr265`, minimum API 3.0.0, `-O 0`.
Use the [minimal console scaffold](../minimal-console/README.md) with this source.
