# Null snapshots during shared initialization

[app.mc](app.mc) and [later.mc](later.mc) check direct forward reads of runtime
slots in global and reopened module scopes. Early aliases and captured values
retain Null after the later slots initialize. Completed fields retain their
ordinary container contracts. See [expected.txt](expected.txt).

```sh
mkdir -p scratch/compiler-null-snapshots
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 5.0.0 \
  --app-id 8744df3eac9d4803bd9e5b11fd417221 --entry MinimalConsoleApp.onStart \
  examples/compiler-null-snapshots/app.mc \
  examples/compiler-null-snapshots/later.mc \
  scratch/compiler-null-snapshots/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-null-snapshots/unsigned.prg \
  scratch/compiler-null-snapshots/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-null-snapshots/generated.prg
```

SDK reference: 9.2.0, `fr265`, minimum API 5.0.0, `-O 0`, default checking policy.
Use the [minimal console scaffold](../minimal-console/README.md) with both source
files and minimum API 5.0.0. In this session startup prints precede console
attachment; expected output begins at `onStart`.
