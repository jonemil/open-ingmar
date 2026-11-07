# Inherited static members

[app.mc](app.mc) covers multilevel class-qualified lookup, separate subclass
static receivers, parent-owned shared storage, inherited static calls through
instances, and one-time initialization. `Empty` needs a static receiver despite
having no own static declarations. Compare [expected.txt](expected.txt).

```sh
mkdir -p scratch/compiler-inherited-statics
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 3.0.0 \
  --app-id b67c1691c9a5422a9d3202a0c016870c --entry MinimalConsoleApp.onStart \
  examples/compiler-inherited-statics/app.mc scratch/compiler-inherited-statics/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-inherited-statics/unsigned.prg \
  scratch/compiler-inherited-statics/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-inherited-statics/generated.prg
```

SDK reference: 9.2.0, `fr265`, minimum API 3.0.0, `-O 0`.
Use the [minimal console scaffold](../minimal-console/README.md) with this source.
