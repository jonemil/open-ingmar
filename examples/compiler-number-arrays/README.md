# Number arrays and UTF-8 conversion

[app.mc](app.mc) exercises `String.toUtf8Array()`, typed Number-array parameters
and results, arithmetic reads, shared references, mutation, transforms, Unicode,
empty conversion, and a guarded nullable String. API minimum: 1.3.0; target:
5.2.0; runtime: SDK 9.2.0 / fr265.

From the repository root, using unused output paths:

```sh
mkdir -p scratch/compiler-number-arrays
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 1.3.0 \
  --app-id 23cb2c25c6a94ad68e936ab46ff4ab9e --entry start \
  examples/compiler-number-arrays/app.mc \
  scratch/compiler-number-arrays/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-number-arrays/unsigned.prg \
  scratch/compiler-number-arrays/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh \
  --prg scratch/compiler-number-arrays/generated.prg
```

Expected output:

```text
[65, 195, 169, 240, 159, 152, 128]
7
364
65
300
true
1
[128, 152, 159, 240, 169, 195, 65]
[195, 169]
9
[120, 121]
true
195
0
effect
122
113
```

Conversion produces UTF-8 byte values, but the element contract is Number:
appending 300 is permitted. Number and Char arrays cannot substitute for each
other. [Number literals](../compiler-number-array-literals/README.md) use the
same element contract; empty source literals still await contextual inference.
See the
[source profile][source], [IR profile][ir], and [evidence][evidence].

[source]: ../../docs/specs/compiler-subset.md#number-arrays
[ir]: ../../docs/specs/compiler-ir-text.md#array-allocation
[evidence]: ../../docs/investigations/compiler-number-arrays.md
