# Source initial View callback

[app.mc](app.mc) supplies AppBase initialization, startup, and `getInitialView`.
An effectful factory constructs the native View returned in its one-slot result.
API minimum: 2.1.0; target: 5.2.0; runtime: SDK 9.2.0 / fr265.

From the repository root, using unused output paths:

```sh
mkdir -p scratch/compiler-initial-view
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 2.1.0 \
  --app-id 48cb2c25c6a94ad68e936ab46ff4ab9e \
  --entry MinimalConsoleApp.onStart examples/compiler-initial-view/app.mc \
  scratch/compiler-initial-view/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-initial-view/unsigned.prg \
  scratch/compiler-initial-view/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh \
  --prg scratch/compiler-initial-view/generated.prg
```

Expected console output:

```text
start
initial
make
```

The application remains running with a blank native View. The capture should
show the watch with no runtime error dialog; active `monkeydo` alone is not
execution evidence. The [visible View example](../compiler-visible-view/README.md)
adds source inheritance and drawing; input handling remains subsequent work.
The [callback contract][source] requires a direct one-slot return literal;
an arbitrary mutable View array is insufficient. See [runtime evidence][evidence].

[source]: ../../docs/specs/compiler-classes.md#source-initial-view-callback
[evidence]: ../../docs/investigations/compiler-initial-view.md
