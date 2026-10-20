# DATA flat constant containers

Editable source: [local pool
source](../assembly/grammar.md#editable-local-pools).

[Arrays](data-arrays.md), [Dictionaries](data-dictionaries.md), and
[ByteArrays](data-byte-arrays.md) share the following framing and optional
record-local String pool. Arrays/Dictionaries use tagged scalar elements;
ByteArrays use compact untagged bytes. Nested containers, Symbol and other
unlisted scalar tags remain unsupported. Wider numeric runtime evidence comes
from [Array](../../investigations/compiler-numeric-arrays.md) and
[Dictionary](../../investigations/compiler-numeric-dictionaries.md) specimens.

## Binary framing and scalars

All integers are big-endian. Starting at a known DATA record boundary:

1. Outer tag `0x03`, then u32 length `N`.
2. Optional string pool: magic `0xabcdabcd`, u32 byte length `P`, then `P` bytes.
3. Value block: magic `0xda7ada7a`, u32 byte length `M`, then `M` bytes.
4. Zero terminator, excluded from `N`.

`N` covers both blocks including their headers, so `N = M + 8` without a pool,
or `N = P + M + 16` with one. Total record size is `N + 6`. The value block
starts with a root type and u32 count: Array `0x05` has that many scalar
elements; Dictionary `0x0b` has that many ordered scalar key/value pairs;
ByteArray `0x14` has that many unsigned bytes. Counts must consume the block
exactly. No trailing bytes are allowed.

A pool is an ordered sequence of u16 lengths followed by UTF-8 bytes and a zero
terminator. Its lengths **include** the terminator. Each string reference is a
u32 offset from the first pool length field to another string's length field; it
must identify an exact string boundary. References may share a string, and
unreferenced/duplicate strings remain distinct pool entries. Pool strings are
not global DATA string records. An explicit empty pool is preserved by the
codec; its runtime behavior has not been separately tested.

| Scalar tag | Value bytes | Interpretation |
|---|---|---|
| `0x00` | none | Null |
| `0x01` | 4 | Number bits |
| `0x02` | 4 | IEEE 754 Float bits |
| `0x03` | 4 | Local string-pool offset |
| `0x09` | 1 | Boolean, exactly 0 or 1 |
| `0x0e` | 8 | Long two's-complement bits |
| `0x0f` | 8 | IEEE 754 Double bits |
| `0x13` | 4 | Char bits |

Numeric/Char bits are retained without Unicode or arithmetic conversion.
Floating signed zero, infinities, and NaN payloads remain exact bit patterns.
Truncation, inconsistent lengths/counts, invalid UTF-8/terminators, unsupported
tags, or references into a string's interior are errors. The complete DATA walk
validates the optional pool and dispatches on the value-block root tag; pool
presence alone does not distinguish Array from Dictionary. It never scans opaque
bytes for apparent records.
