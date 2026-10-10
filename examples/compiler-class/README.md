# Source class and current-receiver calls

[app.mc](app.mc) declares `Counter extends Application.AppBase`. Its `start`
method calls integer/void methods through `self`, including nested argument
calls and a loop with an early return. [numbers.mc](numbers.mc) deliberately
uses the same `answer` name in global and module scopes; self calls select the
class method, while `Numbers.answer()` selects the module member.

From the repository root, use fresh outputs:

```sh
mkdir -p scratch/compiler-class
cargo run -p ciq-compiler -- --unit --emit prg \
  --app-id 3c94c9e5ba2c43fa849d64ecff33264e --entry Counter.start \
  examples/compiler-class/app.mc examples/compiler-class/numbers.mc \
  scratch/compiler-class/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-class/unsigned.prg \
  scratch/compiler-class/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-class/generated.prg
```

Expected console output, with a blank native View:

```text
class startup
1
2
47
receiver call
3
2
9
```

Changing `left()` from `return 50;` to `return 51;` changes `47` to `48`.
Choosing `--entry Counter.left` instead prints `1` followed by its result `50`.

The compiler generates lifecycle methods that invoke the selected method on the
application instance. Base initialization and the blank View remain generated;
this example does not yet define source `onStart`, `initialize`, or
`getInitialView`. Class fields, arbitrary receivers, object construction,
imports, and source inheritance remain unsupported.

Source, typed IR, and saved unit IR produce the same complete unsigned PRG.
See [class rules](../../docs/specs/compiler-classes.md) and
[simulator evidence](../../docs/investigations/compiler-class.md).
