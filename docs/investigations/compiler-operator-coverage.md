# Operator coverage review

Reviewed: 2026-10-15. SDK 9.2.0, `fr265`, debug, `-O 0`.
Question: which operator gaps fit our current core value types before Dictionary?

## Reference review

**Officially documented:** the [Monkey C reference][reference] lists `and`/`or`
as alternatives to the symbolic logical operators. It describes Number/Long `!`
as complement, and broader logical behavior for non-Boolean operands.

Comparing its arithmetic, relational, logical, bitwise, assignment, and ternary
operator tables with our [source contract](../specs/compiler-subset.md) identified
missing keyword spellings and Number `!`. Existing type restrictions remain
intentional; this review is not an exhaustive operand-pair compatibility claim.
Long/Float/Double need their value-type work. Runtime `has`/`instanceof` need
class/symbol contracts; general truthiness and unions remain deferred in the
[plan](../plan.md#next-work).

## SDK probes and outcome

**Experimentally observed:** our minimal console probe under
`scratch/compiler-operator-probe/` compiles and runs. `false and call()` and
`true or call()` skip the call. Mixed spellings preserve AND-before-OR precedence.
The required false/true calls print `no`, then `yes`, once each. Number results:

| Expression | Console value |
|---|---|
| `!0` | -1 |
| `!1` | -2 |
| `!(-1)` | 0 |
| `!2147483647` | -2147483648 |
| `!(-2147483648)` | 2147483647 |
| `!!7` | 7 |
| `(!7) == (~7)` | true |

The [complete example](../../examples/compiler-conditionals/README.md) adds the
same cases, keyword null guards, and inversion of a proven non-null Number.
SDK/generated runs match all 48 expected lines, including `-8` from the guarded
read. Both PRGs export CODE/DATA structurally, reconstruct exactly, and re-export
identically. Source and resumed IR produce identical PRGs. No new opcode or DATA
record is needed: typed `.not` uses the already supported `invv` instruction.
Artifacts and console-checker reports are in `scratch/check-operator-aliases/`.

**Interpretation:** keyword aliases share the symbolic logical AST/IR. Keep `or`
as a word token so type-union syntax remains independent of expression parsing.
Number `!` preserves Number; it does not establish Boolean coercion for branches
or logical operands. Native and text IR recompute this type contract.

## Reproduction

Use the complete example's commands and the existing
[console checker](../../scripts/README.md#console-specimen-check). Point the
shared console scaffold's source path at `../../examples/compiler-conditionals`,
build `scratch/compiler-conditionals`, and prepare the checker with
`--sdk-prg scratch/compiler-conditionals/build/fr265/compiler-conditionals.prg`
and a fresh output directory. Run its simulation phase in the SDK environment.
The minimal probe uses the same wrapper with the expressions in the table;
separate Boolean-returning calls print their labels to expose short-circuiting.

[reference]: https://developer.garmin.com/connect-iq/reference-guides/monkey-c-reference/
