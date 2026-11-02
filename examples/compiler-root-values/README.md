# Root-qualified values

[app.mc](app.mc) checks explicit global-root lookup through scalar constants,
shared variables, nested modules, stored objects, indexed mutation, class enums,
native names, and absolute type/constructor/class-test paths. Root value paths
bypass local and enclosing namespace names while preserving storage and argument
evaluation order. See [expected.txt](expected.txt).

```sh
mkdir -p scratch/compiler-root-values
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 3.1.0 \
  --app-id 5a1e3921a67e49cdb267cb392cad299f --entry MinimalConsoleApp.onStart \
  examples/compiler-root-values/app.mc scratch/compiler-root-values/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-root-values/unsigned.prg \
  scratch/compiler-root-values/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-root-values/generated.prg
```

SDK reference: 9.2.0, `fr265`, minimum API 3.1.0, `-O 0`, default checking policy.
Use the [minimal console scaffold](../minimal-console/README.md) with this source
file and minimum API 3.1.0.
