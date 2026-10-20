# Container and section formats

These pages own binary representation, preservation, and supported operations.
[Assembly grammar](../assembly/grammar.md) owns editable section source syntax.

- [PRG container](prg-container.md) and [section projects](section-project.md).
- DATA [strings](data-strings.md), [classes/modules](data-classes.md),
  [constant framing](data-constants.md), [Arrays](data-arrays.md),
  [Dictionaries](data-dictionaries.md), and [ByteArrays](data-byte-arrays.md).
- [Symbols](symbols.md), [resources](resources.md),
  [DEBUG/line numbers](line-numbers-and-debug.md), and
  [EXCEPTIONS](exceptions.md).
- [Developer signatures](developer-signatures.md).
- Barrels: [package format](barrel-package.md) and [packing](barrel-packing.md).
- IQ: [package/signing format](iq-package.md), [creation](iq-creation.md),
  [repacking](iq-repacking.md), and [verification](package-verification.md).
- Shared [package reading and extraction](package-reading.md).

Unknown sections retain raw bytes. Structured decoding and lossless writing do
not establish relocation safety or device acceptance.
