# DATA strings and opaque record spans

Implemented on 2026-10-08. Evidence: [DATA and resource investigation](../investigations/data-and-resources.md). Section ID: `0xda7ababe`.

## String record format

At a supported DATA record boundary, a string has a one-byte tag `01`, a big-endian u16 UTF-8 byte length, that many content bytes, and a zero terminator. The length excludes the tag, length field, and terminator; the record occupies length + 4 bytes. The parser validates bounds, UTF-8, and termination.

`news` has a big-endian u32 data-address operand. This implementation supports only addresses `0x00xxxxxx`, interpreted as offsets to record tags within DATA. A symbol ID is a different kind of reference. Other address prefixes remain numeric and uninterpreted.

Minimal Console's literal record is at DATA+`0x3279`:

```text
01 00 06 48 65 6c 6c 6f 21 00    # Hello!
```

## Discovering record boundaries

The complete record walk recognizes string records and the framing of class/module records. Class fields remain opaque; walking their boundaries does not constitute a class-definition decoder or relocation implementation.

| Class form | Boundary encoding |
|---|---|
| `c1 a5 5d ef` | Four u32 relationship fields, u16 application flags, u8 field count, then count × 8-byte fields |
| `c2 a5 5d ef` | u8 presence flags, an optional u32 for each of low bits 0–3, u16 application flags, u16 field count, then count × 9-byte fields |

Unknown high presence bits, unknown tags, malformed headers, or fields extending past DATA stop the complete walk. JSON/container records are not supported by this walk. The c2 framing and complete walk are validated in both current SDK 9.2.0 `fr265` specimens; c1 framing is source-derived and covered by an independent synthetic fixture, not observed in these SDK specimens.

When the full walk is unavailable, the exporter can still validate string records at addresses supplied by supported `news` instructions. It does not search opaque bytes for an apparent tag. All other bytes remain opaque. Repeated references share one declaration; overlapping candidate records prevent structured export. Missing/duplicate DATA, unavailable instruction decoding, or unsupported references retain numeric operands and raw input as applicable.

## Assembly source and layout

DATA source combines strings with opaque bytes, using the shared quoted-string escapes documented in [symbol assembly](symbol-assembly.md):

```text
.bytes at=0x00000000 ffc1
.string greeting "Hello!" at=0x00000002
.bytes at=0x0000000c ee
```

`.bytes at=0x... <hex-bytes>` contains literal pairs of hexadecimal digits, without a `0x` prefix on the byte sequence. It retains opaque bytes at their original offsets. Export splits opaque spans into 32-byte lines for inspection; reassembly preserves them exactly.

`.string <alias> "value" [at=0x...]` declares a string record. Aliases follow the same identifier rules as symbols, but occupy a separate namespace. Strings are serialized in source order. With no `at`, a string is appended at the next offset. A supplied `at` must equal that next offset; it does not create padding, reorder records, or overwrite another fragment. Duplicate aliases, oversized names, gaps, overlaps, or unsupported address prefixes are errors. No automatic deduplication is performed.

Export uses `str_` plus the eight-digit record offset as an alias and pins every string and opaque span. All authored DATA must fit the supported 24-bit offset space. Strings with equal content at distinct offsets remain separate records. This policy preserves existing layout while allowing deterministic assembly of independently authored string-only data.

## Typed code references

```text
news &greeting
```

`&alias` is accepted only for the string-address operand of `news`. `@alias` remains a symbol-ID reference; neither may substitute for the other. Numeric data addresses remain supported. Project assembly resolves DATA declarations before encoding code, regardless of manifest order.

In a [version 3 section project](section-project.md), the DATA source is authoritative. Project code files reference its aliases; `.string` declarations embedded in those code files are rejected to avoid silently ignoring or duplicating string content. Other sections, including RESOURCES and signing material, retain their selected raw/structured sources.

`ciq-dis code-symbolic` emits inline, pinned declarations for just the strings referenced by code. The template workflow consumes those declarations and applies same-length string edits to the template's DATA records. `ciq_asm::assemble` only returns code bytes; use `assemble_template` or project assembly to materialize DATA. Inline code declarations require `at`; unconstrained layout belongs in a standalone DATA source. Resizing or overlapping template string edits is rejected.

## Commands

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

`strings` prints DATA offsets, UTF-8 byte lengths, and escaped values. `data-source` requires at least one supported string. The standalone `.bin` is a payload without a section header. Complete export retains raw DATA if it cannot represent supported strings safely. Imports still enforce manifest lengths. Resource strings have a separate inspection command described in [RESOURCES](resources.md).

Changing string content is a program modification. Signature regeneration, runtime acceptance, references within opaque class/module records, and general relocation remain separate work.

Class/module records have [read-only field inspection](data-classes.md). Their original bytes remain opaque `.bytes` spans in DATA assembly; the class listing does not become authoritative source.
