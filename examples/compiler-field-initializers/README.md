# Instance-field declaration initializers

[app.mc](app.mc) and [box.mc](box.mc) check typed and untyped scalar, String,
container, and source-object fields. Declaration expressions run for each
allocation before `initialize()`, while literal defaults are available to
computed expressions. Two instances check independent mutable container storage;
field updates preserve their contracts. See [expected.txt](expected.txt).

```sh
mkdir -p scratch/compiler-field-initializers
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 5.0.0 \
  --app-id 8022a5c1d89449c18a1bf306ca0bbc65 --entry MinimalConsoleApp.onStart \
  examples/compiler-field-initializers/app.mc examples/compiler-field-initializers/box.mc \
  scratch/compiler-field-initializers/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-field-initializers/unsigned.prg \
  scratch/compiler-field-initializers/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-field-initializers/generated.prg
```

SDK reference: 9.2.0, `fr265`, minimum API 5.0.0, `-O 0`, default checking policy.
Use the [minimal console scaffold](../minimal-console/README.md) with both source
files and minimum API 5.0.0.
