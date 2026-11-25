# Operator coverage review

Reviewed: 2026-10-15. SDK 9.2.0, `fr265`, debug, `-O 0`.
Question: which operator gaps fit our current core value types before Dictionary?

## Reference review

**Officially documented:** the [Monkey C reference][reference] lists `and`/`or`
as alternatives to the symbolic logical operators. It describes Number/Long `!`
as complement, and broader logical behavior for non-Boolean operands.

Comparing its arithmetic, relational, logical, bitwise, assignment, and ternary
operator tables with our [source contract](../specs/frontend/README.md) identified
missing keyword spellings and Number `!`. Existing type restrictions remain
intentional; this review is not an exhaustive operand-pair compatibility claim.
Wider numeric combinations have [separate evidence](compiler-wide-operators.md).
Runtime `has`/`instanceof` have [separate predicate evidence](compiler-object-tests.md).
Runtime truth and the distinction between value and condition forms have
[separate evidence](compiler-logical-values.md#runtime-truth-values-and-operand-combination).

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
Number `!` preserves Number. Native and text IR recompute this type contract.

## Reproduction

Use the complete example's commands and the existing
[console checker](../../scripts/README.md#console-specimen-check). Point the
shared console scaffold's source path at `../../examples/compiler-conditionals`,
build `scratch/compiler-conditionals`, and prepare the checker with
`--sdk-prg scratch/compiler-conditionals/build/fr265/compiler-conditionals.prg`
and a fresh output directory. Run its simulation phase in the SDK environment.
The minimal probe uses the same wrapper with the expressions in the table;
separate Boolean-returning calls print their labels to expose short-circuiting.

## Assignment and shift syntax

**Officially documented:** the [Monkey C reference][reference] lists `<<`,
`>>`, and their compound forms. The [control-flow guide][control-flow] excludes
assignment from `if` conditions. Reviewed 2026-11-25.

**Experimentally observed:** SDK 9.2.0 for `fr265` with `-O 0` rejected the
following forms with parser diagnostics and exit status 100. `>>>=`, `&&=`,
`||=`, `and=`, and `or=` were tested in assignment statements, where `>>=` and
ordinary compound assignments are supported.

| Position | Rejected form |
|---|---|
| Shift expression | `n >>> 1` |
| Compound statement | `n >>>= 1`, `n &&= r`, `n ||= r`, `n and= r`, `n or= r` |
| Return or local initializer | `return n = 4;`, `var a = n = 5;` |
| Predicate | `if (n = 1) { ... }`, `for (i = 0; i = 1; i += 1) { ... }` |
| Call argument | `show(n = 3)`, `show(a[i] = 5)` |
| Index expression | `a[i = 0]` |
| Chained write | `a = b = 5 + 2 * 3;` |
| Conditional arm | `true ? (a = 1) : (a = 2)` |

The `>>>` probe also failed with a signed Number parameter. Changing only
type checking to Off (`-l 0`) reproduced the parser failures for `>>>`, `&&=`,
return assignment, and assignment within a conditional arm.

For reproduction, put each form in its own SDK console scaffold and build with
`sh scripts/build-example.sh <probe-directory> fr265`. Declare `n`, `a`, `b`,
`i`, and `r` as needed, initialize `a` as `[0]` for the indexed cases, and use
a one-argument printing function for `show`. As positive controls, run
`for (i = 0; i < 2; i = i + 1) { System.println(i); }` and the equivalent
update `i += 1`. Both loops print `0` and `1`, then printing `i` after each loop
yields `2`. The compound statements `a += b * c;` and `a += (b + c) * 2;`
produce `7` and `17` in sequence with `a = 1`, `b = 2`, and `c = 3` initially.

[reference]: https://developer.garmin.com/connect-iq/reference-guides/monkey-c-reference/
[control-flow]: https://developer.garmin.com/connect-iq/monkey-c/functions/
