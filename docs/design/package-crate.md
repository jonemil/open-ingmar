# ciq-package design

`ciq-package` owns archive/package handling as a library and CLI, separate from
PRG reconstruction and cryptographic primitives. **Stages 1–5 are implemented:**
`list`/`unpack`, modern developer IQ `verify`, deterministic `pack-barrel`,
verified unchanged `repack-iq`, and own-key `pack-iq`. See
[IQ creation](../specs/iq-creation.md). See
[IQ repacking](../specs/iq-repacking.md). See
[packing behavior](../specs/barrel-packing.md) and
[SDK validation](../investigations/barrel-packing.md). See
[verification behavior](../specs/package-verification.md). See
[the reading specification](../specs/package-reading.md) and
[validation](../investigations/package-reading.md). Local references:
[barrel profile](../specs/barrel-package.md),
[IQ profile](../specs/iq-package.md), and
[experiments](../investigations/package-formats.md).

## Boundary and model

`ciq-package` owns archive detection/codec adapters, member inventory, package
manifests, extraction/writing, and product/member relationships. `ciq-prg`
continues owning an individual PRG and its section project. `ciq-sign` owns
cryptographic operations and key parsing. A package is not a VM object and this
work does not introduce a linker, Monkey C compiler, or Jungle evaluator.

Logical boundaries for later expansion (the initial implementation uses `lib`,
`preflight`, `paths`, `manifest`, and `main`):

```text
archive   # ZIP/7z adapters and common member metadata/limits
barrel    # outer/content manifests and library package validation
iq        # product descriptors, manifest bytes, package verification/writing
extract   # filesystem extraction policy and output lifecycle
main      # ciq-package CLI
```

An archive model retains container kind, ordered member metadata, and access to
exact uncompressed bytes. Preserve unknown ordinary members and unknown manifest
fields. Do not eagerly decompress every member for `list`;
decompression/extraction must have explicit entry-count, per-member, total-byte,
and codec-memory limits. Some solid/encoded 7z metadata may require bounded
decoding even to inventory entries. The initial library accepts `Read` input,
buffering a bounded archive for seekable codecs, and exposes filesystem
extraction. A public controlled member-output sink remains future work.

Typed package views retain **raw manifest bytes** alongside parsed
library/product fields. Untouched manifests are never parsed and reserialized
merely to repack them. IQ signing operates on finalized raw bytes; XML layout is
part of the signature message. Package-level aliases do not become VM symbols.

Initial Rust dependencies: `zip` for ZIP/Deflate, `sevenz-rust2` for 7z/LZMA2,
and an XML library such as `quick-xml`. Use maintained codec implementations
rather than implementing ZIP/7z compression or binding libarchive in our Rust
code. `ciq-prg` supports product validation; The stage 2 verifier uses `ciq-prg`
and `ciq-sign`. Cryptography must not spread into the archive adapters.

The implementation retains Rust 1.85 using pinned `zip 6.0.0`,
`sevenz-rust2 0.20.2`, and patched `quick-xml 0.41.0`. Actual full workspace
tests with Rust 1.85.0 pass. Published runtime dependency licenses and all
locked registry versions were reviewed; XML advisory findings changed the
initial candidate version before acceptance. See
[the reader investigation](../investigations/package-reading.md) for evidence
and the existing RSA advisory outside this new dependency boundary. Use that
review process when upgrading libraries; do not silently raise the workspace
compiler requirement.

Disable unnecessary codec/encryption defaults. The initial verified read profile
needs ZIP Store/Deflate and 7z LZMA2; supported 7z Copy/LZMA, solid streams, and
encoded headers can be enabled where the backend handles them with resource
bounds. Enable writers only for the stages that need them. Check crate and
transitive licenses; candidate ZIP is MIT and sevenz-rust2 Apache-2.0. Pin
actual resolved versions in Cargo.lock.

## Extraction and validation

Extract only into a newly created destination. Inventory and validate paths
before committing output: forbid absolute paths, `..`, Windows drive/UNC roots,
backslash traversal aliases, embedded NUL, duplicate normalized filenames,
file/directory collisions, and symlink/hardlink entries. Parent directories
created by extraction must remain inside the destination and must not be
existing links. Reject case collisions where the target filesystem would merge
them. Never execute Jungle/source content or follow an archive link. Stage
output until member CRCs and package checks pass; clean up only files created by
this operation on failure.

For IQ files, a manifest-selected PRG must refer to an ordinary existing archive
member. Verification reports separate results for archive integrity, manifest
signature, product/PRG signatures, embedded-public-key consistency, and optional
caller-supplied key identity. Unknown signature profiles get an explicit
unsupported result, not a green aggregate status. Support multiple product
entries in the model from the beginning; multi-device writer compatibility needs
real specimens before release.

Deterministic writers use a specified stable member order, stable
timestamps/attributes, and fixed codec settings. Archive byte identity is a
preservation option for simply copying the original container; reconstructing a
compressed archive tests member equality and semantics instead. An
unchanged-member IQ repack can retain its exact manifest/signatures without a
key. A changed package must not be emitted as newly verified while retaining
stale signatures.

