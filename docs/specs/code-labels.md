# Shared CODE labels

Evidence: [label and relocation
investigation][ref-label-and-relocation-investigation].

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
[the repeated-module experiment](../investigations/module-method-names.md).

Multiple method fields may refer to one label. Unsupported CODE or invalid
numeric targets remain raw/numeric rather than being guessed. Labels are source
identities, not new VM symbols or evidence of a complete function signature.

[Structured line mappings](line-numbers-and-debug.md) share these labels in
version 4 projects. Export adds labels for verified line-PC instruction
boundaries; named PCs move when those labels move. Named branch operands also
follow layout as described below. Named
[exception references](exceptions.md) also follow these labels. Header offsets,
numeric operands, and opaque metadata are not automatically relocated.
Project import still checks manifest lengths. Explicitly changing those lengths
does not validate other affected references. The append-only experiment in the
investigation tests a larger body without shifting existing instructions; it
does not enable general insertion/deletion within CODE. A subsequent
[in-place size experiment](../investigations/line-metadata.md) is prepared for
branch-free code with empty exceptions and zero background/glance offsets, with
confirmed shrunk-variant simulator execution returning `45`.

[ref-label-and-relocation-investigation]: ../investigations/code-labels-and-relocation.md

## Relative instruction operands

`goto`, `bt`, `bf`, and `jsr` accept `code:label` as their sole operand. The
assembler resolves local labels after layout, emitting the signed 16-bit
big-endian displacement `target - (instruction_offset + 3)`. Forward and
backward targets are supported. Undefined labels, targets at the end of CODE,
and displacements outside `-32768..32767` fail encoding. Other instructions
reject CODE-label operands. There is no automatic long-branch expansion.

Numeric operands remain unsigned hexadecimal bit patterns; for example,
`goto 0xffe8` carries displacement `-24`. Their target validity is unchecked,
and they do not change when labels move. This preserves unfamiliar or invalid
input without silently rewriting it.

Project export and `code-symbolic` name branch targets only when they denote
verified instruction boundaries. Project export reuses method/line labels,
adding `code_<offset>` names for remaining targets. Numeric `code` listings
retain original operand bits. Structured EXCEPTIONS shares labels too. See
[the implementation experiment](../investigations/branch-labels.md).
