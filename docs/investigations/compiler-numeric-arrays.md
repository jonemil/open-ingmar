# Long/Float/Double arrays and DATA constants

2026-10-17, SDK/compiler/simulator 9.2.0, `fr265`, debug, `-O 0`.
Question: can the existing invariant Array contracts carry wider numeric values
through literal records, mutation, and native methods?

**Officially documented:** [Array][api] supports object elements and declares
its generic methods. Our accepted homogeneous/nullable profiles remain narrower
than arbitrary heterogeneous arrays. `removeAll` takes an element, not an array;
the initial SDK probe rejected an Array argument to each numeric specialization.

**Observed:** the [authored specimen][example] compiles with normal SDK checking.
Long/Float/Double literal arrays retain signed boundaries, signed zero, and
values above 32-bit or Float precision limits. A fresh load of the same constant
retains its original contents after a previous load is mutated. Nullable
parameters/results and guarded reads preserve the exact scalar width.

Reads, writes, numeric indexed updates/compounds, contextual empty arrays, and
computed construction with branching element effects work with these contracts.
The native group exercises `size`, `toString`, `indexOf`, `add`, `addAll`,
`remove`, `removeAll`, `slice`, and `reverse`. Slices retain their old elements
across later source-array writes; reversed results retain element types/values.
Nullable native arguments and transformed results retain nullable elements.
Numeric search/removal use the runtime's value behavior; no cross-type coercion
is established by these same-type tests.

## Encoding evidence

The Array root/count/framing match the existing constant-container records.
Additional scalar tags and payloads:

| Tag | Type | Bytes | Example bits |
|---|---|---|---|
| `02` | Float | 4 | `3fa00000` for 1.25 |
| `0e` | Long | 8 | `0000000100000001` for 4294967297 |
| `0f` | Double | 8 | `4170000010000000` for 16777217 |

All payloads are big-endian. Negative Long uses two's-complement bits; negative
floating zero retains only the sign bit. Four Longs have value-block length 41,
outer length 49, and total record size 55, including the terminator. Null retains
its existing one-byte tag in mixed scalar/Null records.

The initial structured export stopped at an unsupported constant scalar tag.
After adding the three shared scalar representations, the complete DATA walk
and CODE export succeed without fallback. Existing `apush`, indexed load/store,
private builders, and native invocation instructions suffice; no opcode change
is required. Compiler Dictionary contracts remain bounded despite the shared
codec recognizing these scalar tags structurally.

## Reproduction and limits

Select the tracked example with an ignored Jungle pointing at its source and
the shared native-names manifest/resources; build with the standard SDK helper.
Use the [console checker](../../scripts/README.md#console-specimen-check) for
saved-stage parity, structured SDK/generated reconstruction and stable re-export,
and simulator comparison with all 68 expected lines. Artifacts:
`scratch/check-numeric-arrays/`.

Tests additionally retain arbitrary floating bits, including NaN payloads, in
DATA/IR text and binary round trips. That codec evidence does not establish
NaN search/equality/removal behavior in the runtime. Numeric Dictionary use,
whole-array conditional joins beyond the established profile, nested containers,
sorting, and ordinary object weak references are separate contracts.

[api]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/Array.html
[example]: ../../examples/compiler-numeric-arrays/README.md
