# ciq-assembly

Shared library for symbolic CODE assembly used by the compiler and assembler.
`Program` retains labels and symbol/String/Array/Dictionary references until
layout and encoding.
Typed construction and text parsing share validation.

Start with `Program::new`, `Program::parse`, and its printing/encoding methods
in the [API source](src/lib.rs). Complete PRG reconstruction is handled by
`ciq-asm`.

See [assembly specification](../../docs/specs/assembly/model.md),
[source conventions](../../docs/specs/assembly/model.md), and
[CODE label rules](../../docs/specs/assembly/labels.md).
