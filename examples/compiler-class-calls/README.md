# Class-qualified static calls

[app.mc](app.mc) checks static lookup before argument effects, callable captures
across branching/constructed arguments, Void calls, aliases/root qualification,
source-class results, and receiver identity. Object/source-class return
annotations through aliases and stored receivers illustrate that a static
receiver need not be an instance. Compare [expected.txt](expected.txt).

```sh
mkdir -p scratch/compiler-class-calls
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 3.0.0 \
  --app-id b67c1691c9a5422a9d3202a0c016870c --entry MinimalConsoleApp.onStart \
  examples/compiler-class-calls/app.mc scratch/compiler-class-calls/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-class-calls/unsigned.prg \
  scratch/compiler-class-calls/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-class-calls/generated.prg
```

SDK reference: 9.2.0, `fr265`, minimum API 3.0.0, `-O 0`.
Use the [minimal console scaffold](../minimal-console/README.md) with this source.
