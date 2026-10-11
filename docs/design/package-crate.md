# Package design

`ciq-package` owns archive inventory, manifests, extraction, verification, and
packing as a library and CLI. Keep it separate from individual PRG handling in
`ciq-prg`, cryptography in `ciq-sign`, and project resolution in the build
layer. A package is a distribution container, not a relocatable VM object.

## Representation and preservation

Retain archive kind, ordered member metadata, exact uncompressed member bytes,
and raw manifest bytes alongside parsed product/library fields. Unknown ordinary
members and XML fields must survive supported preservation operations. Avoid
parse-and-reserialize cycles for untouched signed manifests: their byte layout
is part of the signature message.

Model multiple products and shared product files explicitly. Package aliases do
not become VM symbols. Listing should avoid eagerly decompressing payloads;
codec adapters must bound any decoding required to inspect metadata.

Use maintained ZIP/7z/XML libraries with only the codec features needed by the
supported profiles. Keep dependency, license, and compiler-version checks in
upgrade validation; [dependency evidence](../investigations/package-reading.md)
records the applicable review.

## Extraction and publication

Inventory and validate paths before extracting. Reject traversal, links,
normalized collisions, and unsupported archive features. Enforce entry-count,
member-size, total-size, and codec-memory bounds. Stage output in a private
location; publish only after integrity and package checks succeed. On failure,
clean up only output created by that operation. Never execute archived content.

Outputs must be new paths. Inspection reports stay outside extracted package
content. Deterministic writers use stable member order, metadata, and codec
settings. Repacking tests member equality rather than equality of compressed
archive bytes.

## Signing and verification

Report archive integrity, manifest signatures, product signatures, key
consistency, and optional expected-key identity separately. Unsupported profiles
must not produce a successful aggregate verification result.

Unchanged IQ repacking preserves finalized signed bytes and requires no private
key. Own-key creation signs product PRGs, updates their manifest attributes and
public key, then signs the finalized manifest and archives the result. External
debug information needs an explicit preservation/omission policy. Member and
signature checks do not establish SDK installation or store acceptance.

## Supported operations and extension boundaries

Exact inputs, outputs, limits, and CLI behavior belong in the specifications:
[reading](../specs/package-reading.md),
[verification](../specs/package-verification.md),
[barrel packing](../specs/barrel-packing.md),
[IQ repacking](../specs/iq-repacking.md), and
[IQ creation](../specs/iq-creation.md).

Canonical-tree packing does not select project sources or evaluate Jungle.
[Project builds](project-builds.md) resolve those inputs and generate exported
configuration. Additional archive codecs, product/signature profiles, and
controlled member-output sinks require their own bounded contracts and evidence.
The [plan](../plan.md) owns sequencing.
