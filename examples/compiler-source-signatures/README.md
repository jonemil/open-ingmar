# Source-class parameters and returns

[views.mc](views.mc) constructs, passes, and updates the source View from
[view.mc](view.mc) using explicit HelloView signatures. [app.mc](app.mc) calls
those module functions and returns the same instance through a typed self method.
API minimum 2.1.0, target 5.2.0; coordinates target Forerunner 265.

From the repository root, using unused output paths:

```sh
mkdir -p scratch/compiler-source-signatures
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 2.1.0 \
  --app-id 52cb2c25c6a94ad68e936ab46ff4ab9e \
  --entry MinimalConsoleApp.onStart \
  examples/compiler-source-signatures/app.mc \
  examples/compiler-source-signatures/view.mc \
  examples/compiler-source-signatures/views.mc \
  scratch/compiler-source-signatures/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-source-signatures/unsigned.prg \
  scratch/compiler-source-signatures/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-source-signatures/generated.prg
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

The factory and pass/echo calls preserve the first instance's identity; the same
counter is updated and displayed. Further drawing callbacks increment it again,
so final values depend on session timing. Inspect the screen as well as the log.

The [signature contract](../../docs/specs/frontend/classes.md#source-class-signatures)
requires exact non-null source classes. `as HelloView` retains identity across
calls; `as Toybox.WatchUi.View` exposes the generic base contract. Neither is a
runtime cast. Object-reference fields and constructor arguments remain next work.
See [SDK comparison evidence](../../docs/investigations/compiler-source-signatures.md).
