# Unchanged IQ writer and bounded encoded headers

Question: rebuild our SDK developer IQ package without a private key, preserving
all signed bytes and identity. Scope: stage 4 of
[the package plan](../specs/package-reading.md); own-key IQ creation remains
future work.

## Provenance and implementation

Our Rust adapter follows the repository's [IQ profile](../specs/iq-package.md)
and [repacking rules](../specs/iq-repacking.md). It shares tree inspection and
publication with the barrel writer. Pinned `sevenz-rust2 0.20.2` supplies
encoding; its `compress` feature enables the already locked `lzma-rust2 0.16.5`
encoder. Direct dependencies on that same LZMA version and `crc32fast 1.5.2`
support bounded header decoding and CRC checks. No registry package versions
changed. Both 7z/LZMA crates are Apache-2.0; crc32fast is MIT OR Apache-2.0.
Prior locked-version license/advisory findings remain in
[reader research](package-reading.md). Encryption and optional codec defaults
remain disabled. Our Rust code forbids unsafe code; the existing sevenz
dependency enables the LZMA codec’s `optimization` feature, as shown in the
recorded Cargo feature tree.

Source-derived backend behavior was inspected in the pinned Cargo registry
package, chiefly `sevenz-rust2/src/writer.rs`, `writer/unpack_info.rs`,
`encoder_options.rs`, and `lzma-rust2/src/lzma_reader.rs`. The writer always
attempts LZMA header compression and stores header CRCs in substream metadata.
This required bounded decoding before our existing metadata preflight, rather
than passing compressed headers directly to an allocating backend. No upstream
implementation was copied. Package sources are available at
[sevenz-rust2 0.20.2](https://docs.rs/crate/sevenz-rust2/0.20.2/source/) and
[lzma-rust2 0.16.5](https://docs.rs/crate/lzma-rust2/0.16.5/source/).

An empty-directory test exposed inverted anti-item selection in
`writer.rs::write_file_anti_items`: ordinary streamless entries receive anti
bits. Final validation rejected this output. Our adapter avoids streamless empty
files by encoding them as data streams and explicitly rejects empty directories.
We neither relax anti-item rejection nor modify the downloaded dependency.

## Real specimen experiment

Input: SDK 9.2.0 / `fr265` export `scratch/barrels/barrel-app.iq`, SHA-256
`7213919d07f6c93af812a0ee369cd76f3c7bcd7c6e9dd8b028b5ad2cd9a18746`, previously
extracted into `scratch/package-reader/iq/` and independently verified. Only the
public key from that tree was used for expected identity; no private key was
needed.

```sh
ciq-package repack-iq scratch/package-reader/iq scratch/iq-repacking/first.iq
ciq-package repack-iq scratch/package-reader/iq scratch/iq-repacking/second.iq
cmp scratch/iq-repacking/first.iq scratch/iq-repacking/second.iq
ciq-package verify scratch/iq-repacking/first.iq --expected-key scratch/package-reader/iq/dev_key.pub
ciq-package unpack scratch/iq-repacking/first.iq scratch/iq-repacking/extracted
```

Experimentally observed: repeated output is byte identical. Output has 13745
bytes, SHA-256
`e08f95170ada2211c7f249b2e4b460d4147d0057fa49c954387d8d8983cf8c54`. All five
ordinary members match the original extracted files exactly, including XML
whitespace, manifest signature, SPKI key, device PRG, and external debug XML.
Every modern developer verification check passes, including expected key
identity.

An independent Python/ctypes libarchive reader decoded the actual output file
and compared all five payloads, separately from our internal raw-header
reframing. The experiment script and per-member hashes are ignored under
`scratch/iq-repacking/compare-libarchive.py` and `libarchive-comparison.json`.
It uses `archive_read_open_memory`, `archive_read_next_header`, and
bounded-chunk `archive_read_data` against libarchive.so.13. This confirms
archive readability; it does not establish Garmin upload/install acceptance. The
unchanged extracted PRG already has simulator evidence in
[the multi-file investigation](multi-file-and-barrels.md); no new runtime result
is claimed here.

## Automated validation

Four new tests cover multi-product payload/signature preservation and
deterministic CLI output; changed manifest/PRG/signature/key rejection;
existing/in-source output and size limits; and encoded-header decoding limits,
CRC corruption, and unsupported layouts. Fixtures use independently authored
content and generated ephemeral RSA keys. No real private key is included in
tests.
