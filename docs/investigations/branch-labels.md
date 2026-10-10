# Branch-label assembly and disassembly

2026-10-10. Follow-up to [control-flow research](control-flow.md), using its
SDK 9.2.0, `fr265`, `-O 0` specimen and original device memory allowance.
External provenance and the specimen's source/hash are recorded there.

## Implemented behavior

Independent Rust now supports all 38 observed opcodes. `ciq-mc` preserves
operand bits and exposes a relative-target query; `ciq-assembly` binds typed
`Code` operands and their `code:label` syntax after layout. Forward/backward
displacements must fit signed 16 bits and target instructions. Numeric operands
remain unchecked raw bits. [The label specification](../specs/code-labels.md)
defines these rules.

Project export reuses method/line labels and adds labels for verified branch or
subroutine targets. `code-symbolic` also emits local branch labels; numeric
`code` listings retain bit patterns. Invalid or unrepresentable targets remain
numeric. EXCEPTIONS remains raw; length-changing edits still require accounting
for its offsets and other opaque references.

## Reconstruction and runtime evidence

**Experimentally observed:** the complete control-flow PRG exports to structured
CODE and reconstructs byte for byte, including the original signature and raw
exception table. Change this one line in exported CODE:

```text
# original, at CODE+0xa1
goto code:code_0000008c
# replacement
goto code:code_000000a4
```

Reconstruct with our developer key. The resulting PRG is byte-identical to the
independent raw-edit `loop-once.prg` from the research, SHA-256:

```text
fef9f470f9934b00d05b096a8ff64dea9bf40c4ed5eb8a7c3f0e722f5de09557
```

The simulator prints `41, 43, 2, 41, 43, 45`, and the captured display shows
`45`. This confirms execution of the assembler-produced branch edit. Only CODE
and the developer signature differ from the original; exception and unrelated
section bytes remain unchanged. Captures/logs are saved under ignored
`scratch/control-flow/simulator-branch-assembled/`.

## Reproduction

Build the SDK specimen using [the research procedure](control-flow.md#reproduction).
Use new paths for outputs:

```sh
cargo run -p ciq-dis -- all scratch/control-flow/sdk-probe/build/fr265/sdk-probe.prg scratch/control-flow/branch-project
cargo run -p ciq-asm -- project scratch/control-flow/branch-project scratch/control-flow/branch-roundtrip.prg
cmp scratch/control-flow/sdk-probe/build/fr265/sdk-probe.prg scratch/control-flow/branch-roundtrip.prg
```

Edit `branch-project/003-code-c0debabe.mcasm` as above, keeping its CODE-offset
comment if desired. Reassemble and run:

```sh
cargo run -p ciq-asm -- project scratch/control-flow/branch-project scratch/control-flow/branch-assembled-loop-once.prg -y keys/developer_key.der
cargo run -p ciq-asm -- verify scratch/control-flow/branch-assembled-loop-once.prg -y keys/developer_key.der
xvfb-run -a -s '-screen 0 1280x1024x24' python3 scripts/simulator-session.py --prg scratch/control-flow/branch-assembled-loop-once.prg --device fr265 --observe-seconds 10
```

Rust tests cover signed target interpretation, truncation, typed/text equality,
forward/backward binding after insertion, displacement endpoints/overflow,
invalid reference kinds, undefined/end targets, numeric fallback for malformed
targets, and complete project preservation without DATA or symbol tables.

Next implement structured lossless exception sections and resolve their named
references after CODE layout. Frontend conditions and general relocation remain
deferred.
