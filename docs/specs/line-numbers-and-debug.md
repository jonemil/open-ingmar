# Observed LINE_NUMBERS and DEBUG payloads

Evidence: [controlled SDK comparison](../investigations/code-labels-and-relocation.md), SDK 9.2.0 debug builds / `fr265`. These sections remain raw in our assembly projects; no general relocation writer is implemented.

LINE_NUMBERS (`0xc0de7ab1`) contains a big-endian u16 count followed by that many 16-byte entries:

| Entry offset | Width | Meaning in observed specimens |
|---|---|---|
| 0 | u32 | Address-space-prefixed PC; `0x10xxxxxx` selects ordinary CODE |
| 4 | u32 | Filename record offset within DEBUG |
| 8 | u32 | Function-name record offset within DEBUG |
| 12 | u32 | Source line number |

Both current original examples and controlled SDK variants exactly satisfy the count/length framing. All observed filename and function-name values resolve to string record boundaries in DEBUG, not SYMBOLS. Each DEBUG (`0xd0000d1e`) payload in these specimens is a complete sequence of tag-`01`, u16 UTF-8 byte length, string bytes, and zero terminator records, matching the string framing described in [DATA strings](data-strings.md). Original examples have 225-byte DEBUG payloads; controlled examples have 263 bytes because their build/source paths are longer. Each contains eight strings.

Growing a function changes line PCs after it and can add source mappings inside it. Consequently, blindly adding one delta to all PCs is insufficient. Replacing instructions needs a mapping policy for old PCs inside the edited body; deleting a function needs a policy for its entries. Preserve each address-space prefix and avoid relocating non-CODE entries without evidence for that namespace. Local-variable ranges in external SDK debug XML are separate metadata and are not embedded by our PRG project export.

These are observations for the named SDK/target, not a complete DEBUG format claim for all versions or optimization modes. Unknown framing must remain raw or cause a relocation operation to fail; raw preservation alone cannot guarantee validity after code moves.
