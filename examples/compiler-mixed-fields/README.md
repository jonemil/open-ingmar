# Mixed static and instance fields

[app.mc](app.mc) covers receiver-specific contracts, instance-field precedence,
separate storage, private selectors, readonly checks, and structural witnesses.
Own-class qualification keeps class lookup at runtime: `StaticLeaf.slot` in an
instance body has the instance-view Number contract but returns the static String.
The compiler preserves the SDK's behavior rather than converting that value.
Compare [expected.txt](expected.txt).

```sh
mkdir -p scratch/compiler-mixed-fields
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 3.0.0 \
  --app-id b67c1691c9a5422a9d3202a0c016870c --entry MinimalConsoleApp.onStart \
  examples/compiler-mixed-fields/app.mc scratch/compiler-mixed-fields/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-mixed-fields/unsigned.prg \
  scratch/compiler-mixed-fields/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-mixed-fields/generated.prg
```

SDK reference: 9.2.0, `fr265`, minimum API 3.0.0, `-O 0`.
Use the [minimal console scaffold](../minimal-console/README.md) with this source.
