# Number negation

[app.mc](app.mc) and [numbers.mc](numbers.mc) exercise negation of parameters,
locals, calls, and grouped expressions. The example covers precedence, the i32
minimum, nullable guards, and one observable operand call. API context: 1.0.0;
runtime validation: SDK 9.2.0 / fr265.

From the repository root:

```sh
mkdir -p scratch/compiler-number-negation
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 1.0.0 \
  --app-id ffa08a66fa6e4a00b7a19870f7a38943 --entry start \
  examples/compiler-number-negation/app.mc \
  examples/compiler-number-negation/numbers.mc \
  scratch/compiler-number-negation/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-number-negation/unsigned.prg \
  scratch/compiler-number-negation/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-number-negation/generated.prg
```

Expected output:

```text
-41
41
0
-2147483648
-2147483648
-2147483647
-3
41
41
-41
-1
-7
-7
8
positive
-8
0
effect
-7
```

Postfix calls bind before negation: `-value.abs()` negates the call result.
Negative literal receivers still require parentheses, because the SDK exhibits
inconsistent bare-literal behavior. Repeated negation uses `-(-value)`.
See the [evidence and limits](../../docs/investigations/compiler-number-negation.md).
