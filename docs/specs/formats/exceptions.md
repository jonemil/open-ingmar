# EXCEPTIONS records and assembly

Editable source: [exception source](../assembly/grammar.md#exception-records).

The supported ordinary section is `0x0ece7105`. Source provenance and controlled
SDK/simulator observations are in
[control-flow research](../../investigations/control-flow.md).
Extended `0xeece7105` remains unsupported and raw.

## Binary profile

All fields are big-endian. The payload starts with a u16 record count and
contains exactly that many ordered triples:

| Field | Bytes | Meaning |
|---|---|---|
| begin | 3 | protected-range start, relative to CODE |
| end | 3 | protected-range endpoint, relative to CODE |
| handler | 3 | handler entry, relative to CODE |

Offsets contain no `0x10` CODE address prefix. Their 24-bit limit follows from
the field width. It does not establish a DATA-address or device memory limit.
Truncated counts/records, count mismatches, and trailing bytes are unsupported;
project export preserves these payloads raw.

Parsing and writing retain record order, duplicates, overlaps, and unusual
numeric offsets. They do not sort, merge, infer interval ordering, or validate
numeric targets against CODE. Endpoint inclusivity and broader VM exception
semantics remain unresolved.
