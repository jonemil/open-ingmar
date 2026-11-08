# Method access shadowing

[app.mc](app.mc) exercises public parent slots alongside protected/private child
methods, independent private signatures, scoped calls through explicit receivers,
dispatch through parent references, and method interface witnesses. A private
String-checked explicit receiver call still invokes the public Number method at
runtime. See [expected.txt](expected.txt).

```sh
mkdir -p scratch/compiler-method-access
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 3.0.0 \
  --app-id b67c1691c9a5422a9d3202a0c016870c --entry MinimalConsoleApp.onStart \
  examples/compiler-method-access/app.mc scratch/compiler-method-access/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-method-access/unsigned.prg \
  scratch/compiler-method-access/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-method-access/generated.prg
```

SDK reference: 9.2.0, `fr265`, minimum API 3.0.0, `-O 0`.
Use the [minimal console scaffold](../minimal-console/README.md) with this source.
