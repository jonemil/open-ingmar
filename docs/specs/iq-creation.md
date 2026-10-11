# Own-key IQ creation

Implemented for supplied modern developer-package metadata in
`pack_iq(source, output, key, DebugPolicy, Limits)`:

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

## External debug policy

Default `--debug omit` drops ordinary files whose basename is `debug.xml` or
ends in `.debug.xml`. It prevents carrying old external mappings forward by
default after code changes. `--debug preserve` retains supplied bytes exactly
and makes no assertion that they match edited code. No debug mappings are
regenerated. Other unknown ordinary members remain opaque and unchanged.
Embedded PRG debug/line sections remain under the assembler's metadata policy;
IQ creation does not fix them. Product filenames must not conflict with
generated signing metadata or an omitted debug filename.

## Output and limits

Validate the source tree using the common packing path/count/size rules. Stage
generated PRGs and preserved ordinary payloads in a private sibling directory,
holding one product at a time in memory. The source tree is never modified.
Generate final manifests/key/signature in that directory, then use
[verified deterministic IQ repacking](iq-repacking.md) to encode and check the
archive before exclusive output creation. It checks output limits and all
product/package signatures. Empty directories remain unsupported by the pinned
IQ writer.

The source must remain unchanged during inspection/copying. Temporary decoded
disk usage is bounded by the accepted input tree plus generated signing data,
and final decoded limits are rechecked. Private staging cleanup is best effort
on success/error; process termination can leave a private stage. Publication has
the same non-atomic write and cleanup limits as repacking. Staging directories
are Git-ignored. Existing output files/symlinks are never overwritten.

[Evidence](../investigations/iq-creation.md) includes independent archive
decoding, alternate-key tests, and simulator execution of an assembly-edited PRG
extracted from the created IQ. Upload/store/install acceptance of the IQ archive
itself remains untested. Real multi-device and historical/store variants remain
outside the supported creation profile.
