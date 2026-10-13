# Calls on source object references

SDK 9.2.0 and generated fr265 applications produce matching console output and
displayed counters when calling methods through copied/reassigned source View
references. Checked 2026-10-13 with [the two-file example][example]. Scope:
explicit receivers, scalar arguments/results, Void statements, aliases, and
same-class reassignment. Source inheritance and general object typing are not
established by this experiment.

## Observations and limits

**Experimentally observed:** both applications initialize two independent
instances with `7, 8, 7, 8`. Calling add through the shared reference changes the
first instance to 11; calling add on the second changes it to 9. Reassigning the
shared reference to the second and calling its Void bump method changes it to
10. The first instance still reports 11, then onUpdate advances it to 12 and 13.
Both retained screenshots show 13 without an error dialog. Console logs match
exactly; update counts depend on session timing.

Our SDK-compiled source PRG uses `getlocalv <slot> <member>` for add, bump, and
current, followed by the ordinary invocation convention. Our lowering evaluates
the receiver with the existing expression path and selects its member with
`getsv`. Arguments follow a prepared context; statement results are discarded.
The matching runtime behavior establishes this expanded sequence separately
from the SDK's compact local lookup.

**Implementation boundary:** exact constructor identity is reconstructed from
SourceNew and local operations for both source and saved IR. Generic View
parameters/returns and Array element reads erase it. Unit checking resolves
the selected source method and rejects native callbacks as ordinary callees.
Tests exercise contextual empty Array arguments, nested calls, same-class
assignments through branches/loops, and forged class/result claims. These checks
do not establish arbitrary runtime casts or source superclass lookup.

## Reproduction

Build/sign our application using the example's commands. Create
`scratch/compiler-object-calls/group.jungle` with the local manifest/resources:

```text
project.manifest = ../native-names/manifest.xml
base.sourcePath = ../../examples/compiler-object-calls
base.resourcePath = ../native-names/resources
```

```sh
CIQ_JUNGLE_FILES="$PWD/scratch/compiler-object-calls/group.jungle" \
  sh scripts/build-example.sh scratch/compiler-object-calls
xvfb-run -a -s '-screen 0 1280x1024x24' \
  python3 scripts/simulator-session.py --prg <program.prg> --observe-seconds 15
```

Run SDK and generated PRGs separately; save `scratch/simulator/monkeydo.log`
and `screen.png` after each run. Compare the instance/call values and inspect
the displayed counter. Retained logs/captures are `sdk.log`, `generated.log`,
`sdk-screen.png`, and `generated-screen.png` under `scratch/compiler-object-calls/`.
Export the SDK-compiled source PRG with `ciq-dis all` into a new directory to
inspect the receiver/member lookup sequence. This inspects our compiled source,
not the SDK/compiler implementation.

[example]: ../../examples/compiler-object-calls/README.md
