# Testing the 24-bit DATA string boundary

2026-10-10. SDK/simulator 9.2.0, `fr265`, Java 17. Question: does `news` actually
reject DATA string offsets beyond `0x00ffffff`?

## Result

**Experimentally observed:** `news 0x01000000` successfully loads a string at
DATA offset `0x01000000`. The simulator prints `High DATA address reached`,
continues printing the other values, and displays the function's return value
`43`. This disproves a universal 24-bit string-offset ceiling for this VM.

The experiment required raising the local device profile's watch-app memory
allowance from 786432 bytes to 67108864 bytes. Its original 768 KiB setting
prevented large programs from reaching the print statements. The profile was
restored byte for byte after the experiment. This is simulator evidence under a
modified memory allowance, not evidence that an actual Forerunner 265 can load
this program.

**Interpretation:** the zero-high-byte assumption in our DATA parser/assembler
is too restrictive. A high-nibble selector with a larger offset field is
consistent with the result and the external research, but the exact selector
boundary and the full supported offset range are still unverified. This test
says nothing about CODE/method-address limits.

## Controls and observations

All probes start from the independently generated, signed `43` program in
[the string compiler experiment](compiler-strings.md). The first generated
`news` is at CODE+`0x330`; its u32 big-endian operand starts at `0x331`.

| Case | DATA bytes | First `news` | Original profile result |
|---|---:|---|---|
| Small control | 13164 | `0x00003332` | Prints normally; displays `43` |
| Small, bit 24 set | 13164 | `0x01003332` | Out-of-bounds error |
| 256 KiB control | 262173 | `0x00003332` | Prints normally; displays `43` |
| 1 MiB control | 1048605 | `0x00003332` | Crashes before first greeting |
| Just below 16 MiB | 16777117 | `0x00003332` | Crashes before first greeting |
| Large control | 16777245 | `0x00003332` | Crashes before first greeting |
| Large high offset | 16777245 | `0x01000000` | Crashes before first greeting |

The small high-byte case reports `Illegal Access (Out of Bounds)` at the first
patched literal load. It cannot establish a 24-bit ceiling: its requested
offset is outside that specimen's DATA payload. It does show that bit 24 is
not simply ignored.

With the watch-app memory setting raised to 64 MiB, the small control and large
control both print normally. The large high-offset probe prints:

```text
Hello from Monkey C!
High DATA address reached
héllo from Rust
42
héllo from Rust
43
```

Subsequent screen updates repeat `probe` and its print statements. The captured
screen displays `Hello world!`, `43`, and memory usage approximately
`16391.0/65531.4kB`. The large control displays the original compiler greeting;
the high-offset case displays the distinct marker, so success cannot be
explained by masking the operand back to a low address.

Initial six-second observations of the large probes ended before execution,
with empty console logs and no watch rendered. Longer observations were
necessary. The final successful runs used 20 seconds; startup crashes were
also checked with longer observation. An active client alone was not counted
as acceptance.

## Construction and reproduction

First reproduce `scratch/compiler-strings/generated-43.prg` using the linked
compiler experiment. Its measured SHA-256 is
`e8f560a993491ab765bab5d9d33a04fdd4a25dc093dca0121782471af76a8bc4`.
The measurements/assertions below deliberately fail for a different scaffold.
Use fresh output paths.

The existing DATA prefix remains unchanged. Padding consists of valid string
records, not an unknown blob or a scan for apparent tags: tag `01`, u16 UTF-8
length, ASCII content, zero terminator. Each filler is at most 65535 bytes of
content. The distinct 25-byte marker has this record at DATA+`0x01000000`:

```text
01 00 19 48 69 67 68 20 44 41 54 41 20 61 64 64 72 65
73 73 20 72 65 61 63 68 65 64 00
```

A separate record walk validates every added record and the final DATA length.
Only DATA growth, one CODE operand, resource padding for signature alignment,
and signing material change. There are no function-size or method-target edits.
Build opaque PRG sections directly to bypass our authoring profile's 24-bit
restriction, then use the normal signing/verification tool:

