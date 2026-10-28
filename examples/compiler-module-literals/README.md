# Module activation through scalar literals

[app.mc](app.mc) and [later.mc](later.mc) check that accessing Number, Float,
Boolean, Char, Symbol, and Null constants runs their module's initializers once.
The trace also covers activation through another initializer, startup access,
class allocation, and a reopened module in a separate file. Same-owner reads
use already available literal defaults without reactivating the module.
See [expected.txt](expected.txt).

```sh
mkdir -p scratch/compiler-module-literals
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 5.0.0 \
  --app-id 870326c7480145afbb0ed80764cb0721 --entry MinimalConsoleApp.onStart \
  examples/compiler-module-literals/app.mc \
  examples/compiler-module-literals/later.mc \
  scratch/compiler-module-literals/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-module-literals/unsigned.prg \
  scratch/compiler-module-literals/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-module-literals/generated.prg
```

SDK reference: 9.2.0, `fr265`, minimum API 5.0.0, `-O 0`, default checking policy.
Use the [minimal console scaffold](../minimal-console/README.md) with both source
files and minimum API 5.0.0. Startup's trace print precedes console attachment;
the captured output begins in `onStart`.
