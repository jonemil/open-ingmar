# Package reading and extraction

`ciq-package` reads and extracts the supported profiles. Format references:
[barrels](barrel-package.md), [IQ packages](iq-package.md). Evidence and
dependency review: [reader
investigation](../../investigations/package-reading.md). This is a bounded
supported profile, not a general-purpose archive extractor.

## Representation

Retain archive kind, ordered member metadata, exact uncompressed content, and
raw manifests alongside parsed fields. Model multiple products/shared files
explicitly; package aliases are not VM symbols. Untouched signed manifests must
not be parsed and reserialized. Listing must not eagerly decompress all
payloads; codec adapters bound the work needed to inspect metadata. Reading,
signing, and project resolution remain separate
[architecture](../../architecture.md) layers.

## Commands and outputs

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
[verify command](package-verification.md) checks the modern developer IQ
profile and its PRGs. Canonical barrel packing and
[unchanged IQ repacking](iq-repacking.md) are supported.
[Own-key creation](iq-creation.md) signs edited packages; automatic disassembly
is outside the package tool. Pass an extracted PRG to the existing tools
explicitly:

```sh
cargo run -p ciq-dis -- all scratch/unpacked-application/006-B4257-00/barrel-app.prg scratch/unpacked-prg-project
```

Errors exit with status 1 and a diagnostic on stderr. Exact diagnostic wording
is not a stable API.

## Library and structural checks

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

## Resource bounds and supported codecs

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
profile described in [IQ repacking](iq-repacking.md), internal file metadata,
and one simple Copy, LZMA, or LZMA2 coder per block, with optional substream
descriptions. Simple solid streams are exercised with independently authored
Copy fixtures; the real SDK specimen uses independent LZMA2 blocks. Mixed
per-block CRC availability with an explicit substream CRC table is rejected.
Other encoded-header profiles, external metadata, coder chains/graphs,
encryption, anti-items, and unknown file properties are rejected until bounded
support is implemented. Optional known timestamp/attribute fields are parsed by
the backend; extra ordinary package files remain supported.

These restrictions are implementation limits, not claims that other Garmin
packages cannot use those features.

## Output lifecycle and integrity

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
