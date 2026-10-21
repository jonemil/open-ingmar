# Numeric aliases and native unions

[app.mc](app.mc) checks all 16 Number/Long/Float/Double pairings through
`Lang.Numeric` parameters, integer-only operations through `Lang.Integer`, and
floating results through `Lang.Decimal`. Nullable signatures, guards, local
writes, compound updates, unary signs, and ByteArray decoding exercise the same
contracts without explicit conversions.

The `NativeUnions` group checks Number/String signatures, common methods,
both branches of class/Null tests, changing locals and loop exits, conditional
joins, standalone Null, and guarded nullable Array references. The `SourceUnions`
group checks exact class alternatives, forwarding, both guard edges, changing
locals, loop exits, and structural Comparator witnesses for sorting.

API minimum: 5.0.0; target: 5.2.0; SDK/compiler/simulator: 9.2.0 / fr265.

From the repository root, using unused output paths:

```sh
mkdir -p scratch/compiler-numeric-unions
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 5.0.0 \
  --app-id ef3460cc645c4590a9fe5a288b8a610a --entry MinimalConsoleApp.onStart \
  examples/compiler-numeric-unions/app.mc scratch/compiler-numeric-unions/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-numeric-unions/unsigned.prg \
  scratch/compiler-numeric-unions/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh \
  --prg scratch/compiler-numeric-unions/generated.prg
```

Compare the complete console with [expected.txt](expected.txt). See the
[source contracts](../../docs/specs/frontend/README.md) and
[SDK evidence](../../docs/investigations/compiler-numeric-unions.md).
