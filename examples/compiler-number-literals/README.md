# Signed Number literals

[app.mc](app.mc) and [numbers.mc](numbers.mc) exercise the full signed i32
literal range, immediate-width boundaries, negative constants, typed calls,
local replacement, arithmetic, and comparisons. The API context is 1.0.0;
runtime validation uses SDK 9.2.0 / fr265.

From the repository root:

```sh
mkdir -p scratch/compiler-number-literals
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 1.0.0 \
  --app-id eea08a66fa6e4a00b7a19870f7a38943 --entry start \
  examples/compiler-number-literals/app.mc \
  examples/compiler-number-literals/numbers.mc \
  scratch/compiler-number-literals/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-number-literals/unsigned.prg \
  scratch/compiler-number-literals/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-number-literals/generated.prg
```

Expected output:

```text
0
127
128
255
256
32767
32768
65535
65536
8388607
8388608
2147483647
-1
-128
-129
-32768
-32769
-8388608
-8388609
-2147483648
128
-2147483648
32768
3
65536
true
true
```

Negative constants require parentheses before member calls, for example
`(-128).abs()`. General unary operators remain outside the subset. See the
[encoding and parsing evidence](../../docs/investigations/compiler-number-literals.md).
