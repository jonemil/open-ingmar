# Object arguments to comparison methods

[app.mc](app.mc) checks numeric, Boolean, Char, and String `compareTo` methods
through Object parameters, numeric alias operands, nullable guards, and broader
`equals` arguments. Source objects, Views, and supported containers use ordinary
runtime dispatch and retain their equality behavior.

API minimum: 5.0.0; target: 5.2.0; SDK/compiler/simulator: 9.2.0 / fr265.

From the repository root, using unused output paths:

```sh
mkdir -p scratch/compiler-comparison-objects
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 5.0.0 \
  --app-id 9820eecda3db4ae19d0d3eeb17d4cbb6 --entry MinimalConsoleApp.onStart \
  examples/compiler-comparison-objects/app.mc scratch/compiler-comparison-objects/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-comparison-objects/unsigned.prg \
  scratch/compiler-comparison-objects/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh \
  --prg scratch/compiler-comparison-objects/generated.prg
```

Compare the complete console with [expected.txt](expected.txt).
Object arguments satisfy the static API contract; an incompatible runtime
comparison type can still throw. See the
[method contract](../../docs/specs/compiler-subset.md#comparison-object-arguments)
and [isolated error evidence](../../docs/investigations/compiler-comparison-objects.md).
