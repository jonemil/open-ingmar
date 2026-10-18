# ByteArray allocation and native methods

[app.mc](app.mc) exercises zero-filled allocation, Number/Char indexed writes,
compound updates, aliases, mutations, search, slicing, reversal, equality,
hashing, Method binding, and live weak references. Conditional allocations
check lazy branches and single evaluation of the length.

API minimum: 3.0.0; target: 5.2.0; SDK/compiler/simulator: 9.2.0 / fr265.

From the repository root, using unused output paths:

```sh
mkdir -p scratch/compiler-byte-arrays
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 3.0.0 \
  --app-id a7086949531941379eb8b38164ed99aa --entry start \
  examples/compiler-byte-arrays/app.mc scratch/compiler-byte-arrays/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-byte-arrays/unsigned.prg \
  scratch/compiler-byte-arrays/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh \
  --prg scratch/compiler-byte-arrays/generated.prg
```

Compare the complete console with [expected.txt](expected.txt). See the
[source contract](../../docs/specs/compiler-subset.md#bytearrays) and
[SDK evidence](../../docs/investigations/byte-arrays.md).

Byte literals, nullable ByteArray references, fields/container elements,
numeric encoding options, and recovery from erased Object results remain
outside this example's supported profile.
