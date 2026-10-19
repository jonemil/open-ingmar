# Object arguments to comparison methods

2026-10-19. Question: should comparison/equality calls accept their declared
Object arguments, or only a compiler-selected runtime-safe subset?

**Publicly documented:** [Number.compareTo][number] and
[String.compareTo][string] take Object and return Number from API 5.0.0.
Number documents a type exception outside Numeric/Boolean/Char; String documents
one outside String. The Object signature describes the static boundary; it does
not guarantee a successful comparison for every runtime class.
[String.equals][string] accepts Object or Null. These references were reviewed
on the date above.

**Experimentally observed:** SDK/compiler/simulator 9.2.0, fr265,
[the authored example](../../examples/compiler-comparison-objects/app.mc)
compares each of Number, Long, Float, Double, Boolean, and Char against all six
classes through Object parameters. String comparison accepts a String delivered
through Object. Numeric alias operands use the same method dispatch; all 16
equal-valued numeric class pairings compare as zero. Equality accepts source
objects and Views alongside scalars and containers. Distinct numeric classes
still compare unequal through `equals`, and reference equality retains identity.
SDK and generated programs match all
[44 console lines](../../examples/compiler-comparison-objects/expected.txt).
Both support structured CODE/DATA export/reconstruction and saved-stage
resumption, using existing native call instructions and selectors.

Two isolated authored probes pass a String to Number.compareTo and a Number to
String.compareTo through Object parameters. The SDK compiles both; SDK and
generated programs print their pre-call marker, report an unhandled
`UnexpectedTypeException`, and never print their post-call marker. This verifies
runtime rejection and propagation, not catch/finally behavior or portable error
text. Logs are under `scratch/comparison-error-number/` and
`scratch/comparison-error-string/`, in `sdk-runtime-v2/` and `generated-runtime/`.

For the main SDK build, reuse the minimal-console manifest/resources, set
`base.sourcePath` to `../../examples/compiler-comparison-objects` from
`scratch/compiler-comparison-objects/monkey.jungle`, and build with
`scripts/build-example.sh`. The source owns its AppBase class. Use the console
specimen checker with `compiler-comparison-objects`; current captures are in
`scratch/check-comparison-objects/`.
To reproduce either failure, replace the example's onStart body with a pre-call
print, `Comparisons.number("wrong")` or `Comparisons.stringValue(2)`, and a
post-call print. Run each separately with the simulator session helper and
inspect the exception class and marker order. Ordinary successful-output checks
must not count these crashes as successful application execution.

The [source contract](../specs/compiler-subset.md#comparison-object-arguments)
owns static assignability/nullability and the runtime-domain distinction.

[number]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/Number.html#compareTo-instance_function
[string]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/String.html
