# Structured line metadata and in-place function sizes

Date: 2026-10-09. SDK 9.2.0 / `fr265`, unoptimized debug Hello World and Minimal Console. Follow-up to [shared CODE labels and controlled SDK comparisons](code-labels-and-relocation.md). Both new runtime experiments are prepared; host simulator execution is **pending**.

## Evidence and implementation

The earlier controlled Garmin compiler runs established the local LINE_NUMBERS entry framing and DEBUG string offsets, and showed which method/line PCs move when `probe()` grows. Their source attribution and pinned external references remain in that investigation and [the bootstrap](rust-bootstrap.md). This implementation is independently written Rust using [our metadata specification](../specs/line-numbers-and-debug.md), without external repository dependencies.

Experimentally observed here:

- Both original examples export as version 4 projects with DEBUG strings and line entries as authoritative assembly. Complete reassembly preserves every byte, including signatures.
- Hello World has eight DEBUG strings, 45 line entries, and a 744-byte ordinary CODE payload. Every line PC resolves to a decoded instruction boundary. Every filename/function word resolves to a DEBUG string boundary.
- The exporter shares method labels with DATA and introduces labels for additional line PCs. The importer loads all bindings before assembly, regardless of manifest order.
- Synthetic tests cover label movement, DEBUG string movement, preserved entry order/duplicate PCs, foreign PCs, raw fallback, malformed lengths, undefined references, placement conflicts, and end-of-CODE rejection. All 67 workspace tests pass; Clippy with warnings denied passes.

Original preservation checks:

| Example | PRG bytes | SHA-256, unchanged after full export/import |
|---|---:|---|
| Hello World | 122668 | `8608ed040f18294a2a9af870f53e26e64a0d2fe3ef8c032d600043b591d182d7` |
| Minimal Console | 122188 | `cd361b76dc29e828d43f0511df3bebd96dc63ce6b7be61b70e2be8e1092d95e7` |

## Scope of the in-place experiment

The original Hello World `probe()` occupies CODE `[0x30, 0x3d)`, 13 bytes. Both variants replace that interval, retaining the `probe` entry label at `0x30` and physically moving the later instructions. This extends the previous append-only experiment, which left existing instructions in place.

The specimen has fully decoded branch-free ordinary CODE, an empty EXCEPTIONS count (`0000`), and a 33-byte HEADER_VERSIONED containing `00060002` followed by 29 zero bytes. Its background/glance offsets are zero. This bounds the experiment; no generic relocation operation or handling of nonempty exceptions, nonzero header offsets, extended code, or bundles is implemented.

Grow to 24 bytes, computing `40 + 4` through local slot 1; expected result **44**:

```text
probe:
argcincsp 0x01 0x01
spush @globals
getm
lputv 0x00
ipush1 0x28
lputv 0x01
code_0000003a:
lgetv 0x01
ipush1 0x04
addv
dup 0x00
popv
return
```

Shrink to 5 bytes, returning **45**:

```text
probe:
argc 0x01
code_0000003a:
ipush1 0x2d
return
```

The original source-line entry for the return expression points to `code_0000003a`. Retain that alias deliberately at new CODE `0x3f` for the grown computation and `0x32` for the shrunk literal. Its spelling is a source identity, not a placement constraint. The entry label stays fixed. Existing later labels move by +11 or -8. Source line numbers and file/function identities stay unchanged; this policy maps the edited expression to the original source line and does not claim the original Monkey C text describes the new instructions.

Keep every DATA/DEBUG `at=` constraint. No ignore-placement option is introduced. Explicitly update the manifest CODE length to 755 or 736. As a local experiment precaution, append 21 or 8 zero bytes to RESOURCES so the developer signature payload retains the SDK's observed 32-byte file alignment. Its role is still an interpretation of the compiler-output pattern, **not an established general PRG validity requirement**. Do not normalize arbitrary opaque resource tails or infer general padding rules from this specimen.

## Reproduction

Build the original if necessary with `sh scripts/build-example.sh examples/hello-world`. Use fresh output paths: the tools refuse existing export directories and PRGs.

```sh
mkdir -p scratch/line-metadata
cargo run -p ciq-dis -- all examples/hello-world/build/fr265/hello-world.prg scratch/line-metadata/hello-project
cargo run -p ciq-asm -- project scratch/line-metadata/hello-project scratch/line-metadata/hello-roundtrip.prg
cargo run -p ciq-dis -- all examples/minimal-console/build/fr265/minimal-console.prg scratch/line-metadata/minimal-project
cargo run -p ciq-asm -- project scratch/line-metadata/minimal-project scratch/line-metadata/minimal-roundtrip.prg
```

