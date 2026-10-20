# Source application lifecycle

[app.mc](app.mc) defines AppBase initialization and `onStart(state)`.
The callback copies its opaque nullable Dictionary state, checks it against
null, stores/prints that Boolean decision, and invokes a method that calls
[helpers.mc](helpers.mc). A Boolean local controls a loop and is set to false
after one iteration; equality and printing confirm the resulting value.
The app requests normal termination with `System.exit()` and supplies an
`onStop(state)` callback to verify shutdown and its nullable state.
Constructor and blank View remain generated; this is a console watch app.

From the repository root:

```sh
mkdir -p scratch/compiler-lifecycle
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 1.0.0 \
  --app-id 3c94c9e5ba2c43fa849d64ecff33264e --entry StartupApp.onStart \
  examples/compiler-lifecycle/app.mc examples/compiler-lifecycle/helpers.mc \
  scratch/compiler-lifecycle/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-lifecycle/unsigned.prg \
  scratch/compiler-lifecycle/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-lifecycle/generated.prg
```

Observed console output on SDK 9.2.0 / fr265 simulator:

```text
source initialize
source onStart
true
null state
41
Boolean loop
true
false
source onStop
true
```

Initialization, startup, and shutdown each print once; `monkeydo` exits normally.
This simulator path supplies null at startup and shutdown. A non-null startup
state selects
`dictionary state`; that launch path has not yet been exercised. The compiler
accepts the native Dictionary-or-null contract without dictionary access,
construction, or general object calls. The callbacks' void results are not printed.

See [source callback rules](../../docs/specs/sdk/callbacks.md#source-startup-callback)
and [runtime evidence](../../docs/investigations/compiler-class.md).
The [Boolean experiment](../../docs/investigations/compiler-booleans.md)
records stored decisions and loop behavior.
