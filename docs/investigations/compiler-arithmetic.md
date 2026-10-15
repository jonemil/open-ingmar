# Compiler integer arithmetic

2026-10-10. Independently implemented Rust frontend/IR/backend extension: binary
integer addition/subtraction, left associativity, and parentheses. Both native
composition and text resumption work. Official SDK 9.2.0 `fr265` simulator
console and inspected screen captures confirm `40 + (8 - 5)` returns `43`, and
`2 - 5` returns `-3`.

## Inputs and instruction observations

Tracked input: [arithmetic.mc](../../examples/compiler-probe/arithmetic.mc).
Generated body:

```text
probe:
argc 0x01
ipush1 0x28
ipush1 0x08
ipush1 0x05
subv
addv
return
```

CODE bytes: `35 01 38 28 38 08 38 05 04 03 16` (11 bytes). The negative
comparison emits `35 01 38 02 38 05 04 16` (8 bytes). Actual arithmetic
instructions execute; results are not folded into literals.

These specimens establish integer addition/subtraction and operand order, not
general frames, overflow, promotion, or a full VM representation. Current syntax,
limits, and lowering are in [the source spec](../specs/compiler-subset.md).

## Reproduction

Run from the repository root with new output paths. SDK/device files and your
external developer key must be available for scaffold/signing/runtime steps:

```sh
mkdir -p scratch/arithmetic
cargo build -p ciq-compiler -p ciq-dis -p ciq-asm
sh scripts/build-example.sh examples/hello-world fr265
target/debug/ciq-dis all examples/hello-world/build/fr265/hello-world.prg scratch/arithmetic/base-project
target/debug/ciq-compiler --emit ir examples/compiler-probe/arithmetic.mc scratch/arithmetic/probe.ciqir
target/debug/ciq-compiler --from ir scratch/arithmetic/probe.ciqir scratch/arithmetic/probe.mcasm
target/debug/ciq-compiler --from assembly --emit code scratch/arithmetic/probe.mcasm scratch/arithmetic/text.bin
target/debug/ciq-compiler --emit code examples/compiler-probe/arithmetic.mc scratch/arithmetic/native.bin
cmp scratch/arithmetic/native.bin scratch/arithmetic/text.bin
python3 - <<'PY'
from pathlib import Path
Path('scratch/arithmetic/negative.mc').write_text('function probe() { return 2 - 5; }\n')
PY
target/debug/ciq-compiler scratch/arithmetic/negative.mc scratch/arithmetic/negative.mcasm
```

Explicit integration for the measured 744-byte CODE/7777-byte RESOURCES scaffold
(assertions fail for a different build):

Integrate into a fresh scaffold export: rename the old entry label and its line
references to an unused alias, preserving their body/PCs. Append generated
assembly under the original alias, redirect its method reference, and apply the
DATA/resource lengths and offsets recorded in this record. Reconstruct/sign,
verify our key, and run each case separately using the
[shared procedure](../research.md#shared-compiler-reproduction).

Actual execution reused the Garmin container through container-broker, invoking
`xvfb-run -a -s '-screen 0 1280x1024x24' python3 scripts/simulator-session.py --prg <path> --device fr265 --observe-seconds 8`.
Fixed output under `scratch/simulator/` was copied after each run to
`scratch/arithmetic/simulator-43/` and `simulator-negative-3/`; both screens
were inspected. Console greeting and watch display agree on `43` and `-3`
respectively.

## Preservation and verification

Original CODE is an exact 744-byte prefix; new function starts at `0x2e8`.
DATA's method pointer at `0x49f` changes to `0x100002e8`; all other DATA bytes
are preserved. CODE grows to 755/752 bytes. RESOURCES gains 21/24 trailing
zeros, keeping combined growth at 32 and retaining signature-payload alignment
modulo 32. This is the bounded scaffold technique from
[earlier append experiments](code-labels-and-relocation.md), not general
resource/metadata relocation.

Only DATA, CODE, RESOURCES, and DEVELOPER_SIGNATURE change. Existing line
mappings stay with `probe_original`; the compiled body has no generated debug
mapping. Empty EXCEPTIONS and zero header CODE offsets stay unchanged. Both
developer signature algorithms verify against the supplied key. Independent
section parsing checked these preservation claims. Positive output also exports
and reassembles byte identically:

```sh
target/debug/ciq-dis all scratch/arithmetic/probe-43.prg scratch/arithmetic/roundtrip-project
target/debug/ciq-asm project scratch/arithmetic/roundtrip-project scratch/arithmetic/roundtrip.prg
cmp scratch/arithmetic/probe-43.prg scratch/arithmetic/roundtrip.prg
```

| Artifact | SHA-256 |
|---|---|
| SDK scaffold | `8608ed040f18294a2a9af870f53e26e64a0d2fe3ef8c032d600043b591d182d7` |
| Native arithmetic CODE | `c587b496a100663ad0ae6e850e1d83dd6ed546c3930c043fab1c3a19adecb7d6` |
| Signed `43` PRG | `1ae9a16c76fa8e27e3294e924e3a7e8038a3f33d33eb54d5bf81e463595503bf` |
| Signed `-3` PRG | `dafbcf184124e55e1cf0432deaf75e4e58c19451bf04a6068f59e0a26b94ae6f` |

Seven new tests cover grouping/associativity, operand pairs and negative/large
results, dependency scheduling with reordered definitions, malformed
graphs/source diagnostics, resource limits, and CLI source/IR/assembly
equivalence. The restricted test interpreter checks instruction results without
pretending to model physical frames; simulator evidence remains the runtime
oracle.
