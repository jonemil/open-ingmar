# ciq-prg

Library for ordered PRG containers and supported section metadata. Unknown
sections remain opaque; unchanged parsing/writing preserves their bytes.

Start with `Prg::parse`, `Prg::sections`, and `Prg::to_bytes` in the
[API source](src/lib.rs). Modules provide DATA classes/strings/scalar arrays,
symbols, debug records, line mappings, exceptions, resources, and section
manifests.

See [container framing](../../docs/specs/prg-container.md),
[section projects](../../docs/specs/section-project.md), and
[architecture](../../docs/architecture.md) for supported formats and boundaries.
Instruction encoding belongs to `ciq-mc`; signing belongs to `ciq-sign`.
