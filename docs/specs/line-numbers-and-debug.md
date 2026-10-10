# DEBUG strings and LINE_NUMBERS assembly

Implemented 2026-10-09. Evidence: [controlled SDK comparison](../investigations/code-labels-and-relocation.md) and [structured metadata experiment](../investigations/line-metadata.md), SDK 9.2.0 debug builds / `fr265`. Project version 4 selects structured `debug` and `lines` input when each section is unique and its payload is losslessly representable. Unsupported or duplicate sections remain raw.

## Binary framing

LINE_NUMBERS (`0xc0de7ab1`) contains a big-endian u16 count followed by that many 16-byte entries:

| Entry offset | Width | Meaning in observed specimens |
|---|---|---|
| 0 | u32 | Address-space-prefixed PC; `0x10xxxxxx` selects ordinary CODE |
| 4 | u32 | Filename record offset within DEBUG |
| 8 | u32 | Function-name record offset within DEBUG |
| 12 | u32 | Source line number |

The parser requires the payload length to match the count exactly. Preserve entry order, duplicate PCs, and every numeric value. There is no sorting or deduplication.

In both examples and the controlled SDK variants, all filename/function-name values resolve to string record boundaries within DEBUG, **not SYMBOLS IDs**. DEBUG (`0xd0000d1e`) is a complete sequence of tag-`01`, big-endian u16 UTF-8 byte length, string bytes, and zero terminator records, matching [DATA string framing](data-strings.md). Original examples have eight strings in 225 bytes; controlled builds have 263 bytes because their paths are longer. Other DEBUG record types remain unsupported.

## Source and references

DEBUG source accepts only `.string` declarations, using the DATA string grammar and independent aliases:

```text
.string source_file "example.mc" at=0x00000000
.string function_name "probe"
```

`at=` is optional. Pinned records keep their placement or fail on conflicts; unpinned records use the existing deterministic string layout. Export pins every record and uses aliases `dbg_<original-offset>`. Equal text is not merged. DEBUG aliases belong to a separate namespace from DATA string aliases; a line reference resolves only against DEBUG.

LINE_NUMBERS source uses one declaration per entry, with attributes in this order:

```text
.line pc=code:probe file=&source_file function=&function_name line=22
.line pc=0x1000003a file=0x00000000 function=0x0000000e line=23
```

`line=` is a decimal u32. Numeric PCs and DEBUG offsets are `0x`-prefixed u32 values and serialize unchanged. `pc=code:label` emits `0x10000000 | label_offset`; the offset must fit 24 bits and identify an instruction within the unique ordinary CODE section. End-of-CODE targets and undefined labels fail. `file=&alias` and `function=&alias` resolve DEBUG record offsets, requiring structured DEBUG input. Numeric offsets are preserved even when their target is unknown; this is preservation, not target validation.

Full export names PCs only when their address-space prefix is `0x10` and the decoded CODE offset is an instruction boundary. It reuses method labels or adds `code_<original-offset>` labels. Filename/function references become aliases only at known DEBUG string boundaries. Unsupported CODE, foreign address spaces, invalid PCs, and unknown DEBUG offsets remain numeric. Re-export after an edit may choose different source alias names, while reproducing the same bytes.

Import reads all declarations and CODE labels before resolving metadata, independently of manifest order. Moving a label updates method fields and named line PCs; moving an unpinned DEBUG string updates named filename/function references. The manifest still requires explicit payload lengths. These references do not relocate branches, exception intervals, nonzero header offsets, or opaque payloads.

## Edit policy and limits

Growing a function can move later PCs and add source mappings inside it. Existing mappings inside a replacement need an explicit policy: retain their labels at chosen instructions, edit/delete their entries, or add new entries. The assembler cannot infer source line meanings from instruction changes. Preserve foreign address-space prefixes unless their semantics are established. Local-variable ranges in the SDK's external debug XML are separate metadata and are not embedded by our PRG project export.

These supported framings describe the named specimens, not all SDK versions, targets, optimization modes, or DEBUG record kinds. Successful raw preservation does not guarantee validity after code moves. The [in-place size experiment](../investigations/line-metadata.md) deliberately uses branch-free CODE, empty EXCEPTIONS, and zero background/glance header offsets; the shrunk variant now has confirmed host execution returning `45`. Runtime execution does not independently test debugger behavior or every line mapping.
