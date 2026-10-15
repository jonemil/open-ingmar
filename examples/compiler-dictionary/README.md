# Empty Dictionary values

[app.mc](app.mc) checks `{}`, local copies/reassignment, Dictionary arguments and
results, chained `size`/`isEmpty` calls, and repeated construction in a loop.
SDK and generated `fr265` programs match [expected output](expected.txt).
The launch-state guard checks nullable callback typing; this launch supplies Null.

From the repository root, using fresh output paths:

```sh
mkdir -p scratch/compiler-dictionary
cargo run -p ciq-compiler -- --unit --emit prg --target-api 5.2.0 --min-api 2.1.0 --app-id 869aef856dca4c458cf6879ea717883e --entry MinimalConsoleApp.onStart examples/compiler-dictionary/app.mc scratch/compiler-dictionary/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-dictionary/unsigned.prg scratch/compiler-dictionary/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-dictionary/generated.prg
```

Key/value operations and nonempty literals remain planned. See
[the source profile](../../docs/specs/compiler-subset.md#dictionary-values) and
[SDK evidence](../../docs/investigations/compiler-dictionary.md).
