# Source View construction and drawing

SDK 9.2.0 and independently generated fr265 applications render white
“Hello from Rust” on black using the same two-file [source example][example].
Checked 2026-10-13. This establishes source View construction, native callback
dispatch, and a bounded drawing profile; it does not establish general source
inheritance, fields, input handling, or memory equivalence.

## Public contracts

**Officially documented:** [View][view] defines onShow/onHide with no arguments
and Void results, and onUpdate with a Graphics.Dc parameter and Void result,
since API 1.0.0. Its initializer is available since 2.1.0. [Dc][dc] provides
clear, setColor, and drawText. [Graphics][graphics] defines FONT_MEDIUM as a
FontDefinition value, represented by 3. Reviewed 2026-10-13. Our executable
profile selects Number coordinates/colors/justification and String text from
the broader API signatures.

**Experimentally observed:** the SDK rejects literal Number 3 as drawText's font
argument, naming FontDefinition/FontReference/VectorFont/FontResource alternatives.
The enum-valued FONT_MEDIUM expression compiles. Our IR therefore preserves its
enum type even when the backend folds the published value into an immediate.
The matching rendered text validates that bounded folding on this target.

## Metadata and execution

**Experimentally observed in our SDK-compiled source PRG:** the source View
class's native extends/base reference is `0x40000110`, selecting import 272 in
that specimen. Its constructor invokes the native View constructor, while its
source initializer calls View.initialize. The callback and drawing symbols are:

| Member | Symbol ID |
|---|---|
| onShow | `0x00800203` |
| onUpdate | `0x00800204` |
| onHide | `0x00800205` |
| setColor | `0x0080020b` |
| clear | `0x00800239` |
| drawText | `0x00800246` |

Our smaller import table selects View at index one (`0x40000001`). Source
construction looks up HelloView through globals, allocates it, and invokes its
initializer. Both normal sessions log `start`, `initial`, `show`, and `update`
(twice in these captures), and visually display the greeting without an error
dialog. The number of update calls is not a fixed callback guarantee.

A separate variant adds `System.exit()` immediately after drawing in onUpdate.
Both builds log exactly `start`, `initial`, `show`, `update`, `stop` within a
15-second observation window. Neither logs `hide` on this exit path. onHide's
signature and DATA binding are checked, but its runtime dispatch needs a View
transition specimen. Killing the normal simulator process is not a lifecycle
shutdown test.

## Reproduction

Build/sign our application using the example's commands. For the SDK reference,
create `scratch/compiler-visible-view/group.jungle` with the existing local
MinimalConsoleApp manifest/resources:

```text
project.manifest = ../native-names/manifest.xml
base.sourcePath = ../../examples/compiler-visible-view
base.resourcePath = ../native-names/resources
```

```sh
CIQ_JUNGLE_FILES="$PWD/scratch/compiler-visible-view/group.jungle" \
  sh scripts/build-example.sh scratch/compiler-visible-view
xvfb-run -a -s '-screen 0 1280x1024x24' \
  python3 scripts/simulator-session.py --prg <program.prg> --observe-seconds 15
```

Run the SDK and generated PRGs separately. Save `scratch/simulator/monkeydo.log`
and `screen.png` after each run; inspect visible text and callback order. Retained
normal captures are `sdk-screen.png`/`generated-screen.png` and logs are
`sdk.log`/`generated.log` under `scratch/compiler-visible-view/`.

For shutdown, copy both sources into its `shutdown/` subdirectory and append
System.exit after drawText. Use Jungle sourcePath `.` and manifest/resource
paths `../../native-names/...`; build/sign into new files and run separately.
The retained shutdown logs are under `scratch/compiler-visible-view/shutdown/`.
Export our SDK-compiled PRG with `ciq-dis all` to inspect class/import metadata;
this procedure does not disassemble the SDK/compiler implementation.

[example]: ../../examples/compiler-visible-view/README.md
[view]: https://developer.garmin.com/connect-iq/api-docs/Toybox/WatchUi/View.html
[dc]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Graphics/Dc.html
[graphics]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Graphics.html
