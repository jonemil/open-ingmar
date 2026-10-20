# Developer signatures

Implemented in `ciq-sign`; evidence and attribution are in
[the investigation](../../investigations/developer-signing.md). This describes the
verified developer format, not Garmin store signing or firmware authentication.

## Payload and coverage

The developer section identifier is `0xe1c0de12`. Supported payloads use
RSA-4096, PKCS#1 v1.5 signatures, and big-endian integers:

| Payload offset | Length | Meaning |
|---|---|---|
| 0 | 512 | RSA signature over the SHA-1 digest |
| 512 | 512 | Public modulus |
| 1024 | 4 | Public exponent |
| 1028 | 512 | Optional RSA signature over the SHA-256 digest |

Payload length is exactly 1028 (SHA-1 only) or 1540 (both digests). Both
observed SDK 9.2.0 examples use 1540 bytes; legacy support is source-derived and
tested synthetically, without a legacy SDK specimen. The modulus must have
exactly 4096 bits. The exponent must fit the four-byte field and form a valid
RSA public key.

The signed message concatenates each non-signing section's eight-byte big-endian
identifier/length wrapper and its payload, in original order. Duplicate
ordinary/unknown identifiers remain included. Both developer sections and store
signing sections (`0x00005161`) are excluded, including their wrappers. The
eight-byte zero terminator is excluded. DigestInfo uses the standard SHA-1 or
SHA-256 OID for PKCS#1 v1.5.

## Key and replacement policy

`DeveloperKey` accepts unencrypted PKCS#8 DER RSA-4096 private keys, matching
the local developer key. PEM, encrypted DER, PKCS#1 private-key files, and other
key sizes are unsupported. Private keys remain outside exports and version
control; errors do not print key components. RSA signing uses OS randomness for
blinding and RustCrypto primitives; no external signing executable is required.

Re-signing preserves the supported existing developer payload variant. Unsigned
or store-only input defaults to the dual-digest variant. Duplicate developer
sections or unsupported developer payload lengths are rejected. It is
unnecessary to validate the old signature before replacing it: edited input
commonly has an invalid old signature.

All old developer/store signing sections are removed. Other sections keep their
order, identifiers, and payload bytes; one new developer signature is appended
before the terminator. The embedded public key comes from the supplied private
key. Store provenance is not reproduced. The generated output is verified before
it is returned or written.

Verification requires one final developer section and rejects inputs containing
store signing sections. Every signature in the supported payload must verify.
Without a supplied key, verification establishes integrity against the embedded
public key, not trusted identity. With a supplied key, it additionally requires
matching modulus and exponent. Signature validity does not establish simulator
acceptance or correct application behavior.

## CLI

```sh
cargo run -p ciq-asm -- project scratch/project scratch/signed.prg -y keys/developer_key.der
cargo run -p ciq-asm -- template.prg code.mcasm scratch/signed-code.prg -y keys/developer_key.der
cargo run -p ciq-asm -- sign input.prg scratch/resigned.prg -y keys/developer_key.der
cargo run -p ciq-asm -- verify scratch/resigned.prg
cargo run -p ciq-asm -- verify scratch/resigned.prg -y keys/developer_key.der
```

Output files must not already exist. `-y` is optional for project/template
assembly and required for `sign`; it is rejected for standalone `symbols` or
`data` payload assembly. Without `-y`, assembly preserves original signing
material. Project length constraints and equal-length template edits are
unchanged: signing does not implement relocation.

## Public keys and detached signatures

`ciq-sign::PublicKey` imports/exports DER SubjectPublicKeyInfo RSA-4096 keys,
including the observed `rsaEncryption` profile. Import rejects
malformed/trailing DER, oversized input (over 4096 bytes), other key sizes, and
exponents which cannot fit the PRG u32 field. `DeveloperKey::public_key()`
supplies this public view without exposing private components.

`DeveloperKey::sign_detached(HashAlgorithm, message)` and
`PublicKey::verify_detached(HashAlgorithm, message, signature)` use RSA-4096
PKCS#1 v1.5 with explicit SHA-1/SHA-256 selection. They hash the exact
caller-supplied bytes, accept/produce 512-byte signatures, and do not
canonicalize XML or guess which PRG sections to cover. Signing retains OS-random
blinding. SHA-1 exists for the observed legacy PRG message; its availability
does not imply support for legacy IQ manifest profiles.

`developer_signatures(prg)` exposes structurally checked signature/key fields
and format without cryptographically verifying them. `verify(prg)` checks those
fields with the shared detached-verification operations; the existing PRG
signing/verification policy is unchanged. Package checks use these APIs as
specified in [IQ verification](packages.md#iq-verification). No private key is
required by the package verifier.
