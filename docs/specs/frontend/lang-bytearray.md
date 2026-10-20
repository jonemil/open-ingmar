# Core Lang ByteArray

## ByteArrays

`Toybox.Lang.ByteArray` is a mutable reference, separate from `Array<Number>`.
Canonical names, imported types, and Lang aliases work in parameters/results.
Locals, reassignment, and matching conditional arms preserve the selected
reference; aliases observe its mutations. API minimum: 3.0.0.

`ByteArray or Null` is supported in parameters/results, conditional joins, and
inferred locals. ByteArray/Null alternatives join without allocation or
conversion. Nullable locals retain that contract when assigned either value.
Direct null checks on fresh local/parameter reads establish non-null use in
branches, short-circuit expressions, conditional arms, and loops. Early returns
can exclude Null from a following path. Methods, indexing, and non-null typed
calls require that proof; reassignment invalidates it. An alias or a saved
Boolean does not inherit another read's proof. API availability applies even to
unused nullable declarations or Null-only results.

Nullable storage holds an ordinary strong reference while present. Clearing one
alias leaves the object alive through any remaining strong reference; clearing
the last one allows an existing weak reference to expire.

`new[length]b` allocates a fresh zero-filled ByteArray without expected element
context. Evaluate its non-null Number length once before allocation. Conditional
lengths and allocation arms retain lazy evaluation and effect order. Invalid
sizes and allocation failures remain runtime concerns.

`[elements]b`, including `[]b`, creates a fresh ByteArray with Number/Char
elements. Valid literal Number tokens in `-128..=255` and ASCII Char tokens use
compact DATA constants, normalizing valid signed bytes to their unsigned bits.
Arithmetic and calls are not folded into constants. Computed literals allocate
their final length before evaluating elements left-to-right, storing each value
before evaluating the next. Conditional elements evaluate only their chosen arm.
Private construction cannot escape before every slot has been filled.

Out-of-range Number/Char tokens use indexed stores rather than truncation in a
constant payload, retaining the runtime range check. Other element types are
rejected. Repeated loads of a constant, including an empty constant, produce
independently mutable ByteArrays. No expected element context is required.

Index reads require a non-null Number and return an unsigned Number byte. Writes
accept Number/Char, including indexed compounds whose result matches that
contract. Evaluate receiver, index, then value once, preserving runtime
bounds/range checks. The compiler does not mask or clamp values; a stored `-1`
reads back as `255` in the verified simulator.

Supported native calls use ordinary value-call lowering and API checking:

| Calls | Arguments and result |
|---|---|
| `size()`, `toString()`, `hashCode()` | Number, String, Number respectively |
| `equals(value)` | Supported Object-or-Null value; Boolean result |
| `add(byte)` | Number/Char; ByteArray result |
| `addAll(bytes)` | ByteArray or Number/Char Array; ByteArray result |
| `indexOf(byte)` | Number/Char; Number index or `-1` |
| `remove(byte)`, `removeAll(byte)` | Number/Char; Boolean result |
| `reverse()` | Independent ByteArray with reversed elements |
| `slice(start, end)` | Number-or-Null indexes; independent ByteArray |
| `decodeNumber(format, options)` | NumberFormat and options; Numeric result |
| `encodeNumber(value, format, options)` | Numeric, NumberFormat, options; ByteArray result |

Both append methods mutate the receiver. Bulk Array inputs retain their
invariant non-null Number/Char element contract.

`method(Symbol)` and `weak()` use the existing inherited binding/lifetime
profiles. Weak retrieval remains Object-or-Null. Direct arguments to `addAll`
need an established Array element contract; ambiguous empty Array inference does
not choose Number or Char automatically.

