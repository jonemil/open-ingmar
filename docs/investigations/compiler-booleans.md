# Boolean values in source callbacks

## Contract and scope

Reviewed 2026-10-11. The public [Boolean documentation][boolean] describes
`true`/`false` literals and the Boolean class, available since API 1.0.0.
This experiment covers stored Boolean decisions, literals, assignment,
conditions, equality, and printing on SDK 9.2.0 / fr265. It does not establish
Boolean user-function parameters/returns, logical operators, or truthiness.

## SDK specimen

Use the minimal-console manifest/resources and a Jungle selecting our own
AppBase specimen under `scratch/boolean-values/`. Its generated View scaffold
uses the same source pattern as the [native-call experiment][native]. The
startup body is:

```monkeyc
var nullState = null;
var missing = state == nullState;
System.println(missing);
var again = true;
while (again) {
    System.println("Boolean loop");
    again = false;
}
var stopped = false;
System.println(again == stopped);
System.println(again);
```

Build with `sh scripts/build-example.sh scratch/boolean-values` (`-O 0`, own
developer key), then export `build/fr265/boolean-values.prg` with `ciq-dis all`
into a new scratch directory. No Garmin compiler/SDK implementation is inspected.

**Observed:** true/false literals use `btpush`/`bfpush`; stored values use
`lputv`/`lgetv`; the loop uses `bf`; Boolean equality uses `eq`. These are
existing supported instructions. The first specimen's direct `state == null`
instead encountered unsupported byte `0x20` at CODE+`0x29`; comparing two locals
avoided it. Its precise instruction semantics remain unverified and were not
needed for our existing `npush`/`eq` lowering.

## Independent compiler

The [lifecycle example][example] stores its dictionary/null comparison result,
prints it, selects a branch through a Boolean local, and changes a loop flag
from true to false. It compares that flag with false and prints the final flag.
Follow the example's compilation/signing/simulator commands. Context inputs are
target API 5.2.0 and application minimum 1.0.0, separate from the SDK version.

**Observed:** the complete app prints `source initialize`, `source onStart`,
`true`, `null state`, `41`, `Boolean loop`, `true`, `false`, in order. The loop
executes once; a non-null callback state remains untested. Source and saved unit
IR use the same Boolean operation/type checks and application generation.

Tests execute Boolean branches and mutable loop flags through a bounded
instruction oracle, check saved-IR parity and strict literal syntax, and reject
mixed local types, Boolean arithmetic/returns/arguments, and mixed or ordered
Boolean comparisons. Simulator output supplies runtime evidence independently
of that oracle. PRGs, SDK output, and console logs remain ignored.

[boolean]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/Boolean.html
[native]: compiler-native-calls.md#sdk-specimen
[example]: ../../examples/compiler-lifecycle/README.md
