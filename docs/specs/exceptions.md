# EXCEPTIONS records and assembly

The supported ordinary section is `0x0ece7105`. Source provenance and controlled
SDK/simulator observations are in
[control-flow research](../investigations/control-flow.md).
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

## Section source

Section projects support `exceptions` encoding. Each line has exactly
the following directive and attributes, in this order:

```text
.exception begin=code:try_begin end=code:try_end handler=code:catch_begin
.exception begin=0x00003a end=0x00004c handler=0x00004c
```

Each value is a `0x`-prefixed u24 hexadecimal offset or a `code:label` reference.
Aliases follow ordinary CODE label rules. Blank lines and `#` comments are
allowed; the record count is derived from declarations and must fit u16. An
empty source encodes the two-byte zero count. Canonical numeric output uses six
hexadecimal digits. No range ordering is imposed.

Import requires a unique EXCEPTIONS section when structured encoding is used.
Named offsets resolve after CODE layout regardless of manifest order, requiring
a unique CODE section and defined labels. Named starts and handlers must denote
instructions. A named end may denote an instruction boundary or the end of
CODE. All resolved offsets must fit u24. Numeric values preserve their bits
and remain unchanged when labels move.

Export shares existing method, line, and branch labels, adding `code_<offset>`
labels for verified exception boundaries. It names end-of-CODE only in an end
field. Unknown/invalid target offsets stay numeric; supported exception records
can therefore remain structured even when CODE is unsupported or absent.
Duplicate exception sections export raw. Unchanged export/import must preserve
every payload byte.

Moving labels updates named exception fields, not other opaque references.
Payload lengths must still match the manifest; changing them explicitly does
not establish general relocation safety. See
[implementation and runtime evidence](../investigations/exception-assembly.md).
