# IQ package verification

`ciq-package` verifies IQ packages using `ciq-prg` and `ciq-sign`. The
format/coverage reference is [the observed modern IQ profile](iq-package.md);
[the investigation](../../investigations/package-verification.md) records SDK and
synthetic validation. Archive acceptance still follows
[the package reader's bounds](package-reading.md).

## Commands and trust

```sh
cargo run -p ciq-package -- verify scratch/barrels/barrel-app.iq
cargo run -p ciq-package -- verify scratch/barrels/barrel-app.iq --expected-key publisher-public.der
```

`--expected-key` accepts DER RSA-4096 SubjectPublicKeyInfo, not a private key.
The caller must obtain that public key independently to establish signing
identity relative to it. Passing a key extracted from the same archive merely
exercises equality; it does not add independent trust. Without the flag, checks
establish integrity against the embedded package key. Neither mode establishes
store acceptance, simulator behavior, or publisher provenance beyond the
caller's supplied key.

Verification writes no extracted files and never opens a private developer key.
Success exits 0; any failed, unsupported, or unperformed required check exits 1.
Structural archive/manifest errors can prevent opening the package and produce
an error instead of a report. The optional expected-key check is omitted from
the library report when not requested.

## Report and supported profile

`Package::verify(Option<&ciq_sign::PublicKey>)` returns `VerificationReport`.
`Check` has `Passed`, `Failed(reason)`, `Unsupported(reason)`, and
`NotChecked(reason)` states. `report.verified()` requires all mandatory checks
and every product to pass; unsupported profiles are never a successful
aggregate.

- **Check:** Archive integrity. **Meaning:** Decode all members; check exact
  sizes and available decoded-member CRCs.

- **Check:** Public key. **Meaning:** Parse `dev_key.pub` as supported RSA SPKI.

- **Check:** Manifest signature. **Meaning:** Verify raw 512-byte
  `manifest.sig2` with SHA-256 over exact outer manifest bytes.

- **Check:** Expected key (optional). **Meaning:** Compare modulus and exponent
  against caller's public key.

- **Check:** Product profile. **Meaning:** Require `sigFormat="rsa-4096"` and
  the observed dual-hash developer PRG format.

- **Check:** PRG signatures. **Meaning:** Verify both SHA-1 and SHA-256
  signatures over ordered unsigned section wrappers/payloads.

- **Check:** Product signature attributes. **Meaning:** Decode exactly 1024
  hexadecimal characters each in `sig` and `sig2`, and require equality with the
  PRG's corresponding signatures.

- **Check:** Key consistency. **Meaning:** Require the PRG's embedded key to
  equal the package public key.

Hexadecimal signatures accept upper/lowercase digits, with no whitespace/prefix.
All manifest product descriptors are checked, including descriptors sharing a
file. PRGs are parsed once per selected member; descriptor checks remain
separate. Cryptographic validity of the embedded PRG signatures, equality of
manifest attributes, and package/PRG key agreement are separate results:
equality with invalid signatures is not sufficient.

Missing modern signatures/keys, malformed DER/PRGs, invalid hexadecimal/lengths,
invalid signatures, and mismatching keys produce failures or prerequisite
`NotChecked` results. Unknown product formats, unknown developer-signature
payload lengths, store-signed PRGs, and SHA-1-only IQ product profiles are
unsupported. Presence of legacy `manifest.sig` is unsupported even if
`manifest.sig2` also exists, until that combined/legacy profile has been
investigated. Barrels have no supported signing profile; their CRC checks may
pass, but `verify` cannot return success for a barrel.

The CLI lists ordinary members outside the documented signature coverage in
`crc_only_members`, including `debug.xml` and unknown extra files. Their
modification can leave all signature checks successful. CRCs detect accidental
corruption where present; they neither authenticate those members nor imply that
every file has a CRC. The aggregate describes the supported signed
manifest/product profile, not authentication of the entire archive.
Compressed-stream CRC coverage is not independently promised.

## Memory and errors

Read the package key (maximum 4096 bytes) and manifest signature (maximum 512
bytes) using bounded member access. Then traverse all archive members,
collecting one selected PRG at a time and draining other payloads through the
fixed copy buffer. Discard each PRG after its descriptors have been checked;
retain only reports. Parsing and assembling the PRG signing message require
additional bounded copies of that PRG, so total memory is larger than one
per-member limit. Compressed input, manifests, entry counts, and codec memory
still use the reader's explicit limits. Verification is not a wall-clock-bounded
operation.

An archive decode/CRC failure stops that traversal and fails archive integrity;
products not reached retain `NotChecked` statuses. Other failed cryptographic or
structural product checks do not prevent checking subsequent decodable products.
There is no automatic re-signing, XML rewrite, or package repair.
