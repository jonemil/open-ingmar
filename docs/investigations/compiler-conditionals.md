# Conditional expressions and result joins

Date: 2026-10-15. SDK 9.2.0, `fr265`, debug, `-O 0`.
Question: how does `?:` select a value, join types, and lower to VM instructions?
Inputs are our authored Monkey C programs, compiled with Garmin's SDK. The SDK
and compiler were not disassembled or decompiled.

## Findings

**Officially documented:** the [Monkey C reference][reference], reviewed on the
date above, describes `testExpression ? whenTrueExpression : whenFalseExpression`
as the ternary alternative to an if/else. It does not establish our compiler's
supported type profile.

**Experimentally observed:** two console specimens under
`scratch/compiler-conditional-probe/` and
`scratch/compiler-conditional-semantics/` compile with default SDK checking and
run in the simulator. Their exact expected logs contain 13 and 18 lines.

- Effectful test calls print once, followed only by the selected arm's marker.
  An unselected arm containing integer division by zero does not execute.
- `true ? false : true ? true : true` prints `false`, establishing right
  association in this case. `true ? false ? 1 : 2 : 3` prints `2`.
- `false || true ? 1 : 2` prints `1`; `true ? 1 : 2 + 3` prints `1`;
  `1 + (false ? 2 : 3)` prints `4`. Logical OR and arithmetic bind before `?:`.
- A conditional in the middle argument of a three-argument call prints
  `first`, `chosen`, `last`, then the sum `41`. Only the selected arm executes.
- Mixed Char/Number arms preserve the selected value: `true ? 'a' : 65` prints
  `a`, while the false case prints `65`; there is no observed numeric promotion.
- String/Null selection prints `yes` or `null`. A Number-or-Null read guarded
  by `n != null ? n + 1 : 0` prints `8` after parsing `"7"`.
- Non-Boolean conditions also run: Number zero selects false, Number one selects
  true, an empty String selects true, and Null selects false. These observations
  do not define truthiness for other types.

## Strict type checks

Separate functions take a Boolean parameter and return `b ? left : right`, so
the condition is not a compile-time literal. The SDK help documents
`-l 3` as strict checking. Rebuilding with that option yields:

| Arms | Declared result | Strict SDK result |
|---|---|---|
| Number / Number | Number | Accepted |
| Char / Char | Char | Accepted |
| Number / Null | Number or Null | Accepted |
| String / Null | String or Null | Accepted |
| Number / Char | Number | Rejected: Char-or-Number `PolyType` |
| Char / Number | Char | Rejected: Char-or-Number `PolyType` |
| Number / String | Number or String | Accepted |
| Number array / Number array | Number array | Accepted |
| Number array / Null | Number array or Null | Accepted |
| Empty array / Number array | Number array | Accepted |
| Number / `System.println(...)` | Number | Rejected: Void-or-Number `PolyType` |

All these functions compile with the default check setting; that alone does not
establish compatibility with their declared return type. Strict logs and source
files are under `scratch/compiler-conditional-types/<case>/`. Array and broader
union cases in this table have compile evidence only, not separate runtime runs.

**Interpretation:** join alternatives without changing their runtime values.
The SDK supports unions beyond our current value types. The bounded initial
profile and planned IR representation belong in the
[IR contract](../specs/compiler-ir-text.md#expression-joins).

## VM and reconstruction

The first specimen's SDK CODE at `0xf7` branches on the condition. The true arm
pushes Char `0x61` with `cpush` at `0xfa` and jumps to `0x104`; the false arm
pushes Number `0x41` at `0x102`. Both continue at the same `invokem`. The value
is carried on the VM stack, with no dummy initialization or native selection
call. Existing instructions decode all CODE in both specimens; DATA exports
structurally. Each PRG reconstructs exactly and re-exports to identical files.

SDK PRG SHA-256 values:

- Initial probe:
  `d054a6dd5466c1a00d6c913228cb44dd7bfe0e0f8359ca6daac7640d3a8a763a`.
- Evaluation/precedence probe:
  `024a704438c9a7f874ea8d73060a51d075761e2b61c0e0b973f6a6436ae0e5de`.

## Reproduction

Use the [shared console scaffold](../research.md#shared-compiler-reproduction)
with `MinimalConsoleApp` as entry. For the first probe, print these expressions
from `onStart`, then call `System.exit()`:

```monkeyc
Conditional.test(true) ? Conditional.yes() : Conditional.no()
Conditional.test(false) ? Conditional.yes() : Conditional.no()
true || false ? 1 : 2
false ? 1 : true ? 2 : 3
true ? false ? 1 : 2 : 3
true ? 'a' : 65
false ? 'a' : 65
true ? "yes" : null
false ? "yes" : null
```

Define `Conditional.test(value)` to print `test` and return its argument;
`yes()` prints `yes` and returns 11; `no()` prints `no` and returns 22. Expected
output is `test, yes, 11, test, no, 22, 1, 2, 2, a, 65, yes, null`, one per line.
Retain parentheses around each expression when passing it to `System.println`.

Build with `sh scripts/build-example.sh scratch/compiler-conditional-probe`.
For strict probes, repeat the documented SDK compiler invocation with `-l 3`,
using each case's Jungle file and a distinct output PRG. Run console specimens
with `simulator-session.py --prg ... --output-dir <fresh directory>` under Xvfb.
Use `ciq-dis all`, `ciq-asm project`, and a second export to compare bytes and
trees. Saved `checks.json` files record exact reconstruction, stable export, and
console equality; build/runtime logs and expected text remain beside each probe.

## Complete toolchain comparison

The [tracked example](../../examples/compiler-conditionals/README.md) checks
nested/lazy arms, argument order, nullable guards, shared Char arrays, an expected
empty-array contract, branching array elements, and loops. SDK and generated
programs match all 48 expected lines. Source and resumed unit IR produce identical
PRGs; both SDK/generated PRGs reconstruct exactly through structured CODE/DATA
exports and stable re-exports. No new VM opcode or DATA record was required.
Artifacts and console-checker reports are in `scratch/check-operator-aliases/`.

Guarded Number/String functions return the selected read directly: Number input
7/Null prints `7`/`0`, and String input `value`/Null prints `value`/`fallback`.
Nested guarded arms, a guarded loop-body join, and a branching array element
each print `7`. The SDK accepts these sources; our saved IR contains resolved
non-null join contracts and resumes to the same program. Regression cases reject
unproven reads, stored-test proofs, writes that invalidate a proof, and a saved
IR guard changed to the wrong edge.

The example also checks logical keyword aliases and Number inversion; their
[operator review](compiler-operator-coverage.md) records those results.

The first generated run failed at the first conditional print with
`UnexpectedTypeException: Expected Method, given Number`. Its join value was
below print-call setup on the stack. Staging the result before setup fixes the
run; native/text IR now reject such unsafe direct uses. The failed capture is
`scratch/check-conditionals/runtime/generated/`.

For reproduction, create the shared scaffold pointing its source path at
`../../examples/compiler-conditionals`, then build
`scratch/compiler-conditionals`. Run the console checker preparation with
`--sdk-prg scratch/compiler-conditionals/build/fr265/compiler-conditionals.prg`
and a fresh output directory, followed by its simulation phase in the SDK
environment.

[reference]: https://developer.garmin.com/connect-iq/reference-guides/monkey-c-reference/
