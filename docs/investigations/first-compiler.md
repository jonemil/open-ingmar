# First Rust Monkey C compiler experiment

2026-10-10. Question: can a restricted independent frontend generate a complete
constant-return function and run it inside the known-good application? Result:
source → small semantic IR → readable assembly → assembled/signed PRG →
simulator is verified for `41` and `42`.

## Design and evidence

The [representation decision](../design/compiler-ir.md) compares direct lowering
with a small value-based IR. Chosen initial IR: an integer definition with a
value ID, then a return of that value, both with source spans. It is an internal
Rust representation, not Garmin MIR. The frontend and backend are independently
written in `ciq-compiler`; no external implementation was incorporated.
Opcode/frame assumptions come from
[our instruction specifications](../specs/instructions.md) and
[the prior arithmetic replacement](function-replacement.md). The latter supports
context-independent arithmetic without the original globals-resolution sequence,
but does not prove the complete calling convention.

Before implementation,
[the restricted grammar and lowering contract](../specs/compiler-subset.md) were
documented. Only a parameterless top-level function returning a decimal integer
in `0..=127` is accepted. Unsupported syntax is rejected with locations. This
range avoids unresolved high-bit immediate semantics; it is not the VM's integer
range. Compiler tests use independently authored inputs and require no Garmin
SDK.

SDK/simulator 9.2.0, `fr265`, scaffold compiled with `-O 0`. Input: tracked
[probe.mc](../../examples/compiler-probe/probe.mc), returning `41`. A local
second source replaces that literal with `42`. The original application provides
lifecycle, resources, class/method metadata, symbol IDs, and container headers;
this experiment does not compile those from scratch.

## Reproduction

Run from the repository root. Every output path must be new:

```sh
mkdir -p scratch/compiler
cargo build -p ciq-compiler -p ciq-dis -p ciq-asm
# Build the SDK scaffold if it is not already present.
sh scripts/build-example.sh examples/hello-world fr265
target/debug/ciq-compiler examples/compiler-probe/probe.mc scratch/compiler/probe-41.mcasm
target/debug/ciq-dis all examples/hello-world/build/fr265/hello-world.prg scratch/compiler/hello-project
```

The compiler output is:

```text
probe:
argc 0x01
ipush1 0x29
return
```

Its body is five bytes: `35 01 38 29 16`. The initial IR is conceptually
`v0 = Integer(41); Return(v0)`; these semantic values are independent of
physical VM slots. The backend emits the prologue and selects integer-push
instructions.

Prepare the changed-source comparison and compile it:

```sh
python3 - <<'PY'
from pathlib import Path
source = Path('examples/compiler-probe/probe.mc').read_text()
Path('scratch/compiler/probe-42.mc').write_text(source.replace('return 41;', 'return 42;'))
PY
target/debug/ciq-compiler scratch/compiler/probe-42.mc scratch/compiler/probe-42.mcasm
```

Explicit scaffold integration (specific to the measured 744-byte CODE and
7777-byte RESOURCES specimen; these assertions deliberately fail for a different
build):

```sh
python3 - <<'PY'
from pathlib import Path
import shutil
root = Path('scratch/compiler')
for value in [41, 42]:
    project = root / f'hello-{value}-project'
    shutil.copytree(root / 'hello-project', project)
    manifest = project / 'sections.ciq'
    text = manifest.read_text()
    assert '0xc0debabe 744 assembly' in text
    assert '0xf00d600d 7777 raw' in text
    code = project / '003-code-c0debabe.mcasm'
    body = code.read_text()
    assert body.count('\nprobe:\n') == 1
    code.write_text(body.replace('\nprobe:\n', '\nprobe_original:\n') + '\n' +
                    (root / f'probe-{value}.mcasm').read_text())
    lines = project / '005-line_numbers-c0de7ab1.mcasm'
    lines.write_text(lines.read_text().replace('pc=code:probe ', 'pc=code:probe_original '))
    resources = project / '007-resources-f00d600d.bin'
    resources.write_bytes(resources.read_bytes() + bytes(27))
    manifest.write_text(text.replace('0xc0debabe 744 assembly', '0xc0debabe 749 assembly')
                           .replace('0xf00d600d 7777 raw', '0xf00d600d 7804 raw'))
PY
target/debug/ciq-asm project scratch/compiler/hello-41-project scratch/compiler/hello-41.prg -y keys/developer_key.der
target/debug/ciq-asm project scratch/compiler/hello-42-project scratch/compiler/hello-42.prg -y keys/developer_key.der
target/debug/ciq-asm verify scratch/compiler/hello-41.prg -y keys/developer_key.der
target/debug/ciq-asm verify scratch/compiler/hello-42.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler/hello-41.prg
sh scripts/run-simulator-container.sh --prg scratch/compiler/hello-42.prg
```

