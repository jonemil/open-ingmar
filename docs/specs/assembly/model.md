# Assembly model and layout

Use the same assembly model and syntax for handwritten code, compiler output,
and disassembly. Text is an optional serialization boundary; native producers
construct `ciq-assembly::Program` directly. Crate responsibilities belong in
[architecture](../../architecture.md).

## Model and validation

A `Program` is an ordered sequence of labels, instructions, symbol declarations,
and String/Array/ByteArray/Dictionary literal requirements. Instructions contain
an `Opcode` and `Bits`, `Symbol`, `String`, `Array`, `ByteArray`, `Dictionary`,
or relative `Code` operands. Semantic IR value IDs are absent. Native
construction and text parsing share validation: identifier syntax, operand
kinds/counts/widths, duplicate aliases/labels, and supported layout bounds.
Parsed errors retain assembly line numbers; constructed items use line zero.
Source line numbers do not affect model equality.

Printing preserves declarations, constraints, and instruction content, but not
comments or whitespace. Items are immutable; edits construct and validate a new
program. Encoding writes CODE only; declarations do not allocate SYMBOLS/DATA.
The assembler's public text entry points use this same model and encoder.

## Identity and placement

Aliases identify content independently of physical placement. Symbols also have
stored names and numeric IDs; omitting placement never authorizes changing an
API ID. Unpinned layout must be deterministic. Honor explicit `at=` constraints
or report a conflict; do not silently move pinned records or insert padding
unless the relevant section grammar permits it.

Known references bind after target layout. Symbol IDs, DATA string/array
offsets, and CODE labels have separate reference kinds. CODE labels bind
methods, line metadata, branches, and exception targets. Unknown or ambiguous
references retain numeric representations. Numeric operands preserve bits
without implying that the referenced object is valid or relocatable. Literal
operands use u64 and must fit the selected field width; an eight-byte field
accepts every u64 bit pattern.

Unplaced inline string/array declarations are content requirements. The
compiler's DATA binders require unique matching records; the assembly encoder
itself checks alias resolution and field widths, not external-map content.
Pinned inline literals supply their own offsets. Section-project CODE contains
references only, with authoritative declarations in DATA. Inspection listings
are not a second editable source of truth.

## Section contracts

- [Assembly grammar](grammar.md) and [CODE encodings](../vm/encoding.md)
- [Symbol layout and IDs](symbols.md)
- [DATA strings](../formats/data-strings.md), [arrays](../formats/data-arrays.md), and
  [classes](../formats/data-classes.md)
- [Shared CODE labels and relative branches](labels.md)
- [Ordered exception records](../formats/exceptions.md)
- [Section-project reconstruction](../formats/section-project.md)

These specs own bounds, binding, and binary representation; the grammar owns
source syntax. Unsupported records remain raw. Known label fixups do not
establish general relocation: opaque references and integrity fields must also
remain valid after length changes. Ignoring pins, automatic symbol-ID
allocation, includes, extended CODE, and a separate linker remain outside the
supported profile. See [the plan](../../plan.md).

Dictionary constants use the [DATA source
grammar](grammar.md#dictionary-declarations). `Item::Dictionary` also accepts an
inline `.dictionary` requirement in standalone compiler/assembly programs. This
declares required DATA contents, not CODE bytes; an optional `at=` binds its
address. Unplaced requirements need external DATA bindings. Their references use
`hpush &alias` and a separate map supplied to
`Program::encode_with_data_references`. Duplicate/invalid aliases, unresolved
pool strings, and invalid layouts fail validation. Existing encoding entry
points remain available for programs without external Dictionary aliases.

Section projects retain authoritative Dictionary declarations in DATA; inline
CODE requirements remain rejected there, as for Arrays/Strings.

ByteArray constants use `bpush &alias` and the separate ByteArray map in
`Program::encode_with_byte_arrays`. Their `.bytearray` declarations belong in
[DATA](../formats/data-byte-arrays.md). Standalone programs also accept `Item::ByteArray`
and inline `.bytearray` requirements; these do not emit CODE bytes or allocate
DATA. Pins bind inline offsets; unplaced requirements need external bindings.
The compiler's `byte_array_bindings` matches exact bytes against a unique DATA
record, respecting pins and rejecting missing, ambiguous, or conflicting
matches. Section projects still require authoritative declarations in DATA.
Numeric operands retain raw bits; typed references use the zero-prefix DATA
profile.
