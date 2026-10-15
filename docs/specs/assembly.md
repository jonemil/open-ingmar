# Assembly model and layout

Use the same assembly model and syntax for handwritten code, compiler output,
and disassembly. Text is an optional serialization boundary; native producers
construct `ciq-assembly::Program` directly. Crate responsibilities belong in
[architecture](../architecture.md).

## Model and validation

A `Program` is an ordered sequence of labels, instructions, symbol declarations,
and string/array literal requirements. Instructions contain an `Opcode` and
`Bits`, `Symbol`, `String`, `Array`, `Dictionary`, or relative `Code` operands. Semantic IR
value IDs are absent. Native construction and text parsing share validation:
identifier syntax, operand kinds/counts/widths, duplicate aliases/labels, and
supported layout bounds. Parsed errors retain assembly line numbers; constructed
items use line zero. Source line numbers do not affect model equality.

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

Known references bind after target layout. Symbol IDs, DATA string/array offsets,
and CODE labels have separate reference kinds. CODE labels bind methods, line
metadata, branches, and exception targets. Unknown or ambiguous references retain
numeric representations. Numeric operands preserve bits without implying that
the referenced object is valid or relocatable.

Unplaced inline string/array declarations are content requirements. The compiler's
DATA binders require unique matching records; the assembly encoder itself checks
alias resolution and field widths, not external-map content. Pinned inline
literals supply their own offsets. Section-project CODE contains references
only, with authoritative declarations in DATA. Inspection listings are not a
second editable source of truth.

## Section contracts

- [CODE instructions and syntax](bytecode-subset.md)
- [Symbol declarations and IDs](symbol-assembly.md)
- [DATA strings](data-strings.md), [arrays](data-arrays.md), and
  [classes](data-classes.md)
- [Shared CODE labels and relative branches](code-labels.md)
- [Ordered exception records](exceptions.md)
- [Section-project reconstruction](section-project.md)

These specs own exact grammars, bounds, and binding rules. Unsupported records
remain raw. Known label fixups do not establish general relocation: opaque
references and integrity fields must also remain valid after length changes.
Ignoring pins, automatic symbol-ID allocation, includes, extended CODE, and a
separate linker remain outside the supported profile. See [the plan](../plan.md).

Dictionary constants belong in [DATA source](data-dictionaries.md). Their CODE
references use `hpush &alias` and a separate Dictionary binding map supplied to
`Program::encode_with_data_references`; inline Dictionary requirements are not
yet part of `Program`. Existing encoding entry points remain available for
programs without Dictionary aliases.
