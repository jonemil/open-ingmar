# DEBUG strings and LINE_NUMBERS assembly

Editable source: [DEBUG/line
source](../assembly/grammar.md#debug-strings-and-line-mappings).

Evidence: [controlled SDK
comparison](../../investigations/code-labels-and-relocation.md) and
[structured metadata experiment](../../investigations/line-metadata.md), SDK 9.2.0
debug builds / `fr265`. Section projects select structured `debug` and `lines`
input when each section is unique and its payload is losslessly representable.
Unsupported or duplicate sections remain raw.

## Binary framing

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
[DATA string framing](data-strings.md). Original examples have eight strings in
225 bytes; controlled builds have 263 bytes because their paths are longer.
Other DEBUG record types remain unsupported.

## Edit policy and limits

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
