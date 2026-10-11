# Class ownership and current-receiver calls

2026-10-10. Question: can the frontend preserve class/method ownership, emit
current-receiver calls, and run them on an independently generated application
instance? Environment: SDK 9.2.0, Java 17, Forerunner 265, default simulator
memory, our developer key. See [source/IR profile][profile].

## SDK reference

An ignored copy of the earlier SDK reference lives at
`scratch/compiler-class/sdk-reference/`. Its application class has:

```monkeyc
function onStart(state) { self.start(); }
function start() {
    System.println(self.answer(50, 3));
    self.announce();
}
function answer(left, right) { return left - right; }
function announce() { System.println("done"); }
```

It also includes ordinary AppBase initialization and a native blank View. Build
with `sh scripts/build-example.sh scratch/compiler-class/sdk-reference fr265`;
inspect with `ciq-dis code` at its generated build PRG. Optimization is zero.
No external implementation code was copied.

**Observed SDK CODE**:

- onStart at `0x24`: `argc 2; getselfv start; invokemz; popv; return`.
- start at `0x56`: the print's argument selects `answer` with `getselfv` at
  `0x62`, then `frpush`, `50`, `3`, `invokem 3`. No explicit receiver value is
  pushed. The subsequent print uses the existing counted-call convention.
- The no-argument announce statement uses `getselfv` at `0x71`, then
  `invokemz; popv`.
- answer begins `argc 3`; left/right parameters use slots 1 and 2 before `subv`.

This supports the existing hidden-context slot and a distinct self-call
selection operation. It does not establish arbitrary object representation,
field operations, inherited member lookup, or source superclass behavior.

## Independent generation and runtime

The [two-file example][example] declares an AppBase class with methods and a
module/global function with the same `answer` name. The compiler preserves class
ownership, resolves self calls within it, and emits owner-qualified labels.
DATA binds methods to the source class's generated application record. Generated
onStart invokes `Counter.start` with receiver lookup; base initialization and
the native View remain supplied by the bootstrap.

**Simulator observed**, after own-key signing:

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

The trace establishes nested left-to-right self calls, ordered subtraction,
void statement calls, a loop with early return, and a module call from a method.
Self selects the class answer body; the module's same-name method returns `9`.
The global same-name body returning `7` remains separate. Tests also verify
these methods share a member ID but have distinct DATA CODE targets.

Changing only `left()` from return `50` to `51` changes `47` to `48`; the rest of
the trace is unchanged. Selecting `Counter.left` as the integer startup entry
prints `1` followed by `50`, verifying the wrapper's automatic result print for
instance calls. All three eight-second sessions leave the blank View active
without an error dialog. Console traces establish execution; screen captures
alone do not. Captures are under ignored `scratch/compiler-class/simulator-47/`,
`simulator-48/`, and `simulator-integer-50/`.

Source and saved unit IR produce identical complete unsigned PRGs. Signed PRG
export/reassembly is byte-identical and its signatures verify. The `50` → `51`
edit changes exactly one unsigned CODE byte, offset `204` (`0xcc`); every other
unsigned payload is unchanged. Signing changes signatures as expected.

| Entry/input | DATA | CODE | RESOURCES | Signed PRG |
|---|---|---|---|---|
| Counter.start, 47/48 | 274 | 317 | 20 | 2324 |
| Counter.left, 50 | 274 | 329 | 8 | 2324 |

Sizes are bytes. Signed SHA-256:

```text
47 d713217dc74580e288aadd3bc07e95317e12fb62a3247866a8b1db6d83bcb62d
48 35421855e5a64c9b97e5469a5f52a706378b9aa2c525efea92f4d477456f59f7
50 0ed641d9574ecb9764284ab84ea6b87adf1c750306d2f95bd51bdaaebb3801af
```

## Reproduction and limits

Use the [example commands][example] to build, sign, and run fresh outputs. Save
and resume the same unit, keeping entry selection:

```sh
target/debug/ciq-compiler --unit --emit ir \
  examples/compiler-class/app.mc examples/compiler-class/numbers.mc \
  scratch/compiler-class/unit.ciqir
target/debug/ciq-compiler --unit --from ir --emit prg \
  --app-id 3c94c9e5ba2c43fa849d64ecff33264e --entry Counter.start \
  scratch/compiler-class/unit.ciqir scratch/compiler-class/from-ir.prg
cmp scratch/compiler-class/unsigned.prg scratch/compiler-class/from-ir.prg
target/debug/ciq-dis all scratch/compiler-class/generated.prg \
  scratch/compiler-class/roundtrip-project
target/debug/ciq-asm project scratch/compiler-class/roundtrip-project \
  scratch/compiler-class/roundtrip.prg
cmp scratch/compiler-class/generated.prg scratch/compiler-class/roundtrip.prg
target/debug/ciq-asm verify scratch/compiler-class/roundtrip.prg \
  -y keys/developer_key.der
```

