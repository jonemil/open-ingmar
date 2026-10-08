# Bytecode and initial assembly syntax

Evidence and provenance: [initial Rust investigation](../investigations/rust-bootstrap.md). The following instruction encodings occur in the locally compiled Hello World and Minimal Console ordinary code sections. Widths were cross-checked with the pinned research implementations. Execution semantics remain only partly understood.

An instruction starts with a one-byte opcode and is followed by its fixed-width operand fields. Fields are big-endian. The `ciq-mc` instruction model preserves operand bytes; it does not assign signedness, resolve symbols, relocate branches, or interpret the stack. Symbol reference resolution occurs in the assembler.

| Opcode | Mnemonic | Operand field widths in bytes |
|---|---|---|
| `02` | `popv` | none |
| `03` | `addv` | none |
| `04` | `subv` | none |
| `06` | `divv` | none |
| `08` | `orv` | none |
| `0e` | `putv` | none |
| `0f` | `invokem` | 1 |
| `11` | `aputv` | none |
| `12` | `lgetv` | 1 |
| `13` | `lputv` | 1 |
| `14` | `newa` | none |
| `15` | `newc` | none |
| `16` | `return` | none |
| `18` | `news` | 4 |
| `27` | `spush` | 4 |
| `2a` | `frpush` | none |
| `2e` | `dup` | 1 |
| `30` | `getm` | none |
| `35` | `argc` | 1 |
| `37` | `ipushz` | none |
| `38` | `ipush1` | 1 |
| `43` | `getselfv` | 4 |
| `44` | `getself` | none |
| `45` | `getmv` | 4, 4 |
| `46` | `getlocalv` | 1, 4 |
| `47` | `getsv` | 4 |
| `48` | `invokemz` | none |
| `4a` | `argcincsp` | 1, 1 |

Unsupported opcode bytes cause an error at their code offset. They are never guessed to be one-byte instructions. A truncated operand also causes an error.

## Instruction behavior

See [the instruction reference](instructions.md) for an individual section for every supported opcode, including operands, abstract stack effects, examples, and unresolved semantics. The [evidence record](../investigations/instruction-semantics.md) distinguishes source-derived behavior from compiled observations.

## Assembly syntax

Assembly uses one mnemonic per line, followed by one operand per field: a hexadecimal bit pattern, a declared `@alias` in a known symbol-ID field, or `&alias` in the `news` string-address field. Numeric operands must start with `0x` and fit their field widths. `#` starts a comment. Blank lines are allowed. For example:

```text
argc 0x01             # CODE+0x000000
ipush1 0x29           # integer 41 in the reference probe
return
```

Disassembly emits fixed-width operands and code-offset comments. When the PRG contains a supported symbol table, comments also annotate symbol operands using escaped names; assembly ignores those comments and reproduces the original operand bits. [Symbol declarations](symbol-assembly.md) resolve named operands. Optional commas separate fields. [String declarations](data-strings.md) resolve DATA addresses; labels, includes, and expressions are not implemented. `ciq-asm` can replace an entire ordinary code payload in a PRG template or reconstruct a [complete section project](section-project.md); it is not a Monkey C compiler.
