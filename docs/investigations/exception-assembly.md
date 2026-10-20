# Structured exception assembly and relocation

2026-10-10. Follow-up to [control-flow research](control-flow.md) and
[branch-label assembly](branch-labels.md). Same SDK 9.2.0, `fr265`, `-O 0`
specimen, developer key, and original device memory allowance. Source revision,
license attribution, SDK source, and original hash are recorded in the research.

## Implementation and evidence

Independent `ciq-prg` now parses/writes ordered exception triples and their
`.exception` source. Section projects resolve named offsets after
CODE layout. Export shares method, line, and branch labels, adding verified
exception boundaries. Raw malformed/duplicate sections and invalid numeric
targets remain preserved. See [the exception specification](../specs/formats/exceptions.md).

**Experimentally observed:** the original control-flow PRG exports/imports byte
identically, including both exception entries and signatures. Exported source:

```text
.exception begin=code:code_0000000f end=code:code_00000034 handler=code:code_0000003a
.exception begin=code:code_0000003a end=code:code_0000004c handler=code:code_0000004c
```

Append a copy of `caught` with new local labels, change its catch constant to
`47`, point its DATA method at the new label, and append the copied exception
declarations with those same renamed labels. Update declared CODE/EXCEPTIONS
lengths to 1099/38 bytes. All offsets then resolve through assembly, without
manually calculating the new range or handler positions.

The signed result matches the independent raw-edit `caught-moved.prg` byte for
byte, SHA-256:

```text
f4216015bcbda64f6c767863c77d805a2f4652d85742705a9baa4c090b70cd6d
```

The simulator prints `41, 43, 6, 41, 47, 49`, and the capture displays `49`.
The distinctive `47` confirms the new handler executed. The original CODE
prefix and unrelated sections are preserved; only appended CODE, one DATA
method pointer, EXCEPTIONS, and the developer signature change. Existing line
metadata remains valid for the retained original CODE; no appended debug mapping
is claimed. Captures/logs are under ignored
`scratch/control-flow/simulator-exception-assembled/`.

## Reproduction

Create the SDK specimen using [the research procedure](control-flow.md#reproduction),
then export and verify unchanged reconstruction with new output paths:

```sh
cargo run -p ciq-dis -- all scratch/control-flow/sdk-probe/build/fr265/sdk-probe.prg scratch/control-flow/exception-project
cargo run -p ciq-asm -- project scratch/control-flow/exception-project scratch/control-flow/exception-roundtrip.prg
cmp scratch/control-flow/sdk-probe/build/fr265/sdk-probe.prg scratch/control-flow/exception-roundtrip.prg
```

Prepare the named copy from that export:

```sh
python3 - <<'PY'
from pathlib import Path
import re, shutil
p = Path('scratch/control-flow/exception-project')
q = Path('scratch/control-flow/exception-moved-project')
shutil.copytree(p, q)
f = q / '003-code-c0debabe.mcasm'
s = f.read_text()
body = s[s.index('caught:\n'):s.index('code_00000052:\n')]
names = {n: 'moved_' + n for n in re.findall(r'^(\w+):$', body, re.M)}
assert all(n in names for n in re.findall(r'code:(\w+)', body))
def rename(t):
    t = re.sub(r'^(\w+):$', lambda m: names.get(m[1], m[1]) + ':',
               t, flags=re.M)
    return re.sub(r'code:(\w+)',
                  lambda m: 'code:' + names.get(m[1], m[1]), t)
body = rename(body)
old = 'ipush1 0x2b # CODE+0x00003f'
assert body.count(old) == 1
body = body.replace(old, 'ipush1 0x2f # CODE+0x00003f')
f.write_text(s + body)
f = q / '002-data-da7ababe.mcasm'
s = f.read_text()
assert s.count('value=code:caught ') == 1
f.write_text(s.replace('value=code:caught ', 'value=code:moved_caught '))
f = q / '009-exceptions-0ece7105.mcasm'
s = f.read_text()
f.write_text(s + rename(s))
f = q / 'sections.ciq'
s = f.read_text().replace('section 0xc0debabe 1021 ',
                         'section 0xc0debabe 1099 ')
s = s.replace('section 0x0ece7105 20 ', 'section 0x0ece7105 38 ')
f.write_text(s)
PY
cargo run -p ciq-asm -- project scratch/control-flow/exception-moved-project scratch/control-flow/exception-assembled-moved.prg -y keys/developer_key.der
cargo run -p ciq-asm -- verify scratch/control-flow/exception-assembled-moved.prg -y keys/developer_key.der
xvfb-run -a -s '-screen 0 1280x1024x24' python3 scripts/simulator-session.py --prg scratch/control-flow/exception-assembled-moved.prg --device fr265 --observe-seconds 10
```

## Validation and limits

Rust tests cover exact framing, field/count limits, order/duplicate preservation,
typed/text equivalence, undefined labels, role-specific end labels, metadata
before CODE, updated named targets after insertion, and malformed/duplicate/raw
fallback. Numeric offsets remain deliberately unchecked against CODE.

This establishes named relocation of the observed exception records and the
whole-function experiment. Endpoint inclusivity, nested/typed handlers,
`finally`, handler stack state, extended CODE, other devices, and opaque header
references remain unresolved. Frontend conditions can now be designed on top of
the shared branch model; exception source compilation is not yet supported.
