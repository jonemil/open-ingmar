# Dynamic instance fields

[app.mc](app.mc) checks unannotated fields initialized from Number, Symbol,
String, containers, source objects, and computed expressions. Later writes can
change their runtime types. Guarded local snapshots establish source/Array
member access; two instances retain independent state. Number and Symbol
literals retain metadata defaults. See [expected.txt](expected.txt).

```sh
mkdir -p scratch/compiler-dynamic-fields
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 5.0.0 \
  --app-id 1243d85fd10c457f8a9e6f23a7b0d981 --entry MinimalConsoleApp.onStart \
  examples/compiler-dynamic-fields/app.mc scratch/compiler-dynamic-fields/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-dynamic-fields/unsigned.prg \
  scratch/compiler-dynamic-fields/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-dynamic-fields/generated.prg
```

SDK reference: 9.2.0, `fr265`, minimum API 5.0.0, `-O 0`, default checking policy.
Use the [minimal console scaffold](../minimal-console/README.md) with this source
file and minimum API 5.0.0.
