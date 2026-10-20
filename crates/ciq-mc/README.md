# ciq-mc

Library for the supported Connect IQ VM instruction encodings. Decoding returns
instructions with byte offsets; encoding preserves operand bit patterns.
Unsupported or truncated instructions produce errors.

Start with `decode`, `Instruction`, and `Opcode` in the [API source](src/lib.rs).
This crate handles concrete instruction bytes; symbolic references and layout
belong to `ciq-assembly`.

See the [instruction reference](../../docs/specs/vm/README.md#instruction-reference) for encodings,
observed semantics, and unverified behavior.
