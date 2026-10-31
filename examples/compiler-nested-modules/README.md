# Nested modules

[app.mc](app.mc) and [later.mc](later.mc) exercise three-level namespace lookup,
reopened modules across files, inherited lexical typedef/enum names, and
root-qualified calls. Initialization markers distinguish direct access from
lexical parent lookup: a lexical store evaluates its RHS before activating
parent modules, whereas a qualified store activates its target first. Empty
nested modules also retain their namespace metadata. See
[expected.txt](expected.txt).

```sh
mkdir -p scratch/compiler-nested-modules
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 3.0.0 \
  --app-id b67c1691c9a5422a9d3202a0c016870c --entry MinimalConsoleApp.onStart \
  examples/compiler-nested-modules/app.mc examples/compiler-nested-modules/later.mc \
  scratch/compiler-nested-modules/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-nested-modules/unsigned.prg \
  scratch/compiler-nested-modules/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-nested-modules/generated.prg
```

SDK reference: 9.2.0, `fr265`, minimum API 3.0.0, `-O 0`, default checking policy.
Use the [minimal console scaffold](../minimal-console/README.md) with both source
files and minimum API 3.0.0.
