# Compiler integer arithmetic

2026-10-10. Independently implemented Rust frontend/IR/backend extension: binary integer addition/subtraction, left associativity, and parentheses. Both native composition and text resumption work. Official SDK 9.2.0 `fr265` simulator console and inspected screen captures confirm `40 + (8 - 5)` returns `43`, and `2 - 5` returns `-3`.

## Implementation and interpretation

[Source grammar](../specs/compiler-subset.md), [IR syntax](../specs/compiler-ir-text.md), and [layer design](../design/compiler-ir.md) describe our implementation. Instruction assumptions come from [our instruction reference](../specs/instructions.md) and earlier controlled function experiments; no external implementation code was incorporated.

The frontend builds a private expression AST with source spans, then semantic integer/binary definitions and a final return. Value IDs identify computation, not VM slots. Validation requires earlier, unique definitions and exactly one use per value: this is one pure expression tree. Unused/shared definitions are rejected pending a scheduling policy. Valid definition order need not be stack order. `backend::lower()` traverses dependencies from the return value, emits left operand before right operand, and selects VM instructions. `Function::lower()` delegates to it. There is no constant folding or new crate.

Tracked input: [arithmetic.mc](../../examples/compiler-probe/arithmetic.mc). Generated body:

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

CODE bytes: `35 01 38 28 38 08 38 05 04 03 16` (11 bytes). The negative comparison emits `35 01 38 02 38 05 04 16` (8 bytes). Actual arithmetic instructions execute; results are not folded into literals.

Literals remain decimal `0..=127`. Results can be negative or exceed 127. Nesting is limited to 64 parentheses and functions to 1024 operations including return. Unary signs, other operators, locals, calls, multiple statements/functions, and control flow remain unsupported. Runtime evidence establishes these particular integer operations and left-minus-right order; it does not establish general frames, overflow, promotion, or a full VM model.

## Reproduction

Run from the repository root with new output paths. SDK/device files and your external developer key must be available for scaffold/signing/runtime steps:

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

Explicit integration for the measured 744-byte CODE/7777-byte RESOURCES scaffold (assertions fail for a different build):

```sh
python3 - <<'PY'
from pathlib import Path
import shutil
root = Path('scratch/arithmetic')
for name, assembly, size in [('probe', 'probe.mcasm', 11), ('negative', 'negative.mcasm', 8)]:
    project = root / f'{name}-project'
    shutil.copytree(root / 'base-project', project)
    manifest = project / 'sections.ciq'
    text = manifest.read_text()
    assert '0xc0debabe 744 assembly' in text
    assert '0xf00d600d 7777 raw' in text
    code = project / '003-code-c0debabe.mcasm'
    body = code.read_text()
    assert body.count('\nprobe:\n') == 1
    code.write_text(body.replace('\nprobe:\n', '\nprobe_original:\n') + '\n' +
                    (root / assembly).read_text())
    lines = project / '005-line_numbers-c0de7ab1.mcasm'
    lines.write_text(lines.read_text().replace('pc=code:probe ', 'pc=code:probe_original '))
    resources = project / '007-resources-f00d600d.bin'
    padding = 32 - size
    resources.write_bytes(resources.read_bytes() + bytes(padding))
    manifest.write_text(text.replace('0xc0debabe 744 assembly', f'0xc0debabe {744 + size} assembly')
                           .replace('0xf00d600d 7777 raw', f'0xf00d600d {7777 + padding} raw'))
PY
target/debug/ciq-asm project scratch/arithmetic/probe-project scratch/arithmetic/probe-43.prg -y developer_key.der
target/debug/ciq-asm project scratch/arithmetic/negative-project scratch/arithmetic/negative-3.prg -y developer_key.der
target/debug/ciq-asm verify scratch/arithmetic/probe-43.prg -y developer_key.der
target/debug/ciq-asm verify scratch/arithmetic/negative-3.prg -y developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/arithmetic/probe-43.prg
sh scripts/run-simulator-container.sh --prg scratch/arithmetic/negative-3.prg
```

Actual execution reused the Garmin container through container-broker, invoking `xvfb-run -a -s '-screen 0 1280x1024x24' python3 scripts/simulator-session.py --prg <path> --device fr265 --observe-seconds 8`. Fixed output under `scratch/simulator/` was copied after each run to `scratch/arithmetic/simulator-43/` and `simulator-negative-3/`; both screens were inspected. Console greeting and watch display agree on `43` and `-3` respectively.

## Preservation and verification

Original CODE is an exact 744-byte prefix; new function starts at `0x2e8`. DATA's method pointer at `0x49f` changes to `0x100002e8`; all other DATA bytes are preserved. CODE grows to 755/752 bytes. RESOURCES gains 21/24 trailing zeros, keeping combined growth at 32 and retaining signature-payload alignment modulo 32. This is the bounded scaffold technique from [earlier append experiments](code-labels-and-relocation.md), not general resource/metadata relocation.

Only DATA, CODE, RESOURCES, and DEVELOPER_SIGNATURE change. Existing line mappings stay with `probe_original`; the compiled body has no generated debug mapping. Empty EXCEPTIONS and zero header CODE offsets stay unchanged. Both developer signature algorithms verify against the supplied key. Independent section parsing checked these preservation claims. Positive output also exports and reassembles byte identically:

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

Seven new tests cover grouping/associativity, operand pairs and negative/large results, dependency scheduling with reordered definitions, malformed graphs/source diagnostics, resource limits, and CLI source/IR/assembly equivalence. The restricted test interpreter checks instruction results without pretending to model physical frames; simulator evidence remains the runtime oracle. All 129 workspace tests pass on Rust 1.99, plus formatting and workspace/all-target Clippy with warnings denied. Next: local declarations/reads, then verified calls; branch/exception research remains separate.
