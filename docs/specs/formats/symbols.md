# Symbol table: observed format

Editable source: [symbol
declarations](../assembly/grammar.md#symbol-declarations).

Evidence: [symbol decoding investigation](../../investigations/symbols.md).
Validated against the SDK 9.2.0 `fr265` Minimal Console and Hello World debug
specimens; stripped binaries may omit this section.

Section identifier: `0x5717b015`. All multibyte integers are big-endian. Its
payload starts with a u16 entry count, followed by that many eight-byte entries:
a u32 symbol identifier and a u32 offset into this section payload. The offset
points to the beginning of a name record, not to the character bytes.

Name records fill the rest of the payload. Each has a one-byte tag `0x01`, a u16
UTF-8 byte length, that many bytes, and a zero terminator. The length excludes
the tag, length field, and terminator. Entries need not refer to names in
physical order. Symbol identifiers and names are preserved as observed; for
example `Toybox_System` is not silently rewritten into `Toybox.System`.

Our decoder checks bounds, record tags, UTF-8, terminators, duplicate symbol
identifiers, and references pointing to actual record boundaries. It resolves
names only for symbol operand fields of known instructions: `spush`, `getselfv`,
`getsv`, both fields of `getmv`, and the second field of `getlocalv`. It does
not treat every numeric operand as a symbol.

Symbols have
[assembly declarations and named instruction operands](../assembly/symbols.md).
Export pins IDs, table entry indexes, and name offsets, then verifies exact
serialization before choosing the structured source as authoritative.
Unrepresentable tables retain raw bytes. The `.symbols.txt` listing remains
inspection-only.
