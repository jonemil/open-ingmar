# Shared CODE labels

Implemented 2026-10-08. Evidence: [label and relocation investigation](../investigations/code-labels-and-relocation.md).

A label is an identifier followed by `:`, on its own line (comments allowed). It emits no bytes and denotes the next instruction's CODE-relative offset. Identifier rules match other aliases. Duplicate/invalid labels, mixed label/instruction lines, unsupported instructions, and incorrect operand counts are errors. Layout uses the verified instruction widths, within the 24-bit ordinary CODE address space.

```text
probe:
argc 0x01
ipush1 0x2a
return
```

Class method fields may use `value=code:probe` with type `0x06`. The assembler resolves the label and emits `0x10000000 | offset`. A label is a distinct namespace from `@symbol`, `%class`, and `&string`. Non-method fields reject `code:`. Undefined labels, oversized targets, and method targets at the end of CODE rather than an instruction are rejected. Generic end-of-stream labels are accepted for layout but cannot be method targets.

Project import reads CODE labels before resolving DATA, independently of manifest order. Labels require a unique CODE section. Numeric method values remain supported. The DATA library accepts label bindings via `parse_with_references` / `encode_with_references`; the assembler library exposes `code_labels`. Standalone DATA source remains self-contained and keeps numeric method addresses.

Full project export names verified method instruction-boundary targets in CODE and uses the corresponding `code:` reference in DATA. A unique unambiguous method alias becomes the label; shared/ambiguous targets and names fall back to `code_<offset>`. Multiple method fields may refer to one label. Unsupported CODE or invalid numeric targets remain raw/numeric rather than being guessed. Labels are source identities, not new VM symbols or evidence of a complete function signature.

Labels resolve method addresses only. Branch operands, line PCs, exception intervals, header offsets, and opaque metadata are not automatically relocated. Project import still checks manifest lengths. Explicitly changing those lengths does not validate other affected references. The append-only experiment in the investigation tests a larger body without shifting existing instructions; it does not enable general insertion/deletion within CODE.
