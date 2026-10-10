# Branches and exception metadata

Add assembler control-flow support before extending the frontend with
conditions. The [controlled investigation](../investigations/control-flow.md)
establishes relative branch operands and a nonempty exception-table specimen.
It does not establish general exception execution or arbitrary relocation.

## Assembly responsibilities

Keep concrete operand bytes in `ciq-mc`. Expose which instructions contain a
relative CODE target and how to interpret that displacement, without resolving
names there. Start with observed `goto`, `bt`, `bf`, and `jsr` encodings;
comparisons, Boolean pushes, `throw`, and `ret` need their own documented
encodings. A local subroutine return is distinct from a function return.

In `ciq-assembly`, introduce a typed CODE-label operand. Proposed text syntax
uses the existing `code:` namespace:

```text
loop_test:
lgetv 0x01
ipushz
gt
bf code:loop_end
# loop body goes here
goto code:loop_test
loop_end:
lgetv 0x02
return
```

After layout, calculate `target - (instruction_offset + instruction_length)`.
Reject undefined labels, non-instruction targets, and displacements outside
signed 16-bit range. Do not silently truncate or invent long-branch expansion.
Numeric operands continue to preserve raw bits for inspection and lossless
assembly; they do not acquire automatic relocation.

Project disassembly should name verified branch-target instruction boundaries
using the same labels as methods and line PCs. Preserve numeric operands when
a target cannot be safely named. Tests must cover forward/backward branches,
range endpoints and overflow, invalid targets, and native/text equivalence.

## Exception section responsibilities

`ciq-prg` should parse and write the observed count and ordered triples without
sorting, merging, or deduplicating them. The explicit three-byte field width is
an encoding constraint, independently of the DATA-address investigation.
Unsupported section layouts retain the existing raw fallback.

Expose structured exception source in section projects, with numeric offsets
and optional CODE-label references for range starts, ends, and handlers.
Boundary labels may denote the end of CODE for a range endpoint, but handlers
must denote instructions. Confirm endpoint semantics before specifying strict
interval validation. Keep record order and preserve original bytes on unchanged
export/import. The text syntax is not yet defined.

Resolve this metadata after CODE layout, alongside method and line bindings.
Length-changing edits still require accounting for header offsets and other
opaque references. Branch and exception fixups alone do not establish general
relocation safety.

## Implementation order

1. Completed: observed instruction encodings and branch-label binding/export,
   with an [assembler-produced simulator edit](../investigations/branch-labels.md).
2. Add lossless exception parsing, writing, and project source. Reproduce the
   moved-function experiment through our assembler with named references.
3. Extend semantic IR and frontend conditions after documenting control-flow
   joins, value types, and stack requirements. Exception source syntax can remain
   deferred while exception-bearing scaffolds are preserved correctly.
