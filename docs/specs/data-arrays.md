# DATA scalar arrays

This profile covers flat constant arrays used by `apush`, based on
[constant-array experiments](../investigations/constant-arrays.md). It does not
cover nested containers, ByteArray, or unlisted scalar types.
[Dictionary constants](data-dictionaries.md) have a separate profile.
Unsupported framing or element tags stop the complete DATA walk.

## Binary record

Use the [shared constant-container framing](data-constants.md), including its
optional local String pool and typed scalar tags. The value block starts with
root `0x05`, then u32 element count and ordered scalar elements. The count must
consume the block exactly. Zero-element records are accepted by the codec;
focused `apush` runtime evidence covers nonempty arrays.

## DATA assembly

```text
.array values
.string local "one"
.element string=&local
.element number=0x0000000a
.element null
.element boolean=true
.element char=0x00000061
.element long=0x0000000100000001
.element float=0x3fa00000
.element double=0x4170000010000000
.endarray
```

`.array <alias> [at=0x...]` starts a declaration and `.endarray` closes it.
Each `.element` has exactly one of `null`, `number=0x<u32>`, `char=0x<u32>`,
`float=0x<u32>`, `long=0x<u64>`, `double=0x<u64>`, `boolean=true`,
`boolean=false`, or `string=&<local-alias>`. Values retain their
types and bit patterns; lengths and counts are derived during encoding.
Whitespace/comments follow the
shared [assembly grammar](bytecode-subset.md). Local `.string` and `.pool` fields
follow the [pool rules](data-constants.md#editable-local-pools); nested containers
are rejected.

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
String values compare by text after resolving local aliases; unused pool strings
do not affect semantic matching.
Section projects keep declarations exclusively in authoritative DATA source.

Same-width scalar changes preserve layout. Changing element kinds/counts or
preceding records can move later DATA targets: update placement constraints and
manifest lengths as needed, and account for opaque or numeric references. Named
array CODE references bind to the resolved layout. Class-field array references
are retained numerically; general DATA relocation remains incomplete.

For the tested flat scalar profile, including String, `apush` creates a mutable array
whose contents are independent across repeated loads of the same constant.
Nested/deep-copy behavior and other SDK/device versions remain unverified.
