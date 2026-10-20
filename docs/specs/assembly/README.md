# Assembly

The same symbolic assembly model serves handwritten input, compiler output, and
disassembly. It contains selected VM instructions and references, not semantic
IR computations.

- [Text grammar](grammar.md): CODE and editable section declarations.
- [Model and layout](model.md): validation, pinning, and binding boundaries.
- [Symbols](symbols.md): deterministic table/name layout and export policy.
- [CODE labels](labels.md): instruction boundaries and relative fixups.

[VM bytecode](../vm/README.md) defines concrete encodings.
[Format specifications](../formats/README.md) define binary records and their
preservation; the assembly grammar owns editable source syntax.