## Stages and acceptance criteria

1. **Read/list/unpack both formats — implemented for the initial profile.**
   Create the crate and CLI, detect container by magic, inspect package type
   from the manifest, expose member inventory and product attributes, and
   perform bounded safe extraction. Validate our ZIP barrel and 7z IQ members
   byte for byte against independently obtained payloads. Synthetic cases cover
   traversal, links, duplicates, CRC/truncation failures, unsupported
   codecs/encryption, size limits, and an IQ with multiple product descriptors.
   Do not depend on SDK binaries in unit tests.
2. **Package verification — implemented for the modern developer profile.**
   Extend `ciq-sign` with public-SPKI import/export and narrowly scoped detached
   SHA-1/SHA-256 signing/verification APIs, keeping the private key opaque.
   Independently verify the real SDK modern IQ profile, product signature
   attribute equality, PRG key consistency, exact-byte manifest signing, and
   optional expected key. Test tampered manifest, PRG, signature attributes,
   public key, and missing members; use generated ephemeral keys, never the
   user's private key.
3. **Barrel packer — implemented for the canonical extracted tree.** Retain
   identical manifest copies plus Jungle/source/resource bytes and emit bounded
   deterministic ZIP/Deflate. Both code-only and resource specimens preserve
   every ordinary member byte, repack deterministically, and are consumed by the
   SDK with identical resulting CODE/DATA. See
   [writer policy](../specs/barrel-packing.md) and
   [acceptance evidence](../investigations/barrel-packing.md).
4. **Unchanged IQ repacking — implemented for modern developer packages.**
   Rebuild a package with all signed bytes unchanged, preserving signing
   identity without a key. Verify before/after and compare every member.
   Exercise the chosen 7z writer and deterministic output. Keep this distinct
   from editing/resigning and do not claim store acceptance from simulator
   checks.
5. **Own-key IQ creation/editing — implemented for supplied modern metadata.**
   Implement the signing sequence in the IQ spec: sign PRGs, update product
   attributes and public key, serialize/sign the final manifest, then archive
   and verify. Begin with a supplied SDK manifest/product metadata rather than
   generating device/version metadata from guesses. Capture multi-device and
   legacy/store variants before claiming general compatibility. Reassembled
   extracted PRGs must still run in the official simulator; validate a rebuilt
   distribution package with the applicable SDK/install/upload workflow when
   available and authorized.

## Proposed commands

`list`, `unpack`, `verify`, `pack-barrel`, `repack-iq`, and `pack-iq` exist.

```sh
ciq-package list library.barrel
ciq-package unpack library.barrel scratch/library
ciq-package pack-barrel scratch/library scratch/library-repacked.barrel
ciq-package list application.iq
ciq-package unpack application.iq scratch/application
ciq-package verify application.iq [--expected-key publisher-public.der]
ciq-package repack-iq scratch/application scratch/unchanged.iq
ciq-package pack-iq scratch/application scratch/resigned.iq -y keys/developer_key.der [--debug omit|preserve]
```

`unpack` writes the archive's real tree, preserving its `manifest.xml`,
`barrel.jungle`, and product-relative filenames. Inspection reports stay outside
that tree or on stdout; they are not silently included as new package members.
`pack-barrel` and `repack-iq` initially consume canonical extracted layouts.
`pack-iq -y` takes edited PRGs and the supplied manifest as inputs, with
explicit policy for stale external debug XML. A new-file output rule matches the
existing assembler/disassembler tools. No automatic PRG disassembly is needed to
unpack a package; the user can pass an extracted PRG to `ciq-dis all`
separately.

## Outstanding research

Project-to-package export and future compiler integration are tracked in
[the project-build and Jungle plan](project-builds.md). Canonical-tree packing
does not resolve a project's Jungle configuration or generate the exported
Jungle.

- Extend bounded backend support beyond classic ZIP and unencoded single-coder
  7z headers; add real solid/LZMA specimens and controlled member-output sinks.
  Dependency/MSRV checks are recorded for the initial implementation.
- Additional real SDK products, multi-device exports, older SHA-1 manifest
  signing, store-specific metadata, and optional signatures/public-key formats.
- Barrel annotations, dependent barrels, localization/device resources, optional
  metadata, multiple Jungles, and project-to-package path rewriting.
- Additional unknown XML variants and real SDK examples with duplicated/shared
  product filenames; exact raw XML and synthetic shared-file descriptors are
  already covered.
- Whether SDK/install tooling accepts independently produced 7z packages with
  different archive settings; exact member/signature verification alone does not
  answer this.
- External `debug.xml` updates after assembly edits. Preserve/omit with explicit
  supported policy; do not publish stale mappings as newly correct.

Stages 1–5 are complete for their documented supported profiles;
[verification evidence](../investigations/package-verification.md) and
[barrel packing evidence](../investigations/barrel-packing.md) record acceptance
checks. The [creation experiment](../investigations/iq-creation.md) validates an
edited program extracted from our IQ in the simulator. Return to assembler
naming/control-flow work and [the first compiler milestone](first-compiler.md);
broad package variants and project-to-package export remain follow-up work.