Actual simulator execution used the existing Garmin container through
container-broker with
`xvfb-run -a -s '-screen 0 1280x1024x24' python3 scripts/simulator-session.py --prg <path> --device fr265 --observe-seconds 8`.
Logs/screens were saved under ignored `scratch/compiler/simulator-41/` and
`simulator-42/`. Both captured watch screens were inspected.

## Experimentally observed results

- Both compiled bodies execute through the application's lifecycle and view call
  sites: greeting plus `41`/`42` in the console, and `Hello world!` plus
  `41`/`42` on screen. The changed value distinguishes execution of the
  generated target from the retained SDK original returning `41`.
- Original CODE is an exact 744-byte prefix of both outputs. Generated CODE
  starts at `0x2e8` and grows CODE to 749 bytes. The new `42` suffix is
  `35 01 38 2a 16`.
- DATA's `probe` method value at `0x49f` changes from `0x10000030` to
  `0x100002e8`; every other DATA byte is preserved.
- Existing DEBUG/line metadata remains byte identical because the original probe
  line PC still names `probe_original`. No source map is generated for the
  appended function yet. This keeps the old debug mapping with the old body
  rather than claiming it describes the new one.
- Raw RESOURCES gains 27 trailing zero bytes, giving combined CODE/resource
  growth of 32 bytes and preserving the original signature-payload alignment
  modulo 32. This follows the
  [previous append experiment](code-labels-and-relocation.md), not a universal
  format rule or automatic resource relocation feature.
- All other non-signature sections are byte identical. Both outputs verify
  SHA-1/SHA-256 developer signatures against the supplied external key.
- Empty EXCEPTIONS (`00 00`) and the existing zero header CODE offsets remain
  unchanged. This experiment does not validate branches, exception ranges, or
  nonzero header relocation.
- Exporting/reassembling `hello-42.prg` without signing reproduces it byte for
  byte:

```sh
target/debug/ciq-dis all scratch/compiler/hello-42.prg scratch/compiler/hello-42-roundtrip-project
target/debug/ciq-asm project scratch/compiler/hello-42-roundtrip-project scratch/compiler/hello-42-roundtrip.prg
cmp scratch/compiler/hello-42.prg scratch/compiler/hello-42-roundtrip.prg
```

| Artifact | SHA-256 |
|---|---|
| SDK scaffold | `8608ed040f18294a2a9af870f53e26e64a0d2fe3ef8c032d600043b591d182d7` |
| Generated `probe-41.mcasm` | `557e7f4e88581b07d18e2c149116caf4f4bf3339c7242134f613bde6bb4117a8` |
| Signed generated `41` PRG | `4f68711f23f19333fc0e68e7a9aa492c971980f638046a7e3e16912064eccf10` |
| Signed generated `42` PRG | `3068b5b5f7000479faaf5c8b29e10d9c4d72b54be98b9e6e93414f78bd221bff` |

## Implementation validation and next step

Five new tests cover IR value identity/source spans, every accepted literal
assembled and evaluated by a restricted encoding test, unsupported-input
rejection, Unicode/comment locations, and CLI overwrite/invalid-input behavior.
The encoding test is not a full VM or a substitute for the simulator evidence.
All 116 workspace tests pass with Rust 1.99; formatting and Clippy with warnings
denied pass.

The result supports `argc 0x01` followed by a context-independent integer return
for this global function and its existing call sites. It does not establish
frame setup for arbitrary methods, closures, parameters, or locals. Next extend
the IR/parser/backend to a small arithmetic expression, then locals and calls
using controlled specimens; continue branch/exception investigation before
supporting conditions and changing code layout generally.
