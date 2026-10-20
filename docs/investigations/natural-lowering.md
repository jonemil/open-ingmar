# Natural lowering review

Date: 2026-10-14. Environment: SDK 9.2.0, `fr265`, SDK specimens built with
`-O 0`. Question: where do source → IR → VM assembly paths substitute workaround
sequences for established VM instructions?

Current semantic/lowering contracts are in [the IR spec](../specs/ir/model.md).
This review compares controlled SDK/generated behavior; it does not require
identical instruction sequences or layout.

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

## Number complement

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
64-prefix and nested-call parser limits pass. These are compiler validation checks, not additional runtime observations.

## Array construction

**Experimentally observed:** regenerated [Char literals][char-arrays],
[Number literals][number-arrays], and [native View arrays][view-arrays] match
their retained SDK console logs. These cover ordered calls, logical-expression
joins, repeated construction, guarded values, object construction, mutation,
and shared references. Validation artifacts/logs are under
`scratch/natural-arrays/`, named `char`, `number`, and `view`.

Builder checks reject incomplete/out-of-order fills, wrong types, inactive or
escaping identities, unfinished returns, and inconsistent joins. Native/text
paths agree; builder slots share the 255-slot frame budget. API availability is
checked separately from VM encoding and this runtime profile.

[char-arrays]: ../../examples/compiler-array-literals/README.md
[number-arrays]: ../../examples/compiler-number-array-literals/README.md
[view-arrays]: ../../examples/compiler-native-views/README.md

[boolean]: ../../examples/compiler-boolean-negation/README.md
[nullable]: ../../examples/compiler-nullable-contracts/README.md

## Constant-array selection

Date: 2026-10-15. Environment: SDK 9.2.0 / `fr265`, `-O 0`.
**Experimentally observed:** a complete probe runs the following checks in order:

- `make()` returns `[10, 20, 30]`; calling it twice and changing the first result
  to 99 leaves the second at 10.
- `chars()` returns `['a', 'b']`; changing one result to `z` leaves another at `a`.
- A two-iteration loop creates `[1, 2]`, prints its first element, and overwrites
  it with 99. Both iterations print 1.
- An array of two calls prints the markers 7 and 8 before `[7, 8]`.
- `[(true || false).hashCode(), 2]` prints `[1, 2]`, followed by a `done` marker.

SDK and generated simulator logs match exactly. The generated program uses three
constant records; its branching literal retains the builder path. The SDK uses
an additional constant load for that expression; matching its folding choices
is outside this baseline review.

Artifacts are under `scratch/natural-constant-arrays/`: `app.mc`, the SDK-only
`MinimalConsoleApp` wrapper, `monkey.jungle`, `unit.ciqir`, unsigned/signed PRGs,
SDK/generated exports/reexports, and console/session logs. Reproduce by defining
`make`, `chars`, and a Number identity function that prints its argument, then
running the checks above from `start()` and calling `System.exit()`. Use the
Minimal Console manifest/resources and a wrapper whose `onStart` calls `start`.
Build through `scripts/build-example.sh`; independently compile with:

```sh
cargo run -p ciq-compiler -- --unit --emit prg --target-api 5.2.0 --min-api 1.3.0 --app-id 45cb2c25c6a94ad68e936ab46ff4ab9e --entry start scratch/natural-constant-arrays/app.mc scratch/natural-constant-arrays/unsigned.prg
```

Sign the generated PRG with our key and run each program separately with
[the simulator helper](../../scripts/README.md). Preserve console logs between
runs. Source and resumed-unit-IR PRGs are identical. SDK/generated programs each
reconstruct byte-for-byte through structured CODE/DATA and produce identical
repeat exports. The generated unsigned PRG has SHA-256
`caea7f003d9964327d76a0c961f4a25d3a639c000303f75040b6be4ec838daf5`.
The SDK probe has SHA-256
`56d8be7b50c5934c4ea6f4c3de706cb4a2ea7a52e4e025b278a15768c6fd540e`.

The same text/native model handles raw function CODE: inline `.array`
requirements bind by typed content against explicit DATA. Checks reject missing,
ambiguous, conflicting, or mispinned records, invalid types/bit patterns, and
reordered allocations. Instruction availability beyond the tested SDK/device,
other scalar types, nested arrays, and deep-copy semantics remain unverified.
