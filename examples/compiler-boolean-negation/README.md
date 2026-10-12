# Boolean negation

[app.mc](app.mc) and [flags.mc](flags.mc) exercise `!` on Boolean parameters,
locals, calls, and comparisons, including repeated prefixes and operand effects.
API context: 1.0.0; runtime validation: SDK 9.2.0 / fr265.

From the repository root:

```sh
mkdir -p scratch/compiler-boolean-negation
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 1.0.0 \
  --app-id 62a391ee5a124f73aa81da474077adc7 --entry start \
  examples/compiler-boolean-negation/app.mc \
  examples/compiler-boolean-negation/flags.mc \
  scratch/compiler-boolean-negation/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-boolean-negation/unsigned.prg \
  scratch/compiler-boolean-negation/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-boolean-negation/generated.prg
```

Expected output:

```text
false
true
false
true
true
true
false
true
false
false
inverted
present
effect
false
```

Postfix calls bind before `!`: `!value.equals(1)` inverts the method result.
Other types are not converted to Boolean. Negated null comparisons are accepted
as Boolean expressions and prove non-null on the corresponding branch edge.
See the [evidence and limits](../../docs/investigations/compiler-boolean-negation.md).
