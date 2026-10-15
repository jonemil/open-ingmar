# Own-key IQ creation and edited program execution

Date: 2026-10-10. Scope: stage 5 of
[the package plan](../specs/package-reading.md), limited to the documented modern
developer profile. Implementation follows our
[creation rules](../specs/iq-creation.md), using the existing signing crate and
7z writer. No dependencies changed. Garmin SDK/simulator 9.2.0 and device
`fr265` supply the reference package and runtime oracle.

## Implementation and tests

A Rust adapter stages one signed PRG at a time and streams unknown ordinary
payloads. It patches just product signing attributes in the raw manifest,
preserving other lexical XML bytes, then signs the exact final bytes.
Namespace-aware selection uses the existing pinned `quick-xml 0.41.0`; attribute
editing is independently implemented. No external research implementation was
copied. Archive/signing provenance remains in [repacking](iq-repacking.md) and
[verification](package-verification.md).

Four new tests cover lexical XML preservation (including quotes, escapes,
foreign product elements, comments, unknown data, nonempty product tags, and
insertion of missing attributes), replacement with a different ephemeral RSA
key, unsigned edited products, shared product paths, source preservation, debug
omission/preservation, missing metadata, malformed PRGs, and staging cleanup.
99; formatting and Clippy
(`--workspace --all-targets -- -D warnings`) pass. Rust 1.85 compatibility of
this change was not rerun; earlier baseline evidence is historical. Real private
keys are never embedded in fixtures.

## Real edited package

Start from the SDK IQ tree `scratch/package-reader/iq/`, with provenance in
[the package format investigation](package-formats.md). Disassemble its device
PRG's CODE into `scratch/iq-creation/code.mcasm`. The `Numbers.answer()` body
contains `ipush1 0x28`, then `ipush1 0x06` at CODE offset `0x3c`, followed by
`addv`. Change only that second operand to `0x07`, making the result 47.

Copy the extracted tree to `scratch/iq-creation/edited-tree/`. Preserve its
original PRG separately, then replace the tree's PRG through:

```sh
ciq-asm scratch/iq-creation/original.prg scratch/iq-creation/code.mcasm scratch/iq-creation/edited-tree/006-B4257-00/barrel-app.prg
ciq-package pack-iq scratch/iq-creation/edited-tree scratch/iq-creation/edited-47.iq -y keys/developer_key.der --debug omit
ciq-package pack-iq scratch/iq-creation/edited-tree scratch/iq-creation/edited-47-preserve.iq -y keys/developer_key.der --debug preserve
ciq-package verify scratch/iq-creation/edited-47.iq --expected-key scratch/package-reader/iq/dev_key.pub
ciq-package unpack scratch/iq-creation/edited-47.iq scratch/iq-creation/extracted-47
```

The assembler intentionally retains stale original signatures; IQ creation
replaces them and updates product/manifest signatures. Both debug policies
succeed. Repeating omission into another new output gives byte-identical
archives. Expected identity uses the original package's public key because this
real experiment uses the same local developer key; separate tests exercise
replacing it with a different key. All signature checks pass. Independent
Python/ctypes libarchive decoding compares all four omitted-debug output members
against our extraction; scripts and hashes remain under ignored
`scratch/iq-creation/`.

- **Artifact:** edited-47.iq, debug omitted. **Bytes:** 7893. **SHA-256:**
  `efb2537fdef3cdda2d2caaa88b3cc9678302c48a415809578decd6b10a799d49`.

- **Artifact:** edited-47-preserve.iq. **Bytes:** 13750. **SHA-256:**
  `46264538b014cd840cc4c5890f66f5c9a55bacdf09598d0228961930f5fa1077`.

- **Artifact:** Extracted signed PRG. **Bytes:** 25820. **SHA-256:**
  `b6e3bd6f65114bcf65749170b97dfb887d3bec4a70da95be6680b34318183b53`.

## Runtime evidence and limits

Through the existing Garmin container-broker session, run:

```sh
xvfb-run -a -s '-screen 0 1280x1024x24' python3 scripts/simulator-session.py --prg scratch/iq-creation/extracted-47/006-B4257-00/barrel-app.prg --device fr265 --observe-seconds 8
```

Experimentally observed: console says `Hello from a barrel!` and `47`; the
inspected screen shows `Barrel example` and `47`. Captures are preserved in
`scratch/iq-creation/simulator-47/`. Console SHA-256:
`9d3cd3de8e6e30c9e069e7c2e9dd7e13d15b1ed4b7144d6511af4091bb689714`. Screen
SHA-256: `ac302e7c47161a50cbe4c3ede9c49822d6841d69aa6807fb96b1b0f2841d76fe`.

This validates the edited, own-key signed program extracted from our archive. It
does not establish Garmin store/upload/install acceptance of that archive.
Synthetic shared/multiple-product tests do not establish real multi-device SDK
compatibility. Preserved external debug XML remains potentially stale after
edits; omission is the default.

The initial packaging stages are now complete for their supported profiles.
Return to assembler module naming and branch/exception relocation; the
[first compiler milestone](../architecture.md#shared-compilerassembly-pipeline) gives the next small
executable source-language objective.
