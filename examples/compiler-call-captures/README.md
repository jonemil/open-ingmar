# Captured call targets

[app.mc](app.mc) checks callable/context capture before conditional,
short-circuit, and source-construction arguments. Initialization markers expose
lookup order for direct, lexical, and owner-relative namespace calls, including
nested captures and Void/discarded results. See [expected.txt](expected.txt).

```sh
mkdir -p scratch/compiler-call-captures
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 3.0.0 \
  --app-id 6ed3112cf797467f85ee7525cb00da21 --entry MinimalConsoleApp.onStart \
  examples/compiler-call-captures/app.mc scratch/compiler-call-captures/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-call-captures/unsigned.prg \
  scratch/compiler-call-captures/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-call-captures/generated.prg
```

SDK reference: 9.2.0, `fr265`, minimum API 3.0.0, `-O 0`, default checking policy.
Use the [minimal console scaffold](../minimal-console/README.md) with this source
file and minimum API 3.0.0.
