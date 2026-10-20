# PRG metadata sections

SYMBOLS, DEBUG/LINE_NUMBERS, EXCEPTIONS, and RESOURCES have separate section
payloads, described together here for reference binding and preservation.
Recognized records do not authorize guessing unknown references or relocating
opaque data. [Assembly grammar](../assembly/grammar.md) owns editable syntax.

Contents:

- [SYMBOLS](#symbols)
- [DEBUG and LINE_NUMBERS](#debug-and-line_numbers)
- [EXCEPTIONS](#exceptions)
- [RESOURCES](#resources)

## SYMBOLS

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
[assembly declarations and named instruction operands](../assembly/README.md#symbol-layout-and-binding).
Export pins IDs, table entry indexes, and name offsets, then verifies exact
serialization before choosing the structured source as authoritative.
Unrepresentable tables retain raw bytes. The `.symbols.txt` listing remains
inspection-only.

## DEBUG and LINE_NUMBERS

Editable source: [DEBUG/line
source](../assembly/grammar.md#debug-strings-and-line-mappings).

Evidence: [controlled SDK
comparison](../../investigations/code-labels-and-relocation.md) and
[structured metadata experiment](../../investigations/line-metadata.md), SDK 9.2.0
debug builds / `fr265`. Section projects select structured `debug` and `lines`
input when each section is unique and its payload is losslessly representable.
Unsupported or duplicate sections remain raw.

### Binary framing

LINE_NUMBERS (`0xc0de7ab1`) contains a big-endian u16 count followed by that
many 16-byte entries:

| Entry offset | Width | Meaning in observed specimens |
|---|---|---|
| 0 | u32 | Address-space-prefixed PC; `0x10xxxxxx` selects ordinary CODE |
| 4 | u32 | Filename record offset within DEBUG |
| 8 | u32 | Function-name record offset within DEBUG |
| 12 | u32 | Source line number |

The parser requires the payload length to match the count exactly. Preserve
entry order, duplicate PCs, and every numeric value. There is no sorting or
deduplication.

In both examples and the controlled SDK variants, all filename/function-name
values resolve to string record boundaries within DEBUG, **not SYMBOLS IDs**.
DEBUG (`0xd0000d1e`) is a complete sequence of tag-`01`, big-endian u16 UTF-8
byte length, string bytes, and zero terminator records, matching
[DATA string framing](data.md#strings-and-opaque-spans). Original examples have eight strings in
225 bytes; controlled builds have 263 bytes because their paths are longer.
Other DEBUG record types remain unsupported.

### Edit policy and limits

Growing a function can move later PCs and add source mappings inside it.
Existing mappings inside a replacement need an explicit policy: retain their
labels at chosen instructions, edit/delete their entries, or add new entries.
The assembler cannot infer source line meanings from instruction changes.
Preserve foreign address-space prefixes unless their semantics are established.
Local-variable ranges in the SDK's external debug XML are separate metadata and
are not embedded by our PRG project export.

These supported framings describe the named specimens, not all SDK versions,
targets, optimization modes, or DEBUG record kinds. Successful raw preservation
does not guarantee validity after code moves. Runtime execution alone does not
validate debugger behavior or every line mapping. [Controlled size
experiments](../../investigations/line-metadata.md) record the tested relocation
constraints and runtime observations.

## EXCEPTIONS

Editable source: [exception source](../assembly/grammar.md#exception-records).

The supported ordinary section is `0x0ece7105`. Source provenance and controlled
SDK/simulator observations are in
[control-flow research](../../investigations/control-flow.md).
Extended `0xeece7105` remains unsupported and raw.

### Binary profile

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

## RESOURCES

Section ID: `0xf00d600d`. Evidence:
[DATA and resource investigation](../../investigations/data-and-resources.md).
Resource strings are independently indexed assets; they are not `news` literals
or symbol-name records.

### Read-only string inspection

A supported index consists of a big-endian u16 entry count followed by count ×
8-byte entries: u32 ID and u32 offset relative to the RESOURCES payload. Strings
use three index levels:

1. Top-level category `0x008000a2` points to the string-language index.
2. A language key points to that language's resource-ID index.
3. A resource ID points to a string container.

The observed string container has tag `01`, big-endian u16 UTF-8 byte length,
contents, and a zero terminator. Its format matches the observed DATA string
record, but its address belongs to RESOURCES. IDs and language keys are not
offsets.

```sh
cargo run -p ciq-dis -- resource-strings examples/minimal-console/build/fr265/minimal-console.prg
```

Output for the current Minimal Console specimen:

```text
language=0x008002e7 resource=0x00000083 @0x00001e46 "Minimal Console"
```

The parser validates table bounds, duplicate IDs within each table, string
bounds/encoding/termination, and cyclic or shared language-table offsets. Shared
language indexes are unsupported by the reader. It inspects the string category
only and leaves other categories uninterpreted. String offsets are currently
limited to the same zero-prefix 24-bit range as the shared record decoder.

The language key remains numeric. Our specimens use `0x008002e7`, whose SYMBOLS
name is `<valyrian>`; the source reference's English example uses `0x008002eb`,
named `<eng>` locally. We have not established the compiler's
default-language/fallback rules and do not label the observed key as English.

### Preservation and unsupported scope

RESOURCES remains raw binary in section projects. There is no resource string
assembler or resource relocation yet. The read-only reader can validate the
current string indexes without changing any resource bytes.

Resource-ID binding, generated `Rez` declarations, language selection/fallback,
and lossless resource assembly remain unspecified. Bitmap/font/JSON/animation
containers, background/glance indexes, and compressed/device-specific assets are
outside this inspection profile. [The plan](../../plan.md) owns research
priorities.
