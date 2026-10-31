# Bare instance-field names

[app.mc](app.mc) and [box.mc](box.mc) check reads, writes, compounds, updates,
container receivers, and declaration expressions without `self.`. Parameters
and locals shadow fields; fields shadow globals and user module qualifiers.
A field remains visible before a local declaration and after its block ends.
The explicit root call bypasses a shadowed user module.
See [expected.txt](expected.txt).

```sh
mkdir -p scratch/compiler-bare-fields
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 5.0.0 \
  --app-id c48b23478de04626a83175b99c6de201 --entry MinimalConsoleApp.onStart \
  examples/compiler-bare-fields/app.mc examples/compiler-bare-fields/box.mc \
  scratch/compiler-bare-fields/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-bare-fields/unsigned.prg \
  scratch/compiler-bare-fields/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-bare-fields/generated.prg
```

SDK reference: 9.2.0, `fr265`, minimum API 5.0.0, `-O 0`, default checking policy.
Use the [minimal console scaffold](../minimal-console/README.md) with both source
files and minimum API 5.0.0.
