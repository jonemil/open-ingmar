# Contextual empty arrays

[app.mc](app.mc) creates Number and Char arrays through typed returns,
user/native arguments, and local reassignment. It checks alias preservation and
argument evaluation order. API minimum: 1.3.0; target: 5.2.0; runtime: SDK 9.2.0
/ fr265.

From the repository root, using unused output paths:

```sh
mkdir -p scratch/compiler-empty-arrays
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 1.3.0 \
  --app-id 46cb2c25c6a94ad68e936ab46ff4ab9e --entry start \
  examples/compiler-empty-arrays/app.mc scratch/compiler-empty-arrays/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-empty-arrays/unsigned.prg \
  scratch/compiler-empty-arrays/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh \
  --prg scratch/compiler-empty-arrays/generated.prg
```

Expected output:

```text
0
[7]
0
[7]
0
0
0
[x]
0
10
20
30
```

`var a = []` remains ambiguous. A direct literal can receive its element type
from a declared return/argument or an established local. See the
[source contract][source] and [evidence][evidence].

[source]: ../../docs/specs/compiler-subset.md#empty-array-literals
[evidence]: ../../docs/investigations/compiler-empty-arrays.md
