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

**Observed:** Number conversion produced `41`, `-1`, and `0`, in that order,
on SDK 9.2.0 / fr265. The conversion result is validated as String; storing and
printing it are supported. Tests compare complete PRGs from native and saved IR
paths and check invalid receivers, arity, unsupported members, and effect order.
Captures and large logs remain ignored under `scratch/`.

## Number formatting and evaluation order

The public [Number contract][number] defines `format(String)` returning String,
available since API 1.0.0. Use the same SDK scaffold with startup expressions
`(n + 1).format("%03d")` and a Number-returning module call as receiver. Build
and export under `scratch/number-format/`; generated files remain ignored.

**Observed:** formatting uses receiver evaluation, `getsv @format`, `frpush`,
String argument evaluation, and `invokem 2`. Its symbol ID is `0x0080009c`.
A nested argument call occurs after receiver evaluation/member selection in the
SDK specimen. Our IR retains this order independently of those executable IDs.

**Observed:** the expanded Rust example printed `41`, `-1`, `0`, `042`, `07`,
`receiver`, `argument`, and `0045` on SDK 9.2.0 / fr265. This covers a computed
receiver, a literal receiver, a String local argument, and left-to-right calls
inside receiver arithmetic. Tests also check receiver-before-argument ordering,
bad argument types/arity, saved-IR parity, and the existing nesting bounds.

**Observed limitation:** the runtime rejected format String `"4"` and the
attempted pattern `"%%0%dd"` with `Unexpected input to format`. A String type
alone does not establish valid formatting syntax. The successful example uses
verified zero-padding patterns; no general printf compatibility is established.

**Limits:** no other native arguments, object construction, String receivers,
method-call statements, or String-returning user functions are established.
Catalogue entries for other Number methods do not enable executable calls.
API introduction lookup does not validate a selected device or firmware profile.

## Explicit API context

**Observed:** the downloaded local
`scratch/garmin-data/Devices/fr265/compiler.json` records
`partNumbers[0].connectIQVersion` as `5.2.0` and `firmwareVersion` as `2502`.
This device description is separate from SDK 9.2.0; it does not establish the
API level of the user's installed firmware. Device data remains ignored.

The complete native-call example ran with explicit target API 5.2.0 and
application minimum 1.0.0, printing the same eight expected lines. Reproduce
with the [example commands][example], adding
`--target-api 5.2.0 --min-api 1.0.0` to the compiler invocation. This verifies
that the context preserves the executable path for the example's 1.0.0
contracts; it does not exercise later APIs or prove broader target compatibility.

Tests check Number.toChar's documented 1.3.0 threshold, a deployment minimum
below that threshold, and the separate unsupported-lowering error. Those are
compiler diagnostic checks, not runtime validation of toChar.

## Number-result calls

The public [Number contract][number] defines zero-argument `abs()` returning
Number since API 1.0.0. Reuse the SDK scaffold above under
`scratch/number-abs/` with these startup expressions and a module helper:

```monkeyc
var n = 0 - 41;
System.println(n.abs());
System.println((n + 1).abs().format("%03d"));
System.println(Numbers.absolute(n) + 1);
System.println(0.abs());
System.println(7.abs());
// In module Numbers:
function absolute(number) { return number.abs(); }
```

Build with `sh scripts/build-example.sh scratch/number-abs`; export
`build/fr265/number-abs.prg` using `ciq-dis all` into a fresh scratch directory.
Only our compiled specimen is inspected. **Observed:** parameter/local receivers
use `getlocalv ... @abs; invokemz`; computed receivers use `getsv @abs;
invokemz`. A chained `format` selects its member after this invocation. The
stored `abs` symbol ID is `0x008000a6` in this SDK 9.2.0 / fr265 specimen.

The independently compiled [example][example] binds this ID in the console
profile and uses the ordinary semantic native-call operation. **Observed:**
the simulator prints `42`, `041`, `0`, and `7` after the existing eight lines.
These exercise a Number result returned by a user function and added to one,
a negative computed receiver followed by formatting, zero, and repeated
positive `abs` calls. Source and saved-unit-IR paths produce identical unsigned
PRGs with explicit target 5.2.0 and minimum 1.0.0.

Tests also cover numeric-result comparisons, locals, user-call arguments,
explicit symbol binding, bad receivers/arity, and invalid String results used
as integer returns. No other Number-result methods or signed-overflow behavior
are established by this experiment.

## String receivers and result chains

**Officially documented:** the [String contract][string], reviewed 2026-10-11,
defines zero-argument `length()` returning Number, and `toLower()`, `toUpper()`,
and `toString()` returning String, all since API 1.0.0. `find` and numeric
conversions have nullable results; those are not established by this experiment.

Use the SDK scaffold above under `scratch/string-methods/` and replace its
startup body with these calls followed by `System.exit()`:

```monkeyc
var text = "Hello Monkey C!";
System.println(text.length());
System.println(text.toLower());
System.println(text.toUpper());
System.println(text.toString());
System.println(text);
System.println("".length());
System.println("é猫".length());
System.println(41.toString().toUpper().length().format("%03d"));
```

Build with `sh scripts/build-example.sh scratch/string-methods`; export its
`build/fr265/string-methods.prg` with `ciq-dis all` into a fresh directory.
Only our compiled specimen is inspected. **Observed on SDK 9.2.0 / fr265:**
local String receivers use `getlocalv ... @member; invokemz`; literal/chained
receivers use `getsv @member; invokemz`. Stored symbol IDs are:

| Member | ID |
|---|---|
| `length` | `0x008000bb` |
| `toLower` | `0x008000be` |
| `toUpper` | `0x008000bf` |
| `toString` | `0x008000af` |

The SDK-built specimen prints `15`, `hello monkey c!`, `HELLO MONKEY C!`, the
original greeting twice, `0`, `2`, and `002`. The multibyte literal has two
characters and five UTF-8 bytes; this specimen's length is a character count.
The unchanged greeting after case conversion is consistent with a new result.
It does not establish all Unicode case or character-count rules.

The independently compiled [example][example] produces the same String output
after its Number checks, then `6` from a module function returning a String's
length plus one. Its source and saved unit IR produce identical unsigned PRGs
with explicit target 5.2.0 and minimum 1.0.0. The signed PRG's simulator output
matches all 21 expected lines. String/Boolean user parameters and returns,
nullable results, other String methods, and other devices remain untested here.

[string]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/String.html
[object]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/Object.html
[number]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/Number.html
[example]: ../../examples/compiler-native-calls/README.md
