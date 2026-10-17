# Default Array sorting

[app.mc](app.mc) sorts Number, Long, Float, Double, String, Boolean, and Char
Arrays with `sort(null)`. It checks shared mutation, independent copies,
receiver evaluation, repeated sorting, duplicates, and empty/singleton inputs.
The [expected output](expected.txt) matches SDK/generated `fr265` programs.

From the repository root:

```sh
mkdir -p scratch/compiler-array-sort
cargo run -p ciq-compiler -- --unit --emit prg --target-api 5.2.0 --min-api 5.0.0 --app-id 949ff3758b28492cb54821459b9c90fa --entry MinimalConsoleApp.onStart examples/compiler-array-sort/app.mc scratch/compiler-array-sort/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-array-sort/unsigned.prg scratch/compiler-array-sort/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-array-sort/generated.prg
```

Sorting requires API 5.0.0 and returns Void. Custom Comparator objects and
nullable/object elements remain unsupported. See the
[source contract](../../docs/specs/compiler-subset.md#default-array-sorting).
