# Source View contracts across calls

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
