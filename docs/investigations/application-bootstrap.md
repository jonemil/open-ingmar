# Independent console application bootstrap

2026-10-10. Question: which structures are needed to generate a complete runnable
application without transplanting SDK CODE/DATA or opaque section payloads?
Reference: SDK 9.2.0, Java 17, `fr265`, default simulator memory, our developer key.
The implementation follows this repository's [class format][classes],
[instruction evidence][instructions], and [unit design][unit].

## Reference and reductions

An ignored copy of `examples/minimal-console` supplied an SDK reference with
only these user lifecycle bodies:

```monkeyc
using Toybox.Application;
using Toybox.System;
using Toybox.WatchUi;
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) { System.println(41); }
    function getInitialView() { return [new WatchUi.View()]; }
}
```

Build root: `scratch/application-bootstrap/sdk-example`, using
`scripts/build-example.sh <root> fr265`, optimization zero. Its DATA also includes
many generated resource modules; CODE includes their initializers. Removing
DEBUG, LINE_NUMBERS, SYMBOLS, and SETTINGS, while retaining the remaining SDK
sections, still prints `41` and opens the blank View. This establishes their
omission for this tested app, not every PRG profile.

IMPORTS contains a u16 count `299` and 299 pairs of u32 symbols. Index `0x119`
is `Toybox_Application/AppBase`; index `0x110` is `Toybox_WatchUi/View`.
The app class's `extends=0x40000119` agrees with that AppBase index. An authored
imports table containing only AppBase and View, remapping the base reference to
`0x40000000`, runs successfully. Empty RESOURCES/PERMISSIONS/EXCEPTIONS tables
also work; no SDK resource initialization or Rez/style records are needed for
this native blank View.

Authored probes under ignored `scratch/application-bootstrap/` established:

| Probe | Change/result |
|---|---|
| thin4 | Generated CODE/DATA/header/entry/imports and empty resources print `42`. |
| thin5 | A global method after higher symbol keys fails with Symbol Not Found `2`. |
| thin6 | Sorting fields by key fixes the same global call; it prints `41`. |
| thin7 | Removing the Toybox zero-valued field from thin6 crashes before printing. |

The failed early reductions did not establish individual missing-field causes.
The thin6/thin7 pair isolates the Toybox binding; offsets and resource padding
were adjusted for its removal. Thin5/thin6 change only field ordering. Sorted
fields are therefore an invariant of our generator. Binary-search lookup is a
possible explanation, not a verified VM implementation detail. Existing record
readers/writers continue preserving order for lossless reconstruction.

Header/entry values not isolated in this investigation remain opaque profile
constants. Earlier header interpretation was also checked against the local
[optimizer header reference][optimizer-header], revision
`2920dc88c23caadded9f66ee0613a7de6ab2d1ea` (MIT). No implementation code was copied;
our generation contract uses the observed fields and runtime-tested reductions.

## Rust generation and observations

`ciq-compiler::application` now generates the complete unsigned stream from
validated unit IR and an application ID/entry name. It constructs typed lifecycle
assembly and DATA records, allocates sorted user symbols/module identities,
binds method labels and literals, and emits two native imports and empty tables.
No SDK files or original PRG are read by this path. Signing uses the existing
separate assembler command. See [design][design] and [profile][profile].

**Experimentally observed**, after own-key signing:

- [hello.mc](../../examples/compiler-application/hello.mc) prints
  `Hello from the Rust compiler!` and `41`; the blank View remains active.
- Changing only the source return to `42` prints the same greeting and `42`.
- The existing [two-file unit](../../examples/compiler-unit/README.md), selecting
  `probe`, prints `1`, `2`, `47`, then `48` with independently generated module
  and method DATA. No SDK caller, helper, lifecycle, or metadata body is retained.

Captured displays show the expected blank native View, without an error dialog;
console traces establish actual execution. Captures/logs are retained under
ignored `scratch/application-bootstrap/rust/simulator-41/`, `simulator-42/`, and
`simulator-unit-48/`. Observation windows were eight seconds; indefinite runtime,
real-watch operation, and other device/API profiles are not established.

## Byte checks

Source and saved-unit-IR paths produce byte-identical complete unsigned PRGs for
both hello and the two-file input. Export/reconstruction of the signed hello PRG
is byte-identical and its preserved developer signatures verify.

The `41` → `42` source edit changes exactly one unsigned CODE byte, at offset
`110` (`0x6e`); all other unsigned section payloads are identical. Developer
signatures change as expected and verify independently.

| Input | DATA bytes | CODE bytes | RESOURCES bytes | Signed PRG bytes |
|---|---|---|---|---|
| hello 41/42 | 166 | 112 | 13 | 2004 |
| two-file probe | 204 | 209 | 6 | 2132 |

Each unsigned output has eight sections; signing appends one developer section.
Headers are 33/38 bytes, IMPORTS is 18, and PERMISSIONS/EXCEPTIONS are two bytes.
RESOURCE padding makes the unsigned section stream a multiple of 32 bytes.

Signed SHA-256:

```text
hello41 7b30cc856757602ecffe9f31bc5965deec6405c83066346bb299ec037f558aa3
hello42 f3212012707ede83e740d43b2ba867ae672afcd3d326d972ad739b7ffd1f475e
unit48  848e3382ffe7267221806fdbf83626937dd994e49489c9dbdff17750e9bf32e6
```

## Reproduction and next work

The [example commands](../../examples/compiler-application/README.md) generate,
sign, and run without rebuilding the SDK reference. For saved IR and lossless
reconstruction, use fresh outputs:

```sh
target/debug/ciq-compiler --unit --emit ir \
  examples/compiler-application/hello.mc \
  scratch/application-bootstrap/rust/hello.ciqir
target/debug/ciq-compiler --unit --from ir --emit prg --app-id \
  3c94c9e5ba2c43fa849d64ecff33264e \
  scratch/application-bootstrap/rust/hello.ciqir \
  scratch/application-bootstrap/rust/ir-unsigned.prg
cmp scratch/application-bootstrap/rust/unsigned.prg \
  scratch/application-bootstrap/rust/ir-unsigned.prg
target/debug/ciq-dis all scratch/application-bootstrap/rust/generated.prg \
  scratch/application-bootstrap/rust/roundtrip-project
target/debug/ciq-asm project \
  scratch/application-bootstrap/rust/roundtrip-project \
  scratch/application-bootstrap/rust/roundtrip.prg
cmp scratch/application-bootstrap/rust/generated.prg \
  scratch/application-bootstrap/rust/roundtrip.prg
target/debug/ciq-asm verify scratch/application-bootstrap/rust/roundtrip.prg \
  -y keys/developer_key.der
```

Six tests check typed/source/IR complete-PRG parity, sorted unique fields,
class/module/method resolution, CODE boundaries and DATA strings, native imports,
shared native member identities, entry/namespace restrictions, CLI options, and
output preservation. They run without SDK files or signing keys. Simulator
experiments provide separate runtime evidence.

Next make the lifecycle source-authored with a minimal class/receiver and typed
SDK-call subset, including required void/null/object/array values. Launcher
assets, views that draw, debug/symbol generation, other application/API profiles,
and Jungle/manifest resolution remain future work. Broader section/offset rules
are still driven by concrete generated applications.

[classes]: ../specs/data-classes.md
[instructions]: instruction-semantics.md
[unit]: ../specs/compiler-unit.md
[design]: ../specs/console-application.md
[profile]: ../specs/console-application.md
[optimizer-header]: https://github.com/markw65/monkeyc-optimizer/blob/2920dc88c23caadded9f66ee0613a7de6ab2d1ea/src/readprg/header.ts
