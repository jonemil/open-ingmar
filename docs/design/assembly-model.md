# Shared CODE assembly model

Implemented 2026-10-10 in `ciq-assembly`, a standard-library-only layer
depending on `ciq-mc` and `ciq-prg`. See
[pipeline evidence](../investigations/compiler-pipeline.md). It owns the
existing ordinary CODE assembly grammar/model, not full section-project assembly
or signing.

`Program` is an ordered sequence of immutable items: labels, instructions,
inline symbol declarations, and pinned inline string declarations. An
instruction has an `Opcode` and typed operand fields: `Bits(u64)`,
`Symbol(alias)`, or `String(alias)`. Semantic value IDs from compiler IR are
absent here. Symbol/string references remain symbolic until encoding receives
declarations or external bindings. Branch operands/fixups are not added by this
change.

Both `Program::new(items)` and `Program::parse(text)` apply the same structural
validation: identifier syntax, operand count/width, allowed reference kinds,
pinned inline string placement, duplicate labels, and the 24-bit CODE layout
limit. A parsed model retains assembly line numbers for errors; constructed
items use line zero. Source-line provenance is not part of semantic item
equality.

`Program::to_text()` prints canonical existing `.mcasm` syntax; comments,
optional commas, and whitespace are not preserved. Instruction/label/declaration
content and constraints are retained. `items()` exposes a read-only slice; build
a new program to edit it and reapply validation.

`Program::labels()` computes instruction-aligned CODE offsets. `encode()` /
`encode_with_references(symbols, strings)` share binding resolution, duplicate
symbol/string checks, operand-width checks, and `ciq-mc` byte encoding.
Unresolved or conflicting bindings are errors. Symbol declaration placement
metadata is preserved, but CODE encoding does not write a SYMBOLS section;
project reconstruction does that separately. Inline strings likewise provide
references/patch declarations, not a newly generated DATA section.

The existing `ciq-asm` public `assemble*`, `code_labels`, and `Error` names are
re-exported from this layer. Their text paths parse a `Program` then use the
same encoder as direct model construction. `ciq-asm` still owns PRG
reconstruction, template edits, signing commands, and filesystem output policy.
The compiler depends on the shared model without acquiring signing/archive
dependencies.
