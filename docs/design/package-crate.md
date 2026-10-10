# Planned ciq-package crate

Decision, 2026-10-10: add archive/package support as a sixth workspace crate, `ciq-package`, with a library and a same-named CLI. **This is a plan, not an implemented crate or available command.** Local references: [barrel profile](../specs/barrel-package.md), [IQ profile](../specs/iq-package.md), and [experiments](../investigations/package-formats.md).

## Boundary and model

`ciq-package` owns archive detection/codec adapters, member inventory, package manifests, extraction/writing, and product/member relationships. `ciq-prg` continues owning an individual PRG and its section project. `ciq-sign` owns cryptographic operations and key parsing. A package is not a VM object and this work does not introduce a linker, Monkey C compiler, or Jungle evaluator.

Proposed modules:

```text
archive   # ZIP/7z adapters and common member metadata/limits
barrel    # outer/content manifests and library package validation
iq        # product descriptors, manifest bytes, package verification/writing
extract   # filesystem extraction policy and output lifecycle
main      # ciq-package CLI
```

An archive model retains container kind, ordered member metadata, and access to exact uncompressed bytes. Preserve unknown ordinary members and unknown manifest fields. Do not eagerly decompress every member for `list`; decompression/extraction must have explicit entry-count, per-member, total-byte, and codec-memory limits. Some solid/encoded 7z metadata may require bounded decoding even to inventory entries. The library should accept `Read + Seek` input and controlled output sinks, with filesystem handling in a separate layer.

Typed package views retain **raw manifest bytes** alongside parsed library/product fields. Untouched manifests are never parsed and reserialized merely to repack them. IQ signing operates on finalized raw bytes; XML layout is part of the signature message. Package-level aliases do not become VM symbols.

Initial Rust dependencies: `zip` for ZIP/Deflate, `sevenz-rust2` for 7z/LZMA2, and an XML library such as `quick-xml`. Use maintained codec implementations rather than implementing ZIP/7z compression or binding libarchive in our Rust code. `ciq-prg` supports product validation; `ciq-sign` is introduced when verification/signing is implemented. Cryptography must not spread into the archive adapters.

The workspace currently advertises Rust 1.85. Candidate releases `zip 6.0.0` (declared MSRV 1.83) and `sevenz-rust2 0.20.2` (declared MSRV 1.85) can preserve that boundary, but must undergo advisory/transitive-dependency/MSRV checks before adoption. Current `sevenz-rust2 0.23.0` requires 1.93 and current ZIP releases can require 1.88; do not silently introduce them under the 1.85 promise. Dependency choice remains an implementation gate: either validate compatible maintained versions or explicitly revise the documented compiler requirement. No Cargo dependency changes are made during this planning task.

Disable unnecessary codec/encryption defaults. The initial verified read profile needs ZIP Store/Deflate and 7z LZMA2; supported 7z Copy/LZMA, solid streams, and encoded headers can be enabled where the backend handles them with resource bounds. Enable writers only for the stages that need them. Check crate and transitive licenses; candidate ZIP is MIT and sevenz-rust2 Apache-2.0. Pin actual resolved versions in Cargo.lock.

## Extraction and validation

Extract only into a newly created destination. Inventory and validate paths before committing output: forbid absolute paths, `..`, Windows drive/UNC roots, backslash traversal aliases, embedded NUL, duplicate normalized filenames, file/directory collisions, and symlink/hardlink entries. Parent directories created by extraction must remain inside the destination and must not be existing links. Reject case collisions where the target filesystem would merge them. Never execute Jungle/source content or follow an archive link. Stage output until member CRCs and package checks pass; clean up only files created by this operation on failure.

For IQ files, a manifest-selected PRG must refer to an ordinary existing archive member. Verification reports separate results for archive integrity, manifest signature, product/PRG signatures, embedded-public-key consistency, and optional caller-supplied key identity. Unknown signature profiles get an explicit unsupported result, not a green aggregate status. Support multiple product entries in the model from the beginning; multi-device writer compatibility needs real specimens before release.

