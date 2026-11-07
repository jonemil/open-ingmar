# Source field shadowing

[app.mc](app.mc) compares public field shadows, parent expression defaults versus
child literal defaults, differing declared contracts, access reductions, and
constant/variable shadows. Parent-bound contracts do not convert runtime values;
for example, a Number-declared parent read can observe the child's String.
Compare [expected.txt](expected.txt).

```sh
mkdir -p scratch/compiler-field-shadowing
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 3.0.0 \
  --app-id b67c1691c9a5422a9d3202a0c016870c --entry MinimalConsoleApp.onStart \
  examples/compiler-field-shadowing/app.mc scratch/compiler-field-shadowing/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-field-shadowing/unsigned.prg \
  scratch/compiler-field-shadowing/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-field-shadowing/generated.prg
```

SDK reference: 9.2.0, `fr265`, minimum API 3.0.0, `-O 0`.
Use the [minimal console scaffold](../minimal-console/README.md) with this source.
