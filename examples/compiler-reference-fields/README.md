# Retaining a View through a reference field

[app.mc](app.mc) keeps a controller in an AppBase field. The
[controller](controller.mc) receives a HelloView constructor argument and stores
it in a field. Its methods update or return that same [View](view.mc), created
by [Views.make](views.mc). Reference fields are initialized before lifecycle use.
API minimum 2.1.0, target 5.2.0; coordinates target Forerunner 265.

From the repository root, using unused output paths:

```sh
mkdir -p scratch/compiler-reference-fields
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 2.1.0 \
  --app-id 53cb2c25c6a94ad68e936ab46ff4ab9e \
  --entry MinimalConsoleApp.onStart \
  examples/compiler-reference-fields/app.mc \
  examples/compiler-reference-fields/controller.mc \
  examples/compiler-reference-fields/view.mc \
  examples/compiler-reference-fields/views.mc \
  scratch/compiler-reference-fields/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-reference-fields/unsigned.prg \
  scratch/compiler-reference-fields/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-reference-fields/generated.prg
```

Expected console prefix:

```text
7
8
start
initial
11
show
12
```

View construction runs during AppBase initialization, before onStart. The controller
adds 3 to its retained View's counter; drawing callbacks increment it further.
Final displayed values depend on session timing. Inspect the screen and log.

The [reference-field contract](../../docs/specs/frontend/fields.md#source-reference-fields)
requires a straight-line initialization prefix and exact non-null source classes.
The controller currently extends View to use the supported native class profile;
it is not installed as a UI delegate. InputDelegate support is the next step.
See [constructor and shutdown evidence](../../docs/investigations/compiler-reference-fields.md).
