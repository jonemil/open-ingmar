# Compiler initialized local variables

2026-10-10. Question: can the independent frontend preserve initialized locals through repeated reads, while keeping semantic local identities separate from VM slots? Result: SDK slot comparison, native/text pipeline equivalence, own-key signed PRG integration, and official simulator execution are verified.

## Scope and evidence

Implementation is independent Rust in `ciq-compiler`, against [our instruction reference](../specs/instructions.md), [the earlier local-slot experiment](function-replacement.md), and [compiler arithmetic](compiler-arithmetic.md). No reference-project implementation was incorporated. SDK/compiler/simulator: 9.2.0; target `fr265`; compiler optimization `-O 0`.

Tracked source [locals.mc](../../examples/compiler-probe/locals.mc):

```monkeyc
function probe() {
    var first = 40 + 2;
    var second = first - 5;
    return first + second - first;
}
```

**Experimentally observed:** a copy of SDK Hello World with this function compiles to CODE `[0x30, 0x52)`. It starts with `argcincsp 0x01 0x02`, resolves globals into slot 0, initializes `first` with `lputv 0x01`, initializes `second` with `lputv 0x02`, and returns through repeated loads. The arithmetic/store/read sequence matches our generated function after the SDK globals-resolution sequence. This extends prior one-local evidence to two locals. Source name locations are not assumed to be VM symbol IDs.

Our generated function is 26 bytes:

```text
probe:
argcincsp 0x01 0x02
ipush1 0x28
ipush1 0x02
addv
lputv 0x01
lgetv 0x01
ipush1 0x05
subv
lputv 0x02
lgetv 0x01
lgetv 0x02
addv
lgetv 0x01
subv
return
```

Exact bytes: `4a 01 02 38 28 38 02 03 13 01 12 01 38 05 04 13 02 12 01 12 02 03 12 01 04 16`.

**Experimentally observed:** console prints greeting plus `37`; inspected screen displays `Hello world!` and `37`. A separate source with `var unused = 1; var first = 2 - 5; var second = first + first; return second;` emits a 24-byte function with `argcincsp 0x01 0x03`, stores slots 1/2/3, and reads the negative slot twice. Console and inspected screen show `-6`. Both functions are reached through the original application lifecycle/view call sites. These results distinguish execution from the preserved SDK original returning `41`.

**Interpretation:** allocating extra local slots after the one context slot supports these context-independent parameterless bodies. Locals retain initializer values across repeated reads. This does not establish general frame ownership, parameters, nested scopes, closures, overflow, or device runtime acceptance of all 255 structurally encodable locals.

## Representation and limits

The [frontend specification](../specs/compiler-subset.md) permits initialized `var` declarations before a final return, with earlier-local reads in arithmetic expressions. Names are resolved in declaration order. Self/forward/unknown reads and duplicate declarations produce source diagnostics. Reassignment, uninitialized declarations, comma declarations, nested scopes, calls, and other statements remain rejected.

Semantic `LocalId` is independent of `ValueId` and physical slots. `LocalInitialize` consumes an expression value; `LocalRead` defines a fresh value. Repeated reads share a local identity, not an expression definition. Each expression value still has one use. Initializations are statement sinks, executed in order, including unused locals. Backend slot allocation follows initialization order, regardless of numeric semantic IDs. With no locals, previous arithmetic/literal output is unchanged. The current scheduling policy relies on pure expressions and absence of reassignment; effectful operations require explicit ordering before extension.

[Provisional version-1 IR text](../specs/compiler-ir-text.md) adds `.local $<local-id> %<value-id>` and `.read %<result-id> $<local-id>`. Both use shared validation and optional source spans. No new crate, signing dependency, or application metadata generation is introduced. Limits: 255 locals, 1024 total operations including initialization/read/return, and 64 parenthesis levels.

## Reproduction

Run from the repository root with new output paths. SDK/device data and the external developer key are needed for SDK/signing/runtime stages.

```sh
mkdir -p scratch/compiler-locals
cargo build -p ciq-compiler -p ciq-dis -p ciq-asm
python3 - <<'PY'
from pathlib import Path
import shutil
root = Path('scratch/compiler-locals/sdk-example')
shutil.copytree('examples/hello-world', root, ignore=shutil.ignore_patterns('build'))
p = root / 'source/HelloWorld.mc'
s = p.read_text()
assert s.count('    return 41;') == 1
p.write_text(s.replace('    return 41;', '    var first = 40 + 2;\n    var second = first - 5;\n    return first + second - first;'))
PY
sh scripts/build-example.sh scratch/compiler-locals/sdk-example fr265
target/debug/ciq-dis all scratch/compiler-locals/sdk-example/build/fr265/sdk-example.prg scratch/compiler-locals/sdk-project
# Inspect probe in sdk-project/003-code-c0debabe.mcasm.
target/debug/ciq-compiler --emit ir examples/compiler-probe/locals.mc scratch/compiler-locals/probe.ciqir
target/debug/ciq-compiler --from ir scratch/compiler-locals/probe.ciqir scratch/compiler-locals/probe.mcasm
target/debug/ciq-compiler --emit code examples/compiler-probe/locals.mc scratch/compiler-locals/native.bin
target/debug/ciq-compiler --from assembly --emit code scratch/compiler-locals/probe.mcasm scratch/compiler-locals/text.bin
cmp scratch/compiler-locals/native.bin scratch/compiler-locals/text.bin
python3 - <<'PY'
from pathlib import Path
Path('scratch/compiler-locals/negative.mc').write_text('function probe() { var unused = 1; var first = 2 - 5; var second = first + first; return second; }\n')
PY
target/debug/ciq-compiler scratch/compiler-locals/negative.mc scratch/compiler-locals/negative.mcasm
target/debug/ciq-compiler --emit code scratch/compiler-locals/negative.mc scratch/compiler-locals/negative.bin
# Use the unchanged SDK scaffold for appended-body experiments.
sh scripts/build-example.sh examples/hello-world fr265
target/debug/ciq-dis all examples/hello-world/build/fr265/hello-world.prg scratch/compiler-locals/base-project
```

