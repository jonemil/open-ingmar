# Input-driven counter

[The app](app.mc) returns `[view, delegate]` from getInitialView. Its
[BehaviorDelegate](delegate.mc) retains the [CounterView](view.mc), increments
its Number field on selection, requests a redraw, and returns `true`.
Drawing reads the counter without changing it. API minimum 2.1.0, target 5.2.0;
fixed coordinates target Forerunner 265.

From the repository root, with new output paths:

```sh
mkdir -p scratch/compiler-input-counter
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 2.1.0 \
  --app-id 53cb2c25c6a94ad68e936ab46ff4ab9e \
  --entry MinimalConsoleApp.onStart \
  examples/compiler-input-counter/app.mc \
  examples/compiler-input-counter/delegate.mc \
  examples/compiler-input-counter/view.mc \
  scratch/compiler-input-counter/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-input-counter/unsigned.prg \
  scratch/compiler-input-counter/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --interactive \
  --prg scratch/compiler-input-counter/generated.prg
```

The display starts at 7. Tap the watch display or use its Start/Stop selection
button: each selection increments it once. Console output includes `select`,
the new value, then `draw` and that value. Startup/periodic redraws may repeat
`draw` without changing the counter.

For an automated three-tap check in the configured simulator environment:

```sh
xvfb-run -a -s '-screen 0 1280x1024x24' \
  python3 scripts/check-input-counter.py \
  --prg scratch/compiler-input-counter/generated.prg \
  --output-dir scratch/compiler-input-counter/checked
```

The check requires a fresh Xvfb display without a window manager and the default
Forerunner 265 skin placement. It checks selection/redraw values 8, 9, 10 and
retains logs plus a screenshot; inspect the image to confirm the visible 10.
The helper is specific to this example. Use a new output directory per run.
See [contracts](../../docs/specs/sdk/README.md#source-behavior-delegate)
and [SDK/generated evidence](../../docs/investigations/compiler-input-counter.md).
