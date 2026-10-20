# Char operators

[app.mc](app.mc) and [chars.mc](chars.mc) exercise numeric Char equality and
ordering, Char/Number addition, and Char concatenation. Typed calls, locals,
null guards, and observable operand order share the complete application.
The example requires API 1.3.0 and uses no API 5.0.0 native comparison.

From the repository root:

```sh
mkdir -p scratch/compiler-char-operators
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 1.3.0 \
  --app-id cca08a66fa6e4a00b7a19870f7a38943 --entry start \
  examples/compiler-char-operators/app.mc \
  examples/compiler-char-operators/chars.mc \
  scratch/compiler-char-operators/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-char-operators/unsigned.prg \
  scratch/compiler-char-operators/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-char-operators/generated.prg
```

Expected output, observed on SDK 9.2.0 / fr265:

```text
true
false
true
true
false
true
false
true
true
B
66
@
65
AB
AB
true
234
C
AB
C
B
left
right
B
```

See [the operand contracts](../../docs/specs/frontend/operators.md#operator-contracts)
and [the experiment](../../docs/investigations/compiler-char-operators.md).
