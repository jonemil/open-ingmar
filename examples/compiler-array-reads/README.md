# Char-array element reads

[app.mc](app.mc) and [arrays.mc](arrays.mc) exercise `array[index]` results,
postfix member chains, typed parameters, Unicode, loop indices, nullable Number
index guards, and array/index effects across ordinary and logical expressions.
API context: 1.3.0; runtime validation: SDK 9.2.0 / fr265.

From the repository root:

```sh
mkdir -p scratch/compiler-array-reads
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 1.3.0 \
  --app-id 1ce4dfaf69a446efb77c161b8fb5d06c --entry start \
  examples/compiler-array-reads/app.mc \
  examples/compiler-array-reads/arrays.mc \
  scratch/compiler-array-reads/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-array-reads/unsigned.prg \
  scratch/compiler-array-reads/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-array-reads/generated.prg
```

Expected output:

```text
A
B
67
A
receiver
index
y
Ω
128512
A
b
C
guarded
b
```

Array and index evaluate once, in that order. The index must be Number and
within array bounds at runtime; out-of-range reads produce a fatal VM error in
the tested profile. String/dictionary indexing, nullable arrays, and
other element types beyond [Number](../compiler-number-arrays/README.md) remain
unsupported. See the [source contract][source] and
[evidence][evidence].

[source]: ../../docs/specs/frontend/README.md#char-arrays
[evidence]: ../../docs/investigations/compiler-array-reads.md
