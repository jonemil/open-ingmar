# Source application lifecycle

[app.mc](app.mc) defines AppBase initialization and `onStart(state)`.
The callback copies its opaque nullable Dictionary state, checks it against
null, and invokes a method that calls [helpers.mc](helpers.mc).
Constructor and blank View remain generated; this is a console watch app.

From the repository root:

```sh
mkdir -p scratch/compiler-lifecycle
cargo run -p ciq-compiler -- --unit --emit prg \
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
null state
41
```

Initialization and startup each print once. A non-null state selects
`dictionary state`; that launch path has not yet been exercised. The compiler
accepts the native Dictionary-or-null contract without dictionary access,
construction, or general object calls. The callback's void result is not printed.

See [source callback rules](../../docs/specs/compiler-classes.md#source-startup-callback)
and [runtime evidence](../../docs/investigations/compiler-class.md).
