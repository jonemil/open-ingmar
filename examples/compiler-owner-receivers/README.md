# Owner-relative receivers

[app.mc](app.mc) checks `self` and `me` in global functions, nested modules, and
source instances. Reads, stores, enclosing/global calls, instance identity, and
reflective Method receivers share their existing contracts. Namespace paths
retain receiver lookup: initialization markers show parent search and namespace
traversal before argument evaluation. See [expected.txt](expected.txt).

```sh
mkdir -p scratch/compiler-owner-receivers
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 3.0.0 \
  --app-id b67c1691c9a5422a9d3202a0c016870c --entry MinimalConsoleApp.onStart \
  examples/compiler-owner-receivers/app.mc scratch/compiler-owner-receivers/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-owner-receivers/unsigned.prg \
  scratch/compiler-owner-receivers/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-owner-receivers/generated.prg
```

SDK reference: 9.2.0, `fr265`, minimum API 3.0.0, `-O 0`, default checking policy.
Use the [minimal console scaffold](../minimal-console/README.md) with this source
file and minimum API 3.0.0.
