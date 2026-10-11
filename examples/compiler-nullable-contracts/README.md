# Nullable user-function contracts

[app.mc](app.mc) and [values.mc](values.mc) pass Number-or-Null and String-or-Null
values through module and instance calls. Scalar and Null arguments satisfy
matching nullable parameters. Nullable parameters are copied to locals for
guarded scalar use. The app also retries a nullable user result in a loop.

From the repository root:

```sh
mkdir -p scratch/compiler-nullable-contracts
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 1.0.0 \
  --app-id 23b0dafc002744cd909a0a4678c4d60e --entry NullableApp.onStart \
  examples/compiler-nullable-contracts/app.mc \
  examples/compiler-nullable-contracts/values.mc \
  scratch/compiler-nullable-contracts/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-nullable-contracts/unsigned.prg \
  scratch/compiler-nullable-contracts/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh \
  --prg scratch/compiler-nullable-contracts/generated.prg
```

Expected output, observed on SDK 9.2.0 / fr265:

```text
42
0
7
0
HELLO
fallback
scalar
fallback
retry
5
```

See [source contracts](../../docs/specs/compiler-subset.md#function-type-contracts),
[saved IR](../../docs/specs/compiler-ir-text.md#nullable-user-signatures), and
[experiment](../../docs/investigations/compiler-nullable-contracts.md).
