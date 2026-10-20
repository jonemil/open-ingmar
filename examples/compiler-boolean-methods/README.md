# Boolean method group

[app.mc](app.mc) and [flags.mc](flags.mc) exercise Boolean comparison and
inherited equality, hashing, and string conversion. The example covers literal,
local, parameter, and call-result receivers; mixed Number comparisons; nullable
equality arguments; chaining; and receiver/argument evaluation order.
It requires API 5.0.0 for `compareTo`.

From the repository root:

```sh
mkdir -p scratch/compiler-boolean-methods
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 5.0.0 \
  --app-id 77a08a66fa6e4a00b7a19870f7a38943 --entry start \
  examples/compiler-boolean-methods/app.mc \
  examples/compiler-boolean-methods/flags.mc \
  scratch/compiler-boolean-methods/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-boolean-methods/unsigned.prg \
  scratch/compiler-boolean-methods/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-boolean-methods/generated.prg
```

Expected output, observed on SDK 9.2.0 / fr265:

```text
true
false
true
true
true
true
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
false
false
false
false
false
true
true
TRUE
receiver
argument
true
```

See [accepted contracts](../../docs/specs/frontend/lang-values.md#native-value-calls)
and the [method review](../../docs/investigations/compiler-boolean-methods.md)
for deferred members and the bounded argument profile.
