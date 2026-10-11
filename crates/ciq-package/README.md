# ciq-package

Library and CLI for supported source barrels and IQ distribution archives:
member inventory, bounded extraction, verification, packing, and own-key signing.

From the workspace root (input paths are placeholders):

```sh
cargo run -p ciq-package -- list application.iq
cargo run -p ciq-package -- unpack application.iq scratch/package
cargo run -p ciq-package -- verify application.iq
```

Extraction destinations must be new. The [library API](src/lib.rs) exposes
`Package`, `Limits`, and packing functions. See [reading/extraction rules](../../docs/specs/package-reading.md),
[verification](../../docs/specs/package-verification.md),
[barrel packing](../../docs/specs/barrel-packing.md),
[IQ repacking](../../docs/specs/iq-repacking.md), and
[IQ creation](../../docs/specs/iq-creation.md) for command forms and limits.
PRG reconstruction/signing remains separate from package operations.
