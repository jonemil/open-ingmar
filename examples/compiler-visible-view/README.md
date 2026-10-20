# Source View drawing

[app.mc](app.mc) constructs the source View in [view.mc](view.mc).
The inherited onUpdate callback receives a native drawing context and renders
white text on black. Coordinates target the Forerunner 265 display.
API minimum: 2.1.0; target: 5.2.0; runtime: SDK 9.2.0 / fr265.

From the repository root, using unused output paths:

```sh
mkdir -p scratch/compiler-visible-view
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 2.1.0 \
  --app-id 49cb2c25c6a94ad68e936ab46ff4ab9e \
  --entry MinimalConsoleApp.onStart \
  examples/compiler-visible-view/app.mc examples/compiler-visible-view/view.mc \
  scratch/compiler-visible-view/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-visible-view/unsigned.prg \
  scratch/compiler-visible-view/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh \
  --prg scratch/compiler-visible-view/generated.prg
```

The watch should display **Hello from Rust**. Console output begins with `start`,
`initial`, `show`, then one or more `update` lines. Update counts depend on View
transitions and session timing. Inspect the screen capture as well as the log;
an active simulator alone does not confirm drawing. See the
[interactive simulator instructions](../../scripts/README.md#use-the-simulator-interactively)
for a GUI session.

The [class contract](../../docs/specs/sdk/README.md#application-callbacks)
requires a zero-argument initializer calling View.initialize first. FONT_MEDIUM
retains its enum type; replacing it with the numeric literal 3 is rejected.
Instance fields, input delegates, and portable layout remain subsequent work.
See [SDK comparison and shutdown evidence](../../docs/investigations/compiler-visible-view.md).
