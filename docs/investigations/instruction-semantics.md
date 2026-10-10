# Supported instruction semantics investigation

Date: 2026-10-08. Question: what does each instruction currently supported by
our Rust tools mean, and which parts are actually established?

## Scope and sources

The result is the [28-entry instruction reference](../specs/instructions.md). No
VM implementation or new opcode support was added. External sources informed
independent descriptions; no source code was copied into the implementation.

- **Source:** [MonkeyC Ghidra Plugin][ref-monkeyc-ghidra-plugin]. **Revision and
  license:** `12ae55010721165b4ba98254d5e77db22eaa1b07`, Apache-2.0. **Relevant
  source locations:** `doc/opcodes.md`: instruction descriptions;
  `data/languages/MonkeyC.sinc`: opcode/operand definitions.

- **Source:** [monkeyc-optimizer][ref-monkeyc-optimizer]. **Revision and
  license:** `2920dc88c23caadded9f66ee0613a7de6ab2d1ea`, MIT. **Relevant source
  locations:** `src/readprg/opcodes.ts`: `parseCode`, `opcodeSize`, `getOpInfo`,
  `getOpEffects`; `src/readprg/interp.ts`: partial optimizer interpretation.

- **Source:** Our compiler specimens. **Revision and license:** SDK 9.2.0,
  `fr265`, `-O 0`, debug metadata. **Relevant source locations:**
  [Minimal Console](../../examples/minimal-console/README.md),
  [Hello World](../../examples/hello-world/README.md), PRGs and `.debug.xml`
  files.

The optimizer's stack accounting is an analysis model, not a full VM
specification. Many SLEIGH instruction bodies are empty, so their existence is
not evidence of executable instruction semantics. The external sources and our
compiler samples do not independently establish physical stack-cell sizes,
frame-pointer arithmetic, exception behavior, or numeric boundary behavior.

Specimen hashes and build configuration are retained in
[the bootstrap investigation](rust-bootstrap.md). Minimal Console SHA-256:
`cd361b76dc29e828d43f0511df3bebd96dc63ce6b7be61b70e2be8e1092d95e7`; Hello World
SHA-256: `8608ed040f18294a2a9af870f53e26e64a0d2fe3ef8c032d600043b591d182d7`.
Ordinary code payloads start at file offsets `0x3315` and `0x3371`,
respectively. They decode to 171 and 231 instructions. Their union contains all
28 opcodes supported by `ciq-mc`.

## Reproduction

Rebuild using each example's build script and the SDK/device/key configuration
described in its README. Inspect with our tools:

```sh
cargo run -p ciq-dis -- code examples/minimal-console/build/fr265/minimal-console.prg
cargo run -p ciq-dis -- code examples/hello-world/build/fr265/hello-world.prg
cargo run -p ciq-dis -- symbols examples/minimal-console/build/fr265/minimal-console.prg
```

For each supported mnemonic, find its first occurrence in these disassemblies,
read its bytes from the ordinary code payload, and locate the containing
function using the SDK-generated debug XML. A file offset is the code payload's
start plus CODE offset. The table below records that first occurrence (Minimal
Console takes precedence). Function names are debug mappings, not conclusions
inferred from opcode names. Unmapped stubs have no matching debug function
record.

## Experimentally observed instruction coverage

- **Instruction:** `popv`. **Specimen:** minimal-console. **CODE offset:**
  `0x000022`. **File offset:** `0x003337`. **Bytes:** `02`. **Debug function:**
  `Rez/<init>`.

- **Instruction:** `addv`. **Specimen:** hello-world. **CODE offset:**
  `0x0000c0`. **File offset:** `0x003431`. **Bytes:** `03`. **Debug function:**
  `HelloWorldView/onUpdate`.

- **Instruction:** `subv`. **Specimen:** hello-world. **CODE offset:**
  `0x000098`. **File offset:** `0x003409`. **Bytes:** `04`. **Debug function:**
  `HelloWorldView/onUpdate`.

- **Instruction:** `divv`. **Specimen:** hello-world. **CODE offset:**
  `0x00007c`. **File offset:** `0x0033ed`. **Bytes:** `06`. **Debug function:**
  `HelloWorldView/onUpdate`.

- **Instruction:** `orv`. **Specimen:** minimal-console. **CODE offset:**
  `0x000092`. **File offset:** `0x0033a7`. **Bytes:** `08`. **Debug function:**
  `prompt_font__title/<init>`.

- **Instruction:** `putv`. **Specimen:** minimal-console. **CODE offset:**
  `0x000093`. **File offset:** `0x0033a8`. **Bytes:** `0e`. **Debug function:**
  `prompt_font__title/<init>`.

