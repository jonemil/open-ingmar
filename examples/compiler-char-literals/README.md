# Char literals

[app.mc](app.mc) and [chars.mc](chars.mc) exercise BMP Char literals, escapes,
native calls, typed arguments/returns, and mutable locals. The example requires
API 5.0.0 for comparison; literals themselves require API 1.3.0.

From the repository root:

```sh
mkdir -p scratch/compiler-char-literals
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 5.0.0 \
  --app-id aaa08a66fa6e4a00b7a19870f7a38943 --entry start \
  examples/compiler-char-literals/app.mc \
  examples/compiler-char-literals/chars.mc \
  scratch/compiler-char-literals/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-char-literals/unsigned.prg \
  scratch/compiler-char-literals/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-char-literals/generated.prg
```

Expected output, observed on SDK 9.2.0 / fr265:

```text
A
233
猫
10
13
9
39
92
34
a
A
true
true
A
é
receiver
A
true
```

See the [literal evidence](../../docs/investigations/compiler-char-literals.md)
for encoding, supplementary-character limits, and raw IR probes.
