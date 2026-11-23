# Qualified instance calls

[app.mc](app.mc) checks own-class and parent-qualified methods, a parent in
another module, private/protected access, virtual dispatch within a parent body,
argument order across a branch, and repeated ordinary initialization. Allocation
defaults run once per object; explicit `initialize` calls do not repeat them.
The final call checks own-class initialization from a static body.
See [expected.txt](expected.txt).

```sh
mkdir -p scratch/compiler-qualified-instance-calls
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 3.0.0 \
  --app-id b67c1691c9a5422a9d3202a0c016870c --entry MinimalConsoleApp.onStart \
  examples/compiler-qualified-instance-calls/app.mc scratch/compiler-qualified-instance-calls/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-qualified-instance-calls/unsigned.prg \
  scratch/compiler-qualified-instance-calls/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-qualified-instance-calls/generated.prg
```

SDK reference: 9.2.0, `fr265`, minimum API 3.0.0, `-O 0`.
Use the [minimal console scaffold](../minimal-console/README.md) with this source.
