# Char method group

[app.mc](app.mc) and [chars.mc](chars.mc) exercise Number-to-Char conversion,
Char case/string/number conversion, comparison, equality, and hashing. Char
values pass through typed parameters, returns, locals, and native calls.
The example requires API 5.0.0 for comparison; Char itself dates from 1.3.0.

From the repository root:

```sh
mkdir -p scratch/compiler-char-methods
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 5.0.0 \
  --app-id 99a08a66fa6e4a00b7a19870f7a38943 --entry start \
  examples/compiler-char-methods/app.mc \
  examples/compiler-char-methods/chars.mc \
  scratch/compiler-char-methods/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-char-methods/unsigned.prg \
  scratch/compiler-char-methods/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-char-methods/generated.prg
```

Expected output, observed on SDK 9.2.0 / fr265:

```text
A
65
a
A
é
猫
128512
0
-1
55296
1114112
true
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
true
true
true
true
B
a
receiver
argument
true
```

The [literal example](../compiler-char-literals/README.md) covers BMP constants.
The [operator example](../compiler-char-operators/README.md) covers the supported
arithmetic and comparison pairs. Nullable Char remains unsupported. Numeric
conversion ranges and Unicode behavior are runtime responsibilities; see the
[method review and limits](../../docs/investigations/compiler-char-methods.md).
