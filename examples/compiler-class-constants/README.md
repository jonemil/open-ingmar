# Stored class constants

[app.mc](app.mc) checks per-instance storage, String identity, Long/Double
values, Array aliasing and mutation, Dictionary contents, source objects,
runtime initializer calls, application-owned constants, differing class-union
values, and allocation before effectful/conditional constructor arguments. Calling user
`initialize()` again leaves constant storage intact. See [expected.txt](expected.txt).

```sh
mkdir -p scratch/compiler-class-constants
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 5.0.0 \
  --app-id e20d80cce2c14eb09170d456824b4281 --entry MinimalConsoleApp.onStart \
  examples/compiler-class-constants/app.mc scratch/compiler-class-constants/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-class-constants/unsigned.prg \
  scratch/compiler-class-constants/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-class-constants/generated.prg
```

SDK reference: 9.2.0, `fr265`, minimum API 5.0.0, `-O 0`, default checking policy.
Copy the [minimal console scaffold](../minimal-console/README.md) into scratch,
replace its source with this file, and set minimum API 5.0.0 before building
with `scripts/build-example.sh`.
