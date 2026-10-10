# Initial Rust workspace and PRG round trip

Date: 2026-10-07. Question: what is the smallest useful independent
container/bytecode implementation, and can it disassemble and reassemble a
Garmin-generated specimen without changing its bytes?

## Research sources

External checkouts are ignored under `references/`. No external implementation
code was copied or introduced as a dependency.

- **Source:** [MonkeyC Ghidra Plugin][ref-monkeyc-ghidra-plugin]. **Revision:**
  `12ae55010721165b4ba98254d5e77db22eaa1b07`. **License:** Apache-2.0.
  **Relevant locations:** `doc/prg.md`, `doc/opcodes.md`,
  `data/languages/MonkeyC.sinc`, `src/main/java/monkeyc/MonkeyCLoader.java`.

- **Source:** [monkeyc-optimizer][ref-monkeyc-optimizer]. **Revision:**
  `2920dc88c23caadded9f66ee0613a7de6ab2d1ea`. **License:** MIT (`LICENSE.txt`).
  **Relevant locations:** `src/readprg.ts:readPrgWithOffsets`,
  `src/readprg/bytecode.ts:SectionKinds`,
  `src/readprg/opcodes.ts:parseCode/opcodeSize`, `src/readprg/signer.ts`.

Source-derived: both projects describe big-endian section identifiers and
payload lengths, ordinary code identifier `0xc0debabe`, and the instruction
encodings used in our specimens. The plugin documents an eight-byte zero
terminator. Its SLEIGH definition specifies byte alignment and big-endian
operands. The optimizer includes an emitter, offset fixups for metadata, and
signature recomputation; this is evidence that changed bytecode requires more
than editing a code-section length.

This investigation covers section framing and the subset of instructions used by
the two examples. Data/class/resource layouts, extended code paging, integrity
verification, and the broader opcode set have not been validated or implemented.
The plugin's instruction table calls `shlv`/`shrv` operand-free; the optimizer
comments that Garmin tooling accounts for an extra zero byte, but its decoder
represents the shift as one byte. Neither shift occurs in our specimens, so no
local interpretation has been adopted. Opcode `0x24` is named `ts` by both
sources but the optimizer rejects it. It is unsupported here.

## Minimal Console specimen

The independent
[Minimal Console project](../../examples/minimal-console/README.md) has one
target function, `hello()`, whose body is `System.println("Hello!");`. Watch-app
initialization, startup, and a base view are retained to make it a compilable
application. It deliberately has no custom rendering. Generated
resource/framework code means the PRG is not literally a single-function
executable.

Build: SDK 9.2.0, Temurin 17.0.20.1+1, `fr265`, the existing project-local
device profile and fonts, optimization `-O 0`, debug information retained, and
the user's ignored signing key. `examples/minimal-console/build.sh` returned
`BUILD SUCCESSFUL`. The shared helper also rebuilt Hello World successfully with
its original hash unchanged.

Observed Minimal Console section headers (file offset, identifier, payload
length):

```text
00000000 d000d00d     33
00000029 6060c0de     38
00000057 da7ababe  12974
0000330d c0debabe    556
00003541 d0000d1e    225
0000362a c0de7ab1    546
00003854 c1a557b1   2394
000041b6 f00d600d   7775
0000601d 6000db01      2
00006027 0ece7105      2
00006031 5717b015  95991
0001d730 5e771465      0
0001d738 e1c0de12   1540
0001dd44 00000000      0
```

All lengths land on the next section header; the eight-byte terminator ends the
file exactly. Ordinary code payload starts at file offset `0x3315`. Debug XML
locates `hello()` at virtual PCs `0x10000024` through `0x10000040`, inclusive.
The corresponding code offsets are `[0x24, 0x41)` and file offsets
`[0x3339, 0x3356)`.

Section meanings below are source-derived from the pinned research; the local
Rust reader verifies framing and preserves payloads but does not yet parse these
structures (apart from ordinary code).

- **Identifier:** `d000d00d`. **Research name:** Versioned header. **Purpose:**
  Format/Connect IQ version, flags, background and glance offsets.

- **Identifier:** `6060c0de`. **Research name:** Entry points. **Purpose:**
  Application identity, module/class to start, name/icon resource references,
  app type.

- **Identifier:** `da7ababe`. **Research name:** Data. **Purpose:** Class/module
  definitions, field/method references, strings and serialized values.

- **Identifier:** `c0debabe`. **Research name:** Ordinary code. **Purpose:**
  Executable VM instruction stream.

- **Identifier:** `d0000d1e`. **Research name:** Debug. **Purpose:** Embedded
  debug metadata; layout not investigated.

- **Identifier:** `c0de7ab1`. **Research name:** PC-to-line numbers.
  **Purpose:** Map code addresses to file/symbol/source-line records.

- **Identifier:** `c1a557b1`. **Research name:** Imports / link table.
  **Purpose:** Module/class import records.

- **Identifier:** `f00d600d`. **Research name:** Resources. **Purpose:** Indexed
  application strings, images, fonts and other resources.

- **Identifier:** `6000db01`. **Research name:** Permissions. **Purpose:**
  Permission identifier list.

- **Identifier:** `0ece7105`. **Research name:** Exceptions. **Purpose:**
  Try-region and handler code offsets.

- **Identifier:** `5717b015`. **Research name:** Symbols. **Purpose:** Symbol
  identifiers, name offsets and strings.

