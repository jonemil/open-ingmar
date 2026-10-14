# Natural lowering review

Date: 2026-10-14. Environment: SDK 9.2.0, `fr265`, SDK specimens built with
`-O 0`. Question: where do source → IR → VM assembly paths substitute workaround
sequences for established VM instructions?

## Review findings

| Source operation | Semantic representation | Instruction selection |
|---|---|---|
| Boolean `!value` | Typed `Not` / `.not` | Direct `invv` |
| Equality/inequality with literal null | `Compare` and a `Null` definition | Direct `isnull`/`isnotnull` on the other operand |
| Number `~value` | Parser expands to XOR with -1 | Needs explicit unary representation before direct `invv` |
| Array literal | Empty typed `ArrayNew`, staged values, native `add` calls | Needs construction representation before direct fill/load selection |
| Numeric binary operations/comparisons | Typed `Binary` / `Compare` | Corresponding VM arithmetic/comparison instructions |

**Interpretation:** Boolean negation and literal-null comparison already retain
their meaning sufficiently in IR. Their direct backend selection needs no IR
grammar change. A literal null has no evaluation effects; selecting a unary
check preserves the other operand's evaluation once. This is local instruction
selection, without constant folding or an optimization pass. Comparison with a
stored null-valued operand remains binary because its definition is a read,
not a null literal. Existing type checking and flow proofs still apply.

Number complement loses its unary form during parsing. Array literals lose
construction identity when expanded into native append calls. Those source/IR
boundaries need review before the backend can choose their natural encodings.
Do not recognize arbitrary XOR/native-call sequences as substitutes for the
missing semantic representation. See [the plan](../plan.md#next-work).

Field reads already use direct `getselfv`; capture/staging locals preserve
compound-target order across effects and branches. The SDK's stacked `getv`
sequence is an alternative encoding, not a requirement to duplicate its stack
layout. Native/user calls use member lookup and invoke instructions. Literal
integer widths and arithmetic are already selected directly.

## Boolean and null selection evidence

The SDK encoding/runtime evidence is in [inversion](invert-opcode.md) and
[null checks](null-check-opcodes.md). Independent compiler tests verify:

- `.not` selects one `invv`; `!true` emits `35 01 3e 2d 16`.
- Either operand order for nullable Number comparison with literal null selects
  `argc`, `lgetv 0x01`, `isnull`/`isnotnull`, `return`.
- A native conversion result is evaluated once before its null check.
- `!(value == null)` retains both operations (`isnull`, then `invv`), rather
  than folding the inversion. Ordinary comparison with zero still uses `eq`.
- Source/resumed IR and assembly parsing remain equivalent; existing rejected
  operand types, evaluation-order checks, and nullable branch proofs still pass.

Rebuild the [Boolean-negation example][boolean] and
[nullable-contract example][nullable] into fresh ignored output paths using
their documented compiler/signing commands. The validation outputs here are
`scratch/natural-lowering/boolean.prg` and `nullable.prg`; each runs separately
through [the simulator helper](../../scripts/README.md).

Both generated logs match the retained SDK baseline logs, including effect
markers, guarded scalar results, and nullable retry behavior. Unsigned generated
artifact SHA-256 values:

- Boolean: `c4f1e6df649edbb1acefaff2075fb259149f3d52376ac5658c85861db4bad7db`.
- Nullable: `38d27867c99c5f0ac5f8ca8c86306d9be9e3dabd9be721a06546e47d149651f9`.

This validates direct selection for these profiles; it does not establish VM
compatibility for other SDK/devices or require byte equality with SDK output.
API-version declarations remain distinct from VM instruction availability.

The repeated audit is saved at
`scratch/example-audit/natural-null-boolean/report.json`. All 67 SDK and 48
generated PRGs retain fully structured CODE/DATA, exact reconstruction, and
identical repeat exports. All 62 documented source/resumed-IR commands and both
barrel member checks pass; the audit exposes no new decoding gaps.

[boolean]: ../../examples/compiler-boolean-negation/README.md
[nullable]: ../../examples/compiler-nullable-contracts/README.md
