# Long values, operators, and native methods

[app.mc](app.mc) checks decimal Long literals with `l`/`L`, zero, signed
boundaries, values above 32 bits, local copies/reassignment, module/instance
arguments and results, discarded calls, repeated loads, and printing.
Arithmetic adds mixed Number/Long promotion in both orders, signed division and
remainder, unary signs, comparisons, compound writes, overflow boundaries, and
operand effects through calls and a branching right operand. Bitwise tests
cover both mixed operand orders, full-width inversion, sign-preserving shifts,
count boundaries/wrapping, statement updates, and compound writes. SDK and
generated `fr265` programs match [expected output](expected.txt).

The native group covers Long absolute value, formatting, Number narrowing,
String conversion, identity conversion, equality, hashing, comparison, and
immutable weak results. It also checks Number widening and nullable String-to-Long
parsing with explicit bases, overflow, prefix parsing, and local null proofs.

From the repository root, using fresh output paths:

```sh
mkdir -p scratch/compiler-long-values
cargo run -p ciq-compiler -- --unit --emit prg --target-api 5.2.0 --min-api 5.0.0 --app-id 7b9aef856dca4c458cf6879ea717883e --entry MinimalConsoleApp.onStart examples/compiler-long-values/app.mc scratch/compiler-long-values/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-long-values/unsigned.prg scratch/compiler-long-values/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-long-values/generated.prg
```

Float/Double conversions, Long container elements, and Long fields remain
deferred. See the
[source contract](../../docs/specs/compiler-subset.md#long-values).
