# DATA class/module record inspection

Editable source: [class source](../assembly/grammar.md#class-assembly).

Evidence:
[class/module investigation](../../investigations/data-classes.md). Records belong
to DATA (`0xda7ababe`), interleaved with [string records](data-strings.md). All
multibyte values are big-endian.

## Framing

| Component | Version 1 | Version 2 |
|---|---|---|
| Tag and magic | `c1 a5 5d ef` | `c2 a5 5d ef` |
| Presence byte | absent | low four bits select header references |
| Header references | four u32 values | selected u32 values, in the same order |
| App flags | u16 | u16 |
| Field count | u8 | u16 |
| Each field | u32 key, u32 value | u32 key, u8 type, u32 value |

Header reference order is `extends`, `statics`, `parent`, `module`. In v2, bits
`1`, `2`, `4`, `8` indicate presence respectively. Omitted values remain absent,
rather than becoming zero. Unknown upper presence bits are unsupported. Version
1 has all four references present. Fields occupy eight bytes in v1 and nine
bytes in v2.

`data::classes::records` walks DATA from offset zero, returning strings and
class/module records in order. It validates magic, header bounds, count and
field bounds, and string framing. An unknown record aborts the walk without
scanning forward for another apparent tag. Empty DATA is a valid empty walk.
Class records retain a borrowed raw slice, offsets, all header words, raw
keys/types/values, and ordered fields. Unknown field types are retained because
these field widths are fixed.

## Field and reference interpretation

A field's symbol ID is `key >> 8`. The complete key remains available, and the
listing prints its lower byte as `key_bits`. Version 1's type is `key & 0x0f`;
version 2 stores the type separately. Flag-bit semantics and the app-flags
bitmask remain uninterpreted. Repeated field IDs and fields sharing method
targets are retained.

Type `0x06` identifies a method. An observed value with high byte `0x10`
identifies ordinary CODE, with its lower 24 bits as the code offset. When the
unique ordinary CODE section can be fully decoded, the listing checks whether
that offset is an instruction boundary; otherwise it says `boundary unchecked`.
This check identifies an instruction start, not a proven function signature or
calling convention. Other address spaces retain raw numeric values.

Type `0x07` values are annotated as DATA class references only when they exactly
match a parsed class record's offset. Type `0x03` values are annotated as DATA
strings only when they exactly match a parsed string record's offset. The parser
does not interpret every field value as an address or discover targets by
scanning for tag bytes.

Type `0x08` holds a Symbol default: the value word is a Symbol ID, not a DATA
address or Number. Listings annotate known names. With named-symbol export,
editable DATA uses `value=@alias` when the PRG symbol table resolves that ID;
assembly resolves it through the supplied symbol map. Unknown IDs and raw-symbol
export remain numeric. Symbol value references are valid only with type `0x08`;
Null defaults retain type `0x00` and value zero. See
[field evidence](../../investigations/compiler-symbols.md#symbol-fields). Module
values and other unclassified types stay numeric.

Parent/module header values receive symbol names when the PRG's own symbol table
resolves them. Extends/statics stay raw: in the specimens, `extends=0x40000119`
does not resolve as an ordinary symbol ID and agrees with an API-class field
value. Header references must not universally be classified as symbol IDs.

## Commands and preservation

```sh
cargo run -p ciq-dis -- classes examples/minimal-console/build/fr265/minimal-console.prg
cargo run -p ciq-dis -- classes examples/hello-world/build/fr265/hello-world.prg
```

The listing contains class version, record offset/size, presence flags, app
flags, header values, field offsets, keys, resolved symbol names, types, and
values. Names and string annotations use escaped quoted text. Missing/malformed
symbols leave numeric output available. Missing/unsupported CODE leaves
method-boundary checks unavailable. Missing/duplicate DATA or unsupported DATA
framing is an error for this command.

Complete export adds `NNN-data.classes.txt` when inspection succeeds. It is an
inspection-only sidecar, excluded from `sections.ciq`. Editing it never changes
reassembly. Authoritative DATA source uses the class assembly described below
when the full walk succeeds; partial/raw fallbacks remain supported. Relocation
is not implemented. No signing material or executable bytes are changed by
inspection.
