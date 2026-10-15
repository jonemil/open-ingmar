# DATA scalar Dictionary constants

This profile covers flat Dictionary constants loaded by `hpush`, based on the
[Dictionary experiments](../investigations/compiler-dictionary.md). Nested
containers, Symbol, Long, Float, Double, and other value tags are unsupported.
Entry order and string-pool identity are preserved; the codec does not enforce
source-language key restrictions, uniqueness, hashing, or iteration order.

## Binary record

All integers are big-endian. Starting at a known DATA record boundary:

1. Outer tag `0x03`, then u32 length `N`.
2. Optional string pool: magic `0xabcdabcd`, u32 byte length `P`, then `P` bytes.
3. Value block: magic `0xda7ada7a`, u32 byte length `M`, then `M` bytes.
4. Zero terminator, excluded from `N`.

`N` covers both blocks including their headers, so `N = M + 8` without a pool,
or `N = P + M + 16` with one. Total record size is `N + 6`. The value block
starts with root type `0x0b`, then u32 entry count and that many ordered key/value
pairs. Counts must consume the block exactly. No trailing bytes are allowed.

A pool is an ordered sequence of u16 lengths followed by UTF-8 bytes and a zero
terminator. Its lengths **include** the terminator. Each string reference is a
u32 offset from the first pool length field to another string's length field;
it must identify an exact string boundary. References may share a string, and
unreferenced/duplicate strings remain distinct pool entries. Pool strings are
not global DATA string records. An explicit empty pool is preserved by the
codec; its runtime behavior has not been separately tested.

| Scalar tag | Value bytes | Interpretation |
|---|---|---|
| `0x00` | none | Null |
| `0x01` | 4 | Number bits |
| `0x03` | 4 | Local string-pool offset |
| `0x09` | 1 | Boolean, exactly 0 or 1 |
| `0x13` | 4 | Char bits |

Number/Char bits are retained without Unicode or arithmetic conversion.
Truncation, inconsistent lengths/counts, invalid UTF-8/terminators, unsupported
tags, or references into a string's interior are errors. The complete DATA walk
selects this profile using the pool magic or Dictionary root at a known boundary;
it never scans opaque bytes for apparent records.

## Editable DATA source

```text
.dictionary example at=0x00000000
.string key "one" at=0x00000000
.string value "text" at=0x00000006
.entry string=&key number=0x00000029
.entry number=0x00000002 string=&value
.enddictionary
```

The header's `at=` pins the outer DATA tag. Nested `.string` placements are local
pool offsets. They may be omitted for deterministic consecutive placement;
pinned entries must match the next offset. Local aliases must be valid and
unique within the pool. String length plus terminator must fit u16. `.pool`
retains an explicit empty pool; any `.string` also creates a pool.

Each `.entry` takes exactly two scalars: `null`, `boolean=true|false`,
`number=0x<bits>`, `char=0x<bits>`, or `string=&<local-alias>`. Reference order
within the declaration is unrestricted; all strings resolve during encoding.
The serializer derives lengths/counts and preserves declaration order. Outer
placement follows the existing [DATA layout rules](data-strings.md).

Dictionary declarations live in authoritative DATA source, not inline CODE.
CODE uses `hpush &example`; `Operand::Dictionary` and a separate Dictionary
binding map prevent confusion with Array/String aliases. The current layout
profile permits only zero-prefix DATA addresses below `0x01000000`; numeric
operands retain raw bits without implying a validated record reference.
