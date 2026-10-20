# DATA scalar Dictionary constants

Editable source: [Dictionary
declarations](../assembly/grammar.md#dictionary-declarations).

This profile covers flat Dictionary constants loaded by `hpush`, based on the
[Dictionary experiments](../../investigations/compiler-dictionary.md). Nested
containers, Symbol, and unlisted value tags are unsupported. The shared codec
retains Long/Float/Double tags with
[Dictionary runtime evidence](../../investigations/compiler-numeric-dictionaries.md).
Entry order and string-pool identity are preserved; the codec does not enforce
source-language key restrictions, uniqueness, hashing, or iteration order.

## Binary record

Use the [shared constant-container framing](data-constants.md). The value block
has root `0x0b`, a u32 entry count, and ordered scalar key/value pairs. The
count must consume the value block exactly.
