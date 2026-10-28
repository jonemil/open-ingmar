# Typed constant declaration lists

[app.mc](app.mc) and [later.mc](later.mc) cover independent annotations and
initializers in global, module, and class declaration lists. Declared Object,
container, nullable source-class, and native enum contracts survive reads;
missing initializers produce Null. Cross-file typedefs, mutation through aliases,
per-instance storage, and module trace order are checked in
[expected.txt](expected.txt).

```sh
mkdir -p scratch/compiler-constant-declarations
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 5.0.0 \
  --app-id 43ae045dd18b4b90b84cb151e55a0721 --entry MinimalConsoleApp.onStart \
  examples/compiler-constant-declarations/app.mc \
  examples/compiler-constant-declarations/later.mc \
  scratch/compiler-constant-declarations/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-constant-declarations/unsigned.prg \
  scratch/compiler-constant-declarations/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-constant-declarations/generated.prg
```

SDK reference: 9.2.0, `fr265`, minimum API 5.0.0, `-O 0`, default checking policy.
Use the [minimal console scaffold](../minimal-console/README.md) with both source
files and minimum API 5.0.0. Keep whitespace between a generic type's closing `>`
and `=` when building with the SDK.
