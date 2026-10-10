# Rust barrel packing and SDK consumption

Date: 2026-10-10. Question: can an independently implemented canonical-tree
writer preserve all barrel payloads and produce a library Garmin's SDK consumes
equivalently?

## Implementation and provenance

The implementation follows our [barrel profile](../specs/barrel-package.md),
[packing specification](../specs/barrel-packing.md), and
[package plan](../design/package-crate.md). No external research repository's
implementation was copied. Existing pinned `zip 6.0.0` (MIT) supplies
ZIP/Deflate writing; no dependency or lockfile changes were needed. Codec
versions, license/advisory review and Rust 1.85 baseline evidence are recorded
in [reader research](package-reading.md).

## Reproduction

Inputs are the independently authored SDK 9.2.0 code and resource barrels from
[format research](package-formats.md). Output is ignored under
`scratch/barrel-packing/`:

```sh
ciq-package unpack examples/number-barrel/build/number-barrel.barrel scratch/barrel-packing/code-tree
ciq-package pack-barrel scratch/barrel-packing/code-tree scratch/barrel-packing/code.barrel
ciq-package unpack scratch/package-formats/resource-barrel/build/resource-barrel.barrel scratch/barrel-packing/resource-tree
ciq-package pack-barrel scratch/barrel-packing/resource-tree scratch/barrel-packing/resource.barrel
```

Repeat each pack with a different new output name and compare bytes. Independent
Python `zipfile` inspection compares member name sets and every uncompressed
payload against the SDK original, and confirms fixed timestamps and permissions.

For each barrel, copy `manifest.xml`, `source/`, `resources/`, and
`monkey.jungle` from `examples/barrel-app/` into separate ignored
`<kind>-original` and `<kind>-packed` consumer directories. Set
`base.barrelPath` to the absolute original/packed archive path. Compile each
with:

```sh
sh scripts/build-example.sh scratch/barrel-packing/code-original fr265
sh scripts/build-example.sh scratch/barrel-packing/code-packed fr265
sh scripts/build-example.sh scratch/barrel-packing/resource-original fr265
sh scripts/build-example.sh scratch/barrel-packing/resource-packed fr265
```

All four fresh PRGs were produced successfully with SDK 9.2.0, `fr265`, `-O0`,
and the local developer key. Independent big-endian PRG section parsing compares
CODE (`0xc0debabe`) and DATA (`0xda7ababe`) payloads. Machine-readable results
remain in ignored `scratch/barrel-packing/comparison.json`.

## Experimentally observed results

| Packed specimen | Bytes | SHA-256 |
| --- | ---: | --- |
| code.barrel | 1215 | `d88d631f1ab53cc68b15236910e03a8b0f9aa327a430e8ccafd8dc18ec27a761` |
| resource.barrel | 1402 | `0f0c1535dd0705805bb5f880311545859a9f8bf5205492bc863fdb752a052fbb` |

Each has precisely the original ordinary member set (four/five files) and
identical payloads. Repeated packing is byte identical. Original/packed
consumers have identical CODE and DATA in both experiments:

- **Consumer:** code. **CODE SHA-256:**
  `8a79d7fcc72431842e43b8b9495b9aca707ed0da86ef4edff7889352d7e5557a`. **DATA
  SHA-256:** `5d912cf97b828aaa2ff9209389772ef4e4d8ee65196918311d4fa7a2226b16c8`.

- **Consumer:** resource. **CODE SHA-256:**
  `9f55e93780c66bd46d59b0f869d881f27c97aa07e6382efd8d5342be3ea730fb`. **DATA
  SHA-256:** `514e211891b78922408a99503003cd03a20fe6be67cef8177e884d40d73dbb38`.

This establishes SDK consumption and CODE/DATA equivalence, including
compilation with a resource-bearing library. The consumer does not explicitly
load the library's `Greeting` resource. Full PRG identity is not asserted:
RESOURCES differed in these separately named output builds, and its cause was
not investigated. No new runtime capture or IQ distribution acceptance is
claimed.

Five new integration tests cover payload/empty-directory preservation,
creation-order/timestamp/permission determinism and CLI packing,
mismatched/invalid manifests, existing/in-source output refusal, input/output
limits, symlinks, and case collisions. All 99 workspace tests pass on the
container’s Rust 1.99; formatting and Clippy
(`--workspace --all-targets -- -D warnings`) pass. This change was not rerun on
Rust 1.85; the prior 1.85 verification remains historical evidence. Next
implement unchanged IQ repacking with exact signed member bytes retained, before
introducing own-key IQ creation.
