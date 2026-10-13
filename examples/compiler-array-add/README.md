# Char-array append

[app.mc](app.mc) exercises `Array<Char>.add(Char)`, mutation through shared
references, the returned `Array<Char>`, chained calls, and receiver/argument
evaluation order. API context: 1.3.0; runtime validation: SDK 9.2.0 / fr265.

From the repository root:

```sh
mkdir -p scratch/compiler-array-add
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 1.3.0 \
  --app-id 60dba4a2a4104d9ca1a24a08e9d21731 --entry start \
  examples/compiler-array-add/app.mc \
  scratch/compiler-array-add/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-array-add/unsigned.prg \
  scratch/compiler-array-add/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh \
  --prg scratch/compiler-array-add/generated.prg
```

Expected output:

```text
[a, b, c]
[a, b, c]
[a, b, c]
3
4
[a, b, c, d]
receiver
value
[x, y, z]
```

The shared reference and call result both observe the append. The native receiver
is evaluated before its argument, and each is evaluated once. The [source
contract][spec] and [SDK evidence][evidence] describe the current boundary.

[spec]: ../../docs/specs/compiler-subset.md#char-arrays
[evidence]: ../../docs/investigations/compiler-array-add.md