```sh
mkdir -p scratch/string-address-boundary
python3 - <<'PY'
from pathlib import Path
import struct, subprocess
root = Path('scratch/string-address-boundary')
base = Path('scratch/compiler-strings/generated-43.prg').read_bytes()
sections, offset = [], 0
while True:
    kind, size = struct.unpack_from('>II', base, offset)
    offset += 8
    if not kind:
        assert size == 0 and offset == len(base)
        break
    sections.append((kind, base[offset:offset + size]))
    offset += size
payloads = dict(sections)
DATA, CODE, RES = 0xda7ababe, 0xc0debabe, 0xf00d600d
assert len(payloads[DATA]) == 13164
assert payloads[CODE][0x330:0x335] == bytes.fromhex('1800003332')
def record(text):
    value = text.encode()
    assert len(value) <= 65535
    return b'\x01' + len(value).to_bytes(2, 'big') + value + b'\0'
def enlarged(target):
    data = bytearray(payloads[DATA])
    gap = target - len(data)
    while gap > 65539:
        data.extend(record('p' * 65535))
        gap -= 65539
    assert gap >= 4
    data.extend(record('p' * (gap - 4)))
    assert len(data) == target
    data.extend(record('High DATA address reached'))
    return bytes(data)
large = enlarged(0x01000000)
cases = [
    ('small-control', 0x3332, payloads[DATA]),
    ('small-high-byte', 0x01003332, payloads[DATA]),
    ('medium-control', 0x3332, enlarged(0x40000)),
    ('one-mib-control', 0x3332, enlarged(0x100000)),
    ('below-boundary-control', 0x3332, enlarged(0x00ffff80)),
    ('large-control', 0x3332, large),
    ('large-high-offset', 0x01000000, large),
]
for name, address, data in cases:
    code = bytearray(payloads[CODE])
    code[0x331:0x335] = address.to_bytes(4, 'big')
    growth = len(data) - len(payloads[DATA])
    resources = payloads[RES] + bytes((-growth) % 32)
    replacements = {DATA: data, CODE: bytes(code), RES: resources}
    output = bytearray()
    for kind, old in sections:
        payload = replacements.get(kind, old)
        output.extend(struct.pack('>II', kind, len(payload)))
        output.extend(payload)
    output.extend(bytes(8))
    unsigned, signed = root / f'{name}-unsigned.prg', root / f'{name}.prg'
    unsigned.write_bytes(output)
    subprocess.run(['target/debug/ciq-asm', 'sign', str(unsigned), str(signed),
                    '-y', 'keys/developer_key.der'], check=True)
    subprocess.run(['target/debug/ciq-asm', 'verify', str(signed),
                    '-y', 'keys/developer_key.der'], check=True)
PY
```

Run individual cases with the original profile for controls. For the decisive
pair, run this as a subshell so the profile is restored on exit:

```sh
(
set -eu
profile=scratch/garmin-data/Devices/fr265/compiler.json
backup=scratch/string-address-boundary/compiler.json.original
cp "$profile" "$backup"
trap 'cp "$backup" "$profile"' EXIT
python3 - <<'PY'
from pathlib import Path
import json
p = Path('scratch/garmin-data/Devices/fr265/compiler.json')
value = json.loads(p.read_bytes())
for app in value['appTypes']:
    if app['type'] == 'watchApp':
        app['memoryLimit'] = 64 * 1024 * 1024
p.write_text(json.dumps(value, indent=4) + '\n')
PY
sh scripts/run-simulator-container.sh --prg scratch/string-address-boundary/large-control.prg --observe-seconds 20
cp -r scratch/simulator scratch/string-address-boundary/control-capture
sh scripts/run-simulator-container.sh --prg scratch/string-address-boundary/large-high-offset.prg --observe-seconds 20
cp -r scratch/simulator scratch/string-address-boundary/high-offset-capture
)
```

Actual runs reused the existing Garmin container through container-broker,
with `xvfb-run -a -s '-screen 0 1280x1024x24'` preceding
`python3 scripts/simulator-session.py --prg <path> --device fr265 --observe-seconds 20`.
Logs/captures are retained in `scratch/string-address-boundary/simulator-*/`;
the decisive capture is `simulator-override-large-high-offset/`. The restored
profile was compared byte for byte with its backup.

Signed decisive artifacts:

- Large low-address control:
  `da5c86a097efc89927e033625935b84a55d13afb48e3253502543cc24983ac42`
- Large high-address probe:
  `1f9ded14fcaa3b1b1a23dd22202b7481f7757718d51d245ffc80fec23dbe4800`

## Implementation follow-up

Our authoring checks still reject this proven working offset. Expand DATA
string parsing/layout and named CODE binding together, with boundary tests,
after choosing a supported profile that does not invent an unverified upper
limit. Do not extrapolate this result to all 32-bit addresses or to CODE labels.
