# Scalar expression initialization

[app.mc](app.mc) checks ordered Number/Float arithmetic, scalar comparisons,
forward Null snapshots, and alias/self-read cycles in globals and modules.
Native constants retain NumberFormat/Endian types when passed to ByteArray
encoding and decoding. See [expected.txt](expected.txt).

```sh
mkdir -p scratch/compiler-scalar-initializers
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 5.0.0 \
  --app-id 2af6def0d6bc4e6eacaf521ae0446721 --entry MinimalConsoleApp.onStart \
  examples/compiler-scalar-initializers/app.mc \
  scratch/compiler-scalar-initializers/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-scalar-initializers/unsigned.prg \
  scratch/compiler-scalar-initializers/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-scalar-initializers/generated.prg
```

SDK reference: 9.2.0, `fr265`, minimum API 5.0.0, `-O 0`, default checking policy.
Use the [minimal console scaffold](../minimal-console/README.md) with this source
and minimum API 5.0.0.
