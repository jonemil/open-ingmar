# Complete PRG section project, version 1

`ciq-dis all <input.prg> <directory>` exports the complete section stream. The directory must not already exist. `ciq-asm project <directory> <output.prg>` reconstructs it without the original PRG. The output PRG must not already exist.

`sections.ciq` is a UTF-8 manifest. The first meaningful line is `ciq-project 1`. Each subsequent meaningful line has five whitespace-separated fields:

```text
section 0xd000d00d 33 raw 000-header_versioned-d000d00d.bin
section 0xc0debabe 556 assembly 003-code-c0debabe.mcasm
```

Fields are the `section` keyword, hexadecimal nonzero u32 identifier, original decimal payload length, encoding (`raw` or `assembly`), and a local filename. `#` starts a comment; blank lines are ignored. Manifest order determines section order, including duplicate identifiers. Filenames are simple ASCII basenames using letters, digits, `.`, `_`, or `-`; `.` and `..` are forbidden. Import rejects paths resolving outside the project directory.

Raw files contain exactly the section payload, without the identifier/length wrapper. Ordinary code is exported as assembly when the full payload is supported by the decoder; otherwise it falls back to raw binary with a diagnostic. Other sections retain raw payloads, including unknown identifiers, empty payloads, extended code, and original signing material. Assembly encoding is permitted only for ordinary code (`0xc0debabe`).

When a symbol table decodes successfully, export adds an inspection-only `.symbols.txt` listing and annotates symbol operands in code comments. The symbol section itself remains raw in the manifest. Reassembly does not consume the listing, so editing it does not change the PRG. See [symbol decoding](symbols.md).

Version 1 import requires each reconstructed payload to match the manifest length, because relocation is not implemented. It regenerates section wrappers and the eight-byte terminator. The terminator is implicit in the project format rather than exported as a separate file. Names and original section offsets are comments for inspection, not input semantics. Export writes the manifest last so interrupted exports are not mistaken for complete projects.

This interface reconstructs all sections but does not decode all payload structures or regenerate signatures. Re-signing with a user-supplied key remains a separate planned feature. Exact-byte comparisons apply to unedited preservation tests, not future tests that intentionally replace signatures or transform the program.
