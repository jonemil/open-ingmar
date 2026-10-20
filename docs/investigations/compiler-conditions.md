# Compiler terminal conditions

2026-10-10. Question: can semantic blocks and comparisons preserve native/text
pipeline equivalence and execute both return arms in the official simulator?
SDK 9.2.0, Java 17, `fr265`, original device memory allowance, our developer key.
No new external research was consulted; this uses our documented
[branch encodings](branch-labels.md) and [SDK scaffold](control-flow.md).

## Implementation and boundaries

The [checked-in source](../../examples/compiler-probe/conditions.mc) initializes
`adjusted = value + 1`, then compares `adjusted == 1`, returning `47` on true
and `49` on false. A second input changes only `==` to `>`.

Current graph/type/initialization rules are in the
[IR spec](../specs/ir/control-flow.md#ir-control-flow); this experiment tests the
terminal two-return input above.

**Experimentally observed:** each generated CODE body is 27 bytes. Source-native
and saved-IR encoding are identical. Equality CODE is:

```text
4a020112013801031302120238011a290006190000382f16383116
```

Greater-than changes only `eq` (`1a`) to `gt` (`1d`). The backend emits `bf` to
the false block and `goto` to the true block. No fallthrough optimization is
performed. Rust tests also compare saved assembly encoding, validate typed IR
reconstruction, reject invalid graphs/types/schedules, and exercise forward
joins. Their bounded integer/Boolean instruction oracle checks both outcomes
and empty return stacks; it is not a full VM compatibility test.

## Integration and observations

Start from the unchanged control-flow SDK PRG, whose CODE is 1021 bytes. Export
its sections, rename the original `choose` label to `sdk_choose`, preserve its
line references under that name, and append the generated body at CODE+`0x3fd`.
The existing DATA method reference `code:choose` then selects the new body.
Append five zero bytes to RESOURCES so the total growth is 32 bytes, preserving
the observed developer-signature alignment. Assemble and sign with our key.

**Verified preservation:** original CODE prefix is unchanged; its appended
suffix exactly matches independently emitted native/saved-IR CODE. DATA differs
only in the method word at `0x4a8..0x4ac`, from `0x100000a7` to `0x100003fd`.
RESOURCES retains its full original prefix plus five zeros. All other section
payloads, including EXCEPTIONS, DEBUG, and LINE_NUMBERS, are unchanged except
the regenerated developer signature. Signature verification succeeds.

**Simulator observations:**

| Comparison | `choose(0)` | `choose(1)` | Screen / `probe()` |
|---|---|---|---|
| `adjusted == 1` | 47 | 49 | 55 |
| `adjusted > 1` | 49 | 47 | 53 |

The remaining console results stay `6`, `41`, and `43` for `countdown(3)` and
the two `caught` calls. `probe()` adds `choose(1)` to `countdown(3)`. Both console
and captured display were inspected. Captures/logs are under ignored
`scratch/compiler-conditions/simulator-equal/` and `simulator-greater/`.

SHA-256 for these exact specimen/key outputs:

```text
original ecfeda07cc22a07d048e934596b4cfca0868d6965b4bc3605849e807bc186285
equality 057b9376009bfdfa8ac7a163b6c1b70e47b2b38f1de1ad80504ace37256e28fb
greater  2484bd21f5d93bed97ed0c963b9f29b0a332419987d8adcf98a8a1c0c9f1fe2e
```

## Reproduction

Build the original scaffold using the [control-flow instructions][scaffold].
Commands require new output paths; change names for repeated experiments.

```sh
mkdir -p scratch/compiler-conditions
cargo run -p ciq-compiler -- examples/compiler-probe/conditions.mc scratch/compiler-conditions/conditions.mcasm
cargo run -p ciq-compiler -- --emit ir examples/compiler-probe/conditions.mc scratch/compiler-conditions/conditions.ciqir
cargo run -p ciq-compiler -- --emit code examples/compiler-probe/conditions.mc scratch/compiler-conditions/native.bin
cargo run -p ciq-compiler -- --from ir --emit code scratch/compiler-conditions/conditions.ciqir scratch/compiler-conditions/ir.bin
cmp scratch/compiler-conditions/native.bin scratch/compiler-conditions/ir.bin
cargo run -p ciq-dis -- all scratch/control-flow/sdk-probe/build/fr265/sdk-probe.prg scratch/compiler-conditions/project
```

Apply the bounded integration to this specific export:

```sh
python3 - <<'PY'
from pathlib import Path
import re
p = Path('scratch/compiler-conditions/project')
code = p / '003-code-c0debabe.mcasm'
s = code.read_text()
assert s.count('\nchoose:\n') == 1
s = s.replace('\nchoose:\n', '\nsdk_choose:\n')
s += '\n' + Path('scratch/compiler-conditions/conditions.mcasm').read_text()
code.write_text(s)
lines = p / '005-line_numbers-c0de7ab1.mcasm'
lines.write_text(re.sub(r'\bcode:choose\b', 'code:sdk_choose', lines.read_text()))
resources = p / '007-resources-f00d600d.bin'
assert resources.stat().st_size == 7779
resources.write_bytes(resources.read_bytes() + bytes(5))
manifest = p / 'sections.ciq'
s = manifest.read_text()
assert 'section 0xc0debabe 1021 ' in s
assert 'section 0xf00d600d 7779 ' in s
s = s.replace('section 0xc0debabe 1021 ', 'section 0xc0debabe 1048 ')
s = s.replace('section 0xf00d600d 7779 ', 'section 0xf00d600d 7784 ')
manifest.write_text(s)
PY
cargo run -p ciq-asm -- project scratch/compiler-conditions/project scratch/compiler-conditions/generated-equal.prg -y keys/developer_key.der
cargo run -p ciq-asm -- verify scratch/compiler-conditions/generated-equal.prg -y keys/developer_key.der
xvfb-run -a -s '-screen 0 1280x1024x24' python3 scripts/simulator-session.py --prg scratch/compiler-conditions/generated-equal.prg --device fr265 --observe-seconds 10
```

For greater-than, copy the source to an ignored path and change `==` to `>`.
Repeat compilation and integration in fresh output paths against the original
SDK PRG; section lengths and method offsets remain the same. Preserve each
simulator capture before the next session overwrites `scratch/simulator/`.

These runs establish this bounded integer condition/frame profile on one
device. Effectful branch arms are covered structurally in Rust tests, not by
this simulator experiment. Other source forms require their own specimens; current language support is
specified separately.

[source]: ../specs/frontend/statements.md#terminal-conditions
[scaffold]: control-flow.md#reproduction
