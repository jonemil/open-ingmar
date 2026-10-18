# Bound Method values and erased invocation

2026-10-17, SDK/compiler/simulator 9.2.0, `fr265`, debug, `-O 0`.

**Officially documented:** [Object.method(Symbol)][object] returns a bound
[Method][method], whose variadic `invoke` calls the target. Binding, invocation,
and Method `hashCode` date from API 1.0.0. The published invocation result is
Object; it does not describe a concrete target signature.

**Observed:** the [authored specimen][example] binds native Array, Dictionary,
and Number methods, then invokes zero/one/two-argument targets. Binding retains
the receiver: after replacing the original Array variable, its weak reference
remains live and bound mutation/size calls still reach the original Array.
Replacing every retained Method lets that target expire. A weak Method result
is live while its Method owner remains, and retrieved aliases compare equal.

Method aliases compare equal and have matching hashes. Separate bindings of the
same receiver/member compare unequal in this probe. String conversion is
repeatable within the probe; no representation or hash algorithm is inferred.
Receiver/argument markers execute once, in that order, for discarded invocation.

## Results and errors

Invoking Dictionary `put` through an erased Method parameter produces Null;
Dictionary `get` can also produce Null. Valued invocations produce Number or
Array objects. The compiler therefore preserves Object-or-Null, independently
of observations at one call site. Source tests reject unguarded consumers and
concrete recovery. The SDK checker knows a directly bound `put` returns Void
and rejects assigning that invocation; the specimen uses an ordinary Method
parameter to exercise the erased result without casts.

An isolated SDK app prints `before`, then calls a helper that binds
`:missingMember` on `Array<Number>`. Binding reports `Invalid Value` before
returning: neither `created` nor the later invocation executes. Artifacts:
`scratch/method-missing/`. This establishes one lookup failure, not portable
error text or exception recovery. Target argument mismatches and introduction
levels are not statically established by an erased Method contract.

## Encoding and reproduction

Build the tracked source with an ignored Jungle and the shared native-names
manifest/resources. Use the
[console checker](../../scripts/README.md#console-specimen-check) for saved-stage
parity, structured CODE/DATA/SYMBOLS reconstruction, signing, and runtime checks.

SDK symbols bind `method` to `0x008000b2` and `invoke` to `0x008000ac`.
Existing member lookup, invocation, and local ownership transfers suffice;
no opcode or PRG record is added. Tests cover the 254-argument encoding bound
and raw-CODE symbol binding, including the earlier `stillAlive` alias.

## Source receivers

The [source callback specimen](../../examples/compiler-source-methods/README.md)
binds a source View method directly, inside the View through bare `method`,
and through an opaque Object with a dynamic Symbol parameter. All bindings
mutate the same instance: results are `2`, `5`, `9`. After replacing the source
variable, its bound Method still reaches the original instance (`10`), while
the replacement starts at `0`. An AppBase implicit callback updates its own field
to `15`. Bare `self` can be returned as Object. SDK/generated programs match
all six lines; both have structured CODE/DATA and lossless reconstruction.
Rechecked 2026-10-18 with SDK 9.2.0, `fr265`, debug `-O 0`:
`scratch/check-implicit-methods/`. Both valued and discarded bare bindings run.
Compiler tests compare explicit/implicit IR (excluding source spans) and
assembly, including declared source members named `method`, Void statements,
branching arguments, and contextual Array literals. This does not establish
general unqualified name lookup or source inheritance.

The SDK rejects `view.owner().method(:add)` when `owner` declares Object:
`Could not find method 'add' on object '$.Toybox.Lang.Object'`. Passing the
selector through a Symbol parameter permits the runtime binding. Our erased
lookup profile does not yet perform that SDK literal-target check; checked
signatures remain a separate prerequisite.

Current-receiver capture uses existing `getself`; no opcode/record extension
is needed.

## Nullable ownership

2026-10-18, SDK/compiler/simulator 9.2.0, `fr265`, debug `-O 0`.

**Observed:** the native Method specimen passes Method-or-Null through a
conditional result, an ordinary parameter/result pair, locals, and a loop.
After replacing the source Array and its direct Method variable, a nullable
alias keeps the old receiver alive. Copying that alias, then clearing the
original, retains the receiver; clearing the last alias expires its weak
reference. Guarded invocation still returns its original size (`2`).

`MethodStore` holds the old receiver in a required Method field, then a nullable
field. Replacing the required binding retains the old receiver through the
nullable field; clearing that field retains it through a local snapshot.
Clearing the final snapshot expires its weak reference. The replacement binding
invokes the new receiver. AppBase also stores its own `method(:answer)` in a
nullable field, clears the field, and invokes a saved local (`42`).

`MethodArrays` retains callbacks in non-null/nullable element Arrays and in
reverse/slice copies. Replacement/removal releases each container's ownership;
the last indexed local snapshot retains the old receiver until replaced.
Nullable snapshots remain callable after their element is cleared. Identity
search/removal and ordered literal markers (`40`, `41`) match across compilers.

Both SDK/generated programs match all 57 expected lines, including Null checks,
guarded hash/weak calls, and a loop that clears its Method. Structured section
reconstruction passes for both. Artifacts: `scratch/check-method-arrays/`.
No callable signature or concrete recovery from an erased result is inferred.

[object]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/Object.html
[method]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/Method.html
[example]: ../../examples/compiler-methods/README.md