Deterministic writers use a specified stable member order, stable timestamps/attributes, and fixed codec settings. Archive byte identity is a preservation option for simply copying the original container; reconstructing a compressed archive tests member equality and semantics instead. An unchanged-member IQ repack can retain its exact manifest/signatures without a key. A changed package must not be emitted as newly verified while retaining stale signatures.

## Stages and acceptance criteria

1. **Read/list/unpack both formats.** Create the crate and CLI, detect container by magic, inspect package type from the manifest, expose all member/product metadata, and perform bounded safe extraction. Validate our ZIP barrel and 7z IQ members byte for byte against independently obtained payloads. Synthetic cases cover traversal, links, duplicates, CRC/truncation failures, unsupported codecs/encryption, size limits, and an IQ with multiple product descriptors. Do not depend on SDK binaries in unit tests.
2. **Package verification.** Extend `ciq-sign` with public-SPKI import/export and narrowly scoped detached SHA-1/SHA-256 signing/verification APIs, keeping the private key opaque. Independently verify the real SDK modern IQ profile, product signature attribute equality, PRG key consistency, exact-byte manifest signing, and optional expected key. Test tampered manifest, PRG, signature attributes, public key, and missing members; use generated ephemeral keys, never the user's private key.
3. **Barrel packer.** Start from a canonical unpacked package layout; retain manifest/Jungle/source/resource bytes and emit deterministic ZIP/Deflate. Repack our code-only and resource specimens. Acceptance is successful SDK consumption and identical resulting CODE/DATA, already demonstrated experimentally for two independently created ZIP variants. Add a project-to-content mapper later, with explicit source/resource inputs; avoid an incomplete implicit Jungle interpreter.
4. **Unchanged IQ repacking.** Rebuild a package with all signed bytes unchanged, preserving signing identity without a key. Verify before/after and compare every member. Exercise the chosen 7z writer and deterministic output. Keep this distinct from editing/resigning and do not claim store acceptance from simulator checks.
5. **Own-key IQ creation/editing.** Implement the signing sequence in the IQ spec: sign PRGs, update product attributes and public key, serialize/sign the final manifest, then archive and verify. Begin with a supplied SDK manifest/product metadata rather than generating device/version metadata from guesses. Capture multi-device and legacy/store variants before claiming general compatibility. Reassembled extracted PRGs must still run in the official simulator; validate a rebuilt distribution package with the applicable SDK/install/upload workflow when available and authorized.

## Proposed commands

These spellings are planned examples, not existing CLI options:

```sh
ciq-package list library.barrel
ciq-package unpack library.barrel scratch/library
ciq-package pack-barrel scratch/library scratch/library-repacked.barrel
ciq-package list application.iq
ciq-package unpack application.iq scratch/application
ciq-package verify application.iq
ciq-package repack-iq scratch/application scratch/unchanged.iq
ciq-package pack-iq scratch/application scratch/resigned.iq -y developer_key.der
```

`unpack` writes the archive's real tree, preserving its `manifest.xml`, `barrel.jungle`, and product-relative filenames. Inspection reports stay outside that tree or on stdout; they are not silently included as new package members. `pack-barrel` and `repack-iq` initially consume canonical extracted layouts. `pack-iq -y` takes edited PRGs and the supplied manifest as inputs, with explicit policy for stale external debug XML. A new-file output rule matches the existing assembler/disassembler tools. No automatic PRG disassembly is needed to unpack a package; the user can pass an extracted PRG to `ciq-dis all` separately.

## Outstanding research

- Validate backend limits and error behavior before picking the dependency versions; actually test Rust 1.85 if retaining it.
- Additional real SDK products, multi-device exports, older SHA-1 manifest signing, store-specific metadata, and optional signatures/public-key formats.
- Barrel annotations, dependent barrels, localization/device resources, optional metadata, multiple Jungles, and project-to-package path rewriting.
- Package manifests' unknown XML preservation and duplicated/shared product-file semantics.
- Whether SDK/install tooling accepts independently produced 7z packages with different archive settings; exact member/signature verification alone does not answer this.
- External `debug.xml` updates after assembly edits. Preserve/omit with explicit supported policy; do not publish stale mappings as newly correct.

Start implementation with stage 1. The verified detached signature rules remove an investigation blocker for stages 2 and 5; they do not make those stages implemented.
