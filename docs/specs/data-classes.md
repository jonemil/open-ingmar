# DATA class/module record inspection

Evidence:
[class/module investigation](../investigations/data-classes.md). Records belong
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
[field evidence](../investigations/compiler-symbols.md#symbol-fields). Module
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
reassembly. Authoritative DATA source uses the class assembly described
below when the full walk succeeds; partial/raw fallbacks remain supported.
Relocation is not implemented. No signing material or executable bytes are
changed by inspection.

## Class assembly

Evidence: [serialization and relocation
investigation][ref-serialization-and-relocation-investigati]. Complete supported
DATA exports put class declarations alongside string declarations in the
authoritative DATA source. Opaque `.bytes` spans are also valid input. The class inspection sidecar is still not assembly input.

```text
.class at=0x0000047b version=0x02 extends=none statics=none parent=none module=0x008002e6 app_flags=0x0000
.field key=0x00008146 type=0x06 value=0x10000030
.endclass
```

This is an illustrative method declaration; the real record may have additional
fields. Header attributes must appear in the shown order. Integers use
hexadecimal `0x` prefixes. `none` means an absent header reference and is
distinct from present zero. Version is 1 or 2; v1 requires all four references,
whereas v2 derives its presence byte from the present references. App flags fit
u16. `at` is an optional DATA-relative 24-bit placement constraint and, when
present, must equal the next serialized offset. Without it, classes append in
source order.

Each `.field` has a full u32 `key`, u8 `type`, and u32 `value`, in that order.
Field order, repeated keys, shared targets, unknown types, flag bits, and raw
references are preserved. V1 requires `type` to equal the key's low nibble; v2
stores it separately. Field counts are derived from declarations and must fit u8
(v1) or u16 (v2). `.endclass` closes the block. Nested classes, other directives
inside a class, missing terminators, and extra attributes are errors.

The source model serializes independently of the borrowed inspection model.
Parsed record offsets are inspection metadata; field offsets are recomputed from
the serialized layout. The named forms below resolve typed class/string/symbol
references. Numeric fallbacks remain supported;
[shared code labels](code-labels.md) resolve method targets. Numeric values
are not automatically relocated.

Complete DATA walks export classes and strings only after checking that
re-encoding reproduces the original payload. Unsupported walks retain the
referenced-string/opaque-span fallback or raw DATA. Standalone
`ciq-asm data` accepts this grammar; complete project import still enforces
section lengths. Editing a method target or field is a program modification and
must be re-signed and runtime-validated. Serialization does not establish that a
chosen target has the right calling convention, and does not enable relocation.

## Named declarations and references

Evidence: [named class investigation](../investigations/named-classes.md). An
optional alias follows `.class`, before optional `at`. Aliases obey
symbol/string identifier rules but use a separate class namespace.

```text
.class globals at=0x0000047b version=0x02 extends=none statics=none parent=none module=@globals app_flags=0x007f
.field symbol=@HelloWorldApp bits=0x50 type=0x07 value=%HelloWorldApp
.field symbol=@probe bits=0x10 type=0x06 value=0x10000030 # CODE+0x30
# Remaining fields omitted in this excerpt.
.endclass
```

`symbol=@alias bits=0xNN` replaces `key=0x...`: the resolved ID must fit 24
bits; the low-byte `bits` is preserved independently. Both key syntaxes work for
v1 and v2. Header `parent` and `module` accept `@symbol` or raw integers; other
header roles remain raw. Field `value=%class` is accepted only for type 7;
`value=&string` only for type 3. Raw values remain valid for all types.
Undefined/mismatched references are errors.

Class and string addresses are computed in a layout pass before serialization,
so forward references work. Optional placement constraints are checked;
gaps/overlaps are not silently introduced. This updates explicitly named
references within the authored DATA source, not unknown numeric pointers or
cross-section metadata. Complete project lengths remain enforced.

Project exports reuse aliases from authoritative SYMBOLS source, which import
loads before DATA resolution. Names for class records derive from known module
headers or incoming class fields; collisions use `class_<offset>` fallbacks.
Exports retain pinned offsets, ordered fields, flag bits, and numeric values for
unknown types/address spaces. Field comments show DATA offsets and known CODE
targets for comparison with inspection listings.

Standalone `data-source` retains numeric symbol keys/header values so
`ciq-asm data` can consume it without a separate SYMBOLS file; it still names
classes and strings. Project method fields also accept `value=code:label` for
type 6; the label resolves to a 24-bit CODE offset with prefix `0x10`. Library
callers use `Source::parse_with_references` / `encode_with_references` with both
symbol and code bindings, or `parse_with_symbols` / `encode_with_symbols` for
symbols only; plain `parse`/`encode` operate without such bindings.

[ref-serialization-and-relocation-investigati]: ../investigations/class-assembly.md
