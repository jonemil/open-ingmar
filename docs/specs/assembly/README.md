# Assembly

[Grammar](grammar.md) defines accepted Assembly syntax; this reference owns
meaning, validation, and implementation requirements.

The same symbolic assembly model serves handwritten input, compiler output, and
disassembly. It contains selected VM instructions and references, not semantic
IR computations.

[VM bytecode](../vm/README.md) defines concrete encodings.
[Format specifications](../formats/README.md) define binary records and their
preservation; the assembly grammar owns editable source syntax.

Contents:

- [Model and layout](#model-and-layout)
- [Symbol layout and binding](#symbol-layout-and-binding)
- [CODE labels and fixups](#code-labels-and-fixups)

## Model and layout

Use the same assembly model and syntax for handwritten code, compiler output,
and disassembly. Text is an optional serialization boundary; native producers
construct `ciq-assembly::Program` directly. Crate responsibilities belong in
[architecture](../../architecture.md).

### Model and validation

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

### Identity and placement

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

### Section contracts

- [Assembly grammar](grammar.md) and [CODE encodings](../vm/README.md#encoding)
- [Symbol layout and IDs](#symbol-layout-and-binding)
- [DATA strings](../formats/data.md#strings-and-opaque-spans), [arrays](../formats/data.md#array-constants), and
  [classes](../formats/data.md#classes-and-modules)
- [Shared CODE labels and relative branches](#code-labels-and-fixups)
- [Ordered exception records](../formats/metadata.md#exceptions)
- [Section-project reconstruction](../formats/README.md#section-project)

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
[DATA](../formats/data.md#bytearray-constants). Standalone programs also accept `Item::ByteArray`
and inline `.bytearray` requirements; these do not emit CODE bytes or allocate
DATA. Pins bind inline offsets; unplaced requirements need external bindings.
The compiler's `byte_array_bindings` matches exact bytes against a unique DATA
record, respecting pins and rejecting missing, ambiguous, or conflicting
matches. Section projects still require authoritative declarations in DATA.
Numeric operands retain raw bits; typed references use the zero-prefix DATA
profile.

## Symbol layout and binding

Evidence and validation:
[symbol assembly investigation](../../investigations/symbol-assembly.md). Binary
layout: [symbol table format](../formats/metadata.md#symbols).

### Layout policy

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

### Named instruction operands

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

### Commands and project integration

Export and reconstruct complete PRGs using new paths:

```sh
cargo run -p ciq-dis -- all examples/minimal-console/build/fr265/minimal-console.prg scratch/symbol-project
cargo run -p ciq-asm -- project scratch/symbol-project scratch/symbol-project.prg
```

The [manifest](../formats/README.md#section-project) chooses `symbols` encoding
for the authoritative symbol `.mcasm` file. Code files reference those
declarations even when the symbol section follows CODE in the manifest. There
must be exactly one symbol section when structured symbols are used. Code may
add inline declarations with distinct aliases; only the manifest-selected symbol
source is serialized into SYMBOLS. Inline declarations resolve code IDs and do
not implicitly add table entries. An include mechanism is not implemented.

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
declarations when supported; [template
assembly](../formats/data.md#strings-and-opaque-spans) consumes same-length
string edits. `ciq-dis code` retains the existing numeric output with name
annotations. Missing/malformed symbol metadata leaves code operands numeric.

### Editing boundaries

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

## CODE labels and fixups

Evidence: [label and relocation
investigation](../../investigations/code-labels-and-relocation.md).

[Label syntax](grammar.md#labels) emits no bytes and identifies the next
instruction's CODE-relative offset. Layout validates instruction widths within
the supported ordinary CODE address profile.

Class method fields may use `value=code:probe` with type `0x06`. The assembler
resolves the label and emits `0x10000000 | offset`. A label is a distinct
namespace from `@symbol`, `%class`, and `&string`. Non-method fields reject
`code:`. Undefined labels, oversized targets, and method targets at the end of
CODE rather than an instruction are rejected. Generic end-of-stream labels are
accepted for layout but cannot be method or line-PC targets.

Project import reads CODE labels before resolving DATA, independently of
manifest order. Labels require a unique CODE section. Numeric method values
remain supported. The DATA library accepts label bindings via
`parse_with_references` / `encode_with_references`; the assembler library
exposes `code_labels`. Standalone DATA source remains self-contained and keeps
numeric method addresses.

Full project export names verified method instruction-boundary targets in CODE
and uses the corresponding `code:` reference in DATA. Naming is deterministic:

- A target with exactly one distinct owner/member binding uses the member alias
  when that member names only one target.
- A repeated member across different targets uses
  `method_<DATA-owner-alias>__<member-alias>` when the owner is known. For
  example, `method_Left__answer` and `method_Right__answer` distinguish methods
  with one shared `@answer` symbol.
- Unknown member names, repeated members without known owners, and targets
  shared by distinct bindings use `code_<offset>`. Identical repeated bindings
  do not introduce ambiguity.
- Candidate label collisions fall back for every affected target. Member aliases
  starting `code_`, `sym_`, or `method_` also use numeric labels so they cannot
  collide with generated labels.

This is an export naming policy, not a new assembly grammar restriction.
Handwritten labels retain the same identifier rules. The owner is a DATA
declaration alias, not an inferred Monkey C fully qualified name. Path-like
module symbols may therefore yield labels such as `method_sym_00000002__answer`.
CODE, DATA methods, and line PCs all share the chosen labels in one flat
namespace. See
[the repeated-module experiment](../../investigations/module-method-names.md).

Multiple method fields may refer to one label. Unsupported CODE or invalid
numeric targets remain raw/numeric rather than being guessed. Labels are source
identities, not new VM symbols or evidence of a complete function signature.

[Structured line mappings](../formats/metadata.md#debug-and-line_numbers) share these labels in
section projects. Export adds labels for verified line-PC instruction
boundaries; named PCs move when those labels move. Named branch operands also
follow layout as described below. Named
[exception references](../formats/metadata.md#exceptions) also follow these labels. Header offsets,
numeric operands, and opaque metadata are not automatically relocated. Project
import still checks manifest lengths. Explicitly changing those lengths does not
validate other affected references. The append-only and [in-place size
experiments](../../investigations/line-metadata.md) validate bounded edits, not
arbitrary relocation. Opaque references and nonzero background/glance offsets
require independent treatment.

### Relative instruction operands

`goto`, `bt`, `bf`, and `jsr` accept `code:label` as their sole operand. The
assembler resolves local labels after layout, emitting the signed 16-bit
big-endian displacement `target - (instruction_offset + 3)`. Forward and
backward targets are supported. Undefined labels, targets at the end of CODE,
and displacements outside `-32768..32767` fail encoding. Other instructions
reject CODE-label operands. There is no automatic long-branch expansion.

Numeric operands remain unsigned hexadecimal bit patterns; for example, `goto
0xffe8` carries displacement `-24`. Their target validity is unchecked, and they
do not change when labels move. This preserves unfamiliar or invalid input
without silently rewriting it.

Project export and `code-symbolic` name branch targets only when they denote
verified instruction boundaries. Project export reuses method/line labels,
adding `code_<offset>` names for remaining targets. Numeric `code` listings
retain original operand bits. Structured EXCEPTIONS shares labels too. See
[the implementation experiment](../../investigations/branch-labels.md).
