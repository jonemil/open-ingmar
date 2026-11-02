# Namespace values

[app.mc](app.mc) stores and passes source/native module and class-definition
values through locals, shared slots, instance fields, returns, and tuple slots.
It also checks module/global self values, root values, identity comparisons,
member existence, and lazy module initialization. See [expected.txt](expected.txt).

Namespace values are distinct from Lang.Object instances. Module/class Method
construction and static-member dispatch have separate support requirements.

```sh
mkdir -p scratch/compiler-namespace-values
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 3.0.0 \
  --app-id b67c1691c9a5422a9d3202a0c016870c --entry MinimalConsoleApp.onStart \
  examples/compiler-namespace-values/app.mc scratch/compiler-namespace-values/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-namespace-values/unsigned.prg \
  scratch/compiler-namespace-values/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-namespace-values/generated.prg
```

SDK reference: 9.2.0, `fr265`, minimum API 3.0.0, `-O 0`, default checking policy.
Use the [minimal console scaffold](../minimal-console/README.md) with this source
file and minimum API 3.0.0.
