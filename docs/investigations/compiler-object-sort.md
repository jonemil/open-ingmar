# Object and reference Array sorting

2026-10-19. Question: can Object and other reference-element arrays use native
sorting without narrowing their declared domains or inventing conversions?

**Published contract:** [Array.sort][array] requires API 5.0.0, takes Comparator
or Null, and returns Void. The default comparator orders supported scalars and
throws UnexpectedTypeException for values that are not directly comparable.
Comparator requires `compare(Object, Object) as Number`.

**Observed:** SDK/compiler/simulator 9.2.0, fr265, accept the
[authored example](../../examples/compiler-object-sort/app.mc). SDK and generated
programs match all [27 console lines](../../examples/compiler-object-sort/expected.txt):

- Object arrays default-sort mixed numeric widths and homogeneous Strings.
  The sorted numeric values retain Double, Long, Float, and Number classes.
- A source Comparator sorts mixed Object scalars and source objects through
  string conversion, with stateful callbacks and ordered receiver/argument
  effects. Aliases see mutation; a prior slice retains its original order.
- Custom ordering accepts ByteArray, Symbol, Method, WeakReference, and View
  elements. ByteArray mutation and saved Symbol/Method searches preserve stored
  references. Sorting weak handles does not keep their targets alive after
  their named owners are replaced.
- Empty and singleton Object arrays work under the exercised default/custom
  paths. No comparison count, sort stability, or collation algorithm is inferred.

Structured CODE/DATA reconstruction and saved IR/assembly parity pass. Existing
`sort` member dispatch and container records suffice; no new opcode or framing
was encountered. The [current contract][spec] owns argument domains and effects.

For SDK reproduction, reuse minimal-console's manifest/resources with
`base.sourcePath=../../examples/compiler-object-sort` from
`scratch/compiler-object-sort/monkey.jungle`. Build with
`sh scripts/build-example.sh scratch/compiler-object-sort`, then use the console
specimen checker with `compiler-object-sort`. Captures:
`scratch/check-object-sort/`.

## Failure probes

An isolated `Array<Object>` containing `["1", 1]` compiles in both toolchains.
Default sorting prints a preceding `before` marker, then raises the same
Unhandled Exception / UnexpectedTypeException diagnostic in SDK and generated
apps; the following `after` marker is absent. Both simulator sessions return 1.
Source/build inputs: `scratch/object-sort-error/`; captures:
`scratch/object-sort-error-sdk/`, `scratch/object-sort-error-generated/`.
This matches the public invalid-comparison example and establishes runtime
failure, not recovery or atomic mutation guarantees.

The initial SDK probe sorts `[2, null, 1]` declared as Array<Number or Null>.
Compilation succeeds, but default sorting fails with `Null Reference Error`.
Inputs: `scratch/object-sort-probe/`; capture: `scratch/object-sort-sdk-probe/`.
Nullable sorting remains outside our executable profile. Supporting custom
ordering of nullable elements requires a callback contract that admits Null;
the current checked Comparator parameters are non-null Object. This observation
does not establish every null placement, catchability, or device behavior.

The follow-up `scratch/nullable-sort-probe/app.mc` declares
`compare(Object or Null, Object or Null) as Number`, guarding both parameters.
SDK 9.2.0 rejects this class both at a direct sort call and when passed to a
Comparator-typed helper (`scratch/nullable-sort-sdk-build.log`, exit 105).
Its required interface parameters are exactly Object. This prevents treating
an explicitly nullable implementation as the missing checked callback contract;
do not infer that nullable sorting has a safe signature from API acceptance of
the Array alone.

[array]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/Array.html#sort-instance_function
[spec]: ../specs/frontend/containers.md#source-comparators
