# Float and Double values and methods

[app.mc](app.mc) checks both floating widths, decimal/exponent literals,
signed zero, rounding, mixed arithmetic/comparisons, updates, compound writes,
conditional joins, and nullable parsing. Native calls cover absolute value,
formatting, all four numeric conversions, String conversion, equality, hashing,
comparison, and immutable weak results. Effect markers check evaluation order.
SDK/generated `fr265` programs must match [expected output](expected.txt).

From the repository root, using fresh output paths:

```sh
mkdir -p scratch/compiler-float-values
cargo run -p ciq-compiler -- --unit --emit prg --target-api 5.2.0 --min-api 5.0.0 --app-id c8415b64316b43d0a044699622a72c11 --entry MinimalConsoleApp.onStart examples/compiler-float-values/app.mc scratch/compiler-float-values/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-float-values/unsigned.prg scratch/compiler-float-values/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-float-values/generated.prg
```

Floating fields and general union types remain deferred. See the
[source contract](../../docs/specs/frontend/README.md#numbers-and-numeric-aliases).