- **Identifier:** `5e771465`. **Research name:** Settings. **Purpose:**
  Serialized settings data.

- **Identifier:** `e1c0de12`. **Research name:** Developer signature.
  **Purpose:** Signatures and public-key components, not the private signing
  key.

Observed here: permissions and exceptions each have payload `00 00` (consistent
with zero entries in the research formats), and settings has an empty payload.
The eight-byte `00000000/00000000` record terminates the stream rather than
holding application data. The symbol payload is much larger than code; its
inclusion in a debug specimen should not be confused with executable
instructions. Whether every symbol is necessary at runtime has not been tested.

Observed bytes:

```text
35 01 27 00 80 02 e6 30 13 00 45 00 80 00 05 00 80 00 67
2a 18 00 00 32 79 0f 02 02 16
```

Local disassembly:

```text
argc 0x01                       # CODE+0x000024
spush 0x008002e6                 # CODE+0x000026
getm                            # CODE+0x00002b
lputv 0x00                      # CODE+0x00002c
getmv 0x00800005 0x00800067       # CODE+0x00002e
frpush                          # CODE+0x000037
news 0x00003279                 # CODE+0x000038
invokem 0x02                    # CODE+0x00003d
popv                            # CODE+0x00003f
return                          # CODE+0x000040
```

The compiler's source/debug correspondence establishes which function these
bytes implement. Precise call/frame semantics are still research questions;
operand values remain raw bit patterns in the Rust model.

## Implementation and local verification

Four workspace members now have working implementations: `ciq-prg` preserves
ordered opaque sections, `ciq-mc` decodes/encodes the observed instruction
subset, `ciq-dis` prints section listings or assembly, and `ciq-asm` reassembles
code into an existing PRG template. They use only the Rust standard library and
workspace dependencies. Local specs are
[section framing](../specs/prg-container.md) and
[instruction subset/assembly grammar](../specs/bytecode-subset.md).

Verification used Rust/Cargo 1.99.0. The manifest declares Rust 1.85 as the
intended minimum for edition 2024; that older toolchain has not been tested
here.

- `cargo test --workspace`: ten passing tests covering malformed/truncated
  containers, section preservation, ambiguous code, variable-width instructions,
  unsupported opcodes, and invalid assembly operands.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `cargo fmt --all -- --check`: passed.
- Both complete ordinary code sections decoded: Minimal Console has 171
  instructions; Hello World has 231.
- All debug function ranges with start/end PCs align to decoded instruction
  boundaries: 26 Minimal Console ranges and 29 Hello World ranges. Each sidecar
  also contains 104 entries without PC ranges, which were excluded from this
  check.
- Whole-code disassembly/reassembly followed by PRG reconstruction produced
  byte-identical files for both specimens, checked using Python byte equality.
  This checks the reconstructed section stream and all preserved
  signatures/resources, not merely instruction text.
- Shared build helper shell syntax and the six existing downloader tests passed.

- **Artifact:** Minimal Console original and
  `scratch/rust-bootstrap/minimal-console-roundtrip.prg`. **Bytes:** 122188.
  **SHA-256:**
  `cd361b76dc29e828d43f0511df3bebd96dc63ce6b7be61b70e2be8e1092d95e7`.

- **Artifact:** Hello World original and
  `scratch/rust-bootstrap/hello-world-roundtrip.prg`. **Bytes:** 122668.
  **SHA-256:**
  `8608ed040f18294a2a9af870f53e26e64a0d2fe3ef8c032d600043b591d182d7`.

Reproduce with fresh output filenames (`ciq-asm` refuses to overwrite files):

```sh
examples/minimal-console/build.sh
mkdir -p scratch/roundtrip
cargo run -p ciq-dis -- sections examples/minimal-console/build/fr265/minimal-console.prg
cargo run -p ciq-dis -- code examples/minimal-console/build/fr265/minimal-console.prg > scratch/roundtrip/minimal-console.mcasm
cargo run -p ciq-asm -- examples/minimal-console/build/fr265/minimal-console.prg scratch/roundtrip/minimal-console.mcasm scratch/roundtrip/minimal-console.prg
python3 -c 'from pathlib import Path; assert Path("examples/minimal-console/build/fr265/minimal-console.prg").read_bytes() == Path("scratch/roundtrip/minimal-console.prg").read_bytes()'
```

## Remaining runtime experiment

Podman is available only on the host. No newly emitted PRG has been launched in
the simulator during this investigation. The original Hello World baseline was
already verified in the [simulator investigation](simulator-container.md).

Run the prepared reassembled Minimal Console specimen on the host:

```sh
sh scripts/run-simulator-container.sh --prg scratch/rust-bootstrap/minimal-console-roundtrip.prg
```

Expected console output is `Hello!`; the base view has no custom greeting on
screen. For interaction, put `--interactive` first. After this baseline check,
change the original numeric probe from `41` to `42` at equal instruction length
and test simulator behavior. A later equivalent-but-different program requires
evidence about signature handling and affected references; byte identity here is
the first deliberate milestone, not the final goal.

[ref-monkeyc-ghidra-plugin]: https://github.com/Gualdimar/MonkeyC-Ghidra-Plugin/tree/12ae55010721165b4ba98254d5e77db22eaa1b07
[ref-monkeyc-optimizer]: https://github.com/markw65/monkeyc-optimizer/tree/2920dc88c23caadded9f66ee0613a7de6ab2d1ea