Copy `hello-project` to `hello-inplace-grown` and `hello-inplace-shrunk` under the same scratch directory. In `003-code-c0debabe.mcasm`, replace everything from `probe:` through the instruction immediately before `code_0000003d:` with the corresponding body above. Retain `code_0000003d:` and all following instructions and labels. In `sections.ciq`, change only the CODE payload length (744 → 755/736) and RESOURCES length (7777 → 7798/7785). Append the corresponding zeros to the resource payload, for example:

```sh
python3 - <<'PY'
from pathlib import Path
for name, count in [('grown', 21), ('shrunk', 8)]:
    path = Path(f'scratch/line-metadata/hello-inplace-{name}/007-resources-f00d600d.bin')
    original = path.read_bytes()
    assert len(original) == 7777  # refuse to pad twice
    path.write_bytes(original + bytes(count))
PY
cargo run -p ciq-asm -- project scratch/line-metadata/hello-inplace-grown scratch/line-metadata/hello-inplace-grown.prg -y developer_key.der
cargo run -p ciq-asm -- project scratch/line-metadata/hello-inplace-shrunk scratch/line-metadata/hello-inplace-shrunk.prg -y developer_key.der
cargo run -p ciq-asm -- verify scratch/line-metadata/hello-inplace-grown.prg -y developer_key.der
cargo run -p ciq-asm -- verify scratch/line-metadata/hello-inplace-shrunk.prg -y developer_key.der
```

Method and line targets follow CODE labels automatically; no manual address-word editing is needed. The original signature file remains preservation input; `-y` replaces it after reconstruction. The private key remains outside the export and Git.

## Checks and prepared artifacts

Independent payload comparisons against the original assert:

- Same section identities and order; every section outside CODE, DATA, LINE_NUMBERS, RESOURCES, and DEVELOPER_SIGNATURE is byte identical.
- CODE prefix before `0x30` is unchanged. Original suffix starting at `0x3d` is byte identical at new `0x48` (grown) or `0x35` (shrunk).
- DATA changes are exactly 25 ordinary method address words originally at/after `0x3d`, each by +11 or -8. `probe` stays at `0x30`; all other DATA bytes, including class layout and strings, are unchanged.
- All 45 line entries retain order, file/function offsets, and source line numbers. Exactly 41 PCs change: the expression mapping described above plus 40 entries after the old function. Earlier PCs are unchanged.
- RESOURCES is the complete original payload followed by the chosen zeros. Both signature payload offsets remain divisible by 32 (121152 grown, 121120 shrunk).
- SHA-1 and SHA-256 RSA signatures verify against the supplied developer key. An independent Python public-RSA calculation also validates both digest signatures; the embedded public key matches the original local-key build.
- Each signed variant completes another byte-identical full export/import.

| Variant | PRG bytes | CODE bytes | SHA-256 |
|---|---:|---:|---|
| `scratch/line-metadata/hello-inplace-grown.prg` | 122700 | 755 | `7588f12c1e54ff964144c85b8bb18c78dc84cd003aeec1e93786ce737ffb32b9` |
| `scratch/line-metadata/hello-inplace-shrunk.prg` | 122668 | 736 | `afd1ef9716082fc664680b4bdff572219420c434f01895a3dac815d78e5ec657` |

These hashes depend on the original build and local key; a different key legitimately changes signing material. Generated exports, comparison JSON, and PRGs remain ignored under `scratch/line-metadata/`.

## Next runtime check

Podman is unavailable in the agent container. Run on the host, checking each capture before the next run overwrites it:

```sh
sh scripts/run-simulator-container.sh --prg scratch/line-metadata/hello-inplace-grown.prg
# Expected: greeting and 44 in both console and watch screen.
sh scripts/run-simulator-container.sh --prg scratch/line-metadata/hello-inplace-shrunk.prg
# Expected: greeting and 45 in both console and watch screen.
```

Current saved captures are from the previously verified appended function returning 43. They are not evidence for these new variants. Record fresh logs/screens and PRG hashes after execution. Next investigate more complex multi-file source and barrel/bundle specimens before choosing placement relaxation, automatic layout, or a separate linker design.
