# PRG container: supported subset

Evidence: [initial Rust investigation](../../investigations/rust-bootstrap.md).
This is a structural specification for the two local SDK 9.2.0 `fr265`
specimens, not a complete specification of every PRG version.

A file consists of ordered sections followed by an eight-byte zero terminator.
Each section has a four-byte big-endian identifier, a four-byte big-endian
payload length, and that many payload bytes. Length excludes the eight-byte
section header. Identifier zero is reserved for the zero-length terminator. Our
reader requires that terminator and rejects trailing bytes, truncated headers,
and payloads extending beyond the file.

Preserve section order, duplicate identifiers, unknown identifiers, and all
payload bytes. Ordinary code uses identifier `0xc0debabe`; its payload is an
instruction stream. A code operation requires exactly one ordinary code section.
Extended code (`0xc0de10ad`) has separate paging structures and is not decoded
yet.

The section inventory displays readable names for the identifiers catalogued in
[the investigation](../../investigations/rust-bootstrap.md), alongside their
numeric identifiers. Names are research-derived annotations, not evidence that a
section's payload has been decoded. Unrecognized identifiers are labelled
`UNKNOWN` and preserved.

The writer reconstructs the section headers and terminator. Replacing code
currently requires the same payload length, because internal addresses, line
metadata, and other references are not relocated. Equal length alone does not
establish validity: edits can invalidate references or signatures. The container
layer treats signature payloads as opaque and preserves them; the separate
signing layer can replace and verify developer signatures. Byte equality is a
preservation test, not a prerequisite for running reconstructed output. The
assembler `-y <key.der>` option replaces the original signature using a
user-supplied developer key; such output is expected to differ from the input.
See [the signing
boundary](../../architecture.md#preservation-layout-and-signing).

Reassembly tests with a different signing key must accept expected
signature/public-key differences. They must check the new signature's validity
and independently check program content, preserved raw sections, and runtime
behavior. Exact-byte preservation tests remain applicable when no signing or
content transformation occurs. See
[test expectations](../../validation.md).
