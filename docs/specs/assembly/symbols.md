# Symbol assembly and named operands

Evidence and validation:
[symbol assembly investigation](../../investigations/symbol-assembly.md). Binary
layout: [symbol table format](../formats/symbols.md).

## Layout policy

Pinned table slots are reserved first. Unpinned entries occupy the remaining
slots in declaration order. Pinned name records are reserved at their exact
offsets. Unpinned names are placed, in declaration order, in the first fitting
gap after the table, or appended after existing records. Unconstrained records
are not deduplicated by name.

Multiple declarations may share a pinned name record only when their offset and
name agree. Different records cannot overlap. Every serialized name must fit a
u16 UTF-8 byte length, and the table must fit a u16 entry count. Records remain
packed: padding bytes and leftover gaps are unsupported. The deterministic
allocation policy reports a conflict when it cannot fill a gap; it does not
search alternative packing permutations or silently move pinned records.

Export pins every name offset and table index. It checks that serializing the
declarations reproduces the entire original payload before selecting structured
symbols. Malformed tables, duplicate symbol sections, or tables containing
unreferenced name records retain raw binary input. Shared referenced name
records are supported. Thus exporting a table never silently drops bytes that
its declarations cannot represent.

## Named instruction operands

`@alias` is allowed only in known symbol-ID fields: `spush`, `getselfv`,
`getsv`, both fields of `getmv`, and the second field of `getlocalv`. Numeric
`0x...` operands remain valid and may be mixed with aliases. Declarations are
collected before instructions, so forward references work. Undefined aliases,
duplicate declarations, and symbol references in counts, local indexes, or data
addresses are errors.

```text
getmv @system @println
getlocalv 0x01 @println
news 0x00003279           # a data address, not a symbol reference
```

Export uses a stored name directly as an alias when it is a unique valid
identifier and does not start with reserved export prefix `sym_`. Otherwise it
uses `sym_` plus the eight-digit hexadecimal ID. This makes duplicate names and
names containing punctuation unambiguous. Users can rename aliases by updating
their references; alias spelling does not enter the PRG.

## Commands and project integration

Export and reconstruct complete PRGs using new paths:

```sh
cargo run -p ciq-dis -- all examples/minimal-console/build/fr265/minimal-console.prg scratch/symbol-project
cargo run -p ciq-asm -- project scratch/symbol-project scratch/symbol-project.prg
```

The [manifest](../formats/section-project.md) chooses `symbols` encoding for the
authoritative symbol `.mcasm` file. Code files reference those declarations even
when the symbol section follows CODE in the manifest. There must be exactly one
symbol section when structured symbols are used. Code may add inline
declarations with distinct aliases; only the manifest-selected symbol source is
serialized into SYMBOLS. Inline declarations resolve code IDs and do not
implicitly add table entries. An include mechanism is not implemented.

The `.symbols.txt` sidecar remains inspection-only. Raw and structured payloads
are never both authoritative for the same section. Raw symbol payloads are also
accepted.

Inspect or assemble the symbol payload independently:

```sh
cargo run -p ciq-dis -- symbol-source examples/minimal-console/build/fr265/minimal-console.prg > scratch/symbols.mcasm
cargo run -p ciq-asm -- symbols scratch/symbols.mcasm scratch/symbols.bin
```

The output `.bin` contains just the SYMBOLS payload, without a section header.
`symbol-source` reports an error if declarations cannot preserve the whole
payload; complete export falls back to raw instead. The existing `ciq-dis
symbols` command retains its ID/offset/name inspection listing.

For self-contained symbolic code in the template workflow:

```sh
cargo run -p ciq-dis -- code-symbolic examples/minimal-console/build/fr265/minimal-console.prg > scratch/code-symbolic.mcasm
cargo run -p ciq-asm -- examples/minimal-console/build/fr265/minimal-console.prg scratch/code-symbolic.mcasm scratch/code-symbolic.prg
```

This code includes referenced symbol declarations and resolves their IDs. It
preserves the template's symbol table. It also emits inline pinned DATA string
declarations when supported; [template assembly](../formats/data-strings.md)
consumes same-length string edits. `ciq-dis code` retains the existing numeric
output with name annotations. Missing/malformed symbol metadata leaves code
operands numeric.

## Editing boundaries

Project import still requires every payload to match its declared length. Pinned
layout conflicts fail before output is written. Unconstrained serialization is
useful for independently authored tables, but it does not relocate opaque
references elsewhere in a PRG. Removing constraints or changing IDs in an
exported project requires understanding those references; the assembler does not
update unknown data/class/debug records. An inline declaration's name documents
an ID binding rather than changing the template's stored name.

Without `-y`, unchanged source preserves the original signature and all bytes.
The assembler can re-sign complete PRGs with a supplied developer key;
intentional content changes still require runtime validation. See
[developer signatures](../formats/developer-signatures.md). No new simulator acceptance is
established by successful symbol assembly alone.
