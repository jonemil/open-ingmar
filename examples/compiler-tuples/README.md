# Positional tuple contracts

[app.mc](app.mc) exercises contextual tuple construction, positional and dynamic
reads, nested/source slots, nullable guards, fields, typed callbacks, conditional
construction, Array widening, mixed-token/nested literal inference, named slot
replacement, `add`/`addAll` cardinality, empty append arguments, returned aliases,
branch joins, compound updates, and left-to-right slot effects.
It also covers unknown-length tails, loop-carried Array element unions,
dynamic-index writes, and guarded nullable mutation.
Computed/nested receiver cases check returned aliases, chained appends, nested
stores/compound updates, and receiver/index/value effect order.
Self-field cases check refined reads, branch/loop joins, nullable guards,
reassignment/call invalidation, and captured receivers across field replacement.

From the repository root, using fresh output paths:

```sh
mkdir -p scratch/compiler-tuples
cargo run -p ciq-compiler -- --unit --emit prg --target-api 5.2.0 --min-api 3.0.0 --app-id 60f7923dc92947da9c564c6f8084f8de --entry MinimalConsoleApp.onStart examples/compiler-tuples/app.mc scratch/compiler-tuples/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-tuples/unsigned.prg scratch/compiler-tuples/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-tuples/generated.prg
```

Compare the full console with [expected.txt](expected.txt). Reference environment:
SDK 9.2.0, fr265, minimum API 3.0.0 and target 5.2.0. See the
[tuple contracts](../../docs/specs/frontend/README.md#positional-tuples).
