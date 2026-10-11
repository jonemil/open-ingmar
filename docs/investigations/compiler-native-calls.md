# Native value calls

## Question and scope

Can a Number value invoke inherited `Lang.Object.toString()` without a separate
heap-object construction? Public [Object documentation][object] supplies the
zero-argument String-result contract, available since API 1.0.0. This experiment
covers SDK 9.2.0, fr265, and our own source specimens; other targets are unverified.

## SDK specimen

Use a local copy of the minimal-console manifest/resources and a Jungle selecting
this source, keeping generated files under `scratch/number-method/`:

```monkeyc
using Toybox.Application;
using Toybox.WatchUi;
using Toybox.System;
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function getInitialView() { return [new WatchUi.View()]; }
    function onStart(state) {
        var n = 41;
        System.println(n.toString());
        System.println((n - 42).toString());
    }
}
```

Set `project.manifest`, `base.sourcePath`, and `base.resourcePath` to those local
inputs. Build with `sh scripts/build-example.sh scratch/number-method`, then
export the generated `build/fr265/number-method.prg` with `ciq-dis all` into a
new scratch directory. The build script selects `-O 0` and our own developer key.
No Garmin compiler/SDK binary was disassembled or decompiled.

**Observed:** the local call uses `getlocalv 2 @toString; invokemz`. The computed
receiver uses `lgetv 2; ipush1 42; subv; getsv @toString; invokemz`. The specimen's
symbol table assigns `toString` ID `0x008000af`.

**Interpretation:** evaluate a Number receiver, select its member with `getsv`,
then invoke the zero-argument method. The local-specific lookup is unnecessary
for this bounded lowering. Neither observed sequence inserts an explicit boxing
operation; this does not establish the VM's physical value representation.

## Independently generated application

The [native call example][example] documents compilation, signing, and simulator
commands. Its Number locals cover positive/negative values and its module
function covers a parameter and zero. The Rust backend emits receiver evaluation,
`getsv @__ciq_native_toString`, and `invokemz`; the application profile binds the
alias to the observed ID separately from semantic IR.

**Observed:** our generated app printed exactly `41`, `-1`, and `0`, in that order,
on SDK 9.2.0 / fr265. The conversion result is validated as String; storing and
printing it are supported. Tests compare complete PRGs from native and saved IR
paths and check invalid receivers, arity, unsupported members, and effect order.
Captures and large logs remain ignored under `scratch/`.

**Limits:** no general native arguments, object construction, String receivers,
method-call statements, or String-returning user functions are established.
Catalogue entries for other Number methods do not enable executable calls.
API introduction lookup does not validate a selected device or firmware profile.

[object]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/Object.html
[example]: ../../examples/compiler-native-calls/README.md
