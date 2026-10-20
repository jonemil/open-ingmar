# String method group

[app.mc](app.mc) and [texts.mc](texts.mc) exercise supported String methods
together: comparison, equality, hashing, search, slicing, case/string conversion,
and nullable numeric conversion with explicit bases. Comparison and hash tests
use signs and consistency instead of exact values. The example also checks
receiver/argument order and a short UTF-8 string.

From the repository root:

```sh
mkdir -p scratch/compiler-string-methods
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 5.0.0 \
  --app-id 66a08a66fa6e4a00b7a19870f7a38943 --entry start \
  examples/compiler-string-methods/app.mc \
  examples/compiler-string-methods/texts.mc \
  scratch/compiler-string-methods/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-string-methods/unsigned.prg \
  scratch/compiler-string-methods/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-string-methods/generated.prg
```

Expected output, observed on SDK 9.2.0 / fr265:

```text
hello
HELLO
Hello
0
true
true
true
true
false
false
false
false
true
false
false
false
true
2
null
41
5
255
35
16
8
-16
null
3
1
猫
receiver
argument
true
```

See [accepted contracts](../../docs/specs/frontend/README.md#native-value-calls)
and the [full method review](../../docs/investigations/compiler-string-methods.md).
