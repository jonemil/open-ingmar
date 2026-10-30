# Non-Number enum values

[app.mc](app.mc) checks anonymous initializer types using scalar, container,
Null, and source-object values. Class literal defaults
are available before allocation; their Array keeps mutations across allocations.
Computed members initialize on allocation. See [expected.txt](expected.txt).

```sh
mkdir -p scratch/compiler-enum-values
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 5.0.0 \
  --app-id 2fcb5a7c8c334a009e948607ddc8fdc5 --entry MinimalConsoleApp.onStart \
  examples/compiler-enum-values/app.mc scratch/compiler-enum-values/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-enum-values/unsigned.prg \
  scratch/compiler-enum-values/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-enum-values/generated.prg
```

SDK reference: 9.2.0, `fr265`, minimum API 5.0.0, `-O 0`, default checking policy.
Use the [minimal console scaffold](../minimal-console/README.md) with this source
and minimum API 5.0.0. Named non-Number enums await backing-type support; this specimen covers
anonymous members.
