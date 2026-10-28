# Forward stored constants

[app.mc](app.mc) checks global/module aliases to later literal defaults,
ordered scalar alias chains, and field contracts on a class declared after
its consumer. Runtime initializer calls keep declaration order. String identity
and per-instance allocation remain observable in [expected.txt](expected.txt).

```sh
mkdir -p scratch/compiler-forward-constants
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 5.0.0 \
  --app-id 3170f18fda334595aee207e933ec3c17 --entry MinimalConsoleApp.onStart \
  examples/compiler-forward-constants/app.mc scratch/compiler-forward-constants/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-forward-constants/unsigned.prg \
  scratch/compiler-forward-constants/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-forward-constants/generated.prg
```

SDK reference: 9.2.0, `fr265`, minimum API 5.0.0, `-O 0`, default checking policy.
Use the [minimal console scaffold](../minimal-console/README.md) with this source
and minimum API 5.0.0 for the SDK comparison.
