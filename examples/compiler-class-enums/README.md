# Class enums

[app.mc](app.mc) and [types.mc](types.mc) exercise anonymous and named class enums,
class/instance reads, literal defaults before allocation, runtime initialization,
and cross-file nominal types. A second allocation updates the value seen through
both instances and the class. Calling `initialize()` directly does not repeat
the enum initializer. See [expected.txt](expected.txt).

```sh
mkdir -p scratch/compiler-class-enums
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 5.0.0 \
  --app-id 5dd5f2b708274779b3501baceacdb41b --entry MinimalConsoleApp.onStart \
  examples/compiler-class-enums/app.mc examples/compiler-class-enums/types.mc \
  scratch/compiler-class-enums/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-class-enums/unsigned.prg \
  scratch/compiler-class-enums/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-class-enums/generated.prg
```

SDK reference: 9.2.0, `fr265`, minimum API 5.0.0, `-O 0`, default checking policy.
Use the [minimal console scaffold](../minimal-console/README.md) with both source
files and minimum API 5.0.0.
