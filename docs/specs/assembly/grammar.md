# Assembly text grammar

## Assembly syntax

```text
code-source = (label | instruction | inline-declaration | blank-or-comment)*
label = identifier ":"
instruction = mnemonic operand*
operand = "0x" hex-digits | "@" symbol-alias | "&" literal-alias
        | "code:" label-name
inline-declaration = symbol | string | array | bytearray | dictionary
```

One record occupies one line; declaration blocks have their explicit end
records. Operand count/kind/width follows the [opcode
table](../vm/README.md#encoding) and typed reference rules. The [symbol
declaration rules](#symbol-declarations) define identifiers, quoting, comments,
and field separators for shared section sources. Numeric operands preserve raw
bits, including signed displacements and floating immediates. There are no
arithmetic expressions, includes, or macros.

Assembly uses one mnemonic per line, followed by one operand per field: a
hexadecimal bit pattern, a declared `@alias` in a known symbol-ID field, or
`&alias` in `news`, `apush`, `bpush`, or `hpush` for the respective typed DATA
namespace, or `code:label` in a relative branch/subroutine operand. Numeric
operands must start with `0x` and fit their field widths. `#` starts a comment.
Blank lines are allowed. For example:

```text
argc 0x01             # CODE+0x000000
ipush1 0x29           # integer 41 in the reference probe
return
```

Disassembly emits fixed-width operands and code-offset comments. When the PRG
contains a supported symbol table, comments also annotate symbol operands using
escaped names; assembly ignores those comments and reproduces the original
operand bits. [Symbol declarations](#symbol-declarations) resolve named
operands. Optional commas separate fields. [String
declarations](../formats/data.md#strings-and-opaque-spans) resolve DATA
addresses; `apush &alias` resolves a separately typed
[DATA constant-array declaration](../formats/data.md#array-constants).
[Shared CODE labels](README.md#code-labels-and-fixups) identify instruction boundaries and resolve
signed 16-bit branch fixups. Includes and expressions are not implemented.
`ciq-asm` can replace an entire ordinary code payload in a PRG template or
reconstruct a [complete section project](../formats/README.md#section-project);
it is not a Monkey C compiler.

## Labels

A label is an identifier followed by `:`, on its own line (comments allowed). It
emits no bytes and denotes the next instruction's CODE-relative offset.
Identifier rules match other aliases. Duplicate/invalid labels, mixed
label/instruction lines, unsupported instructions, and incorrect operand counts
are errors. Layout uses the verified instruction widths, within the 24-bit
ordinary CODE address space.

```text
probe:
argc 0x01
ipush1 0x2a
return
```

## Array declarations

```text
.array values
.string local "one"
.element string=&local
.element number=0x0000000a
.element null
.element boolean=true
.element char=0x00000061
.element long=0x0000000100000001
.element float=0x3fa00000
.element double=0x4170000010000000
.endarray
```

`.array <alias> [at=0x...]` starts a declaration and `.endarray` closes it. Each
`.element` has exactly one of `null`, `number=0x<u32>`, `char=0x<u32>`,
`float=0x<u32>`, `long=0x<u64>`, `double=0x<u64>`, `boolean=true`,
`boolean=false`, or `string=&<local-alias>`. Values retain their types and bit
patterns; lengths and counts are derived during encoding. Whitespace/comments
follow the shared [lexical rules](#assembly-text-grammar). Local `.string` and
`.pool` fields follow the [pool rules](#editable-local-pools); nested containers
are rejected.

Declarations share source-order DATA layout with strings, classes, and opaque
spans. Without `at`, an array follows the preceding fragment. A pinned `at` must
match that position; it does not insert padding or perform relocation. Aliases
follow the symbol identifier grammar and must be unique within the array
namespace, separately from strings and classes. DATA layout uses the current
bounded zero-prefix address profile; this is not a proven universal VM limit.

## ByteArray declarations

```text
.bytearray bytes at=0x00000000
.bytes 00017f80ffff80
.endbytearray
```

Placement is optional. `.bytes <hex>` appends hexadecimal pairs, with no prefix
or separators. Repeated lines concatenate; printing wraps at 32 bytes per line.
An empty declaration contains no `.bytes` lines. `.string` and `.pool` retain
optional record-local pools under the shared rules; bytes never reference those
strings.

Aliases are unique within the ByteArray namespace, independently of global
String, Array, Dictionary, and class aliases. Unpinned records follow DATA
layout; pins must match their position. `Source::byte_array_bindings` supplies
record starts for typed `bpush &alias` references. The existing zero-prefix DATA
placement profile applies; raw u32 operands preserve other bit patterns without
asserting runtime validity.

## Class assembly

Evidence: [serialization and relocation
investigation](../../investigations/class-assembly.md). Complete supported DATA
exports put class declarations alongside string declarations in the
authoritative DATA source. Opaque `.bytes` spans are also valid input. The class
inspection sidecar is still not assembly input.

```text
.class at=0x0000047b version=0x02 extends=none statics=none parent=none module=0x008002e6 app_flags=0x0000
.field key=0x00008146 type=0x06 value=0x10000030
.endclass
```

This is an illustrative method declaration; the real record may have additional
fields. Header attributes must appear in the shown order. Integers use
hexadecimal `0x` prefixes. `none` means an absent header reference and is
distinct from present zero. Version is 1 or 2; v1 requires all four references,
whereas v2 derives its presence byte from the present references. App flags fit
u16. `at` is an optional DATA-relative 24-bit placement constraint and, when
present, must equal the next serialized offset. Without it, classes append in
source order.

Each `.field` has a full u32 `key`, u8 `type`, and u32 `value`, in that order.
Field order, repeated keys, shared targets, unknown types, flag bits, and raw
references are preserved. V1 requires `type` to equal the key's low nibble; v2
stores it separately. Field counts are derived from declarations and must fit u8
(v1) or u16 (v2). `.endclass` closes the block. Nested classes, other directives
inside a class, missing terminators, and extra attributes are errors.

The source model serializes independently of the borrowed inspection model.
Parsed record offsets are inspection metadata; field offsets are recomputed from
the serialized layout. The named forms below resolve typed class/string/symbol
references. Numeric fallbacks remain supported;
[shared code labels](README.md#code-labels-and-fixups) resolve method targets. Numeric values
are not automatically relocated.

Complete DATA walks export classes and strings only after checking that
re-encoding reproduces the original payload. Unsupported walks retain the
referenced-string/opaque-span fallback or raw DATA. Standalone `ciq-asm data`
accepts this grammar; complete project import still enforces section lengths.
Editing a method target or field is a program modification and must be re-signed
and runtime-validated. Serialization does not establish that a chosen target has
the right calling convention, and does not enable relocation.

## Named declarations and references

Evidence: [named class investigation](../../investigations/named-classes.md). An
optional alias follows `.class`, before optional `at`. Aliases obey
symbol/string identifier rules but use a separate class namespace.

```text
.class globals at=0x0000047b version=0x02 extends=none statics=none parent=none module=@globals app_flags=0x007f
.field symbol=@HelloWorldApp bits=0x50 type=0x07 value=%HelloWorldApp
.field symbol=@probe bits=0x10 type=0x06 value=0x10000030 # CODE+0x30
# Remaining fields omitted in this excerpt.
.endclass
```

`symbol=@alias bits=0xNN` replaces `key=0x...`: the resolved ID must fit 24
bits; the low-byte `bits` is preserved independently. Both key syntaxes work for
v1 and v2. Header `parent` and `module` accept `@symbol` or raw integers; other
header roles remain raw. Field `value=%class` is accepted only for type 7;
`value=&string` only for type 3. Raw values remain valid for all types.
Undefined/mismatched references are errors.

Class and string addresses are computed in a layout pass before serialization,
so forward references work. Optional placement constraints are checked;
gaps/overlaps are not silently introduced. This updates explicitly named
references within the authored DATA source, not unknown numeric pointers or
cross-section metadata. Complete project lengths remain enforced.

Project exports reuse aliases from authoritative SYMBOLS source, which import
loads before DATA resolution. Names for class records derive from known module
headers or incoming class fields; collisions use `class_<offset>` fallbacks.
Exports retain pinned offsets, ordered fields, flag bits, and numeric values for
unknown types/address spaces. Field comments show DATA offsets and known CODE
targets for comparison with inspection listings.

Standalone `data-source` retains numeric symbol keys/header values so `ciq-asm
data` can consume it without a separate SYMBOLS file; it still names classes and
strings. Project method fields also accept `value=code:label` for type 6; the
label resolves to a 24-bit CODE offset with prefix `0x10`. Library callers use
`Source::parse_with_references` / `encode_with_references` with both symbol and
code bindings, or `parse_with_symbols` / `encode_with_symbols` for symbols only;
plain `parse`/`encode` operate without such bindings.

## Editable local pools

Inside `.array` or `.dictionary`, `.string <alias> "text" [at=0x...]` declares a
local pool entry. Its optional placement is relative to the pool, not DATA; it
must match consecutive layout. Aliases must be valid and unique within that
pool. Global DATA String aliases have a separate namespace. `.pool` retains an
explicit empty pool; any `.string` also creates a pool.

`string=&<local-alias>` refers to a pool entry. Declaration order does not limit
references; all aliases resolve during encoding. Preserve pool order, unused
entries, duplicate text, and shared references when writing an existing record.

## Dictionary declarations

```text
.dictionary example at=0x00000000
.string key "one" at=0x00000000
.string value "text" at=0x00000006
.entry string=&key number=0x00000029
.entry number=0x00000002 string=&value
.enddictionary
```

The header's `at=` pins the outer DATA tag. Strings follow the
[local pool rules](#editable-local-pools).

Each `.entry` takes exactly two scalars: `null`, `boolean=true|false`,
`number=0x<bits>`, `char=0x<bits>`, `float=0x<u32>`, `long=0x<u64>`,
`double=0x<u64>`, or `string=&<local-alias>`. Reference order within the
declaration is unrestricted; all strings resolve during encoding. The serializer
derives lengths/counts and preserves declaration order. Outer placement follows
the existing [DATA layout rules](../formats/data.md#strings-and-opaque-spans).

Section projects keep Dictionary declarations in authoritative DATA source.
Standalone assembly may declare inline [DATA
requirements](README.md#model-and-layout). CODE uses `hpush &example`;
`Operand::Dictionary` and a separate Dictionary binding map prevent confusion
with Array/String aliases. The current layout profile permits only zero-prefix
DATA addresses below `0x01000000`; numeric operands retain raw bits without
implying a validated record reference.

## Strings and opaque spans

DATA source combines strings with [class
declarations](../formats/data.md#classes-and-modules) and opaque bytes, using
the shared quoted-string escapes documented in
[symbol assembly](../formats/metadata.md#symbols):

```text
.bytes at=0x00000000 ffc1
.string greeting "Hello!" at=0x00000002
.bytes at=0x0000000c ee
```

`.bytes at=0x... <hex-bytes>` contains literal pairs of hexadecimal digits,
without a `0x` prefix on the byte sequence. It retains opaque bytes at their
original offsets. Export splits opaque spans into 32-byte lines for inspection;
reassembly preserves them exactly.

`.string <alias> "value" [at=0x...]` declares a string record. Aliases follow
the same identifier rules as symbols, but occupy a separate namespace. Strings
are serialized in source order. With no `at`, a string is appended at the next
offset. A supplied `at` must equal that next offset; it does not create padding,
reorder records, or overwrite another fragment. Duplicate aliases, oversized
names, gaps, overlaps, or unsupported address prefixes are errors. No automatic
deduplication is performed.

Export uses `str_` plus the eight-digit record offset as an alias and pins every
string and opaque span. All authored DATA must fit the supported 24-bit offset
space. Strings with equal content at distinct offsets remain separate records.
This policy preserves existing layout while allowing deterministic assembly of
independently authored string-only data.

## Exception records

Section projects support `exceptions` encoding. Each line has exactly the
following directive and attributes, in this order:

```text
.exception begin=code:try_begin end=code:try_end handler=code:catch_begin
.exception begin=0x00003a end=0x00004c handler=0x00004c
```

Each value is a `0x`-prefixed u24 hexadecimal offset or a `code:label`
reference. Aliases follow ordinary CODE label rules. Blank lines and `#`
comments are allowed; the record count is derived from declarations and must fit
u16. An empty source encodes the two-byte zero count. Canonical numeric output
uses six hexadecimal digits. No range ordering is imposed.

Import requires a unique EXCEPTIONS section when structured encoding is used.
Named offsets resolve after CODE layout regardless of manifest order, requiring
a unique CODE section and defined labels. Named starts and handlers must denote
instructions. A named end may denote an instruction boundary or the end of CODE.
All resolved offsets must fit u24. Numeric values preserve their bits and remain
unchanged when labels move.

Export shares existing method, line, and branch labels, adding `code_<offset>`
labels for verified exception boundaries. It names end-of-CODE only in an end
field. Unknown/invalid target offsets stay numeric; supported exception records
can therefore remain structured even when CODE is unsupported or absent.
Duplicate exception sections export raw. Unchanged export/import must preserve
every payload byte.

Moving labels updates named exception fields, not other opaque references.
Payload lengths must still match the manifest; changing them explicitly does not
establish general relocation safety. See
[implementation and runtime evidence](../../investigations/exception-assembly.md).

## DEBUG strings and line mappings

DEBUG source accepts only `.string` declarations, using the DATA string grammar
and independent aliases:

```text
.string source_file "example.mc" at=0x00000000
.string function_name "probe"
```

`at=` is optional. Pinned records keep their placement or fail on conflicts;
unpinned records use the existing deterministic string layout. Export pins every
record and uses aliases `dbg_<original-offset>`. Equal text is not merged. DEBUG
aliases belong to a separate namespace from DATA string aliases; a line
reference resolves only against DEBUG.

LINE_NUMBERS source uses one declaration per entry, with attributes in this
order:

```text
.line pc=code:probe file=&source_file function=&function_name line=22
.line pc=0x1000003a file=0x00000000 function=0x0000000e line=23
```

`line=` is a decimal u32. Numeric PCs and DEBUG offsets are `0x`-prefixed u32
values and serialize unchanged. `pc=code:label` emits `0x10000000 |
label_offset`; the offset must fit 24 bits and identify an instruction within
the unique ordinary CODE section. End-of-CODE targets and undefined labels fail.
`file=&alias` and `function=&alias` resolve DEBUG record offsets, requiring
structured DEBUG input. Numeric offsets are preserved even when their target is
unknown; this is preservation, not target validation.

Full export names PCs only when their address-space prefix is `0x10` and the
decoded CODE offset is an instruction boundary. It reuses method labels or adds
`code_<original-offset>` labels. Filename/function references become aliases
only at known DEBUG string boundaries. Unsupported CODE, foreign address spaces,
invalid PCs, and unknown DEBUG offsets remain numeric. Re-export after an edit
may choose different source alias names, while reproducing the same bytes.

Import reads all declarations and CODE labels before resolving metadata,
independently of manifest order. Moving a label updates method fields and named
line PCs; moving an unpinned DEBUG string updates named filename/function
references. The manifest still requires explicit payload lengths. These
references do not relocate branches, exception intervals, nonzero header
offsets, or opaque payloads.

## Symbol declarations

The same declaration syntax is used in a standalone symbol file, an exported
project's symbol section, or inline with instructions:

```text
.symbol system "Toybox_System" id=0x00800005
.symbol println "println" id=0x00800067

getmv @system, @println
```

An alias starts with an ASCII letter or underscore and continues with ASCII
letters, digits, or underscores. Aliases are case-sensitive and independent of
stored names. Names must be quoted UTF-8 strings. Supported escapes are `\"`,
`\\`, `\n`, `\r`, `\t`, `\0`, and `\u{hex}` for a valid Unicode scalar. A `#`
inside a quoted name is literal; outside quotes it begins a comment. Whitespace
and commas separate fields. Blank lines are ignored.

`id=0x...` is required, must fit u32, and preserves symbol identity. Automatic
ID allocation is unsupported. Names may repeat, but aliases and IDs must be
unique within a source. Constraints may occur in any order; duplicate or unknown
constraint keys are errors.

Optional constraints control separate properties:

```text
# These offsets are valid for this complete two-symbol example.
.symbol system "Toybox_System" id=0x00800005 name_at=0x00000012 entry_index=0
.symbol println "println" id=0x00800067 name_at=0x00000023 entry_index=1
```

`name_at` is a hexadecimal u32 offset to the record tag within the SYMBOLS
payload. `entry_index` is a decimal zero-based index in the ID/offset table.
Both may be omitted or supplied independently. The entry count determines the
table size; offsets cannot overlap it.
