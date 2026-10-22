# Typedefs across files and owners

[app.mc](app.mc) uses aliases declared in [types.mc](types.mc), including a
forward reference through a module. Global, module, and class-local aliases
cover native unions, nullable source classes, container arguments, fields, and
an existing scalar cast. Alias names resolve before semantic IR construction.

From the repository root, using new output paths:

```sh
mkdir -p scratch/compiler-typedefs
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 3.0.0 \
  --app-id d85412c0790f4ee98cbebc2ac5479af1 --entry MinimalConsoleApp.onStart \
  examples/compiler-typedefs/app.mc examples/compiler-typedefs/types.mc \
  scratch/compiler-typedefs/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-typedefs/unsigned.prg \
  scratch/compiler-typedefs/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-typedefs/generated.prg
```

Compare the full console with [expected.txt](expected.txt). Reference environment:
SDK 9.2.0, fr265, API minimum 3.0.0 and target 5.2.0. See
[alias contracts](../../docs/specs/frontend/README.md#type-aliases) and
[SDK evidence](../../docs/investigations/compiler-numeric-unions.md#source-typedefs).
