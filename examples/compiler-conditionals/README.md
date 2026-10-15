# Conditional expressions

[app.mc](app.mc) checks lazy arms, right association, precedence, ordered call
arguments, nullable joins/guards, shared arrays, empty-array context, staged
array elements, and loops. SDK and generated `fr265` programs match
[expected output](expected.txt).

From the repository root, using fresh output paths:

```sh
mkdir -p scratch/compiler-conditionals
cargo run -p ciq-compiler -- --unit --emit prg --target-api 5.2.0 --min-api 2.1.0 --app-id 869aef856dca4c458cf6879ea717883e --entry MinimalConsoleApp.onStart examples/compiler-conditionals/app.mc scratch/compiler-conditionals/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-conditionals/unsigned.prg scratch/compiler-conditionals/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-conditionals/generated.prg
```

See [source syntax](../../docs/specs/compiler-subset.md#conditional-expressions),
[typed IR joins](../../docs/specs/compiler-ir-text.md#expression-joins), and
[SDK evidence](../../docs/investigations/compiler-conditionals.md).
