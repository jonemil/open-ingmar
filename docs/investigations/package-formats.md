# Barrel and IQ format investigation and packaging plan

Date: 2026-10-10. Follow-up to [multi-file/library/package experiments](multi-file-and-barrels.md). This task investigates and plans; it does not implement a Rust package crate. SDK 9.2.0 / `fr265`, Java 17.0.20.1, `-O 0`. SDK `monkeybrains.jar` SHA-256: `b9be696349c91feec3fb9723584daf624421d197f76ff86f72c7b3f66492fc9f`.

Local implementation references produced here: [barrel package profile](../specs/barrel-package.md), [IQ package/signing profile](../specs/iq-package.md), and [staged ciq-package design](../design/package-crate.md).

## Sources and attribution

- SDK specimens and our independently authored examples are the primary experimental evidence. The earlier investigation records build commands, source, and simulator results.
- [7-Zip's format specification](https://github.com/ip7z/7zip/blob/main/DOC/7zFormat.txt) and [method ID reference](https://github.com/ip7z/7zip/blob/main/DOC/Methods.txt) identify header framing and coder `21` as LZMA2. These general container references do not specify Garmin package semantics.
- [monkeyc-optimizer](https://github.com/markw65/monkeyc-optimizer/tree/2920dc88c23caadded9f66ee0613a7de6ab2d1ea), revision `2920dc88c23caadded9f66ee0613a7de6ab2d1ea`, MIT. `src/readprg.ts:optimizePackage` and `src/readprg/signer.ts` suggest that product attributes carry PRG signatures and that detached manifest signatures sign serialized XML bytes. We wrote independent Python public-RSA checks; no implementation code was incorporated. Older `manifest.sig` handling remains source-derived only.
- Dependency research: [zip 6.0.0 package manifest](https://docs.rs/crate/zip/6.0.0/source/Cargo.toml.orig), [sevenz-rust2 v0.20.2](https://raw.githubusercontent.com/hasenbanck/sevenz-rust2/v0.20.2/Cargo.toml), [v0.23.0](https://raw.githubusercontent.com/hasenbanck/sevenz-rust2/v0.23.0/Cargo.toml), and [current codec capabilities](https://docs.rs/crate/sevenz-rust2/0.23.0). Selection/lockfile changes remain future implementation work.

Research scripts and outputs are ignored under `scratch/package-formats/`. They use Python stdlib and installed libarchive for independent inspection; those are not dependencies of the planned Rust implementation. No signing private-key bytes were read or printed by the verification experiment: it uses the package's public key and signatures.

## Barrel ZIP profile and compatibility

Python `zipfile` inspection of `examples/number-barrel/build/number-barrel.barrel` finds four members: outer and content manifests, root `barrel.jungle`, and original source. All SDK entries use ZIP Deflate (8), flags `0x0808` (UTF-8 / data descriptors), empty extra fields, and external attributes zero. The two manifests are byte identical. Its generated Jungle selects `content/manifest.xml` and rewrites source/personality paths into `content/`.

Independent repacking changes codec/container metadata without changing member contents:

```python
from pathlib import Path
import zipfile
root = Path('scratch/package-formats')
root.mkdir(exist_ok=True)
source = zipfile.ZipFile('examples/number-barrel/build/number-barrel.barrel')
for method, name in [(zipfile.ZIP_STORED, 'stored'), (zipfile.ZIP_DEFLATED, 'deflated')]:
    with zipfile.ZipFile(root / f'repacked-{name}.barrel', 'w', compression=method) as output:
        for member in reversed(source.namelist()):
            output.writestr(member, source.read(member))
```

Copy only `source/`, `resources/`, and `manifest.xml` from `examples/barrel-app/` into ignored `scratch/package-formats/consumer/`. Create `stored.jungle` and `deflated.jungle` there with normal manifest/source/resource properties and `base.barrelPath = ../repacked-<variant>.barrel`. Build each, saving its PRG before the next overwrites the output:

```sh
CIQ_JUNGLE_FILES=stored.jungle sh scripts/build-example.sh scratch/package-formats/consumer
cp scratch/package-formats/consumer/build/fr265/consumer.prg scratch/package-formats/stored-consumer.prg
CIQ_JUNGLE_FILES=deflated.jungle sh scripts/build-example.sh scratch/package-formats/consumer
cp scratch/package-formats/consumer/build/fr265/consumer.prg scratch/package-formats/deflated-consumer.prg
```

Both SDK builds succeed. Independent section parsing confirms CODE and DATA are byte identical to each other and to the original barrel-app archive build. This establishes that ZIP Store and Deflate, changed member order, and independently generated ZIP metadata are accepted for this specimen; exact SDK compressed bytes are unnecessary. It does not validate every ZIP method or a generic source-project mapper.

A second ignored barrel project copies the authored source/manifest/Jungle, adding `resources/strings.xml` with one `Greeting` string. `sh scripts/build-barrel.sh scratch/package-formats/resource-barrel` succeeds. Its fifth archive member is `content/resources/strings.xml`, exactly equal to the authored resource. The generated Jungle remains unchanged, relying on default resource paths relative to the content manifest. This validates resource **packaging**, not resource access from a compiled consumer or annotation/dependency behavior.

## IQ 7z inspection

Original package: `scratch/barrels/barrel-app.iq`, 13895 bytes. Read with libarchive, extracting the five known ordinary members into `scratch/package-formats/extracted/` and retaining exact uncompressed bytes. The existing extraction script was extended locally; its limited specimen checks are not our production extraction policy.

Independent framing checks:

- Magic `377abcaf271c`, version `0002`.
- Start-header CRC-32 matches bytes `[12,32)`.
- Next-header offset 13481 relative to byte 32, size 382; it ends exactly at EOF.
- Next-header CRC-32 matches its bytes.
- Next header begins with `01` (unencoded header); five streams/folders each have one LZMA2 coder (`21`), property `16`.
- Packed stream sizes in member order: 1622, 516, 554, 4957, 5832 bytes. Their sum is 13481. Each folder holds one member, so this specimen is non-solid.
- Names use UTF-16LE. No encrypted coder is present.

These facts support a 7z/LZMA2 read/write backend. The initial reader must still account for bounds and errors and either support or explicitly reject other layouts. Do not implement the whole 7z format based on this single header.

The product manifest has `sigFormat="rsa-4096"` and the product/device metadata recorded in the IQ spec. Package members are manifest, manifest signature, public SPKI key, one PRG, and one external debug XML. There is no store-specific or second-device specimen here.

## Independent IQ signature verification

Procedure (`scratch/package-formats/verify-signatures.py`):

1. Read `dev_key.pub` with a small DER TLV decoder, verifying SubjectPublicKeyInfo, rsaEncryption OID with NULL parameters, a zero-unused-bit BIT STRING, then RSA INTEGER modulus/exponent. Obtain 4096 bits / exponent 65537.
2. Parse manifest XML, read the selected product PRG, and decode its `sig` / `sig2` hex attributes. Compare each with the embedded PRG developer-signature bytes and public components.
3. Collect non-signature PRG section wrappers/payloads, excluding signing sections and the eight-byte zero terminator.
4. For each detached signature compute `signature^exponent mod modulus`, requiring the full RSA-width PKCS#1 v1.5 encoding `00 01 FF...FF 00 DigestInfo`. SHA-1 DigestInfo prefix: `3021300906052b0e03021a05000414`; SHA-256: `3031300d060960864801650304020105000420`.
5. Verify the two product signatures over the PRG message; verify `manifest.sig2` over the exact manifest member bytes. Perform negative checks using the whole PRG and manifest bytes plus one newline.

All positive checks pass. Both negative checks fail as intended. Raw observations are in ignored `signatures.json`.

Consequences, experimentally established for this profile:

- Product attributes duplicate the embedded PRG signature fields; no separate whole-PRG message is required for them.
- The detached manifest signature is SHA-256 over exact XML bytes, not canonicalized XML or archive bytes.
- The public SPKI key and PRG public components agree.
- Re-signing a PRG with a new key requires updating product attributes, the package public key, and the manifest signature.
- Recompressing unchanged members can preserve their signing identity. Actual independently written IQ archive acceptance by SDK/upload/install workflows remains untested.

The observed signatures do not authenticate `debug.xml`: neither the signed manifest nor the signed PRG message includes its bytes or digest. Archive CRCs are integrity checks against accidental corruption, not signing identity. Verification should report coverage precisely rather than implying that every metadata member is authenticated.

Older `manifest.sig` / SHA-1, different RSA sizes/encodings, absent signatures, and store variants are unsupported research cases until independently validated. Do not turn external source hints into universal package rules.

## Artifacts and limits

| Specimen | Bytes | SHA-256 |
|---|---:|---|
| Original SDK barrel | 1271 | `cd5cf17363b59566a6896a8dc063e0dfb748da490ad697e08d774876bc22a4df` |
| Independent ZIP/Store barrel | 1683 | `9de9edda825f999d20e8a360de2d47962f26c5b669951550f9a7accb002004ed` |
| Independent ZIP/Deflate barrel | 1215 | `bcb35f543b068caefcc166f229464597ae5fb82545f5f8e6ebb0e72b2456c935` |
| Store consumer PRG | 123036 | `8f40e64af60e8d32f5a24e84d0de42f116ce2533f93fc07b2f9c737fc32bd2fe` |
| Deflate consumer PRG | 123036 | `27a4634d89bf8c6171ed1bdc59594d38dced6580330eefd2cfe6b4355df3c5ba` |
| Resource barrel | 1473 | `48d9b5167a485910a5ab09ef428e03ea8ded017bfb6ca1663f51387f314e4482` |
| Original SDK IQ | 13895 | `7213919d07f6c93af812a0ee369cd76f3c7bcd7c6e9dd8b028b5ad2cd9a18746` |

SDK temp paths/timestamps can change archive hashes on rebuild. Compare payloads and cryptographic results rather than baking these hashes into Rust tests. No simulator run was necessary for these metadata/ZIP representation checks: the earlier investigation already confirms the corresponding extracted PRG behavior; this turn's independent ZIP consumers were compiled and compared, not separately simulated.

## Plan and dependency findings

The user approved the `ciq-package` boundary; the concrete staged design is in [package-crate.md](../design/package-crate.md). No new crate stub, dependencies, or signing APIs were implemented during this research task.

Candidate libraries can handle established archive formats independently of Garmin implementation code. `zip 6.0.0` declares Rust 1.83 and MIT; `sevenz-rust2 0.20.2` declares 1.85 and Apache-2.0. Current sevenz-rust2 0.23.0 declares 1.93; blindly selecting it would violate our workspace's advertised 1.85 baseline. Pin/select only after compatibility, advisory, transitive-version, and resource-limit checks. The newer release remains an option if the workspace's compiler requirement is deliberately updated later. The local installed libarchive remains an investigation aid, not the planned implementation backend.

First implementation target is list/safe unpack for both formats, with raw XML preservation and member/product inventory. Then add verified signing support, independent barrel repacking, unchanged IQ repacking, and own-key IQ writing. Keep PRG assembly as a separate stage and preserve `at=` behavior. No linker, Jungle evaluator, or archive compressor implementation from scratch is proposed.

Implementation follow-up: [Rust package reading and extraction](package-reading.md) completes stage 1 for its documented profile, including actual dependency/MSRV validation. This record above remains the preceding format-research evidence.