Annotated [fields](fields.md#bytearray-fields) preserve these reference
contracts, with constructor initialization for non-null storage and guarded
local snapshots for nullable reads.

`Array<ByteArray>` and `Array<ByteArray or Null>` preserve invariant element
contracts from API 3.0.0. Nonempty literals can infer non-null ByteArray
elements; typed context also supports empty, nullable, and Null-only literals.
Computed elements use private Array initialization, preserving reference
identity and evaluation order. ByteArray values are never embedded as scalar
Array constant tokens; Null-only literals may use existing DATA constants.

Indexing, writes, and existing native Array methods retain that contract;
`addAll` requires matching element nullability. A nullable indexed read needs a
guarded local snapshot; a check on one indexed read does not prove another.
Sized allocation accepts only nullable elements because each slot starts Null.
Array copies made by `slice`/`reverse` share the ByteArray elements. Replacing
an element leaves saved aliases intact. Elements hold strong references until
removed or replaced, independently of any saved snapshot. Sorting and widening
to ordinary or heterogeneous Array elements remain unsupported.

`Dictionary<K, ByteArray>` and `Dictionary<K, ByteArray or Null>` use existing
supported key contracts from API 3.0.0. Nonempty literals infer ByteArray
values; explicit context also permits empty or Null-only literals. Computed
entries use private ordered Dictionary construction. Value types remain
invariant across parameters, calls, assignments, and saved IR.

Indexed reads and `get` return ByteArray-or-Null even for non-null value
contracts because a key may be absent. Store a read in a local and guard it
before ByteArray operations; `hasKey` or a check on another lookup is not a
non-null proof. Writes and `put` must satisfy the declared value nullability.
`values()` returns an Array with that exact value contract, retaining strong
references independently of later Dictionary changes. Removing/replacing a value
leaves saved aliases and enumerations intact; clearing the last strong reference
allows a weak target to expire. Enumeration order is not specified.

Object/Object-or-Null storage can recover ByteArray through a positive
`instanceof Lang.ByteArray` branch on a fresh local/parameter read. This is a
concrete class contract, with no element parameter to infer. It enables methods,
indexing, and typed calls on the proven path, including saved Object results
from weak retrieval or Method invocation. `as Lang.ByteArray` and `as
Lang.ByteArray or Null` follow the
[unchecked lexical assertion contract](operators.md#lexical-casts); neither checks nor
converts the runtime value. Both forms require API 3.0.0.

`Dictionary<ByteArray, V>` accepts existing value contracts, with API 3.0.0
availability on the key even for unused declarations. Keys must be non-null;
literal construction uses private fills, never scalar DATA keys. `keys()`
returns `Array<ByteArray>` and retains strong references after entry removal.
The verified simulator distinguishes separately allocated ByteArray keys even
when their contents and hashes compare equal. Aliases address the same entry.
The compiler preserves references and performs no key copying, deduplication, or
hash normalization; mutation behavior is delegated to the runtime.

Numeric conversion requires API 3.1.0. `Lang.NUMBER_FORMAT_*` constants are:

| Suffix | Value | Binary representation |
|---|---|---|
| `FLOAT` | 0 | IEEE 754 binary32 |
| `SINT16` | 1 | Signed 16-bit |
| `SINT32` | 2 | Signed 32-bit |
| `SINT8` | 3 | Signed 8-bit |
| `UINT16` | 4 | Unsigned 16-bit |
| `UINT32` | 5 | Unsigned 32-bit |
| `UINT8` | 6 | Unsigned 8-bit |

`Lang.ENDIAN_LITTLE` is 0 and `Lang.ENDIAN_BIG` is 1. These constants require
API 3.1.0. NumberFormat and Endian annotations retain distinct enum contracts;
neither plain Number nor the other enum satisfies them. The VM receives their
integer representations without a separate runtime class or conversion opcode.
An explicit lexical assertion can claim an enum type without validating its
value.

Both conversion calls require an options argument. Direct literals, including
`{}`, receive a checked record contract from call context. Up to two literal
Symbol fields are supported: `:offset` requires Number and defaults to 0;
`:endianness` requires Endian and defaults to little endian. Unknown keys or
wrong field types are rejected. A literal containing an Endian constant/local
under `:endianness` can infer this record contract for reuse through a saved
local. An arbitrary homogeneous Dictionary cannot claim the record proof.
General structural source annotations and record mutation remain unsupported.

Private construction allocates before field effects, preserves source order, and
exposes the record only after every declared field is filled. Values use
ordinary runtime Dictionary storage. Invalid asserted formats/endian values,
offset bounds, conversion ranges, and failures remain native checks; no masking,
conversion, or portable exception guarantee is added by the compiler.

Encoding mutates and returns the receiver. Decoding returns Numeric without
assuming a result class from the format token: UINT32 can require Long, while
FLOAT produces Float in the verified runtime. Use Numeric's common methods or
runtime-class refinement before class-specific operations. See the
[example](../../../examples/compiler-byte-arrays/README.md),
[IR contract](../ir/containers.md#bytearray-values-and-allocation), and
[evidence](../../investigations/byte-arrays.md#source-and-ir-profile).
