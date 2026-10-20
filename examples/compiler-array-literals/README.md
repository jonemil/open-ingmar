# Char-array literals

[app.mc](app.mc) constructs nonempty Char arrays from literals, calls, and
logical expressions. It checks indexing, mutation through shared references,
Unicode, typed returns, and fresh construction in a loop. API minimum: 1.3.0;
target: 5.2.0; runtime: SDK 9.2.0 / fr265.

From the repository root, using unused output paths:

```sh
mkdir -p scratch/compiler-array-literals
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 1.3.0 \
  --app-id 1cb67c43ba254fe8a232f943ed131c9d --entry start \
  examples/compiler-array-literals/app.mc \
  scratch/compiler-array-literals/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-array-literals/unsigned.prg \
  scratch/compiler-array-literals/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh \
  --prg scratch/compiler-array-literals/generated.prg
```

Expected output:

```text
[a, b, c]
b
x
y
[x, y]
1
z
2
[p, q]
1
1
1
```

Element calls run left to right, once each. The logical element skips its
right-hand helper, so `unexpected` must not appear. Source literals currently
require homogeneous Char or [Number elements][numbers]; [empty literals][empty]
require a concrete context. Other element types await broader Array profiles.
See the [source contract][source],
[IR allocation][ir], and [evidence][evidence].

[source]: ../../docs/specs/frontend/README.md#char-arrays
[ir]: ../../docs/specs/ir/README.md#array-allocation
[evidence]: ../../docs/investigations/compiler-array-literals.md

[numbers]: ../compiler-number-array-literals/README.md
[empty]: ../compiler-empty-arrays/README.md
