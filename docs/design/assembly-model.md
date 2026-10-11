# Shared CODE assembly model

`ciq-assembly` is a standard-library-only layer depending on `ciq-mc` and
`ciq-prg`. See
[pipeline evidence](../investigations/compiler-pipeline.md). It owns the
existing ordinary CODE assembly grammar/model, not full section-project assembly
or signing.

`Program` is an ordered sequence of immutable items: labels, instructions,
inline symbol declarations, and inline string declarations with optional
placement. An instruction has an `Opcode` and typed operand fields: `Bits(u64)`,
`Symbol(alias)`, `String(alias)`, or `Code(alias)`. Semantic value IDs from
compiler IR are absent here. Symbol/string references await declarations or
external bindings; CODE references resolve locally to signed relative operands
after layout. See
[branch-label rules](../specs/code-labels.md#relative-instruction-operands).

Both `Program::new(items)` and `Program::parse(text)` apply the same structural
validation: identifier syntax, operand count/width, allowed reference kinds,
optional 24-bit string placement, duplicate string aliases/labels, and the
24-bit CODE layout limit. A parsed model retains assembly line numbers for
errors; constructed items use line zero. Source-line provenance is not part of
semantic item equality.

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

Unplaced string declarations describe literal requirements. CODE encoding uses
external string offsets for their operands; it does not allocate DATA or verify
that an arbitrary external map contains the declared content. The compiler's
explicit DATA binder checks content and layout before supplying the map. Pinned
declarations retain their existing direct encoding/template behavior.
Section-project CODE still contains references only, with declarations in the
authoritative DATA source. See [string assembly](../specs/data-strings.md) and
[compiler evidence](../investigations/compiler-strings.md).
