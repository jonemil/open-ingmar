# Char arrays

[app.mc](app.mc) and [arrays.mc](arrays.mc) exercise String conversion, typed
`Array<Char>` parameters/results, locals, size/string/search methods, empty and
Unicode strings, nullable receiver guards, and one effectful conversion.
API context: 1.3.0; runtime validation: SDK 9.2.0 / fr265.

From the repository root:

```sh
mkdir -p scratch/compiler-char-arrays
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 1.3.0 \
  --app-id f681318df9d94f57a4c72906553ed160 --entry start \
  examples/compiler-char-arrays/app.mc \
  examples/compiler-char-arrays/arrays.mc \
  scratch/compiler-char-arrays/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-char-arrays/unsigned.prg \
  scratch/compiler-char-arrays/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-char-arrays/generated.prg
```

Expected output:

```text
3
[A, b, A]
0
1
-1
0
[]
2
1
1
0
-1
3
[x, y]
[b, c]
effect
3
true
```

`indexOf` requires a Char argument. Convert an array to String before printing.
The [array-read example](../compiler-array-reads/README.md) covers indexing.
Construction, mutation, nullable arrays, and other element
contracts remain unsupported. See the [source contract][source] and [evidence][evidence].

[source]: ../../docs/specs/compiler-subset.md#char-arrays
[evidence]: ../../docs/investigations/compiler-char-arrays.md
