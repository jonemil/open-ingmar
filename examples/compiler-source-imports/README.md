# Source namespace imports

[app.mc](app.mc) and [types.mc](types.mc) cover source module aliases, alias
chains, imported class/typedef/enum names, and reopened modules across files.
Initializer expressions retain their declaration's aliases. A local shadows a
qualified alias, while a same-named field uses explicit `self` access. See [expected.txt](expected.txt).

```sh
mkdir -p scratch/compiler-source-imports
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 3.0.0 \
  --app-id b67c1691c9a5422a9d3202a0c016870c --entry MinimalConsoleApp.onStart \
  examples/compiler-source-imports/app.mc examples/compiler-source-imports/types.mc \
  scratch/compiler-source-imports/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-source-imports/unsigned.prg \
  scratch/compiler-source-imports/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-source-imports/generated.prg
```

SDK reference: 9.2.0, `fr265`, minimum API 3.0.0, `-O 0`, default checking policy.
Use the [minimal console scaffold](../minimal-console/README.md) with both source
files and minimum API 3.0.0.
