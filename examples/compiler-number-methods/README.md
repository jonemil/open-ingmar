# Number method group

[app.mc](app.mc) and [numbers.mc](numbers.mc) exercise the supported Number
methods together: conversion, formatting, absolute value, string conversion,
comparison, equality, and hashing. The example checks comparison signs rather
than exact magnitudes, hash consistency rather than exact hashes, and observable
receiver/argument order. It needs API 5.0.0 for `compareTo`.

From the repository root:

```sh
mkdir -p scratch/compiler-number-methods
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 5.0.0 \
  --app-id 55aef248ff714539943e326652cf3066 --entry start \
  examples/compiler-number-methods/app.mc \
  examples/compiler-number-methods/numbers.mc \
  scratch/compiler-number-methods/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-number-methods/unsigned.prg \
  scratch/compiler-number-methods/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-number-methods/generated.prg
```

Expected output, observed on SDK 9.2.0 / fr265:

```text
7
007
-7
true
true
true
true
true
true
false
false
false
false
true
false
false
false
true
true
receiver
argument
true
```

See [accepted contracts](../../docs/specs/frontend/README.md#native-value-calls)
and the [full method review](../../docs/investigations/compiler-number-methods.md)
for methods deferred until their prerequisites exist.
