# Native View construction and object arrays

SDK 9.2.0 and independently generated fr265 applications agree on all fourteen
lines in [the example][example], checked 2026-10-13. The specimen tests a native
heap object through construction, typed calls, and invariant Array elements.

## Contracts and observations

**Officially documented:** [View][view] inherits Lang.Object and exists from
API 1.0.0; its zero-argument initializer is documented from 2.1.0. The
[AppBase callback][appbase] returns a positional view result with an optional
delegate. A general homogeneous View array does not describe its cardinality
or the heterogeneous delegate case. Reviewed 2026-10-13.

**Experimentally observed:** both programs construct native Views, preserve
references through typed parameters/results and array reads, and append a
previously stored reference. `indexOf` finds the stored View at index zero;
single removal and removal of its remaining occurrence return true. Bulk append,
reverse, and slice preserve usable View-array results. Reassignment to a fresh
empty array leaves a shared reference holding the previous three-element array.
An effectful factory prints once before its caller prints `created`.

**Implementation checks:** source, saved IR, assembly text, and unsigned PRG
paths agree. Constructor resolution requires its initializer's API level;
passing an existing View and allocating an empty View array pass 1.0.0 checks.
IR edits cannot misdeclare a user-call result or mix Number/View elements.
Moving construction before an earlier native call is rejected as an incompatible
effect schedule. Bare unimported View, qualifier shadowing, arguments, nullable
Views, and View/Object methods outside the supported profile are rejected.

## Lowering and limits

Construction uses the native bootstrap's established class lookup and
initialization convention: `getmv; newc; dup 0; getsv initialize; dup 1;
invokem 1; popv`. The new instance survives discarding the initializer result.
Bindings are WatchUi `0x00800011`, View `0x0080019f`, and initialize `0x0080005e`.
The [bootstrap record][bootstrap] establishes these native import identities;
construction reuses the complete application's imports.

`NativeClass::View` identifies standalone references and
`ArrayElement::Object(NativeClass::View)` identifies their invariant container.
This does not imply a runtime layout, SDK memory equivalence, or a new object
implementation in Rust. Equality evidence covers repeated references to the
same native View, not arbitrary subclasses or distinct-object equality.

The created Views are not supplied to the lifecycle callback; the generated
bootstrap still provides its blank View. Source `getInitialView`, positional
callback results, source View inheritance, fields, drawing, and delegates remain
separate work in [the plan][plan].

## Reproduction

Use the example's commands to build/sign our program. Copy the
[console wrapper][wrapper] into `scratch/compiler-native-views/sdk.mc` and
create `group.jungle` there:

```text
project.manifest = ../native-names/manifest.xml
base.sourcePath = ../../examples/compiler-native-views;sdk.mc
base.resourcePath = ../native-names/resources
```

Build the SDK specimen:

```sh
CIQ_JUNGLE_FILES="$PWD/scratch/compiler-native-views/group.jungle" \
  sh scripts/build-example.sh scratch/compiler-native-views
```

Run each PRG separately:

```sh
xvfb-run -a -s '-screen 0 1280x1024x24' \
  python3 scripts/simulator-session.py --prg <program.prg> --observe-seconds 15
```

Save `scratch/simulator/monkeydo.log` after each run and compare both full logs
with the example output. Retained captures are under
`scratch/compiler-native-views/`.

[example]: ../../examples/compiler-native-views/README.md
[view]: https://developer.garmin.com/connect-iq/api-docs/Toybox/WatchUi/View.html
[appbase]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Application/AppBase.html#getInitialView-instance_function
[bootstrap]: application-bootstrap.md
[wrapper]: compiler-boolean-negation.md#reference-and-specimen
[plan]: ../plan.md
