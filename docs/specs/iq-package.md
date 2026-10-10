# Connect IQ distribution package: observed 7z and signing profile

Status: documented 2026-10-10; Rust reading/extraction implemented for [the supported profile](package-reading.md); modern developer [verification](package-verification.md) implemented; [unchanged repacking](iq-repacking.md) is implemented; own-key writing remains planned. Evidence: [independent archive/signature investigation](../investigations/package-formats.md), SDK 9.2.0 / `fr265`. This profile covers our own dual-hash developer-signed export. Store delivery, legacy variants, and general device catalogues are not specified here.

## Archive

Our `.iq` is a 7z archive. Detect the container from bytes, then identify its package role from its manifest; do not rely on an extension alone.

| Start-header position | Width | Meaning |
|---|---:|---|
| 0 | 6 | `37 7a bc af 27 1c` |
| 6 | 2 | 7z version, observed `00 02` |
| 8 | 4 | Little-endian CRC-32 of bytes `[12,32)` |
| 12 | 8 | Little-endian next-header offset, relative to byte 32 |
| 20 | 8 | Little-endian next-header size |
| 28 | 4 | Little-endian next-header CRC-32 |

Delegate the complete 7z grammar and decompression to an archive library. In our specimen, the next header is unencoded (`01`), filenames are encoded as UTF-16LE, and five files have separate compressed folders/streams. Every stream uses LZMA2 (coder ID `21`, property byte `16`); the archive is not solid and not encrypted. These are observed compression choices, not requirements to reject other supported 7z layouts. Support encoded headers/solid streams through the backend within explicit resource limits; reject encryption and unsupported codecs initially.

## Package tree and manifest

```text
manifest.xml
manifest.sig2
dev_key.pub
006-B4257-00/barrel-app.prg
006-B4257-00/debug.xml
```

`dev_key.pub` is DER SubjectPublicKeyInfo with the `rsaEncryption` OID (`1.2.840.113549.1.1.1`) and NULL parameters. Our 550-byte member contains an RSA-4096 modulus and exponent 65537. It is public material. The private developer key is not a package member.

The root XML is namespaced `iq:manifest` version `3` with an `iq:application` and `iq:products`. Each observed `iq:product` supplies `filename`, `partNumber`, `connectIqVersion`, `minFirmwareVersion`, `sig`, `sig2`, and `sigFormat="rsa-4096"`. This specimen uses `filename="006-B4257-00/barrel-app.prg"` and `partNumber="006-B4257-00"`. The application retains its ID, type, entry class, and resource references and adds `apiVersion`.

Paths come from the manifest; do not assume an SDK device ID such as `fr265` is the directory name. The first reader must enumerate all products, not choose the first one silently. Multiple products may need to reference the same file; establish sharing semantics before enforcing one path per product. `debug.xml` is external debug information; it is not required to execute the extracted PRG, and its inclusion policy across export modes is still open.

Our export's PRG has 11 sections: SYMBOLS/DEBUG absent and LINE_NUMBERS an empty count. This is an SDK export result, not a package-writer requirement to strip debugging sections from arbitrary input PRGs.

## Verified signatures

All observed signatures use RSA-4096 PKCS#1 v1.5. Direct public-RSA verification establishes:

1. Product `sig` is 1024 hexadecimal characters decoding to the PRG's 512-byte SHA-1 developer signature.
2. Product `sig2` is 1024 hexadecimal characters decoding to its 512-byte SHA-256 developer signature.
3. Both cover the [PRG developer-signature message](developer-signatures.md): ordered non-signature section wrappers and payloads, excluding developer/store signing sections and the zero terminator. They do **not** sign the entire PRG byte stream.
4. `manifest.sig2` is a raw 512-byte SHA-256 signature over the **exact uncompressed `manifest.xml` bytes**, with no XML canonicalization. Adding a newline makes verification fail.
5. The SPKI public key matches the public components embedded in the device PRG.

There is no archive-byte signature in this specimen. Recompression/timestamps alone need not change these signatures if every signed member remains unchanged. The observed signature messages do not include `debug.xml` or a digest of it; its content is not authenticated by these signatures. Member CRCs detect archive corruption; they are distinct from RSA signatures. Do not describe a successful signature check as store acceptance or trust in a publisher: embedded-key checks establish integrity, and comparison with an explicitly supplied public key establishes identity relative to that key.

`manifest.sig` is a source-derived older SHA-1 variant, not experimentally verified here. Missing/unsupported signing variants must be reported explicitly; do not silently call a partially checked package valid. Signature requirements for store-signed or other SDK packages remain open.

## Own-key writing sequence

Planned modern developer-package writer:

1. Validate all selected PRGs, required product metadata, safe filenames, and supported signature variants.
2. Re-sign every product with the supplied external developer key and verify each PRG. Remove stale signing material using the existing PRG signing policy.
3. Set each product's `sig`/`sig2` from its resulting PRG signatures. Write `dev_key.pub` as SPKI for that key.
4. Serialize the final manifest **once**; sign those exact bytes with SHA-256 into `manifest.sig2`.
5. Archive these exact bytes and members as 7z; reopen and verify the final product/member relationships and signatures.

Unknown XML must survive a controlled edit or cause a clear unsupported-writing error. Do not infer firmware/API/product metadata from bytecode alone. Preserve a supplied debug XML or deliberately omit it where supported; PRG assembly edits do not automatically update its ranges/source mappings. A low-level unchanged-member repack can preserve existing signatures without any private key, but a changed product/manifest requires renewed corresponding signatures.

The first writer targets SDK-produced developer packages. IQ archive acceptance by Garmin upload/install workflows has not been tested independently; existing simulator runs only validate extracted/reassembled PRGs. See [the staged crate plan](../design/package-crate.md).