Eight tests cover source/typed/text parity, receiver ownership, name collisions,
class framing/limits, exact arity and integer/void signatures, effect order,
method fields and distinct owner targets, entry selection, and CLI output
preservation. Tests use no SDK files or key.

General objects, fields, native inherited self calls, and multiple application
classes remain unverified. [The plan](../plan.md) covers native-base calls and
source initialization. Other devices and real-watch execution remain unverified.

## Native name resolution

2026-10-11. Question: which supported source spellings identify the native
Application superclass and System module? Public
[Objects and Memory][objects] distinguishes `using` module names from `import`
class names. The frontend's [bounded name profile][names] retains that
distinction without putting VM symbol/import indexes in the name environment.

**Experimentally observed:** SDK 9.2.0, Java 17, fr265 accepted both
`import Toybox.Application;` with `extends AppBase`, and
`using Toybox.Application as App;` with `extends App.AppBase`. In both specimens
a class-local `using Toybox.System as Sys;` permits `Sys.println`; fully
qualified `Toybox.System.println` and `new Toybox.WatchUi.View()` also compile.
Changing the import specimen to `using Toybox.Application;` fails with
`Cannot resolve super class name 'AppBase'`.

Reproduce with an ignored copy of `examples/minimal-console` and replace its
source with the following import specimen. Keep its manifest/resources and
Jungle path settings. Build with `sh scripts/build-example.sh <copy> fr265`.

```monkeyc
import Toybox.Application;
class MinimalConsoleApp extends AppBase {
    using Toybox.System as Sys;
    function initialize() { AppBase.initialize(); }
    function onStart(state) {
        Sys.println("native names");
        Toybox.System.println(41);
    }
    function getInitialView() { return [new Toybox.WatchUi.View()]; }
}
```

For the alias specimen replace the first line with
`using Toybox.Application as App;`, superclass with `App.AppBase`, and
initializer call with `App.AppBase.initialize()`. For the rejection specimen
change only the import to `using Toybox.Application;`.
Local inputs/logs are under `scratch/native-names/`; each variant uses a Jungle
file selecting its own source. SDK compilation success establishes name
acceptance, not simulator execution or a native dispatch encoding.

**Implementation check:** canonical, qualified, aliased, and imported versions
of our small source class produce identical complete unsigned PRGs. Each also
agrees with saved-unit-IR resumption. Scope tests distinguish class/module-local
native aliases from user modules of the same name and check source diagnostics.
Bindings are file-local in our supported profile; cross-file native binding
visibility and alias shadowing rules in the full language are not established
by these specimens. The VM backend and generated lifecycle remain unchanged.

## Native superclass initialization

2026-10-11. Question: can source initialization call the native superclass
without virtual self dispatch, and replace the generated initializer target?
SDK 9.2.0 / Java 17 / fr265; specimens are the native-name sources above.

**Observed SDK CODE:** the bare `AppBase.initialize()` specimen selects
`getselfv AppBase; getsv initialize; invokemz; popv`. The qualified
`App.AppBase.initialize()` specimen instead selects
`getmv Toybox_Application AppBase; getsv initialize; invokemz; popv`.
These differ from source `self` calls, which select the member directly with
`getselfv`, and module calls, which select `getmv module member`. The
constructor's `<init>` member is distinct from `initialize`.

Rebuild the `import.jungle` and `alias.jungle` variants with
`CIQ_JUNGLE_FILES=<absolute-jungle-path> sh scripts/build-example.sh scratch/native-names`.
Inspect each generated PRG with `cargo run -p ciq-dis -- code <prg>` before
the next build overwrites it. Local CODE listings are
`scratch/native-names/sdk-import-code.txt` and `sdk-code.txt`.

**Interpretation:** resolve supported superclass spellings to a native class
identity, then lower the initializer as a separate ordered effect. Our backend
uses the inherited-base lookup for all spellings; reproducing the SDK's source
spelling-dependent instruction sequence is not required. This establishes only
the AppBase initializer, not arbitrary native members or superclass dispatch.

**Observed simulator execution:** the independently generated source-class app
prints `source initialize` once before `class startup`, followed by
`1`, `2`, `47`, `receiver call`, `3`, `2`, `9`, without a runtime error. The
source initializer calls the native base and owns the only initialize DATA
method record. Native and saved-unit-IR builds have identical unsigned PRGs.
The [example instructions][example] reproduce compilation/signing/execution.
Local PRGs are `scratch/native-names/initialize-unsigned.prg` and
`initialize.prg`; the captured console is `initialize-monkeydo.log` there.

