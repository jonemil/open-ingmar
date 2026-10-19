# Lang.format module calls

2026-10-19. Question: how does the erased Array parameter behave, and can the
existing VM call instructions reproduce value-returning Lang module calls?

**Published contract:** [Garmin's Lang API][lang] declares String and erased
Array parameters, a String result, one-based `$n$` substitutions, and API 1.0.0.
It does not require a mutable `Array<Object>` contract.

**Observed:** SDK/compiler/simulator 9.2.0, fr265, accept the
[authored specimen](../../examples/compiler-lang-format/app.mc). SDK and generated
programs match all [17 console lines](../../examples/compiler-lang-format/expected.txt):

- Mixed scalars use runtime string representations; null becomes `null`.
- Repeated/out-of-order placeholders work. Substituted `$2$` text remains literal
  rather than being recursively expanded. Empty parameters work for plain text.
- Source-object `toString` overrides, nested Arrays, ByteArray, and Unicode work.
- Typed Number Arrays retain their identity and reflect mutation. Conditional
  contents and effectful arguments preserve left-to-right order, including a
  discarded formatting result.

Structured CODE/DATA export and reconstruction pass for SDK and generated PRGs.
Native module dispatch uses the existing `getmv`/`frpush`/`invokem` profile,
with `Toybox_Lang` (`0x00800007`) and `format` (`0x0080009c`). No additional
opcode or DATA framing was needed. Saved IR/assembly resume to equivalent output.

For SDK reproduction, reuse minimal-console's manifest/resources and set
`base.sourcePath=../../examples/compiler-lang-format` from
`scratch/compiler-lang-format/monkey.jungle`. Build with
`sh scripts/build-example.sh scratch/compiler-lang-format`, then use the
specimen checker with `compiler-lang-format`. Captures:
`scratch/check-lang-format/`.

An isolated negative app prints `before`, then prints the result of
`Lang.format("$2$", ["only"])`, followed by an `after` marker. Both SDK and
generated apps print only `before` and fail with `Array Out Of Bounds Error`;
the simulator session returns 1. Inputs/build configuration are in
`scratch/lang-format-missing-app/`; captures are
`scratch/lang-format-missing-sdk/` and `scratch/lang-format-missing-generated/`.
This establishes the tested failure, not catchability, malformed-placeholder
rules, or behavior on every device/runtime.

The [source contract](../specs/compiler-subset.md#native-module-calls) owns
assignability and construction rules; the [IR contract][ir] owns saved calls.

[lang]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang.html#format-instance_function
[ir]: ../specs/compiler-ir-text.md#native-module-values
