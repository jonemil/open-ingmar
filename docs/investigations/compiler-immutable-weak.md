# Immutable-value weak results

Question: how should `Object.weak()` be typed for Number, String, and Boolean?
Reviewed 2026-10-11 against [Garmin's Object API][object]. Runtime experiments:
SDK/compiler/simulator 9.2.0, fr265, optimization disabled.

**Documented:** the signature returns WeakReference from API 1.2.0, but a note
says Number, Float, Long, Double, Boolean, and String return their values.
**Observed:** Number, String, and Boolean results behave as their original value
types in the simulator. **Design interpretation:** preserve the generic result
and specialize these three exact receiver paths. Separate probes extend that
evidence to [Long](compiler-long-values.md),
[Float/Double](compiler-floating-values.md), and
[Char](compiler-wide-operators.md). Ordinary object weak references are not
established by these immutable-value probes.

## Static-checker discrepancy

An ignored `probe.mc` initially printed `41.weak()`, `"hello".weak()`,
`true.weak()`, and `false.weak()`, then consumed results through arithmetic,
String case conversion, and a condition. Build with the normal helper and an
AppBase wrapper whose `onStart(state)` calls `start()` and whose initial View is
`[new Toybox.WatchUi.View()]`:

```sh
sh scripts/build-example.sh scratch/compiler-immutable-weak
```

**Observed:** the default SDK checker rejects adding Number to
`WeakReference<Number>` and cannot find `toUpper` on `WeakReference<String>`.
An explicit cast permits each intended consumer:

```monkeyc
System.println((41.weak() as Toybox.Lang.Number) + 1);
System.println(("hello".weak() as Toybox.Lang.String).toUpper());
if (true.weak() as Toybox.Lang.Boolean) { System.println("condition"); }
```

The casted probe printed `41`, `42`, `hello`, `HELLO`, `true`, `false`, and
`condition`. Generated CODE uses `getsv @weak` and `invokemz`; the casted String
consumer follows with `getsv @toUpper` and `invokemz`. No allocation or cast
instruction appears between those calls. SYMBOLS assigns `weak` the native ID
`0x00800418`. These are observations of our compiled specimen, not inspection
of Garmin's compiler implementation.

## Complete comparison

Use the [complete example][example] without casts. Create an ignored
`group.jungle` selecting its two sources, the same AppBase wrapper, and local
fr265 manifest/resources. After the helper prepares SDK user-home/device links,
compile with the documented `-l 0` option to disable SDK type checking:

```sh
java \
  -Duser.home=/workspace/garmin/scratch/compiler-immutable-weak/build/sdk-user-home \
  -classpath connectiq/bin/monkeybrains.jar \
  com.garmin.monkeybrains.Monkeybrains \
  -f scratch/compiler-immutable-weak/group.jungle -d fr265 \
  -o scratch/compiler-immutable-weak/sdk-group.prg \
  -y keys/developer_key.der -O 0 -l 0
sh scripts/run-simulator-container.sh \
  --prg scratch/compiler-immutable-weak/sdk-group.prg
```

Adjust the absolute user-home path if the checkout differs. Build/sign/run our
output with the example README, save both console logs, and compare actual lines
with the expected output. Both programs produced the same 15 lines. Number
arithmetic, String members, Boolean conditions, typed returns, repeated calls,
and guarded nullable receivers worked. The receiver-effect marker printed once.
Type checking disabled does not establish SDK static acceptance; the initial
casted probe and runtime observations establish the narrower behavior.

Shared validation uses the effective receiver result in source and resumed IR.
Tests cover pipeline equivalence, wrong arity/consumers, unchecked nullable
receivers, edited guards, and API 1.2.0 boundaries. Catalogue tests retain the
generic WeakReference result for other classes and source subclasses; this
avoids treating arbitrary Object instances as immutable scalars. No identity,
reference-count, allocation-cost, or ordinary weak-reference lifetime claim is
made by this experiment.

[object]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/Object.html#weak-instance_function
[example]: ../../examples/compiler-immutable-weak/README.md
