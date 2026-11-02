# Method construction

[app.mc](app.mc) constructs erased Methods from instance, module, global,
scalar, container, and class-definition receivers. It checks argument order,
conditional/private-constructor arguments, receiver retention after reassignment,
and release after the last Method alias disappears. Class binding is constructed
without invoking its non-static target. See [expected.txt](expected.txt).

```sh
mkdir -p scratch/compiler-method-construction
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 3.0.0 \
  --app-id b67c1691c9a5422a9d3202a0c016870c --entry MinimalConsoleApp.onStart \
  examples/compiler-method-construction/app.mc scratch/compiler-method-construction/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-method-construction/unsigned.prg \
  scratch/compiler-method-construction/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-method-construction/generated.prg
```

SDK reference: 9.2.0, `fr265`, minimum API 3.0.0, `-O 0`, default checking policy.
Use the [minimal console scaffold](../minimal-console/README.md) with this source
file and minimum API 3.0.0.
