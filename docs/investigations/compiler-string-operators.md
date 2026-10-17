# String concatenation and identity comparison

Experiments on 2026-10-12: SDK/compiler/simulator 9.2.0, fr265, optimization
disabled. Question: which existing values participate in String operators, and
does equality compare content or identity? Public background: the
[Monkey C reference][reference] identifies `+` as String concatenation and lists
relational operators. Detailed pair/result rules below come from our specimens.

## Probe findings

An ignored probe concatenates String with String, Number, Boolean, Char, and
Null, in both operand orders. It compares String literals, references, generated
strings, and mixed scalar/nullable values. A first version included String/String
`>` and String/Boolean `==`; default SDK checking rejected those two pairs.
Removing them allowed compilation and execution. Their runtime behavior was
not tested with checking disabled.

**Observed:** concatenation returns String, converts scalar values, and formats
Null as `null`. For example, `"n=" + 7` prints `n=7`, `false + "b"` prints
`falseb`, `'A' + "c"` prints `Ac`, and `null + "z"` prints `nullz`. Nullable
Number/String parameters work when an exact String is the other operand.
Formatting content and non-BMP rendering remain runtime responsibilities.

**Observed:** `"same" == "same"` is false for two literal evaluations. A local
compared with itself, a copied reference, or an identity-function result is true.
Comparing that local with a separately concatenated `"s" + "ame"` is false;
its `.equals()` call with the concatenated value is true. String/Number,
String/Char, and String/Null comparisons are false in both operand orders.
Nullable Number/String operands compared with an exact String also produced
false in the tested distinct-reference cases.

**Interpretation and design:** preserve String identity for `==`, and retain
`.equals()` for content comparison. DATA literal deduplication may share bytes,
but separate runtime String evaluations must retain their observable identity.
Do not fold equal-content literal equality to true or merge String allocations
without establishing that identity is unobservable. No physical pointer layout
or intern-table behavior is inferred here.

These probes establish mixed operators with one exact String operand,
including Number/String/Boolean/Char/Null concatenation and nullable
Number/String alternatives. [Wider numeric probes](compiler-wide-operators.md)
extend that evidence to Long/Float/Double. Equality excludes Boolean alternatives.
Neither operation establishes a non-null proof about the other operand. Two
unchecked nullable operands are not admitted merely because one might be String.
Objects, containers, String subtraction, and String ordering remain unsupported.

## Complete comparison

Use the [complete example][example], with a local AppBase wrapper whose
`onStart(state)` calls `start()` and whose initial View is
`[new Toybox.WatchUi.View()]`. Select its sources, wrapper, and local fr265
manifest/resources in ignored `group.jungle`:

```sh
CIQ_JUNGLE_FILES="$PWD/scratch/compiler-string-operators/group.jungle" \
  sh scripts/build-example.sh scratch/compiler-string-operators
sh scripts/run-simulator-container.sh \
  --prg scratch/compiler-string-operators/build/fr265/compiler-string-operators.prg
```

Build/sign/run our output with the example README. SDK and generated programs
produced the same 35 console lines. Alias identity, distinct instances,
nullable conversions, local replacement, chained case conversion, and left/right
call markers matched. Successful source/resumed-IR PRG and assembly round trips
show both pipeline paths preserve the contracts. Our backend uses ordinary
`addv` and `eq`, without emitted native conversion calls.

Validation checks reject unsupported operands, edited operators, incompatible
user signatures/locals, and nullable use after concatenation or non-null identity
comparison. Earlier negative tests use subtraction for newly admitted String
addition pairs. Different optimization settings, SDKs/devices, extreme sizes,
allocation failure behavior, and wide Unicode conversions remain unverified.

[reference]: https://developer.garmin.com/connect-iq/reference-guides/monkey-c-reference/
[example]: ../../examples/compiler-string-operators/README.md
