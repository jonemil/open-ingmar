# DATA ByteArrays

Compact ByteArray constants are loaded by `bpush`, independently of scalar
Arrays loaded by `apush`. This binary/assembly profile is established by the
[SDK specimen](../investigations/byte-arrays.md); it does not imply executable
ByteArray support in the Monkey C frontend.

## Binary record

Use the [shared constant-container framing](data-constants.md). The value block
contains root `0x14`, a big-endian u32 byte count, and exactly that many untagged
bytes. Bytes retain their unsigned values without sign or character conversion.
The body length is `M = 5 + count`; without a pool, the outer length is
`N = 13 + count` and total record size is `19 + count`.

The codec validates all lengths, count, optional local String pool, and final
zero terminator. It preserves empty records and unused/empty pools, although
those forms have not been separately exercised in the simulator. Unknown roots,
truncation, and trailing bytes stop the record walk; no scanning is performed.
Nested containers and scalar tags have no meaning inside the byte payload.

## DATA assembly

```text
.bytearray bytes at=0x00000000
.bytes 00017f80ffff80
.endbytearray
```

Placement is optional.
`.bytes <hex>` appends hexadecimal pairs, with no prefix or separators. Repeated
lines concatenate; printing wraps at 32 bytes per line. An empty declaration
contains no `.bytes` lines. `.string` and `.pool` retain optional record-local
pools under the shared rules; bytes never reference those strings.

Aliases are unique within the ByteArray namespace, independently of global
String, Array, Dictionary, and class aliases. Unpinned records follow DATA
layout; pins must match their position. `Source::byte_array_bindings` supplies
record starts for typed `bpush &alias` references. The existing zero-prefix
DATA placement profile applies; raw u32 operands preserve other bit patterns
without asserting runtime validity.
