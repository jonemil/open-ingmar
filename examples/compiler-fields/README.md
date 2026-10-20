# Number instance fields

[app.mc](app.mc) constructs two instances of [view.mc](view.mc). Each starts
with its own counter at 7, then its initializer calls bump and prints 8.
The displayed instance increments again in onUpdate and draws its value.
Coordinates target Forerunner 265; API minimum 2.1.0, target 5.2.0.

From the repository root, using unused output paths:

```sh
mkdir -p scratch/compiler-fields
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 2.1.0 \
  --app-id 50cb2c25c6a94ad68e936ab46ff4ab9e \
  --entry MinimalConsoleApp.onStart \
  examples/compiler-fields/app.mc examples/compiler-fields/view.mc \
  scratch/compiler-fields/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-fields/unsigned.prg \
  scratch/compiler-fields/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-fields/generated.prg
```

Expected console prefix:

```text
start
initial
7
8
7
8
show
9
```

Later updates print increasing values. The two `7, 8` pairs check that modifying
the first instance does not change the second instance's initial value. Inspect
the displayed number too; update counts vary with session timing.

The [field contract](../../docs/specs/frontend/README.md#instance-fields)
currently supports literal-initialized Number fields and explicit self access.
General object-member calls, object-reference fields, and input delegates are
subsequent work. See [SDK comparison evidence](../../docs/investigations/compiler-fields.md).
