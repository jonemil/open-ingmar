# Char-array bulk append and removals

[app.mc](app.mc) exercises `addAll`, `remove`, and `removeAll` with typed user
calls, shared references, duplicates, absent matches, empty arrays, conditional
effects, and self-appending. API context: 1.3.0; runtime: SDK 9.2.0 / fr265.

From the repository root, using unused output paths:

```sh
mkdir -p scratch/compiler-array-mutation
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 1.3.0 \
  --app-id a632d7b2946449c08f058b90d7e002af --entry start \
  examples/compiler-array-mutation/app.mc \
  scratch/compiler-array-mutation/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-array-mutation/unsigned.prg \
  scratch/compiler-array-mutation/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh \
  --prg scratch/compiler-array-mutation/generated.prg
```

Expected output:

```text
[a, b, a, c, a]
[a, b, a, c, a]
[c, a]
true
[b, a, c, a]
false
true
[b, c]
false
2
false
false
[x, y]
receiver
value
true
false
[a, b, a]
true
[b]
before self append
[x, y, x, y]
```

`addAll` returns the receiver, while removals return whether they removed any
element. All references to the receiver observe its mutations. The tested
self-append copies the original elements once. See the [source contract][spec]
and [evidence][evidence] for the supported profile and limits.

[spec]: ../../docs/specs/compiler-subset.md#char-arrays
[evidence]: ../../docs/investigations/compiler-array-mutation.md
