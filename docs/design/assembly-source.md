# Assembly source and optional layout constraints

Status: design direction agreed on 2026-10-08; example syntax below is a proposal, not implemented grammar. The current numeric grammar remains documented in [the bytecode specification](../specs/bytecode-subset.md).

## One language and shared model

Handwritten assembly, compiler-emitted assembly, and disassembled source should use the same declarations, instructions, and references. Differences should be the amount of explicit metadata and placement constraints, not separate source languages. The compiler may construct the shared model directly and use the same printer to emit assembly for inspection; an intermediate text file is optional.

The model needs logical symbols, data objects, code labels, raw payloads, and typed references. Keep it distinct from concrete decoded operand bytes. Resolve references and encode those bytes after allocation/layout. The current raw instruction model remains useful for preserving encodings and operands we cannot yet interpret.

## Symbols: identity versus placement

A symbol declaration should distinguish:

- A source alias used by instructions.
- The actual symbol name stored in the PRG.
- An optional required numeric symbol identifier (`id`).
- An optional name-record offset within the SYMBOLS payload (`name_at`).
- An optional index in the symbol identifier/offset table (`entry_index`).

Symbol ID, table order, and name-record location are different properties. The second column in today's inspection listing is the name-record location, not the symbol ID or table slot. An offset identifies the record's tag byte, as described in [the symbol format](../specs/symbols.md).

Proposed content-oriented source:

```text
.symbol system "Toybox_System" id=0x00800005
.symbol println "println" id=0x00800067

getmv @system, @println
```

An export requesting layout preservation could add constraints to the same declarations:

```text
# Illustrative positions, not offsets measured from a reference PRG.
.symbol system "Toybox_System" id=0x00800005 name_at=0x00000012 entry_index=0
.symbol println "println" id=0x00800067 name_at=0x00000023 entry_index=1

getmv @system, @println
```

Without `name_at`/`entry_index`, the assembler chooses a deterministic legal layout. With them, it honors those constraints or reports a conflict; it must not silently move pinned records. Validate table-size changes, overlapping records, invalid offsets, contradictory declarations, and duplicate identifiers. Reproducible automatic layout requires a specified allocation policy, not arbitrary iteration order.

Numeric IDs need a separate rule: external/API symbols must retain or resolve to their required bindings. Omitting a placement constraint never means the assembler may invent a different API symbol ID. Allocation of new application-local IDs is allowed only after valid namespaces/ranges and all affected references are understood. Existing exported IDs can remain explicit until that support exists.

## Files and other references

Declarations may live beside code or in a separate file using the same grammar. A proposed include mechanism could load `symbols.mcasm`; file boundaries should not change resolution or introduce a different symbol language. An include mechanism and its path semantics have not been implemented.

Strings/data should follow the same pattern: content plus an alias, optional explicit placement, and a typed reference. For example, proposed `.string greeting "Hello!"` and `news &greeting` distinguish a data-address reference from a symbol-ID reference such as `@println`. An inline string operand could be shorthand for a generated data declaration, once the actual layout and identity semantics are understood. Do not assume equal string contents imply that records can always be merged.

The complete project manifest still handles section order and raw sections. A future project-format version can select structured assembly as the authoritative input for decoded symbols/data. Do not make both a raw payload and a structured listing independently authoritative for the same section. Today's `.symbols.txt` is inspection-only.

## Preservation and implementation sequence

For sections or embedded records with unknown references, retain raw bytes and the placement needed to keep those references valid. Automatic layout cannot move a record safely while an opaque structure still holds an unrelocated pointer to it. Report unsupported relocation rather than silently produce inconsistent output.

Implementation can proceed in bounded steps:

1. Decode data strings/class references and distinguish reference kinds with evidence.
2. Introduce declarations and typed symbol references, initially resolving existing IDs while retaining the surrounding known layout.
3. Add symbol serialization with automatic or explicitly constrained record layout and table order.
4. Extend data serialization and relocation only for references we can update completely.

Test handwritten and printed source through the same parser/model; test deterministic allocation and clear conflicts for explicit constraints. Exact-byte checks apply to preservation cases; own-key signed output and intentionally transformed code are checked for expected content and behavior instead.
