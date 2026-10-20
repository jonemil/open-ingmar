# Nullable user-function contracts

[app.mc](app.mc) and [values.mc](values.mc) pass Number, String, Boolean, and Char `or Null`
values through module and instance calls. Scalar and Null arguments satisfy
matching nullable parameters. Direct parameter null guards allow scalar use
without an intermediate local. The app also retries a nullable user result in a loop.

From the repository root:

```sh
mkdir -p scratch/compiler-nullable-contracts
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 1.3.0 \
  --app-id 23b0dafc002744cd909a0a4678c4d60e --entry NullableApp.onStart \
  examples/compiler-nullable-contracts/app.mc \
  examples/compiler-nullable-contracts/values.mc \
  scratch/compiler-nullable-contracts/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-nullable-contracts/unsigned.prg \
  scratch/compiler-nullable-contracts/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh \
  --prg scratch/compiler-nullable-contracts/generated.prg
```

SDK and generated programs match [expected output](expected.txt) on SDK 9.2.0 /
fr265. Boolean checks distinguish a present false value from Null. Char checks
exercise guarded arithmetic/method calls, pass-through results, and Null writes.
Conditional results, retry loops, repeated guards, and local assignments retain
their nullable contracts.

See [source contracts](../../docs/specs/frontend/types.md#types-and-value-contracts),
[saved IR](../../docs/specs/ir/values.md#nullable-user-signatures), and
[experiment](../../docs/investigations/compiler-nullable-contracts.md).
