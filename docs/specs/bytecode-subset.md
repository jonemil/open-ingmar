# Bytecode and assembly syntax

Evidence and provenance:
[initial Rust investigation](../investigations/rust-bootstrap.md). The following
instruction encodings occur in Hello World, Minimal Console, and the
[control-flow specimen](../investigations/control-flow.md).
Widths were cross-checked with the pinned
research implementations. Execution semantics remain only partly understood.

An instruction starts with a one-byte opcode and is followed by its fixed-width
operand fields. Fields are big-endian. The `ciq-mc` instruction model preserves
operand bytes. A separate target query interprets relative operands as signed
16-bit displacements; encoding still preserves raw bits. Symbol and CODE-label
resolution occur in assembly, without stack interpretation.

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
| `17` | `ret` | none |
| `18` | `news` | 4 |
| `19` | `goto` | 2 |
| `1a` | `eq` | none |
| `1d` | `gt` | none |
| `23` | `jsr` | 2 |
| `27` | `spush` | 4 |
| `28` | `bt` | 2 |
| `29` | `bf` | 2 |
| `2a` | `frpush` | none |
| `2c` | `npush` | none |
| `2e` | `dup` | 1 |
| `30` | `getm` | none |
| `33` | `throw` | none |
| `35` | `argc` | 1 |
| `37` | `ipushz` | none |
| `38` | `ipush1` | 1 |
| `3e` | `btpush` | none |
| `3f` | `bfpush` | none |
| `43` | `getselfv` | 4 |
| `44` | `getself` | none |
| `45` | `getmv` | 4, 4 |
| `46` | `getlocalv` | 1, 4 |
| `47` | `getsv` | 4 |
| `48` | `invokemz` | none |
| `4a` | `argcincsp` | 1, 1 |

Unsupported opcode bytes cause an error at their code offset. They are never
guessed to be one-byte instructions. A truncated operand also causes an error.

## Instruction behavior

See [the instruction reference](instructions.md) for an individual section for
every supported opcode, including operands, abstract stack effects, examples,
and unresolved semantics. The
[evidence record](../investigations/instruction-semantics.md) distinguishes
source-derived behavior from compiled observations.

## Assembly syntax

Assembly uses one mnemonic per line, followed by one operand per field: a
hexadecimal bit pattern, a declared `@alias` in a known symbol-ID field, or
`&alias` in the `news` string-address field, or `code:label` in a relative
branch/subroutine operand. Numeric operands must start with
`0x` and fit their field widths. `#` starts a comment. Blank lines are allowed.
For example:

```text
argc 0x01             # CODE+0x000000
ipush1 0x29           # integer 41 in the reference probe
return
```

Disassembly emits fixed-width operands and code-offset comments. When the PRG
contains a supported symbol table, comments also annotate symbol operands using
escaped names; assembly ignores those comments and reproduces the original
operand bits. [Symbol declarations](symbol-assembly.md) resolve named operands.
Optional commas separate fields. [String declarations](data-strings.md) resolve
DATA addresses; [shared CODE labels](code-labels.md) identify instruction
boundaries and resolve signed 16-bit branch fixups. Includes and expressions
are not implemented.
`ciq-asm` can replace an entire ordinary code payload in a PRG template or
reconstruct a [complete section project](section-project.md); it is not a Monkey
C compiler.
