# Symbol decoding and instruction annotations

Date: 2026-10-08. Goal: resolve numeric symbol operands to readable names
without changing assembled instructions.

## Sources and method

Source-derived starting point: [monkeyc-optimizer][ref-monkeyc-optimizer] at
`2920dc88c23caadded9f66ee0613a7de6ab2d1ea`, MIT; `SymbolTable.parseSymbolTable`,
`parse`, and `parseSymbol`. This describes a u16 count, u32 identifier/offset
pairs, and length-prefixed names. Independent Rust code was written from the
local [symbol specification](../specs/formats/metadata.md#symbols); no reference implementation
code was copied.

The ordinary instruction operand roles were cross-checked against the same
revision's opcode model and the plugin's `data/languages/MonkeyC.sinc` at
`12ae55010721165b4ba98254d5e77db22eaa1b07` (Apache-2.0). Provenance is also
recorded in [the bootstrap investigation](rust-bootstrap.md). No SDK
implementation code was inspected.

Local specimens retain their original SDK 9.2.0/`fr265` hashes from the
bootstrap record. Experimental parsing confirmed the name-record tag `0x01`,
UTF-8 byte lengths, zero terminators, and section-relative record offsets. Every
entry pointed to an actual name boundary. There were no duplicate symbol
identifiers in either specimen.

- **Specimen:** Minimal Console. **Entries / name records:** 3224 / 3224.
  **Offset of first physical name record:** `0x64c2`. **Names independently
  matched to debug XML:** 293.

- **Specimen:** Hello World. **Entries / name records:** 3225 / 3225. **Offset
  of first physical name record:** `0x64ca`. **Names independently matched to
  debug XML:** 295.

The cross-check used the actual Rust `ciq-dis symbols` output and the compiler's
`.prg.debug.xml` symbol entries. Debug XML contains only a subset of names; no
claim is made that those comparisons independently validate every API symbol.

## Observed resolution

In Minimal Console:

| Identifier | Name |
|---|---|
| `0x00800005` | `Toybox_System` |
| `0x00800067` | `println` |
| `0x008002e6` | `globals` |
| `0x00000080` | `hello` |

These names come from the PRG itself, without requiring the SDK's API database
or the debug XML at disassembly time. Names are not rewritten to another
spelling. The relevant output is now:

```text
spush 0x008002e6 # CODE+0x000026; symbol="globals"
getmv 0x00800005 0x00800067 # CODE+0x00002e; module="Toybox_System"; member="println"
news 0x00003279 # CODE+0x000038
```

`news` is a data pointer, so its value is not looked up as a symbol. Resolving
it to the string `Hello!` is the next data-decoding step. Names currently
annotate numeric operands rather than replace them in the assembly grammar.

## Tool behavior and validation

`ciq-dis symbols <prg>` prints identifier, name-record offset, and escaped name,
sorted by identifier. Explicit symbol dumping fails on missing, duplicate, or
malformed symbol sections. Code disassembly instead retains numeric output if
symbol annotations are unavailable, with a diagnostic for malformed/ambiguous
tables. Missing tables are valid for numeric disassembly.

`ciq-dis all` still exports the symbol section as raw binary, and adds an
inspection-only `.symbols.txt` sidecar when decoding succeeds. That sidecar is
not part of the manifest and is not consumed by assembly. Code files include the
same annotations as ordinary code output. Escaping newlines and quotes in names
prevents names from introducing extra assembly lines.

Validation: 22 workspace tests passed; Clippy with warnings denied and
formatting passed. New tests cover out-of-order name offsets, every truncation
of a synthetic table, dangling offsets, duplicate IDs, invalid
tags/terminators/UTF-8, annotation escaping, symbol operand selection,
absent/malformed symbol fallback, and annotated full-project round trips.

Both complete specimens were exported under `scratch/symbols/` and imported
without their original PRG templates. Python byte equality checks confirmed the
reconstructed files match the originals. These remain preservation tests, with
signing material unchanged.

Commands:

```sh
cargo run -p ciq-dis -- symbols examples/minimal-console/build/fr265/minimal-console.prg
cargo run -p ciq-dis -- code examples/minimal-console/build/fr265/minimal-console.prg
```

Structured symbol editing, named assembly operands, data/class decoding, and
own-key signing remain to be implemented.

[ref-monkeyc-optimizer]: https://github.com/markw65/monkeyc-optimizer/blob/2920dc88c23caadded9f66ee0613a7de6ab2d1ea/src/readprg/symbols.ts
