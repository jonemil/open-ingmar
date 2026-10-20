# Shared code labels and controlled function-size experiments

Date: 2026-10-08. SDK 9.2.0, `fr265`, debug builds, `-O 0`. Follow-up to
[named classes](named-classes.md) and
[the arithmetic replacement](function-replacement.md). The appended larger-body
experiment was successfully executed on the host on 2026-10-09; fresh console
and screen captures were inspected by the agent.

## Shared labels

Independent Rust implements [CODE labels](../specs/assembly/README.md#code-labels-and-fixups) and
`value=code:label` method references. Labels share existing symbol/class/string
naming conventions but a separate namespace. Full export replaces verified
method targets with label references and emits the matching label before its
CODE instruction. Project import resolves labels before DATA, regardless of
manifest order. Original Hello World and Minimal Console reconstruct byte
identically, including signatures, after this change.

Artifacts: `scratch/relocation/hello-labels/`, `minimal-labels/`, and matching
reconstructed PRGs. Validation: 61 Rust tests pass; Clippy passes with warnings
denied. Tests include zero-byte labels and instruction alignment,
duplicate/malformed labels, code-reference typing/24-bit overflow, method
targets at end-of-CODE, and export/import across DATA-before-CODE order.
Existing numeric sources remain supported.

## Controlled official compiler comparison

Copied the independently authored Hello World manifest, resources, jungle, and
source into ignored `scratch/relocation/sdk-variant/specimen/`. All three
compiler runs use that same directory and source filename, our local developer
key, and `sh scripts/build-example.sh scratch/relocation/sdk-variant/specimen`.
Output is `specimen/build/fr265/specimen.prg`; each run's PRG and `.debug.xml`
were saved before the next run. No tracked example source was edited.

Variants of `probe()`:

```text
// baseline
function probe() { return 41; }

// grown: actual source uses one statement per line
function probe() {
    var result = 40;
    return result + 2;
}

// shrunk: same line layout as baseline
function probe() { return 42; }
```

In the actual source, baseline/shrunk retain the existing multiline function
layout, replacing only the original `return 41;` line. Grown replaces that line
with the two lines shown. The function begins at CODE `0x30` in all variants.

- **Variant:** Baseline. **PRG bytes:** 122700. **CODE bytes:** 744. **Function
  bytes:** 13. **LINE entries:** 45. **RESOURCES bytes:** 7771. **SHA-256:**
  `58153070dba12d0e09790e750fb6992cb296b351dfd3833252cb4c99b15a2d09`.

- **Variant:** Grown. **PRG bytes:** 122732. **CODE bytes:** 752. **Function
  bytes:** 21. **LINE entries:** 46. **RESOURCES bytes:** 7779. **SHA-256:**
  `40a1eb766ff3063d486dbb1e1685340b7bccac610f002ff707755e46cec586ae`.

- **Variant:** Shrunk. **PRG bytes:** 122700. **CODE bytes:** 744. **Function
  bytes:** 13. **LINE entries:** 45. **RESOURCES bytes:** 7771. **SHA-256:**
  `cd099d437bd8a0372cec245147fc3b6c60a7222f7dd3f14187c37c504a3f1848`.

**Measured:** Grown CODE prefix before `0x30` equals baseline; suffix from grown
`0x45` equals baseline from `0x3d`. All 25 ordinary-CODE method targets at or
after the old next-function boundary move by +8. Earlier targets do not move.
Rewriting exactly those type-6 values completely explains all DATA differences;
DATA length remains 13066 bytes. Shrinking restores DATA exactly to baseline.

LINE_NUMBERS gains one entry and updates code PCs and source lines. Shrinking
restores its exact baseline payload. DEBUG stays byte identical in all three
variants. Its entire 263 bytes decode as eight string records; every line-table
filename and function-name field points to a DEBUG record boundary. Controlled
`probe` entries in baseline use filename offset 107 and function-name offset
254, agreeing with the decoded source path and `probe` string. These are DEBUG
offsets, not SYMBOLS IDs.
[Local metadata specification](../specs/formats/metadata.md#debug-and-line_numbers) records this
distinction.

HEADER_VERSIONED, ENTRY_POINTS, IMPORTS, PERMISSIONS, EXCEPTIONS, SYMBOLS, and
SETTINGS remain byte identical. Both header code offsets are zero and exceptions
are empty, so this does not verify their nonzero/nonempty relocation semantics.
PRG signatures change as expected.

Grown RESOURCES equals the entire baseline payload plus eight zero bytes.
Existing indexed resource content/offsets do not change. Shrinking restores the
exact original payload. All three signature payloads start on a 32-byte
boundary. **Interpretation:** the trailing zeros are alignment padding
compensating for the changed combined section lengths. This is an observed
pattern, not a universal signing or resource-format requirement. Existing
research's unaligned writer is not proof that every Garmin loader accepts
unaligned output.

Ignored `scratch/relocation/sdk-variant/compare.py` independently parses section
wrappers and v2 method fields; `comparison.json`, snapshots, and compiler logs
retain the local experiment. Format source attribution remains pinned in
[the class assembly investigation](class-assembly.md); no external
implementation was copied.

## Validated larger-body method relocation

To test a larger function before shifting existing line PCs, append a new body
to the original Hello World CODE and redirect its method field with the shared
label. This is an explicit bounded experiment, not automatic general relocation.

Copied `hello-labels/` to `hello-grown/`; renamed the original `probe:` label to
`probe_original:` and appended:

```text
probe:
argcincsp 0x01 0x01
spush @globals
getm
lputv 0x00
ipush1 0x28
lputv 0x01
lgetv 0x01
ipush1 0x03
addv
dup 0x00
popv
return
```

This 24-byte body computes 40 + 3 = 43, using the same prologue/local/global
pattern as the controlled SDK's grown body, plus a balanced duplicate/pop. CODE
grows from 744 to 768 bytes. The unchanged DATA source still says
`value=code:probe`; assembly now resolves it to `0x100002e8`. The old 13-byte
body remains present and unreferenced by its method field; existing code offsets
and their line mappings remain unchanged. There is no debug mapping for the new
appended body.

Explicit experiment manifest edits: CODE length 744 → 768; RESOURCES length 7777
→ 7785. Appended eight zero bytes to the original raw RESOURCES file, preserving
the SDK-observed 32-byte signature-payload alignment after the 24-byte CODE
growth. No other section source or payload changed. This padding choice remains
part of the runtime experiment.

```sh
cargo run -p ciq-asm -- project scratch/relocation/hello-grown scratch/relocation/hello-grown.prg -y keys/developer_key.der
cargo run -p ciq-asm -- verify scratch/relocation/hello-grown.prg -y keys/developer_key.der
```

Measured output: 13 sections, 122700 bytes, SHA-256
`8490d03cfe3075ba89caf9e8013a0faf87feb814706ddb7f27703d4dcbe5569c`. The complete
old CODE is an exact prefix; the only DATA change is the `probe` value at DATA
`0x49f`. RESOURCES is the original payload plus eight zeros. Other non-signature
payloads remain identical; key fields remain identical. Both signatures verify
independently in Rust and Python public-RSA checks. Full export/import of this
output reconstructs it byte identically (`hello-grown-disassembled/` and
`hello-grown-roundtrip.prg`).

Run on the host (Podman is unavailable in the agent container):

```sh
sh scripts/run-simulator-container.sh --prg scratch/relocation/hello-grown.prg
```

Host result, 2026-10-09: the user reported success. Fresh captures updated at
19:53 show console `Hello from Monkey C!` and `43`, and the agent visually
confirmed `Hello world!` and `43` on the watch screen. The PRG hash still
matches the output above. Console SHA-256:
`eae7e6208c3b17ff50600fa4759cfd1a35bd5f207f9844c0580d2718923f046f`; screen
SHA-256: `373154adacb17b10405bcf9b56db99824f39bf2b367d86bbcc11528e84db970d`.
Captures remain ignored under `scratch/simulator/`.

The value `43` distinguishes execution of the new appended target from the
retained original `41` body and prior `42` experiments. This validates the
24-byte function, its label-resolved method binding at CODE `0x2e8`, and the
chosen packaging/padding for this artifact. It does not establish general
in-place relocation or a universal alignment requirement.

## Remaining relocation work

Shared labels relocate method bindings, but general in-place insertion/deletion
still needs line-table serialization with an explicit policy for edited PCs,
nonempty exception interval evidence, nonzero header code-offset evidence, and
unsupported sections/control-flow handling. Do not drop raw metadata or relax
manifest checks globally. The next production step is structured DEBUG/line
metadata tied to code labels, followed by a restricted relocation mode that
rejects unknown affected formats.
