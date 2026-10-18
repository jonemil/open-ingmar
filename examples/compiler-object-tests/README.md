# Runtime object typing

[app.mc](app.mc) checks `instanceof` against supported concrete Lang classes,
`has` with literal and computed Symbols, source fields/methods, null handling,
short-circuiting, and operand evaluation order. See [expected output](expected.txt).
Guarded Object reads support Number arithmetic, String methods, nullable inputs,
conditional arms, loops, and numeric comparator sorting. Lexical casts cover
scalar/reference assertions, nullable targets, erasure/recovery, runtime identity,
and one-time operand effects. [source-types.mc](source-types.mc) checks cross-file
source targets, guarded calls, exact return contracts, and Object/View/delegate
projections.

From the repository root:

```sh
mkdir -p scratch/compiler-object-tests
cargo run -p ciq-compiler -- --unit --emit prg --target-api 5.2.0 --min-api 5.0.0 --app-id 123456789abcdef0123456789abcdef0 --entry MinimalConsoleApp.onStart examples/compiler-object-tests/app.mc examples/compiler-object-tests/source-types.mc scratch/compiler-object-tests/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-object-tests/unsigned.prg scratch/compiler-object-tests/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-object-tests/generated.prg
```

See the [source contract](../../docs/specs/compiler-subset.md#runtime-object-predicates)
and [runtime evidence](../../docs/investigations/compiler-object-tests.md).
