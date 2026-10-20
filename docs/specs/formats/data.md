# DATA records

DATA strings, classes/modules, and constant containers share one record walk and
source-order layout. This page owns their binary contracts and CODE binding.
[Assembly grammar](../assembly/grammar.md) owns editable declarations.

Contents:

- [Strings and opaque spans](#strings-and-opaque-spans)
- [Classes and modules](#classes-and-modules)
- [Constant-container framing](#constant-container-framing)
- [Array constants](#array-constants)
- [Dictionary constants](#dictionary-constants)
- [ByteArray constants](#bytearray-constants)

## Strings and opaque spans

Editable source: [string/opaque
source](../assembly/grammar.md#strings-and-opaque-spans).

Evidence:
[DATA and resource investigation](../../investigations/data-and-resources.md).
Section ID: `0xda7ababe`.

### String record format

At a supported DATA record boundary, a string has a one-byte tag `01`, a
big-endian u16 UTF-8 byte length, that many content bytes, and a zero
terminator. The length excludes the tag, length field, and terminator; the
record occupies length + 4 bytes. The parser validates bounds, UTF-8, and
termination.

`news` has a big-endian u32 data-address operand. This implementation supports
only addresses `0x00xxxxxx`, interpreted as offsets to record tags within DATA.
A symbol ID is a different kind of reference. Other address prefixes remain
numeric and uninterpreted.

The 24-bit offset restriction is an implementation policy adopted from external
research's prefixed-address patterns. It excludes a working VM address: [a
simulator experiment](../../investigations/string-address-boundary.md) loaded a
literal at DATA+`0x01000000` after increasing the simulated app memory
allowance. The authoring profile has not yet been expanded. The exact upper
limit and selector/offset boundary remain unresolved; do not treat 16 MiB as a
VM capacity specification. See also
[the original provenance](../../investigations/data-and-resources.md).

Minimal Console's literal record is at DATA+`0x3279`:

```text
01 00 06 48 65 6c 6c 6f 21 00    # Hello!
```

### Discovering record boundaries

The complete record walk recognizes strings, [scalar arrays](#array-constants),
[scalar Dictionaries](#dictionary-constants),
and the framing of class/module records. Class fields have a lossless decoder
and constrained serializer; see [class assembly](#classes-and-modules). Record
walking does not implement relocation.

- **Class form:** `c1 a5 5d ef`. **Boundary encoding:** Four u32 relationship
  fields, u16 application flags, u8 field count, then count × 8-byte fields.

- **Class form:** `c2 a5 5d ef`. **Boundary encoding:** u8 presence flags, an
  optional u32 for each of low bits 0–3, u16 application flags, u16 field count,
  then count × 9-byte fields.

Unknown high presence bits, unknown tags, malformed headers, or fields extending
past DATA stop the complete walk. Container forms outside the scalar Array and
Dictionary profiles remain unsupported. The c2 framing and complete walk are
validated in current SDK 9.2.0 `fr265` specimens; c1 framing is source-derived
and covered by an independent synthetic fixture, not observed in these SDK
specimens.

When the full walk is unavailable, the exporter can still validate string
records at addresses supplied by supported `news` instructions. It does not
search opaque bytes for an apparent tag. All other bytes remain opaque. Repeated
references share one declaration; overlapping candidate records prevent
structured export. Missing/duplicate DATA, unavailable instruction decoding, or
unsupported references retain numeric operands and raw input as applicable.

### Typed code references

```text
news &greeting
```

`&alias` is accepted only for the string-address operand of `news`. `@alias`
remains a symbol-ID reference; neither may substitute for the other. Numeric
data addresses remain supported. Project assembly resolves DATA declarations
before encoding code, regardless of manifest order.

In a [section project](README.md#section-project), the DATA source is
authoritative. Project code files reference its aliases; `.string` declarations
embedded in those code files are rejected to avoid silently ignoring or
duplicating string content. Other sections, including RESOURCES and signing
material, retain their selected raw/structured sources.

`ciq-dis code-symbolic` emits inline, pinned declarations for just the strings
referenced by code. The template workflow consumes those declarations and
applies same-length string edits to the template's DATA records.
`ciq_asm::assemble` only returns code bytes; use `assemble_template` or project
assembly to materialize DATA. Template string edits still require `at`; resizing
or overlapping edits are rejected. Shared CODE assembly also accepts unplaced
`.string` declarations as content requirements. They need external DATA offsets
when referenced and do not allocate records. A standalone DATA source remains
responsible for unconstrained layout.

The compiler's `--data` binding checks each unplaced requirement against exactly
one DATA record with matching content. Pinned requirements must also match their
offset. Layout-only DATA parsing retains unresolved class/method aliases so
strings can be bound before CODE is encoded; final PRG reconstruction must
resolve those fields. See [compiler
pipeline](../ir/README.md#model-and-validation) and
[runtime evidence](../../investigations/compiler-strings.md).

### Commands

```sh
# Inspect all strings when the record walk succeeds, or supported news references otherwise.
cargo run -p ciq-dis -- strings examples/minimal-console/build/fr265/minimal-console.prg
# Print reassemblable DATA, including opaque spans.
cargo run -p ciq-dis -- data-source examples/minimal-console/build/fr265/minimal-console.prg > scratch/data.mcasm
cargo run -p ciq-asm -- data scratch/data.mcasm scratch/data.bin
# Export and import a complete project using new paths.
cargo run -p ciq-dis -- all examples/minimal-console/build/fr265/minimal-console.prg scratch/string-project
cargo run -p ciq-asm -- project scratch/string-project scratch/string-project.prg
```

`strings` prints DATA offsets, UTF-8 byte lengths, and escaped values.
`data-source` accepts a complete supported class/string walk, including
class-only or empty DATA; partial fallback requires a supported string. The
standalone `.bin` is a payload without a section header. Complete export retains
raw DATA if it cannot represent supported strings safely. Imports still enforce
manifest lengths. Resource strings have a separate inspection command described
in [RESOURCES](metadata.md#resources).

Changing string content is a program modification. Own-key signing is
implemented; runtime acceptance of a particular string change and general
relocation remain separate checks.

Class/module records have
[field inspection and constrained assembly](#classes-and-modules). Full supported
exports use `.class` blocks; partial exports may retain opaque `.bytes` spans.
The class inspection listing does not become authoritative source.

## Classes and modules

Editable source: [class source](../assembly/grammar.md#class-assembly).

Evidence:
[class/module investigation](../../investigations/data-classes.md). Records belong
to DATA (`0xda7ababe`), interleaved with [string
records](#strings-and-opaque-spans). All multibyte values are big-endian.

### Framing

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

### Field and reference interpretation

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

### Commands and preservation

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

## Constant-container framing

Editable source: [local pool
source](../assembly/grammar.md#editable-local-pools).

[Arrays](#array-constants), [Dictionaries](#dictionary-constants), and
[ByteArrays](#bytearray-constants) share the following framing and optional
record-local String pool. Arrays/Dictionaries use tagged scalar elements;
ByteArrays use compact untagged bytes. Nested containers, Symbol and other
unlisted scalar tags remain unsupported. Wider numeric runtime evidence comes
from [Array](../../investigations/compiler-numeric-arrays.md) and
[Dictionary](../../investigations/compiler-numeric-dictionaries.md) specimens.

### Binary framing and scalars

All integers are big-endian. Starting at a known DATA record boundary:

1. Outer tag `0x03`, then u32 length `N`.
2. Optional string pool: magic `0xabcdabcd`, u32 byte length `P`, then `P` bytes.
3. Value block: magic `0xda7ada7a`, u32 byte length `M`, then `M` bytes.
4. Zero terminator, excluded from `N`.

`N` covers both blocks including their headers, so `N = M + 8` without a pool,
or `N = P + M + 16` with one. Total record size is `N + 6`. The value block
starts with a root type and u32 count: Array `0x05` has that many scalar
elements; Dictionary `0x0b` has that many ordered scalar key/value pairs;
ByteArray `0x14` has that many unsigned bytes. Counts must consume the block
exactly. No trailing bytes are allowed.

A pool is an ordered sequence of u16 lengths followed by UTF-8 bytes and a zero
terminator. Its lengths **include** the terminator. Each string reference is a
u32 offset from the first pool length field to another string's length field; it
must identify an exact string boundary. References may share a string, and
unreferenced/duplicate strings remain distinct pool entries. Pool strings are
not global DATA string records. An explicit empty pool is preserved by the
codec; its runtime behavior has not been separately tested.

| Scalar tag | Value bytes | Interpretation |
|---|---|---|
| `0x00` | none | Null |
| `0x01` | 4 | Number bits |
| `0x02` | 4 | IEEE 754 Float bits |
| `0x03` | 4 | Local string-pool offset |
| `0x09` | 1 | Boolean, exactly 0 or 1 |
| `0x0e` | 8 | Long two's-complement bits |
| `0x0f` | 8 | IEEE 754 Double bits |
| `0x13` | 4 | Char bits |

Numeric/Char bits are retained without Unicode or arithmetic conversion.
Floating signed zero, infinities, and NaN payloads remain exact bit patterns.
Truncation, inconsistent lengths/counts, invalid UTF-8/terminators, unsupported
tags, or references into a string's interior are errors. The complete DATA walk
validates the optional pool and dispatches on the value-block root tag; pool
presence alone does not distinguish Array from Dictionary. It never scans opaque
bytes for apparent records.

## Array constants

Editable source: [Array
declarations](../assembly/grammar.md#array-declarations).

This profile covers flat constant arrays used by `apush`, based on
[constant-array experiments](../../investigations/constant-arrays.md). It does not
cover nested containers or unlisted scalar types. Compact
[ByteArray records](#bytearray-constants) have a distinct root and load opcode.
[Dictionary constants](#dictionary-constants) have a separate profile.
Unsupported framing or element tags stop the complete DATA walk.

### Binary record

Use the [shared constant-container framing](#constant-container-framing),
including its optional local String pool and typed scalar tags. The value block
starts with root `0x05`, then u32 element count and ordered scalar elements. The
count must consume the block exactly. Zero-element records are accepted by the
codec; focused `apush` runtime evidence covers nonempty arrays.

### CODE binding and editing

`apush` (`0x40`) has one big-endian u32 operand pointing to the **outer tag** at
the record start. A section project's CODE can use `apush &values`; the
assembler resolves it through the DATA array bindings. `news &values` cannot
resolve an array alias, and `apush` cannot resolve a string alias. Numeric
operands retain their raw bits, including addresses outside the named-binding
profile.

`ciq-dis all` emits pinned `.array array_<offset>` declarations and matching
named CODE references for recognized targets. Standalone symbolic CODE export
keeps `apush` numeric because it does not embed array declarations. Library
users can supply separate maps through `Program::encode_with_literals`.

Standalone/compiler CODE can also carry inline `.array` requirements using the
same grammar. They occupy no CODE bytes and do not allocate DATA. An `at` binds
the reference directly; an unplaced requirement needs an external array map. The
compiler's `array_bindings` checks typed content against an explicit DATA
source: exactly one record must match, and pinned offsets must also match.
String values compare by text after resolving local aliases; unused pool strings
do not affect semantic matching. Section projects keep declarations exclusively
in authoritative DATA source.

Same-width scalar changes preserve layout. Changing element kinds/counts or
preceding records can move later DATA targets: update placement constraints and
manifest lengths as needed, and account for opaque or numeric references. Named
array CODE references bind to the resolved layout. Class-field array references
are retained numerically; general DATA relocation remains incomplete.

For the tested flat scalar profile, including String, `apush` creates a mutable
array whose contents are independent across repeated loads of the same constant.
Nested/deep-copy behavior and other SDK/device versions remain unverified.

## Dictionary constants

Editable source: [Dictionary
declarations](../assembly/grammar.md#dictionary-declarations).

This profile covers flat Dictionary constants loaded by `hpush`, based on the
[Dictionary experiments](../../investigations/compiler-dictionary.md). Nested
containers, Symbol, and unlisted value tags are unsupported. The shared codec
retains Long/Float/Double tags with
[Dictionary runtime evidence](../../investigations/compiler-numeric-dictionaries.md).
Entry order and string-pool identity are preserved; the codec does not enforce
source-language key restrictions, uniqueness, hashing, or iteration order.

### Binary record

Use the [shared constant-container framing](#constant-container-framing). The
value block has root `0x0b`, a u32 entry count, and ordered scalar key/value
pairs. The count must consume the value block exactly.

## ByteArray constants

Editable source: [ByteArray
declarations](../assembly/grammar.md#bytearray-declarations).

Compact ByteArray constants are loaded by `bpush`, independently of scalar
Arrays loaded by `apush`. This binary/assembly profile is established by the
[SDK specimen](../../investigations/byte-arrays.md). The
[source profile](../frontend/README.md#bytearray) separately defines executable
ByteArray support in the Monkey C frontend.

### Binary record

Use the [shared constant-container framing](#constant-container-framing). The
value block contains root `0x14`, a big-endian u32 byte count, and exactly that
many untagged bytes. Bytes retain their unsigned values without sign or
character conversion. The body length is `M = 5 + count`; without a pool, the
outer length is `N = 13 + count` and total record size is `19 + count`.

The codec validates all lengths, count, optional local String pool, and final
zero terminator. Empty records are covered by literal runtime checks;
unused/empty pools are preserved but remain untested in the simulator. Unknown
roots, truncation, and trailing bytes stop the record walk; no scanning is
performed. Nested containers and scalar tags have no meaning inside the byte
payload.
