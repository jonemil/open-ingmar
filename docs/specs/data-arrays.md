# DATA scalar arrays

This profile covers flat constant arrays used by `apush`, based on
[constant-array experiments](../investigations/constant-arrays.md). It does not
cover string pools, nested containers, Dictionary, ByteArray, or other scalar
types. [Dictionary constants](data-dictionaries.md) have a separate profile.
Unsupported framing or element tags stop the complete DATA walk.

## Binary record

Offsets below are relative to the record's DATA address. Integers are big-endian.

| Offset | Width | Meaning |
|---|---|---|
| `0` | 1 | Record tag `0x03` |
| `1` | 4 | Outer length `N`, excluding tag/length and final terminator |
| `5` | 4 | Magic `0xda7ada7a` |
| `9` | 4 | Inner length `M = N - 8` |
| `13` | 1 | Root type `0x05` (Array) |
| `14` | 4 | Element count |
| `18` | variable | Typed scalar elements |
| `5 + N` | 1 | Zero terminator |

`M` includes the root type, count, and all element bytes, but excludes the
terminator. Total record size is `N + 6`. The element count must consume exactly
the declared element bytes; trailing bytes, truncation, inconsistent lengths,
unsupported magic/root type, and a missing terminator are errors.

| Element tag | Value bytes | Supported interpretation |
|---|---|---|
| `0x00` | none | Null |
| `0x01` | 4 | Number, preserving raw 32-bit bits |
| `0x09` | 1 | Boolean: `0` false, `1` true |
| `0x13` | 4 | Char, preserving raw 32-bit bits |

Char bits are not validated as Unicode by this container codec. Boolean bytes
other than 0/1 are rejected. Zero-element records are accepted by the codec;
the focused `apush` runtime evidence covers nonempty arrays.

## DATA assembly

```text
.array values
.element number=0x0000000a
.element null
.element boolean=true
.element char=0x00000061
.endarray
```

`.array <alias> [at=0x...]` starts a declaration and `.endarray` closes it.
Each `.element` has exactly one of `null`, `number=0x<u32>`, `char=0x<u32>`,
`boolean=true`, or `boolean=false`. Values retain their types and bit patterns;
lengths and counts are derived during encoding. Whitespace/comments follow the
shared [assembly grammar](bytecode-subset.md). Nested declarations are rejected.

Declarations share source-order DATA layout with strings, classes, and opaque
spans. Without `at`, an array follows the preceding fragment. A pinned `at` must
match that position; it does not insert padding or perform relocation. Aliases
follow the symbol identifier grammar and must be unique within the array
namespace, separately from strings and classes. DATA layout uses the current
bounded zero-prefix address profile; this is not a proven universal VM limit.

## CODE binding and editing

`apush` (`0x40`) has one big-endian u32 operand pointing to the **outer tag** at
the record start. A section project's CODE can use `apush &values`; the assembler
resolves it through the DATA array bindings. `news &values` cannot resolve an
array alias, and `apush` cannot resolve a string alias. Numeric operands retain
their raw bits, including addresses outside the named-binding profile.

`ciq-dis all` emits pinned `.array array_<offset>` declarations and matching
named CODE references for recognized targets. Standalone symbolic CODE export
keeps `apush` numeric because it does not embed array declarations. Library users
can supply separate maps through `Program::encode_with_literals`.

Standalone/compiler CODE can also carry inline `.array` requirements using the
same grammar. They occupy no CODE bytes and do not allocate DATA. An `at` binds
the reference directly; an unplaced requirement needs an external array map.
The compiler's `array_bindings` checks typed content against an explicit DATA
source: exactly one record must match, and pinned offsets must also match.
Section projects keep declarations exclusively in authoritative DATA source.

Same-width scalar changes preserve layout. Changing element kinds/counts or
preceding records can move later DATA targets: update placement constraints and
manifest lengths as needed, and account for opaque or numeric references. Named
array CODE references bind to the resolved layout. Class-field array references
are retained numerically; general DATA relocation remains incomplete.

For the tested flat scalar profile, executing `apush` creates a mutable array
whose contents are independent across repeated loads of the same constant.
Nested/deep-copy behavior and other SDK/device versions remain unverified.
