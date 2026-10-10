# Monkey Barrel package: observed ZIP profile

Status: documented 2026-10-10; Rust reading/extraction implemented for
[the supported profile](package-reading.md); deterministic
[canonical-tree packing](barrel-packing.md) is implemented and SDK-validated.
Evidence: [package investigation](../investigations/package-formats.md), SDK
9.2.0 / `fr265`, and the earlier
[library experiment](../investigations/multi-file-and-barrels.md). This is the
verified profile of our specimens, not a claim about every historical SDK.

## Archive and members

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

## Reader/writer scope

Implemented reader: recognize ZIP framing, inventory members, parse library
identity and the canonical `content/manifest.xml`, and extract source/resources
as opaque member bytes. Arbitrary Jungle-selected manifest paths are deferred.
Unknown ordinary members remain available for preservation. Validate CRCs and
reject duplicate or conflicting paths, unsupported compression/encryption,
links, and out-of-root paths rather than overwriting another entry.

Implemented first writer: repack an already canonical extracted package tree,
validating the required manifests and `barrel.jungle`, then emit deterministic
ZIP/Deflate with stable order/timestamps. Its success criterion is equal member
bytes and SDK consumption, not identical compressed archive bytes. A later
project-to-barrel builder can construct the `content/` mapping explicitly;
arbitrary Jungle interpretation, source syntax checking, and compilation are
separate work.

Two entries with the same filename are not two declarations to merge. Namespace
resolution and generated module bytecode remain the compiler's responsibility.
Adding/removing library sources is not a PRG relocation operation.

Limits still to investigate: nested dependencies, annotation selection,
localization, device-specific assets, multiple Jungle inputs, optional
metadata/licensing files, ZIP64/large members, and archive formats in other SDK
versions. See [the implementation plan](../design/package-crate.md).
