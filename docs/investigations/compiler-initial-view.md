# Source getInitialView and positional results

SDK 9.2.0 and independently generated fr265 applications both log `start`,
`initial`, and `make`, then remain running with a blank native View. Checked
2026-10-13 using [the complete example][example]. This tests replacement of the
generated lifecycle target with a source callback, not only object construction.

## Contract and evidence

**Officially documented:** [AppBase.getInitialView][appbase] takes no arguments
and returns a positional view result with an optional delegate. The bounded
catalogue models one View or View/delegate slots using their base classes;
it does not enumerate the SDK's complete typedef alternatives. The executable
profile admits only the one-view form. Reviewed 2026-10-13.

**Experimentally observed:** startup runs before the initial-view callback,
whose factory prints once before returning a constructed native View. Both
applications remain active during the 15-second observation window. Captures
show a blank watch display without a runtime error dialog, and console logs
match exactly. This does not validate drawing, input, or source View inheritance.

**Implementation checks:** the return carries `TupleShape::OneView` through
source, saved IR, assembly text, and complete unsigned PRG paths. DATA decoding
finds exactly one getInitialView field pointing to its source body. Empty,
multi-slot, Null, non-View, and mutable Array results fail shared validation;
branches must return the shape on every path. Native callbacks cannot be invoked
as ordinary source callees.

## Packing and limits

The source builder evaluates the View expression into a hidden local before
`TupleNew` packs a fixed one-slot runtime array. IR requires stored View reads,
so packing allocation cannot move before operand effects. Lowering uses length
one `newa`, duplicate/index-zero setup, the stored View read, and `aputv`.
The shape has no generic mutable Array profile; cardinality is established by
construction rather than inferred from aliases or later mutations.

The tuple's Array/View identities require API 1.0.0; constructing the native
View still requires its initializer's 2.1.0 level. SDK allocation/failure timing
and memory equivalence are not established. Delegate pairs, arbitrary tuple
annotations, and general result-expression inference remain separate work.

## Reproduction

Use the example's commands to build/sign our application. The SDK specimen uses
the same source class directly, without an additional console wrapper. Create
`scratch/compiler-initial-view/group.jungle`:

```text
project.manifest = ../native-names/manifest.xml
base.sourcePath = ../../examples/compiler-initial-view
base.resourcePath = ../native-names/resources
```

Build the SDK specimen:

```sh
CIQ_JUNGLE_FILES="$PWD/scratch/compiler-initial-view/group.jungle" \
  sh scripts/build-example.sh scratch/compiler-initial-view
```

Run the SDK-produced and independently generated PRGs separately:

```sh
xvfb-run -a -s '-screen 0 1280x1024x24' \
  python3 scripts/simulator-session.py --prg <program.prg> --observe-seconds 15
```

Save `scratch/simulator/monkeydo.log` and `screen.png` after each session. Compare
full console logs with the expected lines and inspect the displays. Retained
SDK/generated captures are under `scratch/compiler-initial-view/`.

[example]: ../../examples/compiler-initial-view/README.md
[appbase]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Application/AppBase.html#getInitialView-instance_function
