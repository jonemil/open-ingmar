# Rust package reader and extraction

Date: 2026-10-10. Implements stage 1 of
[the package design](../specs/formats/packages.md#shared-reading-and-extraction), following
[the format investigation](package-formats.md). Scope: list and bounded
extraction, preserving exact payloads. No package signing/writing or new
simulator experiment. SDK specimens are the previously recorded SDK 9.2.0 /
`fr265` builds.

## Dependencies and evidence

**Source-derived:** codec adapters use independently published crates, with
exact direct versions and resolved dependencies in `Cargo.lock`:

| Library | Version | Declared Rust minimum | License |
| --- | --- | --- | --- |
| [zip](https://docs.rs/zip/6.0.0/zip/) | 6.0.0 | 1.83 | MIT |
| [sevenz-rust2](https://docs.rs/sevenz-rust2/0.20.2/sevenz_rust2/) | 0.20.2 | 1.85 | Apache-2.0 |
| [quick-xml](https://docs.rs/quick-xml/0.41.0/quick_xml/) | 0.41.0 | 1.79 | MIT |

ZIP enables only `deflate-flate2-zlib-rs`; sevenz and XML use no default
features. Actual runtime dependencies on Linux are `crc32fast 1.5.2`,
`cfg-if 1.0.5`, `lzma-rust2 0.16.5`, `memchr 2.8.3`, `flate2 1.1.10`,
`zlib-rs 0.6.8`, `indexmap 2.14.2`, `equivalent 1.0.2`, and `hashbrown 0.17.1`.
Their published manifests were checked: licenses are MIT/Apache alternatives,
Apache-2.0 for LZMA, MIT/Unlicense for memchr, and Zlib for zlib-rs. `crc32fast`
is also a dev dependency for our independent test encoders. No backend
implementation code was copied; those libraries are dependencies.

Backend source review at these released versions (`reader.rs`, `block.rs`,
`archive.rs` for sevenz; `read.rs` for ZIP) found metadata allocations which
cannot be bounded through the public constructor. Our `preflight` therefore
bounds headers before invoking constructors. In particular, 7z
file/folder/substream counts, packed/unpacked sizes, coder/dictionary
properties, filename terminators, and file/stream counts need checking. Encoded
headers are explicitly deferred rather than decoded without caller-controlled
bounds. `ArchiveReader::read_file` can allocate preceding members of a solid
block; our adapter instead uses entry callbacks and fixed-buffer draining. The
backend's outer callback traversal can still visit later blocks after a callback
returns false, so later callbacks immediately return without reading once the
manifest is found. Per-block codec allocation remains bounded.

**Dependency advisory review:** queried the
[OSV batch API](https://google.github.io/osv.dev/api/#osv-api) against all 69
registry versions in the lockfile on 2026-10-10. The initial XML candidate
0.38.3 matched [RUSTSEC-2026-0194][ref-rustsec-2026-0194] and
[RUSTSEC-2026-0195][ref-rustsec-2026-0195], affecting duplicate-attribute
complexity and namespace allocations before consumer checks. Selected patched
0.41.0 and configured its namespace limit. The final query reports no advisories
for newly introduced package dependencies. It does report the pre-existing
signing dependency `rsa 0.9.10` / [RUSTSEC-2023-0071][ref-rustsec-2023-0071];
this task neither changes that dependency nor adds cryptography. This is an
advisory snapshot, not a claim of a security audit. `cargo-audit` was
unavailable; the query result is local
`scratch/package-reader/osv-lockfile.json`.

ZIP 6 is outside the affected versions of
[RUSTSEC-2025-0168][ref-rustsec-2025-0168]. The implementation additionally
avoids backend convenience extraction functions and uses its own path/output
policy. Preserve these bounds when changing versions; do not infer safety from
choosing a recent package alone.

## Independent specimens and exact comparison

**Experimentally observed:** `ciq-package list` and `unpack` succeed for all
three specimens below. ZIP outputs were compared against Python stdlib `zipfile`
extraction; IQ outputs against the earlier independent libarchive extraction
under `scratch/package-formats/extracted/`. Compare both filename sets and every
member byte, not only selected fields.

- **Input:** `examples/number-barrel/build/number-barrel.barrel`. **Ordinary
  members:** 4. **Archive SHA-256:**
  `cd5cf17363b59566a6896a8dc063e0dfb748da490ad697e08d774876bc22a4df`.

- **Input:**
  `scratch/package-formats/resource-barrel/build/resource-barrel.barrel`.
  **Ordinary members:** 5. **Archive SHA-256:**
  `48d9b5167a485910a5ab09ef428e03ea8ded017bfb6ca1663f51387f314e4482`.

- **Input:** `scratch/barrels/barrel-app.iq`. **Ordinary members:** 5. **Archive
  SHA-256:** `7213919d07f6c93af812a0ee369cd76f3c7bcd7c6e9dd8b028b5ad2cd9a18746`.

All 14 payload comparisons passed. The extracted IQ `manifest.xml` remains 2985
bytes, SHA-256
`d2e2276f27d054122218d31f26d149dc3dcf186135d2b454204ea01ca00a2b9d`; device PRG
remains 25820 bytes, SHA-256
`3dbbeb65176f10d89ac943eed889730f7112e6ed1af823e730963f85f482414a`. This
preserves the exact manifest signing message and previously executed PRG. It
does not establish package signature verification by the new crate or
independent IQ installation acceptance.

Commands (destinations must be new; specimen creation is documented in the
linked format investigation):

```sh
cargo build -p ciq-package --locked
target/debug/ciq-package list examples/number-barrel/build/number-barrel.barrel
target/debug/ciq-package unpack examples/number-barrel/build/number-barrel.barrel scratch/package-reader/code-barrel
target/debug/ciq-package unpack scratch/package-formats/resource-barrel/build/resource-barrel.barrel scratch/package-reader/resource-barrel
target/debug/ciq-package list scratch/barrels/barrel-app.iq
target/debug/ciq-package unpack scratch/barrels/barrel-app.iq scratch/package-reader/iq
```

Local listing captures and `comparisons.json` retain every extracted member hash
under `scratch/package-reader/`. To reproduce the barrel comparison, build maps
`{name: archive.read(name)}` from `zipfile.ZipFile` and
`{relative_path: path.read_bytes()}` from extracted ordinary files and assert
equality. For IQ, construct the second map from each independently extracted
directory and compare them. Keep those specimens and reports ignored.

## Checks and supported scope

**Experimentally observed:** workspace tests pass with actual Rust **1.85.0**
(85 tests, including 18 new package tests); workspace Clippy with Rust 1.99 and
`-D warnings` and formatting checks pass. Existing PRG/signing tests remain
unchanged. Run:

```sh
cargo +1.85.0 test --workspace --locked
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
```

Tests construct classic stored ZIP and solid Copy 7z archives independently in
Rust, with explicit CRC/header fields; the Deflate test also uses the ZIP
writer. Fixtures contain our own tiny strings/opaque bytes and test manifests,
not SDK code, keys, or downloaded archives. Coverage includes
unknown-member/raw-XML preservation, multiple IQ products (including shared
filenames), source/resources, empty files/directories, traversal/drive/control
names, duplicate/case/parent conflicts, link/special-file modes and reparse
attributes, encryption/unsupported coders,
dictionary/input/header/member/total/count/path/manifest limits,
namespace/nesting complexity, malformed manifests/missing products, forged
counts, CRC damage, every truncated prefix, and systematic bounded next-header
byte mutations. CLI tests exercise both operations and output refusal. Corrupt
late payload tests verify no published files remain after failure and no staging
tree leaks.

**Implementation limits:**
[the reading specification](../specs/formats/packages.md#shared-reading-and-extraction) records the accepted
archive profile, output lifecycle, default limits, Linux verification boundary,
and unsupported ZIP64/encoded-header/coder-graph cases. This stage reads the
observed canonical barrel manifest path rather than evaluating Jungle. Parsed
metadata is a view; exact raw members remain authoritative. Simple solid Copy is
tested synthetically; additional real solid/LZMA/older/multi-device specimens
are needed before claiming broader Garmin compatibility.

Next implement stage 2: SPKI/detached signature APIs in `ciq-sign`, then package
verification with separate manifest, PRG/product, key-consistency, and optional
expected-key results. Barrel packing follows verification in the agreed
sequence.

Follow-up: [package signature verification](package-verification.md) implements
stage 2. The test/dependency boundary above records the reader milestone;
current workspace status is in the plan.

[ref-rustsec-2026-0194]: https://rustsec.org/advisories/RUSTSEC-2026-0194.html
[ref-rustsec-2026-0195]: https://rustsec.org/advisories/RUSTSEC-2026-0195.html
[ref-rustsec-2023-0071]: https://rustsec.org/advisories/RUSTSEC-2023-0071.html
[ref-rustsec-2025-0168]: https://rustsec.org/advisories/RUSTSEC-2025-0168.html
