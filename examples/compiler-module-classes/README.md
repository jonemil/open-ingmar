# Classes inside modules

[app.mc](app.mc) and [later.mc](later.mc) exercise nested module-owned classes,
relative/qualified type names, construction, casts, and runtime type tests.
The `Token` classes keep distinct identities. Parent-module reads in an
instance method retain lexical lookup, while direct construction activates only
the declaring module. Bare class targets skip intermediate parents; the
module-owned app entry leaves its module cold. See [expected.txt](expected.txt).

```sh
mkdir -p scratch/compiler-module-classes
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 3.0.0 \
  --app-id b67c1691c9a5422a9d3202a0c016870c --entry Apps.MinimalConsoleApp.onStart \
  examples/compiler-module-classes/app.mc examples/compiler-module-classes/later.mc \
  scratch/compiler-module-classes/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-module-classes/unsigned.prg \
  scratch/compiler-module-classes/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-module-classes/generated.prg
```

SDK reference: 9.2.0, `fr265`, minimum API 3.0.0, `-O 0`, default checking policy.
Use the [minimal console scaffold](../minimal-console/README.md) with both source
files, entry `Apps.MinimalConsoleApp`, and minimum API 3.0.0.
