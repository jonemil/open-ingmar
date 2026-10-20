# Number comparisons and inequality guards

[Comparisons.run](comparisons.mc) checks `<`, `<=`, `>=`, and `!=`, signed Number
boundaries, Number/Char comparisons, Boolean/String identity inequalities,
operand-call order, and a loop. `parsed()` uses `value != null && value <= 17`
to guard arithmetic on String.toNumber's nullable result.
[The app](app.mc) runs the probe from onStart, then exits.
API minimum 2.1.0, target 5.2.0; results go to the console.

From the repository root, with new output paths:

```sh
mkdir -p scratch/compiler-number-comparisons
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 2.1.0 \
  --app-id 53cb2c25c6a94ad68e936ab46ff4ab9e \
  --entry MinimalConsoleApp.onStart \
  examples/compiler-number-comparisons/app.mc \
  examples/compiler-number-comparisons/comparisons.mc \
  scratch/compiler-number-comparisons/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-number-comparisons/unsigned.prg \
  scratch/compiler-number-comparisons/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-number-comparisons/generated.prg
cmp examples/compiler-number-comparisons/expected.txt scratch/simulator/monkeydo.log
```

Expected output is [expected.txt](expected.txt). Each effectful comparison prints
`left` before `right`; parsed inputs produce 34, 0, 0, and the loop ends at 3.
String inequality uses identity rather than content. See
[comparison contracts](../../docs/specs/frontend/statements.md#terminal-conditions)
and [SDK/generated evidence](../../docs/investigations/compiler-number-comparisons.md).
