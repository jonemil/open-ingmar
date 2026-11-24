# Native-rooted source construction

[app.mc](app.mc) checks default View/delegate initializers, source-parent
allocation hooks, defaults before argument effects, explicit parent constructor
assignments, and optional, late, conditional, and repeated native base calls.
An omitted child initializer stays empty rather than invoking its parameterized
source parent. The final two lines compare a directly constructed native
Object with itself and with a separate allocation. See [expected.txt](expected.txt).

```sh
mkdir -p scratch/compiler-native-constructors
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 3.0.0 \
  --app-id b67c1691c9a5422a9d3202a0c016870c --entry MinimalConsoleApp.onStart \
  examples/compiler-native-constructors/app.mc scratch/compiler-native-constructors/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-native-constructors/unsigned.prg \
  scratch/compiler-native-constructors/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-native-constructors/generated.prg
```

SDK reference: 9.2.0, `fr265`, minimum API 3.0.0, `-O 0`.
Use the [minimal console scaffold](../minimal-console/README.md) with this source.
