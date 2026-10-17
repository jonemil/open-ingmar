# Wider numeric Dictionaries and key equality

2026-10-17, SDK/compiler/simulator 9.2.0, `fr265`, debug, `-O 0`.

**Officially documented:** [Dictionary][api] maps object keys to object values;
keys should supply suitable hashing. Iteration order is unspecified. These
contracts do not establish equality between numeric classes or NaN key behavior.

**Observed:** the [authored specimen][example] accepts Long/Float/Double keys,
values, and nullable values under SDK checking. Constant/computed construction,
empty/null-only contexts, indexing, `get`, `put`, `remove`, `hasKey`, `size`,
`isEmpty`, `keys`, `values`, conversion, and identity behave consistently between
SDK/generated applications. All 74 expected lines match. Long keys distinguish
1 from 4294967297, while repeated equal keys retain the last value. Signed
boundaries, floating precision, and negative zero survive DATA loads and reads.
Collection methods preserve key/value Array contracts; tests use membership
instead of assuming iteration order. Effects occur receiver/key/value first,
and computed entries execute in source order before later entries.

## Signed zero and NaN probes

The separate key probe creates `{1.25 => 10, -0.0 => 20, 0.0 => 30}` and the
Double equivalent. Both have size 3; each zero retrieves its own value.
Replacing 1.25 updates the existing entry without increasing size. Thus ordinary
numeric equality is insufficient for constant-key coalescing. Repeated finite
keys with identical bits collapse; opposite zero signs remain distinct.
Artifacts: `scratch/numeric-dictionary-key-runtime/`.

Trying `var z = 0.0; var nan = z / z;` failed with `Invalid Value` before the
key tests. This establishes a division boundary on this simulator, not the
absence of NaN values. To test keys independently, compile an authored app with
Float markers 1.234567 and Double markers 9.87654321d. Export its PRG with our
section tools, replace the two `fpush 0x3f9e064b` operands with `0x7fc00000`,
and the two `dpush 0x4023c0ca4588f633` operands with `0x7ff8000000000000`.
Reassemble and sign with our key. This edits our authored application's output;
it does not inspect Garmin's compiler/SDK implementation.

Both constants render as `nan`; operator equality with themselves is false.
For Float, inserting one NaN key allows lookup with either same-bit value;
putting the second retains size 1 and replaces the value. For Double, even the
same variable is not found: inserting a second NaN increases size to 2 and get
returns Null. Artifacts: `scratch/numeric-dictionary-nan-constant-runtime/`.

A second controlled version loads a constant Dictionary with two distinct
Double marker keys, 9.87654321d and 9.87654322d. Replace their DATA scalar bits
`0x4023c0ca4588f633` and `0x4023c0ca45dedc6f` with the same Double NaN bits.
Its two entries remain distinct, with absent lookup. Artifacts:
`scratch/numeric-dictionary-nan-data-runtime/`. Constant construction therefore
retains Double NaN entries instead of coalescing them. These probes cover the
listed quiet NaN patterns; other payload relationships and other runtimes remain
unverified. No physical hash algorithm is inferred.

## Reproduction and limits

Select the tracked example using an ignored Jungle and the shared native-names
manifest/resources. Use the [console checker][checker] for source/saved-IR/native
parity, structured SDK/generated reconstruction, stable re-export, signing, and
runtime comparison. Artifacts: `scratch/check-numeric-dictionaries/`.
The isolated probe sources live under `scratch/numeric-dictionary-key-probe/`
and `scratch/numeric-dictionary-nan-probe/`; the preceding descriptions preserve
the essential inputs and exact substitutions.

The existing shared DATA Long/Float/Double tags and VM `hpush`, private builders,
indexed load/store, and invocation instructions suffice. There is no new opcode
or record encoding. General heterogeneous/nested values, Null keys, ordinary
weak references, and portable non-finite conversion behavior remain separate.

[api]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/Dictionary.html
[example]: ../../examples/compiler-numeric-dictionaries/README.md
[checker]: ../../scripts/README.md#console-specimen-check
