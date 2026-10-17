# DATA scalar Dictionary constants

This profile covers flat Dictionary constants loaded by `hpush`, based on the
[Dictionary experiments](../investigations/compiler-dictionary.md). Nested
containers, Symbol, and unlisted value tags are unsupported. The shared codec
retains Long/Float/Double tags with
[Dictionary runtime evidence](../investigations/compiler-numeric-dictionaries.md).
Entry order and string-pool identity are preserved; the codec does not enforce
source-language key restrictions, uniqueness, hashing, or iteration order.

## Binary record

Use the [shared constant-container framing](data-constants.md). The value block
has root `0x0b`, a u32 entry count, and ordered scalar key/value pairs. The count
must consume the value block exactly.

## Editable DATA source

```text
.dictionary example at=0x00000000
.string key "one" at=0x00000000
.string value "text" at=0x00000006
.entry string=&key number=0x00000029
.entry number=0x00000002 string=&value
.enddictionary
```

The header's `at=` pins the outer DATA tag. Strings follow the
[local pool rules](data-constants.md#editable-local-pools).

Each `.entry` takes exactly two scalars: `null`, `boolean=true|false`,
`number=0x<bits>`, `char=0x<bits>`, `float=0x<u32>`, `long=0x<u64>`,
`double=0x<u64>`, or `string=&<local-alias>`. Reference order
within the declaration is unrestricted; all strings resolve during encoding.
The serializer derives lengths/counts and preserves declaration order. Outer
placement follows the existing [DATA layout rules](data-strings.md).

Section projects keep Dictionary declarations in authoritative DATA source.
Standalone assembly may declare inline [DATA requirements](assembly.md).
CODE uses `hpush &example`; `Operand::Dictionary` and a separate Dictionary
binding map prevent confusion with Array/String aliases. The current layout
profile permits only zero-prefix DATA addresses below `0x01000000`; numeric
operands retain raw bits without implying a validated record reference.
