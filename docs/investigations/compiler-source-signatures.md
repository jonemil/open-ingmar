# Source-class reference contracts

SDK 9.2.0 and independently generated fr265 applications produce matching logs
and displayed counters when source View references cross module and self calls.
Checked 2026-10-13 with [the three-file example][example]. Scope: explicit source
parameters/results, factories, reference forwarding, and subsequent source
dispatch. Object-reference fields, constructor arguments, and broader source
inheritance are not established by this experiment.

**Experimentally observed:** Views.make returns a newly constructed HelloView.
Views.pass receives and returns that exact source class; Views.update calls its
add method through the parameter. AppBase.echo returns the same reference for
the initial-view result. Both sessions log independent initialization pairs
`7, 8, 7, 8`, followed by call results `11, 9, 10, 11`, then `show`, `12`, `13`.
Both captured displays show 13 without an error dialog; logs match exactly.
Update counts themselves depend on session timing.

**Interpretation:** these calls pass the same runtime reference, preserving
the first View's state through the factory/forwarding/update path. Exact source
signatures require no new VM value representation or runtime cast in this profile.
The compiler retains the class contract in semantic metadata and checks it at
function/unit boundaries. Native View parameters/returns deliberately erase exact
identity; generic native references cannot satisfy source contracts.

Tests additionally cover source results from explicit object calls, chained
dispatch, cross-file declaration resolution, unrelated classes, generic View
arguments/returns, Null, unknown signature classes, and edited IR claims.
These are our bounded checking rules, not proof of Garmin's complete typing policy.

## Nullable source signatures and fields

2026-10-20, SDK/compiler/simulator 9.2.0, `fr265`, `-O 0`.
The `NullableSourceTests` module in the [object-test specimen][nullable-example]
declares `Probe or Null`, `LocalView or Null`, and `LocalDelegate or Null`
parameters. It returns either a source instance or Null, forwards nullable
results, copies them into locals, guards source calls, and resets locals to Null.
`NullableSourceHolder` stores the same three source types in nullable fields,
checks implicit/explicit Null defaults, copies reads, and clears the fields
before using the snapshots. A typed getter forwards the nullable source contract.

**Experimentally observed:** SDK and generated console output matches the
expected text, including the added group `1, -1, 2, -2, 3, -3, 1, true, 1, true`.
The field group prints `true, true, true, 1, 2, 3, 1, true`, confirming default
Null slots and retained snapshot references after field clearing.
Both captured displays show the ordinary application screen without an error
dialog. SDK/generated CODE and DATA export structurally and reconstruct exactly;
native, saved-IR, and assembly paths agree. No new opcode or DATA framing is
needed for these nullable references.

Reproduce with `scripts/build-example.sh scratch/compiler-object-tests`, using
that directory's Jungle file pointing to the tracked sources and local
manifest/resources. Then run the shared console checker with the resulting
SDK PRG: `scripts/check-example.py prepare compiler-object-tests` with
`--sdk-prg` and a fresh `--output`, followed by `simulate` in the SDK environment.
This run is captured in `scratch/nullable-source-fields/check/`.

**Limits:** these observations cover signatures, inferred locals, and nullable
source fields, not container elements, conditional-value joins, inheritance, or
general unions. Missing-proof and unrelated-class rejections are our
checking policy; they do not establish the SDK's full static-checking rules.

## Nullable source assertions

2026-10-20, SDK/compiler/simulator 9.2.0, `fr265`, `-O 0`.
`NullableSourceCasts` in the same specimen checks `value as Probe or Null`,
Object-erased View/delegate recovery, Null operands, and chained assertions.

**Experimentally observed:** SDK/generated runs match the expected group
`1, true, true, 2, 3, source-cast-effect, 1, true, true, true, 1`.
The effect marker appears once. A Number asserted as nullable Probe still tests
as Number; an unrelated View retains its original class. The probe avoids
dispatch through false assertions. Structured SDK/generated PRG reconstruction
and saved-stage equivalence pass. Captures are under
`scratch/nullable-source-casts/check/`, using the same reproduction procedure.

**Interpretation:** nullable source assertions retain static identity and Null
without runtime conversion or validation, consistent with the existing lexical
cast contract. This does not establish source inheritance, general union casts,
or recovery safety after an incorrect assertion.

## Source conditional values

2026-10-20, SDK/compiler/simulator 9.2.0, `fr265`, `-O 0`.
`SourceJoins` in the [object-test specimen][nullable-example] selects matching
source instances, source-or-Null results, nested choices, and guarded Object
reads. View and delegate choices retain the selected instance's source methods.

**Experimentally observed:** SDK/generated output matches the group
`1, true, 1, 1, 2, 3, source-join-left, 1, source-join-right, 1, 1, true`.
Only the selected arm's marker appears for each effectful choice. Saved IR
retains source join tags, and SDK/generated PRGs reconstruct structurally without
new opcodes or DATA framing. Reproduce using the same shared console checker;
captures are under `scratch/source-joins/check/`.

**Limits:** this verifies matching exact classes and optional Null, not unrelated
source unions, inheritance joins, or container element variance. Rejection of
source/native mixtures without explicit erasure is our bounded checking policy.

## Reproduction

Build/sign our application using the example's commands. Create
`scratch/compiler-source-signatures/group.jungle` with local manifest/resources:

```text
project.manifest = ../native-names/manifest.xml
base.sourcePath = ../../examples/compiler-source-signatures
base.resourcePath = ../native-names/resources
```

```sh
CIQ_JUNGLE_FILES="$PWD/scratch/compiler-source-signatures/group.jungle" \
  sh scripts/build-example.sh scratch/compiler-source-signatures
xvfb-run -a -s '-screen 0 1280x1024x24' \
  python3 scripts/simulator-session.py --prg <program.prg> --observe-seconds 15
```

Run the SDK and generated PRGs separately. Save `scratch/simulator/monkeydo.log`
and `screen.png` after each run, compare reference/counter observations, and inspect
the displayed value. Retained logs/captures are `sdk.log`, `generated.log`,
`sdk-screen.png`, and `generated-screen.png` under `scratch/compiler-source-signatures/`.

[example]: ../../examples/compiler-source-signatures/README.md
[nullable-example]: ../../examples/compiler-object-tests/README.md

## Structural and union assertions

Reviewed 2026-10-25 with SDK 9.2.0, strict checking, fr265. The public
[Monkey Types guide](https://developer.garmin.com/connect-iq/monkey-c/monkey-types/)
states that lexical casts do not change runtime values. Authored SDK probes
accept typed Arrays, option records, tuples, structural interfaces, native unions,
and Method signatures as assertion targets. `x as Lang.Boolean or flag` is
rejected because the SDK resolves `flag` as a type; `(x as Lang.Boolean) or flag`
is accepted.

The [cast example](../../examples/compiler-casts/README.md) uses API 5.0.0 for
Comparable. SDK and generated PRGs match all 14 expected lines, including the
single `once` effect marker. The authored program exercises Dictionary and source
containers, native/structural interfaces, source unions, callbacks, and mutation
through an asserted record alias. Saved-stage and complete CODE/DATA reconstruction
checks pass without new VM instructions or record formats. False assertions
remain caller responsibility; this experiment does not validate their safety.