- **Instruction:** `invokem`. **Specimen:** minimal-console. **CODE offset:**
  `0x000020`. **File offset:** `0x003335`. **Bytes:** `0f 03`. **Debug
  function:** `Rez/<init>`.

- **Instruction:** `aputv`. **Specimen:** minimal-console. **CODE offset:**
  `0x000069`. **File offset:** `0x00337e`. **Bytes:** `11`. **Debug function:**
  `MinimalConsoleApp/getInitialView`.

- **Instruction:** `lgetv`. **Specimen:** hello-world. **CODE offset:**
  `0x000092`. **File offset:** `0x003403`. **Bytes:** `12 02`. **Debug
  function:** `HelloWorldView/onUpdate`.

- **Instruction:** `lputv`. **Specimen:** minimal-console. **CODE offset:**
  `0x00002c`. **File offset:** `0x003341`. **Bytes:** `13 00`. **Debug
  function:** `/hello`.

- **Instruction:** `newa`. **Specimen:** minimal-console. **CODE offset:**
  `0x00004f`. **File offset:** `0x003364`. **Bytes:** `14`. **Debug function:**
  `MinimalConsoleApp/getInitialView`.

- **Instruction:** `newc`. **Specimen:** minimal-console. **CODE offset:**
  `0x00005c`. **File offset:** `0x003371`. **Bytes:** `15`. **Debug function:**
  `MinimalConsoleApp/getInitialView`.

- **Instruction:** `return`. **Specimen:** minimal-console. **CODE offset:**
  `0x000002`. **File offset:** `0x003317`. **Bytes:** `16`. **Debug function:**
  `unmapped`.

- **Instruction:** `news`. **Specimen:** minimal-console. **CODE offset:**
  `0x000038`. **File offset:** `0x00334d`. **Bytes:** `18 00 00 32 79`. **Debug
  function:** `/hello`.

- **Instruction:** `spush`. **Specimen:** minimal-console. **CODE offset:**
  `0x00001b`. **File offset:** `0x003330`. **Bytes:** `27 00 80 00 a1`. **Debug
  function:** `Rez/<init>`.

- **Instruction:** `frpush`. **Specimen:** minimal-console. **CODE offset:**
  `0x000019`. **File offset:** `0x00332e`. **Bytes:** `2a`. **Debug function:**
  `Rez/<init>`.

- **Instruction:** `dup`. **Specimen:** minimal-console. **CODE offset:**
  `0x000050`. **File offset:** `0x003365`. **Bytes:** `2e 00`. **Debug
  function:** `MinimalConsoleApp/getInitialView`.

- **Instruction:** `getm`. **Specimen:** minimal-console. **CODE offset:**
  `0x00002b`. **File offset:** `0x003340`. **Bytes:** `30`. **Debug function:**
  `/hello`.

- **Instruction:** `argc`. **Specimen:** minimal-console. **CODE offset:**
  `0x000000`. **File offset:** `0x003315`. **Bytes:** `35 01`. **Debug
  function:** `unmapped`.

- **Instruction:** `ipushz`. **Specimen:** minimal-console. **CODE offset:**
  `0x000052`. **File offset:** `0x003367`. **Bytes:** `37`. **Debug function:**
  `MinimalConsoleApp/getInitialView`.

- **Instruction:** `ipush1`. **Specimen:** minimal-console. **CODE offset:**
  `0x00004d`. **File offset:** `0x003362`. **Bytes:** `38 01`. **Debug
  function:** `MinimalConsoleApp/getInitialView`.

- **Instruction:** `getselfv`. **Specimen:** minimal-console. **CODE offset:**
  `0x000004`. **File offset:** `0x003319`. **Bytes:** `43 00 80 02 9c`. **Debug
  function:** `MinimalConsoleApp/<init>`.

- **Instruction:** `getself`. **Specimen:** minimal-console. **CODE offset:**
  `0x00001a`. **File offset:** `0x00332f`. **Bytes:** `44`. **Debug function:**
  `Rez/<init>`.

- **Instruction:** `getmv`. **Specimen:** minimal-console. **CODE offset:**
  `0x000010`. **File offset:** `0x003325`. **Bytes:**
  `45 00 80 00 15 00 80 03 11`. **Debug function:** `Rez/<init>`.

- **Instruction:** `getlocalv`. **Specimen:** hello-world. **CODE offset:**
  `0x00004f`. **File offset:** `0x0033c0`. **Bytes:** `46 01 00 80 02 0b`.
  **Debug function:** `HelloWorldView/onUpdate`.

- **Instruction:** `getsv`. **Specimen:** minimal-console. **CODE offset:**
  `0x000009`. **File offset:** `0x00331e`. **Bytes:** `47 00 80 00 18`. **Debug
  function:** `MinimalConsoleApp/<init>`.

