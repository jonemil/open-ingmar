# Branches and exception metadata

Keep concrete instruction encodings in `ciq-mc`, named branch binding in
`ciq-assembly`, and exception records in `ciq-prg`. Source conditions and loops
lower through semantic blocks; source exception handling still needs unwinding
and frame research. See [compiler representation](compiler-ir.md).

## Assembly responsibilities

Expose relative-target instruction fields without resolving names in the
instruction crate. Assembly resolves typed CODE-label operands after layout;
methods, line metadata, branches, and exception records share labels.

Undefined labels, invalid target boundaries, and displacement overflow are
errors. Do not silently truncate operands or invent long-branch expansion.
Numeric operands preserve raw bits and do not imply automatic relocation.
[CODE-label rules](../specs/code-labels.md#relative-instruction-operands) specify
encoding and target constraints.

## Exception section responsibilities

Preserve ordered records without sorting, merging, or deduplicating them.
Unsupported layouts retain raw fallback. Structured records can refer to CODE
labels for range boundaries and handlers; resolve them after CODE layout.
The [exception specification](../specs/exceptions.md) defines framing and
accepted source, including the distinction between range endpoints and handlers.

Keep unverified interval semantics explicit. Branch and exception fixups alone
do not establish general relocation safety: length-changing edits must account
for header offsets and opaque references as well.

## Validation

Cover forward/backward branches, displacement endpoints and overflow, invalid
targets, ordered exception preservation, and native/text equivalence. Use
controlled runtime experiments to establish changed behavior separately from
successful reconstruction. Evidence: [branch
labels](../investigations/branch-labels.md) and [exception
assembly](../investigations/exception-assembly.md).
