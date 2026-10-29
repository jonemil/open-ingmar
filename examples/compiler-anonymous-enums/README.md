# Anonymous enum numbering

[app.mc](app.mc) and [later.mc](later.mc) check global/module anonymous enums,
literal numbering resets, runtime explicit values, 32-bit wraparound, reopened
modules, and forward reads of declaration defaults. Parenthesized values keep
runtime placement and do not advance the numbering counter. See
[expected.txt](expected.txt).

```sh
mkdir -p scratch/compiler-anonymous-enums
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 5.0.0 \
  --app-id 2e0caf1206d441caa6fb00fdd46f0721 --entry MinimalConsoleApp.onStart \
  examples/compiler-anonymous-enums/app.mc \
  examples/compiler-anonymous-enums/later.mc \
  scratch/compiler-anonymous-enums/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-anonymous-enums/unsigned.prg \
  scratch/compiler-anonymous-enums/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-anonymous-enums/generated.prg
```

SDK reference: 9.2.0, `fr265`, minimum API 5.0.0, `-O 0`, default checking policy.
Use the [minimal console scaffold](../minimal-console/README.md) with both source
files and minimum API 5.0.0. This supported profile uses Number-valued members.
Module trace prints appear on first access during `onStart`.
