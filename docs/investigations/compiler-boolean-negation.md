# Boolean logical negation

SDK 9.2.0 and our generated Forerunner 265 application agree on all 14 console
lines in the [example](../../examples/compiler-boolean-negation/README.md).
Boolean inversion evaluates one operand once, including an effectful call.
This investigation establishes the Boolean subset, not general truth-value
conversion or short-circuit semantics. Reviewed 2026-10-12.

## Reference and specimen

The [Monkey C reference][reference] describes `!` as reversing a logical
expression. Our bounded contract accepts only Boolean; other operand types
remain unsupported regardless of broader language behavior.

The specimen uses `app.mc` and `flags.mc` from the example. An SDK-only wrapper
in ignored `scratch/compiler-boolean-negation/sdk.mc` supplies the lifecycle:

```monkeyc
import Toybox.Application;
class MinimalConsoleApp extends AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) { start(); }
    function getInitialView() { return [new Toybox.WatchUi.View()]; }
}
```

`scratch/compiler-boolean-negation/group.jungle` selects our sources and reuses
our existing console specimen's manifest/resources:

```text
project.manifest = ../native-names/manifest.xml
base.sourcePath = ../../examples/compiler-boolean-negation;sdk.mc
base.resourcePath = ../native-names/resources
```

Build the SDK reference with local device data and key:

```sh
CIQ_JUNGLE_FILES="$PWD/scratch/compiler-boolean-negation/group.jungle" \
  sh scripts/build-example.sh scratch/compiler-boolean-negation
```

Run its `build/fr265/compiler-boolean-negation.prg` and the signed generated PRG
from the example separately through `scripts/simulator-session.py`, using
`xvfb-run -a -s '-screen 0 1280x1024x24'` and `--observe-seconds 15` in the
simulator container. Preserve `scratch/simulator/monkeydo.log` after each run.
The comparison uses the entire 14-line output, including `effect` exactly once,
followed by `false`; successful process exits alone are insufficient.

## Observations and limits

Experimentally observed in both applications:

- `!true` is false, `!false` is true, and `!!true` is true.
- Typed Boolean parameters and reassigned locals retain Boolean results.
- `!true.equals(1)` is true: the postfix call runs before inversion.
  `(!true).equals(1)` is false, through the explicitly inverted receiver.
  These distinguish the two evaluation orders because Boolean `equals(Number)`
  is false for either Boolean value.
- Grouped numeric comparisons invert to Boolean. Branches accept the result;
  `!(n == null)` also runs as a Boolean condition.
- The effectful Boolean helper prints once before its result is inverted.

Our lowering uses operand evaluation followed by `bfpush; eq`. Generated
simulator behavior verifies this sequence for the specimen; we did not infer
or inspect the SDK compiler's implementation or require matching SDK CODE.

Saved IR preserves a typed `.not` operation. Pipeline tests compare source and
resumed unit IR, generated unsigned PRGs, and assembly round trips. Adversarial
IR edits reject non-Boolean operands, missing definitions, and reordered calls
with otherwise valid definitions. Parser tests cover source spans and the
64-level nesting limit. These establish shared validation, not new runtime
claims.

Non-Boolean operands, nullable Boolean contracts, and `!=` remain unsupported.
Negated null-proof propagation and conditional logical effects are investigated
in the [short-circuit follow-up](compiler-short-circuit.md). Boolean results
across joins remain in [the plan](../plan.md).

[reference]: https://developer.garmin.com/connect-iq/reference-guides/monkey-c-reference/
