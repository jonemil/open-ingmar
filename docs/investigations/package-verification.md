# IQ package signature verification implementation

Date: 2026-10-10. Stage 2 of [the package design](../specs/formats/package-reading.md),
following [the reader](package-reading.md) and
[independent format/signature research](package-formats.md). Scope: public-SPKI
and detached-signature APIs, then read-only IQ verification. No writer,
private-key access to the user's key, SDK upload/install, or new simulator run.

## Implementation reference and attribution

Implementation follows our
[developer-signature specification](../specs/formats/developer-signatures.md),
[IQ profile](../specs/formats/iq-package.md), and new
[verification behavior](../specs/formats/package-verification.md). The existing
independent Python public-RSA experiment was re-run as an oracle; no external
project implementation code was copied.

Cryptography remains in `ciq-sign`, using the same locked `rsa 0.9.10`,
`sha1 0.10.7`, `sha2 0.10.9`, and OS randomness as before. RustCrypto APIs are
the published [RSA release documentation](https://docs.rs/rsa/0.9.10/rsa/) and
[PKCS#8/SPKI traits](https://docs.rs/rsa/0.9.10/rsa/pkcs8/). RSA and SHA
libraries use MIT/Apache-2.0 alternatives. No registry versions or new
codec/cryptographic dependencies were added: `ciq-package` now depends on our
`ciq-prg`/`ciq-sign` crates, and its test fixtures use the existing
RSA/randomness libraries. The existing advisory snapshot in the reader
investigation still applies; this change does not claim to resolve the existing
RSA timing advisory.

`ciq-sign` now exposes an opaque private key's public view, RSA-4096 SPKI
import/export, explicit SHA-1/SHA-256 detached signing/verification over exact
caller bytes, and a structurally decoded view of PRG signature fields. Existing
PRG verification uses those shared public-key/signature operations. Accessing
fields is deliberately distinct from verifying them. Private keys remain opaque
and absent from diagnostics/debug output; detached signing retains OS-random
blinding.

`ciq-package` reports independent archive, manifest, PRG, attribute-equality,
key-consistency, and optional expected-identity checks. It drains unsigned extra
members for available CRC checks and identifies them as outside signature
coverage. It checks all descriptors while retaining only one decoded PRG at a
time. Unsupported signing profiles cannot become successful aggregates.

## SDK observation and independent agreement

**Experimentally observed:** the SDK 9.2.0 / `fr265` export
`scratch/barrels/barrel-app.iq`, SHA-256
`7213919d07f6c93af812a0ee369cd76f3c7bcd7c6e9dd8b028b5ad2cd9a18746`, passes every
implemented modern-profile check. The manifest/PRG hashes and five-member tree
remain those recorded in the reader/format investigations.

```sh
cargo build -p ciq-package --locked
target/debug/ciq-package verify scratch/barrels/barrel-app.iq
target/debug/ciq-package verify scratch/barrels/barrel-app.iq --expected-key scratch/package-formats/extracted/dev_key.pub
python3 scratch/package-formats/verify-signatures.py
```

The expected-key command uses a previously extracted public key to test matching
components. This is an equality/compatibility experiment, not independently
authenticated publisher identity. The independent Python script implements
public modular RSA checks with Python standard-library hashing and XML; its
procedure, covered byte offsets, and provenance are recorded in the prior format
investigation. Re-running confirms the manifest's exact-byte SHA-256 message,
both PRG unsigned-section messages, and product signature equality; hashing the
complete PRG or appending a manifest newline fails as expected.

Both Rust commands exit 0. The report explicitly lists `006-B4257-00/debug.xml`
outside signature coverage. Console captures and independent results live under
ignored `scratch/package-verification/`. No extracted files are produced by the
new command.

## Tests and verification

**Experimentally observed:** 94 workspace tests pass with Rust 1.99 and actual
Rust 1.85.0; formatting and workspace Clippy `-D warnings` pass. Nine new tests
(one signing test and eight package tests) supplement the existing 85.

```sh
cargo test --workspace --locked
cargo +1.85.0 test --workspace --locked
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
```

Fixtures generate ephemeral RSA-4096 keys in tests using OS randomness, then
construct our own signed PRGs, manifests, and solid Copy 7z archives. They do
not read the developer key, SDK binaries, or ignored external projects. Coverage
includes:

- SPKI round trip, malformed/trailing DER and wrong key sizes; exact-message,
  digest-algorithm, signature-length, and wrong-key checks for both detached
  hashes.
- Two product descriptors with independent reports, optional
  matching/mismatching expected key, and CLI reporting/status without extracted
  output.
- A newline added to the raw manifest, detached signature tampering, edited PRG
  code, and mismatching or invalid-hex manifest signature attributes. Re-signing
  a changed manifest with the ephemeral key restores its signature, but does not
  conceal product failures.
- Substituted package public key and an independently re-signed product whose
  key differs from the package key, even when its signatures and updated
  attributes are internally valid.
- Missing signing/key/product members, malformed public key and signature
  lengths, legacy manifest/PRG formats, unknown product formats, and store
  signing. Unsupported/incomplete reports never pass.
- Edited unsigned debug XML with valid refreshed archive CRCs passes the signed
  profile while remaining listed as unsigned. Damage with stale CRC fails
  archive integrity. Barrel integrity can pass while its signing profile remains
  unsupported.

The existing PRG re-signing and assembler workflows still pass after sharing
public-key/signature decoding. Test logs are local
`scratch/package-verification/tests-rust199.log` and `tests-rust185.log`.

## Limits and next step

**Interpretation:** agreement with independent public-RSA checks and focused
tampering tests supports the documented modern developer profile. It does not
establish old/store signing support, general 7z compatibility, trust in a
publisher without an independent expected key, unsigned debug authenticity, or
Garmin upload acceptance. Multi-product behavior has synthetic coverage; a real
multi-device SDK specimen remains future evidence.

Next implement stage 3: deterministic canonical-layout barrel packing,
preserving manifest/Jungle/source/resource bytes. Validate emitted ZIPs through
SDK consumption and resulting CODE/DATA equality. IQ repacking and own-key
creation remain later stages.
