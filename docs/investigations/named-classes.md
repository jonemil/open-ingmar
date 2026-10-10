# Named class declarations and typed DATA references

Date: 2026-10-08. Follow-up to [numeric class assembly](class-assembly.md).
Trigger: class inspection listed resolved names and targets, while assembly
required manually interpreting numeric keys and values. Apply the existing
symbol/string assembly approach to classes.

## Implemented model

Class declarations now have an optional source alias and optional `at`
constraint. A named alias identifies an object in DATA; it is distinct from the
SYMBOLS ID naming the field that refers to it. Without `at`, source-order layout
appends the class deterministically. Exports keep their existing offsets pinned.
No existing opaque reference is automatically relocated.

Field keys accept either the old complete `key=0x...` or
`symbol=@alias bits=0x...`. The second form constructs
`(symbol_id << 8) | bits`; IDs must fit 24 bits. Keeping the complete low byte
prevents losing flag/type bits whose semantics are not yet fully known. V1 still
requires its embedded type to match the key's low nibble.

Typed value references use `%alias` for class records (type 7) and `&alias` for
string records (type 3). Parent/module headers can use `@symbol`;
extends/statics remain raw because their namespaces are not uniformly
established. Method values remain numeric, annotated with CODE targets; code
labels and relocation are separate work. Numeric fallback values remain
available for every field.

Two-pass DATA assembly first computes class/string placement and then resolves
references. This supports forward references and keeps class, string, and symbol
namespaces separate. Duplicate aliases within a namespace, undefined references,
mismatched reference types, oversized symbol IDs, and conflicting pinned offsets
fail. Project import loads SYMBOLS declarations before resolving DATA regardless
of manifest order.

## Export naming and matching inspection

Full project exports use the same aliases as their SYMBOLS source. Class names
are chosen from a resolvable module header or an incoming verified type-7 class
field. Ambiguous/repeated candidates use deterministic `class_<DATA-offset>`
aliases. These are local source names, not newly allocated VM symbols. Fields of
unverified value kinds remain numeric, even when their value resembles a DATA
offset. Supported class/string targets are named only at exact known record
boundaries.

Actual Hello World fragments:

```text
.class globals at=0x0000047b version=0x02 extends=none statics=none parent=none module=@globals app_flags=0x007f
.field symbol=@HelloWorldApp bits=0x50 type=0x07 value=%HelloWorldApp # DATA+0x00000488
.field symbol=@probe bits=0x10 type=0x06 value=0x10000030 # DATA+0x0000049a -> CODE+0x00000030
# Other fields omitted here, not in the export.
.endclass

.class HelloWorldApp at=0x00000528 version=0x02 extends=0x40000119 statics=none parent=@globals module=none app_flags=0x007f
# Fields omitted here.
.endclass
```

The record at `0x47b` contains the field that points to the class at `0x528`;
`probe` belongs to globals, not HelloWorldApp. Inspection uses the same DATA
offsets and decoded field names. Field-offset comments and CODE annotations are
for navigation; assembly ignores them.

`ciq-dis data-source` stays self-contained for `ciq-asm data`: it includes class
aliases and typed class/string references, but keeps numeric symbol IDs because
standalone DATA does not load an external SYMBOLS file. Full project exports use
`@alias` references to the manifest-selected SYMBOLS source. The `.classes.txt`
file remains inspection only and now correctly directs edits to authoritative
DATA assembly.

## Validation

New exports and reconstructed files live under ignored `scratch/named-classes/`.
Both existing SDK 9.2.0 / `fr265` reference PRGs reconstruct byte for byte: 128
classes/52 strings in Minimal Console, 129 classes/53 strings in Hello World.
Original SHA-256 hashes remain those in
[the signing record](developer-signing.md). Standalone Hello World DATA
disassembly/assembly reproduces all 13066 bytes.

```sh
mkdir -p scratch/named-classes
cargo run -p ciq-dis -- all examples/hello-world/build/fr265/hello-world.prg scratch/named-classes/hello-readable
cargo run -p ciq-asm -- project scratch/named-classes/hello-readable scratch/named-classes/hello-readable.prg
cargo run -p ciq-dis -- data-source examples/hello-world/build/fr265/hello-world.prg > scratch/named-classes/hello-data.mcasm
cargo run -p ciq-asm -- data scratch/named-classes/hello-data.mcasm scratch/named-classes/hello-data.bin
```

Use fresh directories and output files. Tests exercise forward references,
automatic placement, class/string namespace separation, typed errors, symbol-ID
overflow, undefined/duplicate aliases, and exact reconstructed bytes. The
workspace has 58 passing tests; Clippy passes with warnings denied. No changed
application or new simulator experiment is required for these byte-identical
outputs. General relocation, code-label resolution, and resource assembly remain
outstanding.

No additional external implementation was incorporated. Format attribution
remains in [the class inspection record](data-classes.md); this change concerns
our independently designed assembly grammar and shared model.
