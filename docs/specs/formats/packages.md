# Barrel and IQ packages

The package layer shares archive limits, path handling, extraction, and
integrity policy. Barrels distribute source/build inputs in ZIP; IQ packages
distribute device PRGs and signed metadata in 7z. This page owns both supported
profiles and their operations. [PRG developer signing](developer-signatures.md)
owns the underlying cryptographic messages and key policy.

Contents:

- [Shared reading and extraction](#shared-reading-and-extraction)
- [Barrel format](#barrel-format)
- [Barrel packing](#barrel-packing)
- [IQ format and signing](#iq-format-and-signing)
- [IQ verification](#iq-verification)
- [IQ creation](#iq-creation)
- [IQ repacking](#iq-repacking)

## Shared reading and extraction

`ciq-package` reads and extracts the supported profiles. Format references:
[barrels](#barrel-format), [IQ packages](#iq-format-and-signing). Evidence and
dependency review: [reader
investigation](../../investigations/package-reading.md). This is a bounded
supported profile, not a general-purpose archive extractor.

### Representation

Retain archive kind, ordered member metadata, exact uncompressed content, and
raw manifests alongside parsed fields. Model multiple products/shared files
explicitly; package aliases are not VM symbols. Untouched signed manifests must
not be parsed and reserialized. Listing must not eagerly decompress all
payloads; codec adapters bound the work needed to inspect metadata. Reading,
signing, and project resolution remain separate
[architecture](../../architecture.md) layers.

### Commands and outputs

```sh
cargo run -p ciq-package -- list examples/number-barrel/build/number-barrel.barrel
cargo run -p ciq-package -- unpack examples/number-barrel/build/number-barrel.barrel scratch/unpacked-library
cargo run -p ciq-package -- list scratch/barrels/barrel-app.iq
cargo run -p ciq-package -- unpack scratch/barrels/barrel-app.iq scratch/unpacked-application
```

`list` prints package identity, all IQ product descriptors, and each member's
type (`f`/`d`), uncompressed size, compression method, and original relative
filename. It decodes manifests for package checks, but does not verify every
payload CRC. For a solid 7z block, reaching the manifest can also require
streaming preceding members. No source or Jungle content is executed.

`unpack` creates a **new** destination; an existing directory, file, or symlink
is refused. The destination's parent must already exist. It writes the archive's
actual tree, preserving every ordinary member's bytes, including unknown files,
XML whitespace, signatures, resources, and external debug metadata. It creates
directories and empty files, but does not restore archive timestamps, ownership,
executable bits, or other permissions. Inspection reports go to stdout, not into
the extracted package.

Both commands print `Signatures were not verified.` The separate
[verify command](#iq-verification) checks the modern developer IQ
profile and its PRGs. Canonical barrel packing and
[unchanged IQ repacking](#iq-repacking) are supported.
[Own-key creation](#iq-creation) signs edited packages; automatic disassembly
is outside the package tool. Pass an extracted PRG to the existing tools
explicitly:

```sh
cargo run -p ciq-dis -- all scratch/unpacked-application/006-B4257-00/barrel-app.prg scratch/unpacked-prg-project
```

Errors exit with status 1 and a diagnostic on stderr. Exact diagnostic wording
is not a stable API.

### Library and structural checks

`Package::open(path, Limits)` and `Package::read(reader, Limits)` return a
package with read-only `kind()`, `members()`, `manifest()`, and `manifest_xml()`
accessors. `read` accepts `Read`; it buffers the bounded compressed archive to
supply seekable codec input. This trades streaming compressed input for a simple
owned package which can be listed and then extracted.
`Package::unpack(new_directory)` performs filesystem extraction. There is no
public general member-output sink yet.

Detect container by magic, not extension. ZIP is accepted as a barrel; 7z is
accepted as an IQ distribution package. The outer `manifest.xml` must be an
ordinary member containing a UTF-8 Connect IQ namespace/version-3 manifest and
one direct `barrel` or `application` identity. Namespace URIs determine
identity, regardless of the prefix. XML 1.0 declarations are optional; other
declared versions/encodings, DTDs, undefined entities, duplicate attributes,
multiple roots, and incomplete nesting are rejected. Unknown ordinary elements
are retained in the raw XML. Parsed identity/product attributes include unknown
attributes; they are not a complete XML tree.

Barrels require `barrel.jungle` and the observed canonical
`content/manifest.xml`; outer/content manifests must agree on barrel kind, `id`,
`module`, and `version`. Other attributes and raw XML can differ. There is no
Jungle evaluator or arbitrary build-manifest path selection yet.

IQ manifests must enumerate at least one direct `application/products/product`,
each with nonempty `filename` and `partNumber`. Every filename must select an
existing ordinary member with a safe relative path. All descriptors are
retained; multiple descriptors may select the same member. No mapping from part
number to marketing device ID is assumed. The reader retains `sig`, `sig2`, and
other attributes as strings; the separate verification stage interprets the
supported signing attributes.

### Resource bounds and supported codecs

`Limits::default()` is used by the CLI; library callers can supply other
explicit limits.

| Bound | Default |
| --- | ---: |
| Compressed archive input | 64 MiB |
| Central/next header | 4 MiB |
| Number of entries/blocks/substreams | 4096 |
| One decoded member | 128 MiB |
| Total decoded bytes | 1 GiB |
| One package manifest | 2 MiB |
| LZMA/LZMA2 dictionary | 64 MiB |
| One member path in UTF-8 bytes | 1024 |

XML nesting, attributes per element, and namespace declarations per element are
each limited to 128. Extraction copies through a fixed 64 KiB buffer; only
bounded manifest bodies are collected into vectors. Decode work can still be
substantial within these bounds; they are size/allocation limits, not a CPU
timeout or a bound on total process memory. Sevenz decoding uses one thread.

Own header preflight validates counts, sizes, coder properties, and stream/file
relationships **before** calling backends which allocate from metadata. The
codec libraries still parse headers and perform decompression/checksum handling;
this crate does not implement compression algorithms.

Supported ZIP: ordinary single-disk classic ZIP, UTF-8 names, Store and Deflate,
including the SDK's data descriptors. ZIP64, split archives, non-UTF-8 legacy
names, encryption, special entries, and other methods are rejected. The central
directory must end immediately before the end-directory record, allowing its
ordinary archive comment.

Supported 7z: an unencoded next header or the bounded single-LZMA encoded-header
profile described in [IQ repacking](#iq-repacking), internal file metadata, and
one simple Copy, LZMA, or LZMA2 coder per block, with optional substream
descriptions. Simple solid streams are exercised with independently authored
Copy fixtures; the real SDK specimen uses independent LZMA2 blocks. Mixed
per-block CRC availability with an explicit substream CRC table is rejected.
Other encoded-header profiles, external metadata, coder chains/graphs,
encryption, anti-items, and unknown file properties are rejected until bounded
support is implemented. Optional known timestamp/attribute fields are parsed by
the backend; extra ordinary package files remain supported.

These restrictions are implementation limits, not claims that other Garmin
packages cannot use those features.

### Output lifecycle and integrity

Validate the whole inventory before creating output. Reject absolute/drive/UNC
paths, backslashes, colons, control/NUL characters, empty/`.`/`..` components,
trailing dot/space components, common Windows reserved names, duplicate paths,
file/directory conflicts, and case aliases in files or parents (comparison uses
Unicode lowercase). Reject Unix special-file/link modes and 7z reparse-point
attributes. Output parents are built only in our private staging tree, and files
use exclusive creation. Filesystem-specific Unicode normalization is not
modeled; compatibility testing here is on Linux.

Reserve a new destination and a private sibling staging directory (mode 0700 on
Unix). Decode all members to the staging tree, checking exact declared sizes and
available decoded-member CRCs. An empty 7z member's CRC, if present, must be
zero. CRCs are accidental-corruption checks, not signature verification; absent
CRCs provide no checksum assurance. Compressed-stream CRC coverage is not
independently promised.

Publish staged top-level entries only after decoding succeeds. On an error,
remove only the destination/staging tree created by this invocation. An existing
destination is never removed or overwritten. Publication uses several renames,
not an atomic whole-directory transaction; cleanup after an I/O error is best
effort, and process termination can leave a private empty/partial destination or
staging tree. The API reports I/O failures rather than asserting success.

## Barrel format

Reading and extraction support
[the supported profile](#shared-reading-and-extraction); deterministic
[canonical-tree packing](#barrel-packing) supports the canonical exported tree.
Evidence: [package investigation](../../investigations/package-formats.md), SDK
9.2.0 / `fr265`, and the earlier
[library experiment](../../investigations/multi-file-and-barrels.md). This is the
verified profile of our specimens, not a claim about every historical SDK.

### Archive and members

A `.barrel` is a ZIP archive. The SDK emits Deflate entries (method 8), ordinary
ZIP CRC-32, UTF-8 filenames, and data descriptors (general flags `0x0808`). Our
independent ZIP repacks with both Store (method 0) and Deflate, reversed member
order, different timestamps/attributes, and no data descriptors are accepted by
the SDK and produce identical CODE/DATA. Those ZIP representation details are
not semantic requirements of this profile. No package-level signature or
developer key appears in these specimens.

Observed layout:

```text
manifest.xml
barrel.jungle
content/manifest.xml
content/source/Numbers.mc
content/resources/strings.xml   # present in the resource variant
```

`manifest.xml` is the outer library declaration. `content/manifest.xml` is the
build manifest selected by the generated Jungle. Both are byte identical in our
specimens; whether differing declarations are legal is unverified. Our first
writer requires coherent library identity and byte-identical copies, preserving
both rather than rewriting them.

The namespaced root is `iq:manifest` (`http://www.garmin.com/xml/connectiq`),
version `3`, containing `iq:barrel` with `id`, `module`, `version`, and
`minApiLevel`, plus products, permissions, languages, dependencies
(`iq:barrels`), and annotations. The observed module/version are `Numbers` /
`1.0.0`. Manifest parsing must preserve unknown attributes/elements and raw XML
for unchanged repacking; a source-format extension is not implied by this
layout.

The root `barrel.jungle` selects the content manifest and source/resource
configuration:

```text
project.manifest = content/manifest.xml
base.sourcePath = content/source
fr265.personality = content/resources
```

This is the actual generated file in our small specimen, not a mandatory
complete grammar. Default resources resolve relative to the selected manifest
directory, so the resource variant does not need an explicit `base.resourcePath`
line. Other Jungles may contain device/language conditions, resource paths,
exclusions, annotations, or library dependencies. Preserve Jungle content as
UTF-8 source initially; do not pretend that handling this example is a complete
Jungle evaluator.

Monkey C source and resource members are packaged unchanged. An independently
authored string resource appears as `content/resources/strings.xml`, byte
identical to its input. No compiled PRG/object member is present. Barrel
consumption is part of application compilation; extraction alone does not
produce a runnable program.

### Reader/writer scope

The reader must recognize ZIP framing, inventory members, parse library identity
and the canonical `content/manifest.xml`, and extract source/resources as opaque
member bytes. Arbitrary Jungle-selected manifest paths are deferred. Unknown
ordinary members remain available for preservation. Validate CRCs and reject
duplicate or conflicting paths, unsupported compression/encryption, links, and
out-of-root paths rather than overwriting another entry.

The writer must repack an already canonical extracted package tree, validating
the required manifests and `barrel.jungle`, then emit deterministic ZIP/Deflate
with stable order/timestamps. Its success criterion is equal member bytes and
SDK consumption, not identical compressed archive bytes. A later
project-to-barrel builder can construct the `content/` mapping explicitly;
arbitrary Jungle interpretation, source syntax checking, and compilation are
separate work.

Two entries with the same filename are not two declarations to merge. Namespace
resolution and generated module bytecode remain the compiler's responsibility.
Adding/removing library sources is not a PRG relocation operation.

Limits still to investigate: nested dependencies, annotation selection,
localization, device-specific assets, multiple Jungle inputs, optional
metadata/licensing files, ZIP64/large members, and archive formats in other SDK
versions. See [package design](#shared-reading-and-extraction).

## Barrel packing

Packing uses `ciq-package::pack_barrel(source, output, Limits)` and:

```sh
ciq-package unpack library.barrel scratch/library
ciq-package pack-barrel scratch/library scratch/library-repacked.barrel
```

Evidence:
[SDK consumption and round trips](../../investigations/barrel-packing.md). This is
a writer for the [canonical barrel profile](#barrel-format), not a project
builder or Jungle interpreter.

### Input and preservation

The input is an ordinary directory containing ordinary `manifest.xml`,
`content/manifest.xml`, and `barrel.jungle` files. Both manifests must be **byte
identical**, valid supported manifest XML, and declare a barrel identity. The
reader permits differing XML with matching identity; this writer deliberately
requires identical copies instead of deciding which to rewrite. XML and Jungle
bytes are never regenerated. Source, resources, and unknown ordinary files are
copied unchanged.

Input names must be UTF-8 and satisfy the reader's portable path policy,
including case collision checks. Links and special files are rejected. Every
directory discovered counts toward the input entry limit; files count toward
per-member and total decoded-byte limits. Manifests have their separate size
cap. Empty directories are emitted; nonempty directories are represented by
descendants. Extraction cannot recover whether an original archive explicitly
recorded a nonempty directory, so directory record identity is not promised.

The source must remain unchanged while packing. Observed type and size changes
are rejected, but filesystem inspection and opening are separate operations.
This API does not provide a snapshot or protection against concurrent malicious
filesystem replacement.

### ZIP representation

Members use ascending Rust string order of their UTF-8 names, Deflate level 6,
DOS timestamp 1980-01-01 00:00:00, ordinary file permissions 0644 and empty
directory permissions 0755. No source timestamps, permissions, archive comment,
or application-specific extra metadata are copied. Determinism applies to
unchanged contents and this pinned ZIP/Deflate implementation; codec upgrades
may change compressed bytes.

The compressed ZIP buffer is bounded by `archive_bytes`, including backpatched
headers. Before writing any output file, the archive passes the existing
reader's preflight, manifest/profile, path, decoded-size and CRC checks. This
also enforces the header cap and rejects unsupported ZIP64. The implementation
retains a bounded compressed buffer and a reader copy during validation, and
streams file contents rather than retaining all decoded members.

### Output lifecycle

The output's parent must exist. The output must be outside the canonical source
tree and must be a new file; existing files or symlinks are never overwritten.
Invalid input or archive validation leaves no output file. After validation,
exclusive creation is followed by writing and flushing. A write/flush failure
removes the file created by this operation on a best-effort basis. Publication
is not atomic, no durability sync is promised, and process termination can leave
a partial output.

No developer key or signature is required for this observed source-barrel
format. IQ writing uses its own [package profile](#iq-format-and-signing).

## IQ format and signing

Supported operations are [reading/extraction](#shared-reading-and-extraction),
modern developer [verification](#iq-verification),
[unchanged repacking](#iq-repacking), and
[own-key creation](#iq-creation) using supplied modern metadata.
Evidence: [independent archive/signature
investigation](../../investigations/package-formats.md), SDK 9.2.0 / `fr265`.
This profile covers our own dual-hash developer-signed export. Store delivery,
legacy variants, and general device catalogues are not specified here.

### Archive

Our `.iq` is a 7z archive. Detect the container from bytes, then identify its
package role from its manifest; do not rely on an extension alone.

| Start-header position | Width | Meaning |
|---|---:|---|
| 0 | 6 | `37 7a bc af 27 1c` |
| 6 | 2 | 7z version, observed `00 02` |
| 8 | 4 | Little-endian CRC-32 of bytes `[12,32)` |
| 12 | 8 | Little-endian next-header offset, relative to byte 32 |
| 20 | 8 | Little-endian next-header size |
| 28 | 4 | Little-endian next-header CRC-32 |

Delegate the complete 7z grammar and decompression to an archive library. In our
specimen, the next header is unencoded (`01`), filenames are encoded as
UTF-16LE, and five files have separate compressed folders/streams. Every stream
uses LZMA2 (coder ID `21`, property byte `16`); the archive is not solid and not
encrypted. These are observed compression choices, not requirements to reject
other supported 7z layouts. Support encoded headers/solid streams through the
backend within explicit resource limits; reject encryption and unsupported
codecs initially.

### Package tree and manifest

```text
manifest.xml
manifest.sig2
dev_key.pub
006-B4257-00/barrel-app.prg
006-B4257-00/debug.xml
```

`dev_key.pub` is DER SubjectPublicKeyInfo with the `rsaEncryption` OID
(`1.2.840.113549.1.1.1`) and NULL parameters. Our 550-byte member contains an
RSA-4096 modulus and exponent 65537. It is public material. The private
developer key is not a package member.

The root XML is namespaced `iq:manifest` version `3` with an `iq:application`
and `iq:products`. Each observed `iq:product` supplies `filename`, `partNumber`,
`connectIqVersion`, `minFirmwareVersion`, `sig`, `sig2`, and
`sigFormat="rsa-4096"`. This specimen uses
`filename="006-B4257-00/barrel-app.prg"` and `partNumber="006-B4257-00"`. The
application retains its ID, type, entry class, and resource references and adds
`apiVersion`.

Paths come from the manifest; do not assume an SDK device ID such as `fr265` is
the directory name. The reader must enumerate all products, not choose the first
one silently. Multiple products may need to reference the same file; establish
sharing semantics before enforcing one path per product. `debug.xml` is external
debug information; it is not required to execute the extracted PRG, and its
inclusion policy across export modes is still open.

Our export's PRG has 11 sections: SYMBOLS/DEBUG absent and LINE_NUMBERS an empty
count. This is an SDK export result, not a package-writer requirement to strip
debugging sections from arbitrary input PRGs.

### Verified signatures

All observed signatures use RSA-4096 PKCS#1 v1.5. Direct public-RSA verification
establishes:

1. Product `sig` is 1024 hexadecimal characters decoding to the PRG's 512-byte
   SHA-1 developer signature.
2. Product `sig2` is 1024 hexadecimal characters decoding to its 512-byte
   SHA-256 developer signature.
3. Both cover the [PRG developer-signature message](developer-signatures.md):
   ordered non-signature section wrappers and payloads, excluding
   developer/store signing sections and the zero terminator. They do **not**
   sign the entire PRG byte stream.
4. `manifest.sig2` is a raw 512-byte SHA-256 signature over the **exact
   uncompressed `manifest.xml` bytes**, with no XML canonicalization. Adding a
   newline makes verification fail.
5. The SPKI public key matches the public components embedded in the device PRG.

There is no archive-byte signature in this specimen. Recompression/timestamps
alone need not change these signatures if every signed member remains unchanged.
The observed signature messages do not include `debug.xml` or a digest of it;
its content is not authenticated by these signatures. Member CRCs detect archive
corruption; they are distinct from RSA signatures. Do not describe a successful
signature check as store acceptance or trust in a publisher: embedded-key checks
establish integrity, and comparison with an explicitly supplied public key
establishes identity relative to that key.

`manifest.sig` is a source-derived older SHA-1 variant, not experimentally
verified here. Missing/unsupported signing variants must be reported explicitly;
do not silently call a partially checked package valid. Signature requirements
for store-signed or other SDK packages remain open.

### Own-key writing sequence

Implemented modern developer-package writer (see
[exact input restrictions and debug policy](#iq-creation)):

1. Validate all selected PRGs, required product metadata, safe filenames, and
   supported signature variants.
2. Re-sign every product with the supplied external developer key and verify
   each PRG. Remove stale signing material using the existing PRG signing
   policy.
3. Set each product's `sig`/`sig2` from its resulting PRG signatures. Write
   `dev_key.pub` as SPKI for that key.
4. Serialize the final manifest **once**; sign those exact bytes with SHA-256
   into `manifest.sig2`.
5. Archive these exact bytes and members as 7z; reopen and verify the final
   product/member relationships and signatures.

Unknown XML must survive a controlled edit or cause a clear unsupported-writing
error. Do not infer firmware/API/product metadata from bytecode alone. Preserve
a supplied debug XML or deliberately omit it where supported; PRG assembly edits
do not automatically update its ranges/source mappings. A low-level
unchanged-member repack can preserve existing signatures without any private
key, but a changed product/manifest requires renewed corresponding signatures.

The writer targets SDK-produced developer packages. IQ archive acceptance by
Garmin upload/install workflows has not been tested independently; existing
simulator runs only validate extracted/reassembled PRGs. See
[package design](#shared-reading-and-extraction).

## IQ verification

`ciq-package` verifies IQ packages using `ciq-prg` and `ciq-sign`. The
format/coverage reference is [the observed modern IQ
profile](#iq-format-and-signing);
[the investigation](../../investigations/package-verification.md) records SDK and
synthetic validation. Archive acceptance still follows
[the package reader's bounds](#shared-reading-and-extraction).

### Commands and trust

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

### Report and supported profile

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

### Memory and errors

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

## IQ creation

Implemented for supplied modern developer-package metadata in `pack_iq(source,
output, key, DebugPolicy, Limits)`:

```sh
ciq-package pack-iq scratch/application scratch/edited.iq -y keys/developer_key.der
ciq-package pack-iq scratch/application scratch/edited-with-debug.iq -y keys/developer_key.der --debug preserve
```

The source is an extracted IQ-style directory. Its manifest must be a supported
version-3 application manifest with `apiVersion`. Every product needs safe
`filename`, `partNumber`, `connectIqVersion`, and `minFirmwareVersion` values
and an existing ordinary PRG. These metadata values are supplied, not inferred
or fetched. `sigFormat` may be missing or `rsa-4096`; other formats are
unsupported. Existing manifest/product signatures and the old package public key
may be absent or stale because they are regenerated. Input PRGs may be unsigned
or use the supported dual-hash developer format. SHA-1-only, unknown
developer-signature, and store PRG profiles are rejected; a legacy
`manifest.sig` member is also rejected.

The private RSA-4096 PKCS#8 DER key remains external. The CLI bounds its input
to 64 KiB and refuses a key located within the source tree. No private key is
included in generated metadata. The library accepts an opaque `DeveloperKey`
rather than a filename.

For each unique selected PRG, preserve non-signature section bytes/order,
replace developer signing material with signatures from the supplied key, and
verify the result. Shared product paths are signed once and all descriptors
updated. Replace/add only `sig`, `sig2`, and `sigFormat` on direct Garmin
product elements. All other raw XML bytes survive, including unknown
attributes/elements, namespace declarations, comments, whitespace, quote styles,
and escaped filenames. Missing signing attributes are inserted; unknown
surrounding XML is not reserialized. Reparse the final manifest and enforce its
size limit. Write the key's SPKI as `dev_key.pub` and sign the exact final
manifest bytes with SHA-256 into `manifest.sig2`.

### External debug policy

Default `--debug omit` drops ordinary files whose basename is `debug.xml` or
ends in `.debug.xml`. It prevents carrying old external mappings forward by
default after code changes. `--debug preserve` retains supplied bytes exactly
and makes no assertion that they match edited code. No debug mappings are
regenerated. Other unknown ordinary members remain opaque and unchanged.
Embedded PRG debug/line sections remain under the assembler's metadata policy;
IQ creation does not fix them. Product filenames must not conflict with
generated signing metadata or an omitted debug filename.

### Output and limits

Validate the source tree using the common packing path/count/size rules. Stage
generated PRGs and preserved ordinary payloads in a private sibling directory,
holding one product at a time in memory. The source tree is never modified.
Generate final manifests/key/signature in that directory, then use
[verified deterministic IQ repacking](#iq-repacking) to encode and check the
archive before exclusive output creation. It checks output limits and all
product/package signatures. Empty directories remain unsupported by the pinned
IQ writer.

The source must remain unchanged during inspection/copying. Temporary decoded
disk usage is bounded by the accepted input tree plus generated signing data,
and final decoded limits are rechecked. Private staging cleanup is best effort
on success/error; process termination can leave a private stage. Publication has
the same non-atomic write and cleanup limits as repacking. Staging directories
are Git-ignored. Existing output files/symlinks are never overwritten.

[Evidence](../../investigations/iq-creation.md) includes independent archive
decoding, alternate-key tests, and simulator execution of an assembly-edited PRG
extracted from the created IQ. Upload/store/install acceptance of the IQ archive
itself remains untested. Real multi-device and historical/store variants remain
outside the supported creation profile.

## IQ repacking

Implemented in `ciq-package::repack_iq(source, output, Limits)` and:

```sh
ciq-package unpack application.iq scratch/application
ciq-package repack-iq scratch/application scratch/unchanged.iq
ciq-package verify scratch/unchanged.iq --expected-key scratch/application/dev_key.pub
```

The source is an extracted IQ tree with its existing manifest, public key,
signatures, product PRGs, debug files, and any unknown ordinary files. All
ordinary file bytes are retained. No private key is loaded, signatures are never
regenerated, and manifest XML is not reserialized. Changed signed inputs that
invalidate verification are refused. The operation cannot prove equality to an
unavailable original archive; unchanged means that the writer itself does not
edit the supplied files. Unsigned extras are preserved but remain outside
signature coverage.

The output is a deterministic 7z archive: sorted UTF-8 paths, independent
single-thread LZMA2 streams at preset 6, no filesystem timestamps or attributes,
and the pinned writer's ordinary or single-LZMA encoded header. No encryption is
enabled. Empty files receive data streams. Nonempty directories are represented
by descendants; **empty directories are explicitly unsupported** because the
pinned backend incorrectly emits anti-item flags for streamless entries. This
limitation is recorded in
[the investigation](../../investigations/iq-repacking.md), and does not change
reader support for ordinary directories.

Input path, entry-count, member/total-size, manifest-size, source stability, and
new-file publication rules are shared with [barrel packing](#barrel-packing).
Output must be outside the source tree, with an existing parent. The compressed
output buffer is bounded, codec dictionaries must fit the supplied dictionary
cap, and final archive/header limits are checked before publication. Fixed
encoder working memory is additional to dictionary storage; limits are not a
total-process-memory or CPU-time guarantee.

Before output creation, the archive is reopened and the complete modern
developer [verification profile](#iq-verification) must pass, including decoded
sizes/CRCs, exact-byte manifest signatures, product attributes, PRG signatures,
and key consistency. Older, store, unsigned, and otherwise unsupported signature
profiles are rejected. Existing output files/symlinks are never replaced. Write
failure cleanup is best effort; process termination can leave a partial output.

Encoded-header reading permits only one internal LZMA stream with bounded
compressed and decoded header sizes, a bounded dictionary, and a decoded CRC.
Start/next-header and present packed-header CRCs are checked. Decoded metadata
passes the existing preflight before archive-backend allocation. Nested
encoding, other header codecs, graphs, external metadata, and encryption remain
unsupported. Internally the decoder reframes a raw header; this can add up to
the header cap to the bounded archive buffer. It does not alter the archive
written to disk or any extracted file bytes.

Successful verification establishes integrity under the embedded key.
Caller-supplied expected public keys establish identity relative to that key.
Garmin store/upload/install acceptance of this new archive representation
remains untested.
