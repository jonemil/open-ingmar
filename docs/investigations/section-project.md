# Complete section export/import

Date: 2026-10-08. Goal: represent every PRG section in an exported project and
reconstruct a complete PRG without using the original as a template.

The [section project](../specs/formats/section-project.md) is now implemented.
`ciq-dis all` exports every payload and an ordered `sections.ciq` manifest.
Ordinary code becomes assembly if the complete stream decodes; unsupported code
falls back to raw binary. Other sections are raw binary, including
known-but-undecoded data, resources, symbol tables, and signing material.
Duplicate identifiers and empty sections are retained. Each filename includes
its section index, research name, and identifier.

`ciq-asm project` reads the manifest and exported payloads, then constructs a
new section stream and terminator. It does not read the original PRG. It
requires payload lengths to match the manifest, rejects path traversal and paths
resolving outside the export directory, and refuses to overwrite an existing
output. Export refuses existing directories and publishes the manifest by
renaming a completed temporary manifest after all payload files are written.

## Actual specimen results

Commands run from the repository root:

```sh
cargo build --workspace
mkdir -p scratch/full-sections
target/debug/ciq-dis all examples/minimal-console/build/fr265/minimal-console.prg scratch/full-sections/minimal-console
target/debug/ciq-asm project scratch/full-sections/minimal-console scratch/full-sections/minimal-console-rebuilt.prg
target/debug/ciq-dis all examples/hello-world/build/fr265/hello-world.prg scratch/full-sections/hello-world
target/debug/ciq-asm project scratch/full-sections/hello-world scratch/full-sections/hello-world-rebuilt.prg
```

Each export contains 13 section payload files (one `.mcasm`, twelve `.bin`) plus
the manifest. Python byte equality checks confirmed both reconstructed PRGs
equal their original inputs. These are opaque-preservation tests without
resigning; exact equality is not the intended requirement for future own-key
signing tests.

- **Reconstructed artifact:**
  `scratch/full-sections/minimal-console-rebuilt.prg`. **Bytes:** 122188.
  **SHA-256:**
  `cd361b76dc29e828d43f0511df3bebd96dc63ce6b7be61b70e2be8e1092d95e7`.

- **Reconstructed artifact:** `scratch/full-sections/hello-world-rebuilt.prg`.
  **Bytes:** 122668. **SHA-256:**
  `8608ed040f18294a2a9af870f53e26e64a0d2fe3ef8c032d600043b591d182d7`.

Input provenance, compiler configuration, framing, and opcode research remain in
[the bootstrap investigation](rust-bootstrap.md). Exports and reconstructed
binaries stay under ignored `scratch/`.

## Validation and limits

`cargo test --workspace` now passes 17 tests. New coverage checks manifest
versions and syntax, local filenames, duplicate identifiers, empty and unknown
raw sections, preservation of signing payloads, full export/import,
unsupported-code fallback, refusal to overwrite an export, equal-length code
edits, payload-length mismatches, and external section/manifest symlinks. Clippy
with warnings denied and formatting checks pass.

This does not yet decode the structures inside the raw sections. Import does not
relocate references or verify/recompute signatures, and the manifest lengths are
declarations rather than authenticated original metadata. Changing declarations
cannot make relocated references valid. The planned `-y <developer-key>`
interface remains unimplemented. Simulator execution of these reconstructed
artifacts remains a host-side experiment; no claim of changed-program runtime
acceptance follows from the preservation tests.

On the host, to run the reconstructed Minimal Console specimen:

```sh
sh scripts/run-simulator-container.sh --prg scratch/full-sections/minimal-console-rebuilt.prg
```

Expected console output: `Hello!`.
