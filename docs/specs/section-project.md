# Complete PRG section project, versions 1–5

`ciq-dis all <input.prg> <directory>` exports the complete section stream. The
directory must not already exist. `ciq-asm project <directory> <output.prg>`
reconstructs it without the original PRG. The output PRG must not already exist.

`sections.ciq` is a UTF-8 manifest. The first meaningful line is
`ciq-project 1` through `ciq-project 5`. Version 2
enables structured symbol input; version 3 adds DATA strings and opaque spans;
version 4 adds structured DEBUG strings and LINE_NUMBERS; version 5 adds
structured EXCEPTIONS. Older projects remain
supported. Each subsequent meaningful line has five whitespace-separated fields:

```text
section 0xd000d00d 33 raw 000-header_versioned-d000d00d.bin
section 0xc0debabe 556 assembly 003-code-c0debabe.mcasm
```

Fields are the `section` keyword, hexadecimal nonzero u32 identifier, original
decimal payload length, encoding (`raw`, `assembly`, or version 2+ `symbols`, or
version 3+ `data`, version 4+ `debug`/`lines`, or version 5 `exceptions`), and a local filename. `#`
starts a comment; blank lines are ignored. Manifest order determines section
order, including duplicate identifiers. Filenames are simple ASCII basenames
using letters, digits, `.`, `_`, or `-`; `.` and `..` are forbidden. Import
rejects paths resolving outside the project directory.

Raw files contain exactly the section payload, without the identifier/length
wrapper. Ordinary code is exported as assembly when the full payload is
supported by the decoder; otherwise it falls back to raw binary with a
diagnostic. A losslessly representable unique symbol table is exported as
`symbols` source. DATA with a complete supported record walk is exported as
`data` source with pinned class and string declarations. Partial
referenced-string exports combine pinned strings and opaque byte spans.
Supported unique DEBUG and LINE_NUMBERS sections use `debug` and `lines` source.
Supported unique EXCEPTIONS uses `exceptions` source.
Other sections retain raw payloads, including unknown identifiers, empty
payloads, extended code, and original signing material. Assembly encoding is
permitted only for ordinary code (`0xc0debabe`); `symbols` encoding is permitted
only for SYMBOLS (`0x5717b015`) in versions 2–5. DATA encoding is permitted only
for DATA (`0xda7ababe`) in versions 3–5.

When a symbol table decodes successfully, export adds an inspection-only
`.symbols.txt` listing. Losslessly representable unique tables become
authoritative `.mcasm` declarations and code uses named symbol references. Other
tables remain raw and code stays numeric with annotations when available.
Reassembly consumes only the manifest-selected source; editing the sidecar does
not change the PRG. Symbol declarations are loaded before code regardless of
section order. See [symbol assembly](symbol-assembly.md).

```text
ciq-project 2
section 0xc0debabe 556 assembly 003-code-c0debabe.mcasm
section 0x5717b015 95991 symbols 010-symbols-5717b015.mcasm
```

All versions require each reconstructed payload to match the manifest length.
Shared labels resolve supported method, line, branch, and exception references, but general
relocation is not implemented; changing a declared length is an explicit edit,
not a check that opaque references remain valid. It regenerates section wrappers
and the eight-byte terminator. The terminator is implicit in the project format
rather than exported as a separate file. Names and original section offsets are
comments for inspection, not input semantics. Export writes the manifest last so
interrupted exports are not mistaken for complete projects.

Project reconstruction does not decode all payload structures. Without `-y`,
signatures are preserved.
`ciq-asm project <directory> <new-output.prg> -y <key.der>` signs after
reconstruction: original developer/store signatures are removed and one
developer signature is appended. Manifest payload-length checks still apply
before signing; the final signing stage may change the section count or
signature payload length. Keys are never part of the export. See
[developer signatures](developer-signatures.md). Exact-byte comparisons apply to
unedited preservation tests; signed transformations compare program content and
signature validity separately.

## DATA source in versions 3–5

```text
ciq-project 3
section 0xda7ababe 12974 data 002-data-da7ababe.mcasm
section 0xc0debabe 556 assembly 003-code-c0debabe.mcasm
section 0x5717b015 95991 symbols 010-symbols-5717b015.mcasm
```

Structured DATA requires a unique DATA section. Its strings, class declarations,
and opaque spans reproduce the section bytes; class/string layout and references
resolve against SYMBOLS before code assembly. Named class fields share the same
aliases as code; method fields may reference CODE labels with
`value=code:label`. Labels are resolved before DATA, independently of manifest
order. Numeric source remains supported. Code uses `news &alias` for supported
string references. Project code files cannot contain inline `.string`
declarations: content belongs in the authoritative DATA source. Unsupported DATA
remains raw. RESOURCES remains raw regardless of read-only string inspection.
See [DATA strings](data-strings.md) and [RESOURCES](resources.md).

Supported DATA also gets an inspection-only `NNN-data.classes.txt` listing from
[class/module inspection](data-classes.md). It is absent from the manifest and
ignored by import, like the symbol listing. Class declarations in authoritative
DATA source are used for full supported walks; opaque spans or raw payload
remain fallbacks. See [class assembly](data-classes.md).

## DEBUG and line source in versions 4 and 5

```text
ciq-project 4
section 0xc0debabe 744 assembly 003-code-c0debabe.mcasm
section 0xd0000d1e 225 debug 004-debug-d0000d1e.mcasm
section 0xc0de7ab1 722 lines 005-line_numbers-c0de7ab1.mcasm
```

`debug` is allowed only for DEBUG (`0xd0000d1e`); `lines` only for LINE_NUMBERS
(`0xc0de7ab1`). Each structured section must be unique. DEBUG uses `.string`
declarations; line source uses
`.line pc=code:label file=&debug_alias function=&debug_alias line=<decimal>`.
DEBUG aliases resolve independently of DATA strings. All bindings load before
encoding, regardless of manifest order. Numeric references remain supported;
invalid/foreign targets are preserved numerically rather than inferred. Export
pins DEBUG strings and verifies structured serialization preserves the original
bytes. See [metadata framing and syntax](line-numbers-and-debug.md).

Supported CODE line targets get labels in the shared code source. Moving these
labels updates named method and line PCs; moving DEBUG strings updates named
line string references. Counts and payload lengths are checked, and named line
PCs must identify instructions, not the end of CODE. Raw sections and numeric
references have no inferred fixups. Unknown DEBUG record kinds, malformed line
framing, and duplicate metadata sections export raw and preserve their bytes.
Placement constraints remain enforced; there is no ignore-`at` option.

## Exception source in version 5

```text
ciq-project 5
section 0xc0debabe 1021 assembly 003-code-c0debabe.mcasm
section 0x0ece7105 20 exceptions 009-exceptions-0ece7105.mcasm
```

`exceptions` is allowed only for ordinary EXCEPTIONS (`0x0ece7105`), which
must be unique when structured. Source uses ordered `.exception` declarations
with numeric CODE offsets or shared `code:label` references. Named starts and
handlers must denote instructions; named ends may denote end-of-CODE. Labels
resolve before exception encoding, regardless of manifest order. Duplicate or
malformed sections remain raw. See [the exception profile](exceptions.md).
