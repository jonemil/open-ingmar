# Complete PRG section project, versions 1 and 2

`ciq-dis all <input.prg> <directory>` exports the complete section stream. The directory must not already exist. `ciq-asm project <directory> <output.prg>` reconstructs it without the original PRG. The output PRG must not already exist.

`sections.ciq` is a UTF-8 manifest. The first meaningful line is `ciq-project 1` or `ciq-project 2`. Version 2 enables structured symbol input; existing version 1 projects remain supported. Each subsequent meaningful line has five whitespace-separated fields:

```text
section 0xd000d00d 33 raw 000-header_versioned-d000d00d.bin
section 0xc0debabe 556 assembly 003-code-c0debabe.mcasm
```

Fields are the `section` keyword, hexadecimal nonzero u32 identifier, original decimal payload length, encoding (`raw`, `assembly`, or version 2 `symbols`), and a local filename. `#` starts a comment; blank lines are ignored. Manifest order determines section order, including duplicate identifiers. Filenames are simple ASCII basenames using letters, digits, `.`, `_`, or `-`; `.` and `..` are forbidden. Import rejects paths resolving outside the project directory.

Raw files contain exactly the section payload, without the identifier/length wrapper. Ordinary code is exported as assembly when the full payload is supported by the decoder; otherwise it falls back to raw binary with a diagnostic. A losslessly representable unique symbol table is exported as `symbols` source. Other sections retain raw payloads, including unknown identifiers, empty payloads, extended code, and original signing material. Assembly encoding is permitted only for ordinary code (`0xc0debabe`); `symbols` encoding is permitted only for SYMBOLS (`0x5717b015`) in version 2.

When a symbol table decodes successfully, export adds an inspection-only `.symbols.txt` listing. Losslessly representable unique tables become authoritative `.mcasm` declarations and code uses named symbol references. Other tables remain raw and code stays numeric with annotations when available. Reassembly consumes only the manifest-selected source; editing the sidecar does not change the PRG. Symbol declarations are loaded before code regardless of section order. See [symbol assembly](symbol-assembly.md).

```text
ciq-project 2
section 0xc0debabe 556 assembly 003-code-c0debabe.mcasm
section 0x5717b015 95991 symbols 010-symbols-5717b015.mcasm
```

Both versions require each reconstructed payload to match the manifest length, because relocation is not implemented. It regenerates section wrappers and the eight-byte terminator. The terminator is implicit in the project format rather than exported as a separate file. Names and original section offsets are comments for inspection, not input semantics. Export writes the manifest last so interrupted exports are not mistaken for complete projects.

This interface reconstructs all sections but does not decode all payload structures or regenerate signatures. Re-signing with a user-supplied key remains a separate planned feature. Exact-byte comparisons apply to unedited preservation tests, not future tests that intentionally replace signatures or transform the program.
