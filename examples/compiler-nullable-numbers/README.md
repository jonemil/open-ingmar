# Nullable native Number results

[app.mc](app.mc) and [numbers.mc](numbers.mc) use String `find()` and `toNumber()`
results through null checks. Successful values feed formatting, arithmetic,
Number methods, and user-function returns. The example also checks index zero,
negative conversion, reassignment to null, and a loop that retries conversion.

From the repository root:

```sh
mkdir -p scratch/compiler-nullable-numbers
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 1.0.0 \
  --app-id d0f96a1076904c4493ad7908fae68642 --entry start \
  examples/compiler-nullable-numbers/app.mc examples/compiler-nullable-numbers/numbers.mc \
  scratch/compiler-nullable-numbers/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-nullable-numbers/unsigned.prg \
  scratch/compiler-nullable-numbers/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-nullable-numbers/generated.prg
```

Expected output, confirmed on SDK 9.2.0 / fr265:

```text
03
not found
00
42
0
0
-7
41
became null
retry
8
```

See [null-check rules][source], [IR validation][ir], and [runtime evidence][evidence].
Unchecked nullable values cannot be printed, returned, passed to user functions,
or used as Number operands. Every write invalidates a local's non-null proof;
check again before using its replacement value.

[source]: ../../docs/specs/compiler-subset.md#nullable-results
[ir]: ../../docs/specs/compiler-ir-text.md#nullable-results
[evidence]: ../../docs/investigations/compiler-nullable-numbers.md