Specific integration for the measured 744-byte CODE/7777-byte RESOURCES scaffold (assertions deliberately reject other builds):

```sh
python3 - <<'PY'
from pathlib import Path
import shutil
root = Path('scratch/compiler-locals')
for name in ['probe', 'negative']:
    project = root / f'{name}-project'
    shutil.copytree(root / 'base-project', project)
    size = len((root / ('native.bin' if name == 'probe' else 'negative.bin')).read_bytes())
    padding = (-size) % 32
    manifest = project / 'sections.ciq'
    text = manifest.read_text()
    assert '0xc0debabe 744 assembly' in text and '0xf00d600d 7777 raw' in text
    code = project / '003-code-c0debabe.mcasm'
    body = code.read_text()
    assert body.count('\nprobe:\n') == 1
    code.write_text(body.replace('\nprobe:\n', '\nprobe_original:\n') + '\n' +
                    (root / f'{name}.mcasm').read_text())
    lines = project / '005-line_numbers-c0de7ab1.mcasm'
    lines.write_text(lines.read_text().replace('pc=code:probe ', 'pc=code:probe_original '))
    resources = project / '007-resources-f00d600d.bin'
    resources.write_bytes(resources.read_bytes() + bytes(padding))
    manifest.write_text(text.replace('0xc0debabe 744 assembly', f'0xc0debabe {744 + size} assembly')
                           .replace('0xf00d600d 7777 raw', f'0xf00d600d {7777 + padding} raw'))
PY
target/debug/ciq-asm project scratch/compiler-locals/probe-project scratch/compiler-locals/probe-37.prg -y developer_key.der
target/debug/ciq-asm project scratch/compiler-locals/negative-project scratch/compiler-locals/negative-6.prg -y developer_key.der
target/debug/ciq-asm verify scratch/compiler-locals/probe-37.prg -y developer_key.der
target/debug/ciq-asm verify scratch/compiler-locals/negative-6.prg -y developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-locals/probe-37.prg
sh scripts/run-simulator-container.sh --prg scratch/compiler-locals/negative-6.prg
```

Actual integration reused `scratch/arithmetic/base-project`, the identical baseline export. Runtime reused the existing Garmin container through container-broker with `xvfb-run -a -s '-screen 0 1280x1024x24' python3 scripts/simulator-session.py --prg <path> --device fr265 --observe-seconds 8`. Capture/logs from `scratch/simulator/` were copied after each run to `scratch/compiler-locals/simulator-37/` and `simulator-negative-6/`; both screen images were inspected.

## Preservation and validation

Independent section-wrapper parsing confirms only DATA, CODE, RESOURCES, and DEVELOPER_SIGNATURE change. Original CODE is an exact 744-byte prefix, followed by generated bytes at `0x2e8`. DATA's method value at `0x49f` becomes `0x100002e8`; other DATA bytes are unchanged. CODE grows to 770/768 bytes and raw RESOURCES gains 6/8 trailing zeros, preserving combined growth of 32 and signature-payload alignment modulo 32. This remains a bounded scaffold experiment, not a general relocation feature.

Existing debug/line mappings continue to name the original body. The generated body has no new debug mapping. Empty EXCEPTIONS and zero header CODE offsets remain unchanged. Both PRGs verify SHA-1/SHA-256 developer signatures against the supplied key. The positive output round-trips byte identically:

```sh
target/debug/ciq-dis all scratch/compiler-locals/probe-37.prg scratch/compiler-locals/roundtrip-project
target/debug/ciq-asm project scratch/compiler-locals/roundtrip-project scratch/compiler-locals/roundtrip.prg
cmp scratch/compiler-locals/probe-37.prg scratch/compiler-locals/roundtrip.prg
```

| Artifact | SHA-256 |
|---|---|
| Unchanged SDK scaffold | `8608ed040f18294a2a9af870f53e26e64a0d2fe3ef8c032d600043b591d182d7` |
| SDK two-local specimen | `3e1f727575ea1328ff9bc8a85d69150f21b5accf066476ffc8fb11580ed4f9b3` |
| Native two-local CODE | `b7919cd9a322b27eab2e8d89684a951ce6d1da358bba87ce748204ad554a4002` |
| Native three-local CODE | `6ef3f37a12ee4f2da120a70812784ac6db0a68a54f55636d8e3fec96e62886ab` |
| Signed `37` PRG | `94b39bf332b56568031c87413ee8ef06a17871a1d9c408d0726f2e1d3692e3ea` |
| Signed `-6` PRG | `e74030888db75bf74bf97502bc0c83222caa990327404cac199180daeed521be` |

Seven new tests cover initialization/read semantics and all pipeline paths, source spans, invalid names/reads/declarations, shared text/typed validation, noncontiguous semantic IDs versus slots, source/IR local and total-operation limits, and CLI output/diagnostics. The restricted instruction evaluator checks stack balance and store-before-read within this subset, not physical frames. All 136 workspace tests pass on Rust 1.99; formatting and workspace/all-target Clippy with warnings denied pass. Next: controlled call/frame specimens and a first supported function-call form. Reassignment needs its own semantic ordering extension.
