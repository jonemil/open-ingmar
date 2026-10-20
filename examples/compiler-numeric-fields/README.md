# Numeric alias fields

[app.mc](app.mc) stores Numeric, Integer, and Decimal values in class fields.
It checks constructor initialization, type changes within each alias, compound
writes and updates, nullable defaults, and guarded snapshots after clearing.
Two instances retain independent state; a saved value keeps its original
numeric class after the field is replaced.

API minimum: 2.1.0; target: 5.2.0; SDK/compiler/simulator: 9.2.0 / fr265.

From the repository root, using unused output paths:

```sh
mkdir -p scratch/compiler-numeric-fields
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 2.1.0 \
  --app-id 345758f83cfa4aaf814040968b95ce13 --entry MinimalConsoleApp.onStart \
  examples/compiler-numeric-fields/app.mc scratch/compiler-numeric-fields/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-numeric-fields/unsigned.prg \
  scratch/compiler-numeric-fields/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh \
  --prg scratch/compiler-numeric-fields/generated.prg
```

Compare the complete console with [expected.txt](expected.txt). See the
[field contracts](../../docs/specs/frontend/fields.md#numeric-alias-fields) and
[evidence](../../docs/investigations/compiler-numeric-unions.md).
