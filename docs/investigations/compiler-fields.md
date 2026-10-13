# Number instance field initialization and mutation

SDK 9.2.0 and generated fr265 applications keep separate Number state for two
instances of the same source View and draw matching counter values. Checked
2026-10-13 with [the two-file example][example]. Scope: constant Number field
initialization, current-instance reads/writes, and state across callback calls.
Other field types, computed initializers, static storage, and source inheritance
are not established by this experiment.

## Observations and interpretation

**Experimentally observed:** our SDK-compiled source PRG records a field declared
`var count = 0;` as a class member with key bits `0x00`, type `0x01`, and value
zero. Reads use `getselfv count`. Assignment emits `getself`, `spush count`, RHS
instructions, and `putv`. This specimen is retained in
`scratch/compiler-fields/sdk-export/`; the inspected PRG contains our source,
not disassembled SDK/compiler implementation.

The complete example starts the field at 7. Each initializer logs 7, invokes a
source self method to write 8, then logs 8. Two constructions produce `7, 8, 7, 8`
in both toolchains. The first instance is returned by getInitialView; successive
onUpdate callbacks log 9 and 10 and draw their value. Both screenshots display 10
without an error dialog in the retained sessions. The final SDK/generated console
logs match exactly. Update counts themselves depend on session timing.

**Interpretation:** Number field initial values are established separately for
each instance before its source initializer. The first instance's mutation does
not replace the default used by a second construction. The matching mutation and
drawing establish this metadata/lowering profile on fr265; they do not establish
allocation timing, reference-valued defaults, memory equivalence, or other targets.

Saved-IR tests additionally reject undeclared/conflicting fields and non-Number
writes. A read defined before a write cannot be consumed after that write unless
its value was captured into a local. This preserves the value observed at the
source read rather than delaying the lookup until its later use.

## Reproduction

Build/sign our application with the example's commands. Create
`scratch/compiler-fields/final.jungle` using the local MinimalConsoleApp manifest:

```text
project.manifest = ../native-names/manifest.xml
base.sourcePath = ../../examples/compiler-fields
base.resourcePath = ../native-names/resources
```

```sh
CIQ_JUNGLE_FILES="$PWD/scratch/compiler-fields/final.jungle" \
  sh scripts/build-example.sh scratch/compiler-fields
xvfb-run -a -s '-screen 0 1280x1024x24' \
  python3 scripts/simulator-session.py --prg <program.prg> --observe-seconds 15
```

Run the SDK and generated PRGs separately. Save `scratch/simulator/monkeydo.log`
and `screen.png` after each session. Compare console initialization pairs and
callback values, and inspect the displayed counter. Retained final logs are
`sdk.log` and `final-generated.log`, with captures `sdk-screen.png` and
`final-generated-screen.png`, under `scratch/compiler-fields/`.

To reproduce the zero-initialized metadata probe, copy the visible View example,
add `var count = 0;`, increment self.count in onUpdate, and draw its toString result.
Compile it with the SDK and export the resulting PRG using `ciq-dis all` into a
new directory. Its class field record and assignment sequence establish the
encoding separately from the final instance-isolation test.

[example]: ../../examples/compiler-fields/README.md
