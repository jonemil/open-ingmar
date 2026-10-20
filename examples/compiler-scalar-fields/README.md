# Typed scalar fields

[app.mc](app.mc) exercises required and nullable Number, Long, Float, Double,
Boolean, Char, and String fields. It checks runtime classes, exact numeric
widths and signed zero, compound writes, independent instances, and guarded
snapshots retained after clearing nullable fields. An AppBase field uses the
same required-initialization contract.

API minimum: 2.1.0; target: 5.2.0; SDK/compiler/simulator: 9.2.0 / fr265.

From the repository root, using unused output paths:

```sh
mkdir -p scratch/compiler-scalar-fields
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 2.1.0 \
  --app-id 9c77dc1202384df19b7be6ddc92a48bc --entry MinimalConsoleApp.onStart \
  examples/compiler-scalar-fields/app.mc scratch/compiler-scalar-fields/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-scalar-fields/unsigned.prg \
  scratch/compiler-scalar-fields/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh \
  --prg scratch/compiler-scalar-fields/generated.prg
```

Compare the console with [expected.txt](expected.txt). See the
[field contract](../../docs/specs/frontend/README.md#typed-scalar-fields) and
[evidence](../../docs/investigations/compiler-scalar-fields.md).
