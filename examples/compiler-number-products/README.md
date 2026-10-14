# Number products, division, and remainder

[Products.run](products.mc) checks signed arithmetic, precedence, associativity,
left-to-right operand effects, and representative multiplication overflow.
[The app](app.mc) runs it from onStart, then exits. Results go to the console;
there is no custom View or drawing. API minimum 2.1.0, target 5.2.0.

From the repository root, using new output paths:

```sh
mkdir -p scratch/compiler-number-products
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 2.1.0 \
  --app-id 53cb2c25c6a94ad68e936ab46ff4ab9e \
  --entry MinimalConsoleApp.onStart \
  examples/compiler-number-products/app.mc \
  examples/compiler-number-products/products.mc \
  scratch/compiler-number-products/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-number-products/unsigned.prg \
  scratch/compiler-number-products/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-number-products/generated.prg
```

Expected output is exactly [expected.txt](expected.txt); each effectful operand
pair prints `left` before `right`. In the simulator environment, check it with:

```sh
cmp examples/compiler-number-products/expected.txt scratch/simulator/monkeydo.log
```

Zero-divisor and minimum-Number/-1 failure probes are separate so they cannot
hide later results. See [contracts](../../docs/specs/compiler-subset.md#arithmetic-and-lowering-contract)
and [SDK/generated evidence](../../docs/investigations/compiler-number-products.md).
