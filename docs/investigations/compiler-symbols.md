# Symbol values and binding

2026-10-17 to 2026-10-18, SDK/compiler/simulator 9.2.0, `fr265`, debug, `-O 0`.

**Officially documented:** [Symbol][symbol] is a constant identifier since API
1.0.0; assigned numbers may change between builds. `toNumber()` requires 2.3.0.
Development `toString()` uses names, while release output may use numeric text.
[Method][method] is the callback value consumed by `invoke()`.

**Observed:** the [authored specimen][example] keeps Symbol distinct from its
name String and numeric ID. Matching names compare equal through operators and
`equals`; different names compare unequal. Matching Symbols have matching
hashes and numeric conversions in the tested calls. No particular hash or
cross-build number is inferred.

An isolated SDK probe checks `a.weak() instanceof Lang.Symbol` and
`a.weak() instanceof Lang.WeakReference` for `a = :alpha`: true and false.
The compiler specializes the effective weak result to Symbol, as for the other
verified immutable values. The SDK checker nevertheless types the result as
WeakReference and rejects chained `toNumber()` without a cast. The tracked
specimen uses the mutually accepted equality consumer; compiler tests also
check the effective Symbol conversion contract.

## Encoding and reproduction

Select the tracked source with an ignored Jungle and the shared native-names
manifest/resources. Use the
[console checker](../../scripts/README.md#console-specimen-check) for source and
saved-stage parity, structured CODE/DATA/SYMBOLS reconstruction, signing, and
runtime comparison. SDK/generated output matches all 32 expected lines.
Artifacts: `scratch/check-nullable-symbols/`; the isolated probe remains under
`scratch/compiler-symbols/`.

Symbol literals use the existing `spush` opcode (`0x27`) with a big-endian u32
symbol ID. Semantic IR retains the name; raw CODE uses explicit name binding.
Complete applications share literal identities with method/field names and
pinned native symbols. They emit a SYMBOLS section for literal development
names: both `:alpha.toString()` and the native `:size.toString()` match the SDK.
This does not establish release-mode reflection behavior for our generator.

The initial SDK probe also calls `[1].method(:size).invoke()` and prints 1.
That establishes a minimal accepted callback path, not Method ownership,
variadic arity/result contracts, Void callbacks, or constructor semantics.
[Method evidence](compiler-methods.md) extends that probe to ownership and erased
invocation. Array comparator contracts remain separate.

## Nullable Symbols

**Experimentally observed:** the tracked `NullableSymbols` forwards Symbol or
Null through parameters/results and conditional arms in both orders. Null guards
permit `toString`, `toNumber`, `hashCode`, `equals`, immutable `weak`, Symbol
equality, and a `has` selector. A saved local keeps its Symbol after another
local is cleared. A loop invokes the guarded Symbol once before assigning Null
and exiting. Native `equals` accepts nullable arguments without requiring a
receiver proof for that argument.

The shared console checker verifies SDK/generated text, saved stages, and
structured reconstruction. SDK build log: `scratch/nullable-symbol-sdk-build.log`.
Existing `spush`, Null, comparison, branch, and native-call instructions suffice;
no new opcode or DATA record was encountered. These cases establish nullable
value flow, not Symbol fields, container encodings, or a general nullable
receiver/cast profile.

[symbol]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/Symbol.html
[method]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/Method.html
[example]: ../../examples/compiler-symbols/README.md