- **Instruction:** `invokemz`. **Specimen:** minimal-console. **CODE offset:**
  `0x00000e`. **File offset:** `0x003323`. **Bytes:** `48`. **Debug function:**
  `MinimalConsoleApp/<init>`.

- **Instruction:** `argcincsp`. **Specimen:** hello-world. **CODE offset:**
  `0x00004c`. **File offset:** `0x0033bd`. **Bytes:** `4a 02 02`. **Debug
  function:** `HelloWorldView/onUpdate`.

## Findings and conflicts

**Experimentally observed:** opcode bytes, field widths, and the complete
decoding boundaries match both generated code payloads. Symbol operands resolve
through the PRG's own table as recorded in
[the symbols investigation](symbols.md). The reference documents numeric
operands; symbolic source syntax remains a design proposal.

**Experimentally observed:** Minimal Console's `hello()` is CODE `[0x24, 0x41)`.
Its relevant sequence is:

```text
argc 0x01
spush 0x008002e6       # globals
getm
lputv 0x00
getmv 0x00800005 0x00800067 # Toybox_System.println
frpush
news 0x00003279
invokem 0x02
popv
return
```

The data payload at offset `0x3279` starts with `01 00 06 48 65 6c 6c 6f 21 00`:
string tag, big-endian length 6, `Hello!`, terminating zero. This establishes
this particular `news` reference and record; it does not yet specify all
serialized data or string addressing. Module/member operands in `getmv` are IDs;
the printable names come from a separate symbol-table lookup.

**Interpretation:** the call count includes a hidden context slot. One explicit
string argument uses `invokem 0x02`; the global no-parameter `hello()` prologue
uses `argc 0x01`, and `onStart(state)` uses `argc 0x02`. The plugin's generic
wording about argument counts must not be read as explicit source-argument
counts. `frpush` supplies an extra call-preparation value in the observed
sequence, but its exact representation remains unknown.

**Source-derived:** `getOpInfo` models `invokem n` as pop `n + 1`, push 1;
`invokemz` as pop 1, push 1; and `return` as pop 1, push 0. The plugin describes
`invokemz` as the compact equivalent of `frpush; invokem 1`. We have not
independently proven that expansion for all callable kinds.

**Conflict/unknown:** Minimal Console starts with `35 01 16 16`: an `argc 1`
followed by returns, without an explicit constant push. Other functions discard
a call result before returning. Therefore the optimizer's `return` pop count
cannot yet be treated as a sufficient standalone physical-stack rule. Default
returns, implicit frame storage, and the calling convention need further work.

**Interpretation:** `argcincsp 2, 2` at HelloWorldView/onUpdate is consistent
with a context plus its drawing-context parameter and two additional locals.
`argc` and `argcincsp` have no logical evaluation-stack pop/push in the
optimizer, but clearly concern frame setup. Physical storage units and
stack-pointer movement remain unverified.

**Experimentally observed:** `hello()` writes the resolved globals module into
local slot 0. Do not adopt the plugin's blanket “local 0 is always this” wording
as a universal specification. The relationship between current receiver, context
slots, and writable locals is still open.

**Source-derived:** `ipush1` is decoded as unsigned u8 by the optimizer.
**Experimentally observed:** `0x29` is used for 41, and the original Hello World
displays/prints 41 in the simulator. Sign/zero extension at `0x80` and `0xff`
has not been tested independently.

**Source-derived:** `newc` can have generated initializer effects.
**Experimentally observed:** generated view construction also performs an
explicit `initialize` member lookup and call after allocation. Do not conflate
the generated `<init>` with the public `initialize` method.

## Validation and next experiments

Documentation coverage was checked against `ciq-mc`'s 28 opcode definitions:
every mnemonic has exactly one instruction section, matching opcode, operand
widths, and total length. Observed bytes were checked for opcode/length
agreement. This is documentation/encoding validation, not an execution test.

Runtime evidence remains the original Hello World's simulator capture and
console output, as recorded in [the build](hello-world-build.md) and
[simulator investigation](simulator-container.md). This analysis adds no new
simulator execution. Next run the reconstructed PRGs, then make the planned
same-length `41` to `42` edit. Later controlled specimens should distinguish
call context and default returns, test integer-immediate boundaries and
arithmetic types, and exercise object/array lookup failures. Keep signing and
relocation validation separate from decoding success.

[ref-monkeyc-ghidra-plugin]: https://github.com/Gualdimar/MonkeyC-Ghidra-Plugin/tree/12ae55010721165b4ba98254d5e77db22eaa1b07
[ref-monkeyc-optimizer]: https://github.com/markw65/monkeyc-optimizer/tree/2920dc88c23caadded9f66ee0613a7de6ab2d1ea