The simulator used default memory and our developer key. Constructor, startup
wrapper, and blank View remain generated. Callback state types, source
`onStart`, and other devices remain unverified.

## Source startup and nullable state

2026-10-11. Question: can source onStart receive the native state value without
an integer parameter assumption, and run after source initialization?

**Officially documented:** [AppBase.onStart][appbase] takes a Dictionary or null
and returns void. Its launch-state prose describes an empty Dictionary without
an Intent. Keep the nullable signature; a normal simulator launch need not
demonstrate a dictionary-backed launch. API level 1.0.0 is documented.

**Observed SDK CODE:** SDK 9.2.0 / Java 17 / fr265 / `-O 0`, using our minimal
console scaffold with this body:

```monkeyc
function onStart(state) {
    var missing = null;
    if (state == missing) { System.println("null state"); }
    else { System.println("dictionary state"); }
}
```

At CODE+`0x24`: `argcincsp 2 1; npush; lputv 2; lgetv 1; lgetv 2; eq; bf ...`.
The null push is byte `0x2c` with no operand. The explicit state occupies slot 1;
the hidden context occupies slot 0. A direct `state == null` specimen instead
uses an unimplemented `0x20` instruction; its decoding is deferred. The frontend
does not require that optimization and uses null plus ordinary equality.
Local source/Jungle/logs are `scratch/native-names/startup.*` and
`startup-sdk.log`; build with the corresponding absolute Jungle path as above.

**Observed simulator execution:** the independent two-file
[lifecycle example][lifecycle-example] prints `source initialize`,
`source onStart`, `null state`, and `41`, each once in that order, without a
runtime error. This verifies source callback selection, null handling, locals,
self/module calls, and the void callback result. It does not verify a non-null
launch state or dictionary access. Default simulator memory and our developer
key were used. Native and saved-IR PRGs agree; callback signature/type checks
also reject integer operations on state and invalid resumed signatures.

The example documents compilation/signing/simulator commands. Local PRGs are
`scratch/native-names/startup-unsigned.prg` and `startup.prg`; captured console
is `startup-monkeydo.log` there. Constructor and blank View remain generated.
Resource generation, additional callbacks, and real-watch execution remain
unverified.

## Normal exit and shutdown callback

Reviewed 2026-10-11. Public [System documentation][system-exit] defines
zero-argument `exit()` returning Void since API 1.0.0; [AppBase.onStop][app-stop]
takes Dictionary-or-null and returns Void at the same level. Its parameter notes
describe an empty Dictionary for ordinary termination, while the declared type
also permits null.

Reuse the minimal-console manifest/resources under `scratch/lifecycle-stop/`
with our own source AppBase: `onStart(state)` prints a marker and calls
`System.exit()`; `onStop(state)` prints a shutdown marker and `state == missing`
with local `missing = null`. Keep the ordinary initializer and generated View
source scaffold. Build with `sh scripts/build-example.sh scratch/lifecycle-stop`
and export `build/fr265/lifecycle-stop.prg` with `ciq-dis all` into a fresh
scratch directory. No compiler/SDK implementation is inspected.

**Observed SDK 9.2.0 / fr265:** exit uses `getmv @Toybox_System @exit`,
`invokemz`, `popv`; its stored symbol is `0x0080006d`. The shutdown callback's
stored symbol is `0x008002a1`. Our catalogue stores their contracts separately
from those executable bindings.

**Observed independent app:** the [lifecycle example][lifecycle-example] prints
`source onStop` and `true` after its startup/Boolean sequence, then monkeydo
exits with code 0. Startup and shutdown each execute once. Thus this exit path
supplies null to onStop, despite the parameter notes' Dictionary description;
other termination states and Intent payloads remain unverified. Exiting during
startup does not establish initial-View execution. Native and resumed unit-IR
PRGs match; tests reject bad signatures, user calls to callbacks, wrong exit
arity, and reordered effectful IR.

[system-exit]: https://developer.garmin.com/connect-iq/api-docs/Toybox/System.html#exit-instance_function
[app-stop]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Application/AppBase.html#onStop-instance_function
[profile]: ../specs/compiler-classes.md
[example]: ../../examples/compiler-class/README.md
[objects]: https://developer.garmin.com/connect-iq/monkey-c/objects-and-memory/
[names]: ../specs/compiler-subset.md#native-name-declarations
[appbase]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Application/AppBase.html#onStart-instance_function
[lifecycle-example]: ../../examples/compiler-lifecycle/README.md
