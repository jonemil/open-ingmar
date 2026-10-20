# Calls through source object references

[app.mc](app.mc) constructs two instances of [view.mc](view.mc), copies a
reference, calls value/Void instance methods, and reassigns the copy to the second
instance. The first remains the displayed View. API minimum 2.1.0, target 5.2.0;
coordinates target Forerunner 265.

From the repository root, using unused output paths:

```sh
mkdir -p scratch/compiler-object-calls
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 2.1.0 \
  --app-id 51cb2c25c6a94ad68e936ab46ff4ab9e \
  --entry MinimalConsoleApp.onStart \
  examples/compiler-object-calls/app.mc examples/compiler-object-calls/view.mc \
  scratch/compiler-object-calls/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-object-calls/unsigned.prg \
  scratch/compiler-object-calls/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-object-calls/generated.prg
```

Expected console prefix:

```text
start
initial
7
8
7
8
11
9
10
11
show
12
```

The initialization pairs establish independent state. The shared reference first
updates the first instance to 11, then refers to the second instance and updates
it to 10. The first stays at 11 until its drawing callbacks increment it again.
Inspect the displayed number; update counts depend on session timing.

The [source-call profile](../../docs/specs/frontend/README.md#calls-on-constructed-source-objects)
requires a proven constructor class and preserves it through local copies and
same-class reassignment. Generic View parameters/returns do not authorize source
dispatch. Object-reference fields, constructor arguments, and input delegates
remain subsequent work. See [SDK comparison evidence](../../docs/investigations/compiler-object-calls.md).
