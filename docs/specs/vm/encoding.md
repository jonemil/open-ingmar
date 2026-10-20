# VM bytecode encoding

Evidence and provenance:
[initial Rust investigation](../../investigations/rust-bootstrap.md). Initial encodings were established with Hello World, Minimal Console, and the
[control-flow specimen](../../investigations/control-flow.md).
Char constants have [separate encoding
evidence](../../investigations/compiler-char-literals.md). Floating immediates
have [IEEE encoding evidence](../../investigations/compiler-floating-values.md).
Long immediates have [64-bit encoding
evidence](../../investigations/compiler-long-values.md). Signed Number
immediates have [boundary
evidence](../../investigations/compiler-number-literals.md). Null checks have
[focused encoding/runtime evidence](../../investigations/null-check-opcodes.md).
Stacked member reads have [field-update
evidence](../../investigations/member-get-opcode.md). Inversion has
[Number/Boolean evidence](../../investigations/invert-opcode.md). Retaining
array stores have [literal-fill
evidence](../../investigations/array-fill-opcode.md). Constant arrays have
[record/load evidence](../../investigations/constant-arrays.md). Widths were
cross-checked with the pinned research implementations. Execution semantics
remain only partly understood.

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
| `05` | `mulv` | none |
| `06` | `divv` | none |
| `07` | `andv` | none |
| `08` | `orv` | none |
| `09` | `modv` | none |
| `0a` | `shlv` | none |
| `0b` | `shrv` | none |
| `0c` | `xorv` | none |
| `0d` | `getv` | none |
| `0e` | `putv` | none |
| `0f` | `invokem` | 1 |
| `10` | `agetv` | none |
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
| `1b` | `lt` | none |
| `1c` | `lte` | none |
| `1d` | `gt` | none |
| `1e` | `gte` | none |
| `1f` | `ne` | none |
| `20` | `isnull` | none |
| `21` | `isa` | none |
| `22` | `hasv` | none |
| `23` | `jsr` | 2 |
| `25` | `ipush4` | 4 |
| `26` | `fpush` | 4 |
| `27` | `spush` | 4 |
| `28` | `bt` | 2 |
| `29` | `bf` | 2 |
| `2a` | `frpush` | none |
| `2c` | `npush` | none |
| `2d` | `invv` | none |
| `2e` | `dup` | 1 |
| `2f` | `newd` | none |
| `30` | `getm` | none |
| `31` | `lpush` | 8 |
| `32` | `dpush` | 8 |
| `33` | `throw` | none |
| `34` | `cpush` | 4 |
| `35` | `argc` | 1 |
| `36` | `newba` | none |
| `37` | `ipushz` | none |
| `38` | `ipush1` | 1 |
| `39` | `ipush2` | 2 |
| `3a` | `ipush3` | 3 |
| `3b` | `fpushz` | none |
| `3c` | `lpushz` | none |
| `3d` | `dpushz` | none |
| `3e` | `btpush` | none |
| `3f` | `bfpush` | none |
| `40` | `apush` | 4 |
| `41` | `bpush` | 4 |
| `42` | `hpush` | 4 |
| `43` | `getselfv` | 4 |
| `44` | `getself` | none |
| `45` | `getmv` | 4, 4 |
| `46` | `getlocalv` | 1, 4 |
| `47` | `getsv` | 4 |
| `48` | `invokemz` | none |
| `49` | `aputvdup` | none |
| `4a` | `argcincsp` | 1, 1 |
| `4b` | `isnotnull` | none |

Unsupported opcode bytes cause an error at their code offset. They are never
guessed to be one-byte instructions. A truncated operand also causes an error.

## Instruction behavior

See [the instruction reference](instructions.md) for supported opcode behavior,
including operands, abstract stack effects, examples, and unresolved semantics.
The
[evidence record](../../investigations/instruction-semantics.md) distinguishes
source-derived behavior from compiled observations.
