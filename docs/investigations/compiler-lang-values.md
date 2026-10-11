# String and Boolean function contracts

## Question and public contract

Can our existing positional-call convention carry String and Boolean arguments
and results across module and instance methods without a new VM calling form?
The [official type-system reading][types] describes optional `as` annotations;
our exact signature checks and Number defaults are deliberately narrower than
general Monkey C typing.

## Reproduction

Use [the typed Lang example][example] as the independent source specimen. To
compile the same files with SDK 9.2.0, create `scratch/lang-values/monkey.jungle`:

```text
project.manifest = manifest.xml
base.sourcePath = ../../examples/compiler-lang-values/app.mc;../../examples/compiler-lang-values/text.mc
base.resourcePath = resources
```

Provide ordinary string/icon resources and a watch-app manifest with entry
`LangApp`, product `fr265`, and minimum API 3.0.0, as in the
[Hello World build](hello-world-build.md). Run:

```sh
sh scripts/build-example.sh scratch/lang-values
sh scripts/run-simulator-container.sh --prg scratch/lang-values/build/fr265/lang-values.prg
```

Use the example's commands for our compiler. For pipeline comparison, emit unit
IR with `--unit --emit ir`, then resume it with `--unit --from ir --emit prg`
and the same API/app options. Compare the unsigned PRGs. Only independently
authored source and its generated programs are used; no SDK implementation is
disassembled or decompiled.

## Observations and limits

**Observed, 2026-10-11, SDK 9.2.0 / fr265:** the fully qualified String/Boolean
annotations compile with the official compiler. Its program and our signed
program produce exactly the example's 12 expected lines. These cover typed
module and `self` calls, String and Boolean returns, conditional String returns,
a returned Boolean consumed by printing, a returned String passed to another
function, argument markers in source order, and discarded String/Void results.
Our source and saved unit IR produce identical unsigned PRGs with target API
5.2.0 and minimum 1.0.0.

**Interpretation:** the established context/positional-argument convention and
ordinary return instruction work for these values. Our compiler's type contracts
constrain consumers; they do not require another executable calling convention.

Separate independently generated global-entry specimens validate bootstrap
printing of noninteger results:

```monkeyc
function start() as Toybox.Lang.String { return "entry string"; }
// In a separate specimen:
function start() as Toybox.Lang.Boolean { return true; }
```

Compile each with `--unit --emit prg --entry start`, sign, and observe it in the
simulator as above. **Observed:** their console output is respectively
`entry string` and `true` on the same SDK/device profile.

Tests cover forward/recursive resolution and rejection of mismatched call-result
contracts after IR edits; recursion was not exercised in this simulator run.
Nullable results, general object arguments, dynamic signature inference, and
other devices are outside this evidence.

[types]: official-garmin-documentation.md#monkey-c-topic-subpages
[example]: ../../examples/compiler-lang-values/README.md
