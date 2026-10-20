# DATA ByteArrays

Editable source: [ByteArray
declarations](../assembly/grammar.md#bytearray-declarations).

Compact ByteArray constants are loaded by `bpush`, independently of scalar
Arrays loaded by `apush`. This binary/assembly profile is established by the
[SDK specimen](../../investigations/byte-arrays.md). The
[source profile](../frontend/lang-bytearray.md#bytearrays) separately defines executable
ByteArray support in the Monkey C frontend.

## Binary record

Use the [shared constant-container framing](data-constants.md). The value block
contains root `0x14`, a big-endian u32 byte count, and exactly that many
untagged bytes. Bytes retain their unsigned values without sign or character
conversion. The body length is `M = 5 + count`; without a pool, the outer length
is `N = 13 + count` and total record size is `19 + count`.

The codec validates all lengths, count, optional local String pool, and final
zero terminator. Empty records are covered by literal runtime checks;
unused/empty pools are preserved but remain untested in the simulator. Unknown
roots, truncation, and trailing bytes stop the record walk; no scanning is
performed. Nested containers and scalar tags have no meaning inside the byte
payload.
