# Natural lowering review

Date: 2026-10-14. Environment: SDK 9.2.0, `fr265`, SDK specimens built with
`-O 0`. Question: where do source → IR → VM assembly paths substitute workaround
sequences for established VM instructions?

## Review findings

| Source operation | Semantic representation | Instruction selection |
|---|---|---|
| Boolean `!value` | Typed `Not` / `.not` | Direct `invv` |
| Equality/inequality with literal null | `Compare` and a `Null` definition | Direct `isnull`/`isnotnull` on the other operand |
| Number `~value` | Typed `Complement` / `.complement` | Direct `invv` |
| Array literal | Private begin/fill/finish construction, with grouped or staged elements | Direct `newa`, `aputvdup`, and `aputv`; constant-array selection remains open |
| Numeric binary operations/comparisons | Typed `Binary` / `Compare` | Corresponding VM arithmetic/comparison instructions |

**Interpretation:** Boolean negation and literal-null comparison already retain
their meaning sufficiently in IR. Their direct backend selection needs no IR
grammar change. A literal null has no evaluation effects; selecting a unary
check preserves the other operand's evaluation once. This is local instruction
selection, without constant folding or an optimization pass. Comparison with a
stored null-valued operand remains binary because its definition is a read,
not a null literal. Existing type checking and flow proofs still apply.

Array construction retains allocation, initialization, and publication in IR.
Constant-array selection and DATA binding remain the next source/IR boundary
to review before choosing `apush`.
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

## Number complement

Source `~value` now retains a unary parser node, `Operation::Complement`, and
`.complement` in saved IR. Both paths require a non-null Number and preserve
operand spans, single-use dependencies, effect scheduling, and nullable proofs.
Lowering emits the operand once followed by `invv`. Ordinary XOR stays binary;
`~~value` retains two unary operations rather than folding them away.

**Experimentally observed:** regenerating the
[Number-bit example](../../examples/compiler-number-bits/README.md) into
`scratch/natural-complement/generated.prg` matches all 25 expected lines and
the retained SDK console output, including zero/minimum-Number complements and
a guarded nullable operand. Unsigned artifact SHA-256:
`9b515079851cefedb8c11638598ca831ec5667cb1a13f8f1c22d81cf971b7d69`.

A focused module checks call effects and staging:

```monkeyc
using Toybox.System;
module Probe {
    function operand() { System.println("once"); return 42; }
    function flag() as Toybox.Lang.Boolean {
        System.println("flag"); return true;
    }
    function run() {
        System.println(~Probe.operand());
        System.println(~(false || Probe.flag()).hashCode());
        System.println(~~Probe.operand());
        System.exit();
    }
}
```

Reuse the Number-bit app wrapper, changing its call to `Probe.run()`. Compile
with the same API/application options and sign with our key. For the SDK,
select these two independently authored files in a local Jungle using the
Minimal Console manifest/resources. The generated and SDK PRGs both print
`once`, `-43`, `flag`, `-2`, `once`, `42`, one per line. Native and resumed unit IR
produce identical unsigned PRGs. Artifacts and logs are under
`scratch/natural-complement/`; the SDK project is `effects-sdk/`.

Shared tests reject Boolean/Char/null and unguarded nullable operands, malformed
saved operands, and missing definitions. Source/text round trips and the
64-prefix and nested-call parser limits pass. Array construction remains the
next representation question; no native-call sequence recognition is added.

The repeated audit at `scratch/example-audit/natural-complement/report.json`
retains all 115 structured PRG round trips, all documented source/resumed-IR
comparisons, and both barrel member checks, with no new decoding gaps.

## Array construction

Nonempty literals retain private begin/fill/finish operations in IR. A builder
is a construction identity rather than a language value; a CFG check requires
ordered initialization of every index and identical progress on incoming paths.
Only finish publishes the typed array. This permits final-length allocation
before element effects without exposing Null slots under a non-null element
contract. Empty literals keep the already-safe empty `ArrayNew` operation.

Straight-line literals use `.array-fill-many`: the backend retains the array
with `aputvdup` between elements and consumes it with final `aputv`. Each element
executes and is stored before the next. Elements requiring branching/staged
construction use individual fills through a private frame slot, plus a reusable
element local; CFG edges still have empty stacks. A finished builder can be
allocated again when a loop repeats its declaration. Frame-budget checks count
builder and ordinary slots separately, including overlapping numeric identities.

**Experimentally observed:** regenerated [Char literals][char-arrays],
[Number literals][number-arrays], and [native View arrays][view-arrays] match
their retained SDK console logs. These cover ordered calls, logical-expression
joins, repeated construction, guarded values, object construction, mutation,
and shared references. Validation artifacts/logs are under
`scratch/natural-arrays/`, named `char`, `number`, and `view`.

The removal of native append calls also removes their incidental API requirement
from literals. Number construction needs Array/Number availability (1.0.0), and
Char still needs 1.3.0. Element calls and explicit Array methods retain their
individual requirements. This is a declaration/selection change; simulator
evidence remains limited to SDK 9.2.0 and `fr265`.

Builder tests reject premature publication, skipped/repeated/out-of-range fills,
wrong element types, inactive identities, escaping through ordinary reads,
unfinished returns, and inconsistent CFG joins. Boundary tests verify private
slots share the 255-slot frame budget. Source/text paths retain the same builder
validation. Grouped/individual stores use supported VM opcodes; `apush` selection
and constant DATA binding remain the next review increment.

The audit at `scratch/example-audit/natural-arrays/report.json` retains all 115
fully structured PRG round trips, all 62 documented source/resumed-IR paths,
and both barrel member checks. No new decoding gap is exposed.

[char-arrays]: ../../examples/compiler-array-literals/README.md
[number-arrays]: ../../examples/compiler-number-array-literals/README.md
[view-arrays]: ../../examples/compiler-native-views/README.md

[boolean]: ../../examples/compiler-boolean-negation/README.md
[nullable]: ../../examples/compiler-nullable-contracts/README.md
