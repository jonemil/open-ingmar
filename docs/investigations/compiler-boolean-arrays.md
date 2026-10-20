# Boolean Array elements

Date: 2026-10-16. SDK 9.2.0, `fr265`, debug, `-O 0`.
The [authored specimen](../../examples/compiler-boolean-arrays/README.md) checks
Boolean constants, computed/conditional/short-circuit elements, native Array
methods, indexed mutation, shared references, fresh loads, and empty literals.
SDK and generated simulators match all 23 expected lines. The checker confirms
saved-stage equivalence, structured CODE/DATA, exact reconstruction, and stable
re-export for both PRGs. Artifacts: `scratch/check-boolean-arrays/`.

A local copy sees a write while a later load from the same constant retains its
original value. Search/removal find Boolean elements. Markers show conditional
branch selection and left-to-right construction. Reads feed Boolean conditions
and logical operators without Number conversion. Existing scalar Boolean DATA
tags and Array opcodes suffice; no new binary profile is required.

Source and saved IR enforce Boolean-only elements and API availability: types,
construction, indexing, `size`, and `toString` at 1.0.0, other supported native
Array methods at 1.3.0. Nullable Boolean, sorting, and inherited Object behavior
remain outside this evidence. See the [source profile](../specs/frontend/containers.md#boolean-arrays)
and [constant IR](../specs/ir/containers.md#constant-arrays).
