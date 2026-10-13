# Constructor arguments and retained source fields

SDK 9.2.0 and generated fr265 applications retain a controller/View reference
chain across lifecycle callbacks and display matching counter values. Checked
2026-10-13 using [the four-file example][example]. Scope: source constructor
arguments, null reference slots filled during initialization, typed field reads/
writes, and retained references. InputDelegate behavior and GC/failure guarantees
are not established by this experiment.

## Encoding and behavior

**Experimentally observed:** the SDK accepts `var target as HelloView;`, then
`self.target = view;` in a parameterized source initializer. Our compiled source
PRG records the field with key bits zero, type `0x00`, and value zero. It reads
the field with getselfv and writes it with getself/spush/value/putv. The initial
probe export is retained under `scratch/compiler-reference-fields/sdk-export/`.

The SDK constructor sequence selects ViewController, allocates it, duplicates
the instance, selects initialize, duplicates its receiver, evaluates the source
argument, and invokes with count two before discarding the initializer result.
The argument includes a source call. Our lowering uses that expanded sequence;
invocation count is the declared argument count plus one. Native View's base
constructor continues to receive no user arguments.

The final example initializes AppBase.controller with a ViewController holding
Views.make's HelloView. Both sessions log `7, 8, start, initial, 11, show, 12, 13`.
Captures display 13 without an error dialog. The View returned through the
controller is the one whose counter the controller updated. Update counts vary
with session timing; the full retained logs match in these sessions.

A shutdown variant adds System.exit after drawing in onUpdate. Both builds log
`7, 8, start, initial, 11, show, 12, stop, 12`. The final read goes through
AppBase.controller and its target field in onStop, after the initial-view callback
has returned. This establishes usable retained references across these callbacks,
not all garbage-collection or abnormal-construction behavior.

**Checking policy:** reference slots are non-null only after source initialize
assigns them. Our compiler requires those assignments before reads, self calls,
control-flow edges, returns, or calls that may request shutdown. Source exit
effects propagate through module/self/object calls and constructors, including
conditional/recursive call graphs. This avoids exposing uninitialized fields to
shutdown callbacks; it is a conservative subset rule, not a reconstructed SDK
definite-initialization algorithm. Runtime failures remain outside that proof.

Constructor argument effects precede source initialize. Allocation occurs before
ordinary expression arguments in the observed SDK sequence and our lowering.
Arguments requiring staged control flow/array construction may be evaluated into
locals before allocation. Allocation-failure timing and memory equivalence are
not established; ordinary argument order and state effects remain checked.

## Reproduction

Build/sign our application using the example's commands. For the SDK, create
`scratch/compiler-reference-fields/final.jungle`:

```text
project.manifest = ../native-names/manifest.xml
base.sourcePath = ../../examples/compiler-reference-fields
base.resourcePath = ../native-names/resources
```

```sh
CIQ_JUNGLE_FILES="$PWD/scratch/compiler-reference-fields/final.jungle" \
  sh scripts/build-example.sh scratch/compiler-reference-fields
xvfb-run -a -s '-screen 0 1280x1024x24' \
  python3 scripts/simulator-session.py --prg <program.prg> --observe-seconds 15
```

Run each PRG separately. Save `scratch/simulator/monkeydo.log` and `screen.png`
after each run; compare callback values and inspect the displayed counter.
Normal retained logs/captures are `sdk.log`, `generated.log`, `sdk-screen.png`,
and `generated-screen.png` under `scratch/compiler-reference-fields/`.

For shutdown, copy the four sources into its `shutdown/` subdirectory and append
System.exit after drawText. Use sourcePath `.` and manifest/resource paths
`../../native-names/...`; build/sign into new outputs and run separately.
Retained shutdown logs are under `scratch/compiler-reference-fields/shutdown/`.
The probe export inspects our SDK-compiled source PRG, not SDK implementation.

## Input delegate follow-up

Official documentation reviewed 2026-10-13: [BehaviorDelegate] extends
InputDelegate. Its `onSelect()` callback takes no arguments, returns Boolean
(`true` means handled), and is available since API 1.2.0. Start/Enter or a tap
typically triggers selection. This makes it a useful first delegate probe
without requiring KeyEvent handling. Target-specific input dispatch and native
class metadata remain to be tested; this is a proposed experiment, not runtime
evidence. See the [plan](../plan.md#next-work) for implementation sequencing.

[example]: ../../examples/compiler-reference-fields/README.md
[BehaviorDelegate]: https://developer.garmin.com/connect-iq/api-docs/Toybox/WatchUi/BehaviorDelegate.html
