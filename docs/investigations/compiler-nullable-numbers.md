# Nullable native Number results

## Public contracts

The [String API][string], reviewed 2026-10-11, defines `find(String)` and
`toNumber()` as returning Number or Null, both since API 1.0.0. A failed search
returns null. Number conversion accepts a numeric prefix; invalid or
out-of-range values return null. These runtime contracts must not be reduced
to unconditional Number results in the compiler.

## Dispatch specimen

Use the SDK scaffold from [native calls][native] under
`scratch/nullable-numbers/`, with this startup body followed by `System.exit()`:

```monkeyc
var text = "Go bananas!";
System.println(text.find("bananas"));
System.println(text.find("missing"));
System.println("41tail".toNumber());
System.println("bad".toNumber());
```

Build with `sh scripts/build-example.sh scratch/nullable-numbers` and export
the generated `build/fr265/nullable-numbers.prg` with `ciq-dis all`. Only our
source specimen's output is inspected, not Garmin's compiler or SDK binaries.
**Observed, SDK 9.2.0 / fr265:** local `find` uses `getlocalv ... @find`,
`frpush`, its String argument, and `invokem 2`. Literal `toNumber` uses `getsv
@toNumber; invokemz`. Stored IDs are `find = 0x008000bc` and
`toNumber = 0x008000a7`. These fit the existing native dispatch forms.

## Complete-app comparison

The [nullable Number example][example] contains independently authored source
and documents our compilation/signing commands. For an SDK comparison, add the
ordinary MinimalConsoleApp SDK scaffold above with `onStart(state)` calling
the example's global `start()`. Include both example source files and the
scaffold in `base.sourcePath`; use the ordinary resources and manifest entry
`MinimalConsoleApp`. Compile with the same script and run both PRGs with the
simulator script. Use at least 15 seconds of observation; a five-second SDK
session ended before producing console output in this experiment.

**Observed, 2026-10-11, SDK 9.2.0 / fr265:** both programs produce the example's
11 expected lines. They establish found/missing searches and index zero,
numeric-prefix and negative conversion, failed conversion, guarded Number
formatting/arithmetic/methods/returns, reassignment to null followed by another
check, and a retry loop whose exit supplies a Number. Source and saved unit IR
produce identical unsigned PRGs with target API 5.2.0 and minimum 1.0.0.

**Interpretation:** nullable results use ordinary VM values; checked Number
consumption needs compiler flow validation rather than a runtime conversion or
unchecked cast. Null comparison uses our established `npush`/`eq` lowering.

Contract tests reject unguarded use, wrong-path use, proof invalidated by a write,
joins with an unchecked exit, unrelated local checks, and edited branch targets.
Stored Boolean guards, nullable user signatures, nullable String results,
out-of-range conversion behavior, Unicode search-index rules, and other devices
are outside this experiment's runtime evidence.

[string]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/String.html
[native]: compiler-native-calls.md
[example]: ../../examples/compiler-nullable-numbers/README.md
