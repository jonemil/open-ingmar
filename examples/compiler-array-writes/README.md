# Number-array element writes

This console specimen checks `array[index] = value;`, alias visibility, operand
calls, a logical right-hand expression, loop writes, and guarded nullable
Number values. SDK 9.2.0 and our generated Forerunner 265 application produce
the same [expected output](expected.txt).

From the repository root, compile, sign, and run:

```sh
mkdir -p scratch/compiler-array-writes
cargo run -p ciq-compiler -- --unit --emit prg --target-api 5.2.0 --min-api 2.1.0 --app-id 53cb2c25c6a94ad68e936ab46ff4ab9e --entry MinimalConsoleApp.onStart examples/compiler-array-writes/app.mc examples/compiler-array-writes/writes.mc scratch/compiler-array-writes/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-array-writes/unsigned.prg scratch/compiler-array-writes/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-array-writes/generated.prg
```

Use fresh output paths on repeats. The first value helper removes an element
before replacement; the copied reference then prints `[20, 99]`. Receiver and
index still run before the second write's logical expression and selected-value
call. Loop writes retain the shortened length, and the guarded conversion
writes Number 41.

Char/View writes use the same contract; see the
[typed-array example](../compiler-typed-array-writes/README.md). Bounds checks
remain with the runtime;
an index equal to the length does not append. See the
[source contract](../../docs/specs/compiler-subset.md#array-element-writes)
and [investigation](../../docs/investigations/compiler-array-writes.md).
