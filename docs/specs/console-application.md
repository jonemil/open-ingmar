# Console application bootstrap profile

Generate a complete unsigned watch-app PRG from a [compilation unit][unit]. This
profile is simulator-verified with SDK 9.2.0 and Forerunner 265. It uses fixed
native identities and constrained container fields; compatibility with other
API/device profiles is not established. No SDK file or original PRG is an input.
See [runtime evidence][evidence] and [application design][design].

## Input and lifecycle

The caller supplies sixteen application-ID bytes and a global entry function
name. The entry must have zero parameters and return an integer under the
existing function rules. CLI default: `main`. The ID identifies the application;
it is independent of the signing key.

Generated lifecycle behavior:

- A synthesized class extends native `Application.AppBase`. Its constructor and
  `initialize` invoke the corresponding native base methods.
- `onStart(state)` ignores state, calls the global entry once, prints its integer
  result using `System.println`, discards the print result, and returns void.
- `getInitialView()` returns a one-element array containing an initialized native
  `WatchUi.View`. This is a blank view; greeting/result output is in the console.
- Globals and user modules have empty initializer methods. A leading empty
  bootstrap body occupies CODE offset zero before lifecycle and user code.

User functions may print strings/integers, call declared module members, and use
existing locals/control flow. This does not add source class, receiver, array,
object, null, or void syntax. Generated lifecycle assembly supplies those fixed
operations separately from semantic function IR.

Modules named `globals`, `statics`, or `Toybox`, and global functions named
`Toybox`, collide with bootstrap fields and are rejected. The unit's existing
`System` restriction also applies. Source diagnostics and signature checks
precede application generation.

## Symbol and DATA layout

Synthesized application-class symbol: `1`. Distinct user plain names, including
module names and function names, receive IDs from `2` in ASCII lexicographic
order. The following predefined member names reuse their native IDs:
`initialize`, `println`, `View`, `AppBase`, `onStart`, `getInitialView`.
Module lookup identities then receive separate IDs in module-name order. A
module's plain field identity and its lookup identity are distinct.

Fixed native symbol IDs in this profile:

| Name | ID |
|---|---|
| statics | `0x00800002` |
| Toybox | `0x00800003` |
| Toybox_System | `0x00800005` |
| Toybox_WatchUi | `0x00800011` |
| Toybox_Application | `0x00800015` |
| `<init>` | `0x00800018` |
| initialize | `0x0080005e` |
| println | `0x00800067` |
| View | `0x0080019f` |
| AppBase | `0x0080029c` |
| onStart | `0x0080029f` |
| getInitialView | `0x008002a0` |
| globals | `0x008002e6` |

DATA contains [version-2 records][classes] in this order: statics, globals,
synthesized app class, user modules in lexicographic order, then CODE literal
strings in assembly order. Positions are calculated from record lengths; no SDK
DATA prefix or opaque spans are transplanted.

Statics maps module lookup IDs, including globals, to numeric DATA offsets.
Globals binds the synthesized app class, global methods, user module fields,
and a `Toybox` field with type `0x00` and value zero. User modules have parent
`globals`, their own module lookup ID, member methods, and an empty initializer.
Method values resolve their declaration's unit CODE label with the supported
ordinary-CODE prefix. Parent/module headers retain symbol identities; they are
not converted to DATA addresses.

Generated fields are sorted by full numeric key. This is a generator invariant
supported by the lookup experiment, not a rule that changes lossless handling
of existing records. Fields use observed bits: `0x10` for methods, `0x30` for
initializers, `0x50` for application classes/module fields, and `0x70` for the
native base-class field. App flags remain `0x007f`; broader flag semantics are
not inferred.

Class/string references and CODE aliases resolve through the shared layout and
encoder. Strings remain deduplicated per function. Existing encoder limits,
including the constrained DATA range and ordinary method-offset encoding,
still apply. Device memory budgets are not checked by this profile.

## Generated sections

Eight unsigned sections, in order:

1. HEADER_VERSIONED (`0xd000d00d`): 33 bytes, big-endian stamp `0x00060002`
   followed by 29 zero bytes. Additional header fields are not authored.
2. ENTRY_POINTS (`0x6060c0de`): big-endian u16 count `1`, sixteen ID bytes,
   then five u32 words: globals symbol ID, app-class symbol ID `1`, `0x82`,
   `0x26`, `1`. The last three words remain fixed opaque profile values;
   their complete semantics are unresolved.
3. DATA: generated records and literals described above.
4. CODE: generated lifecycle followed by unit bodies in declaration order.
5. IMPORTS (`0xc1a557b1`): u16 count `2` and two `(module-ID, member-ID)`
   u32 pairs, `Toybox_Application/AppBase` then `Toybox_WatchUi/View`.
6. RESOURCES: u16 count zero and deterministic trailing zero padding.
7. PERMISSIONS (`0x6000db01`): u16 count zero.
8. EXCEPTIONS: u16 count zero.

The native `AppBase` class reference is `0x40000000`, selecting import index
zero. The SDK reference's `0x40000119` selected index `0x119` in its larger
imports table; the reduced table and remapped base reference are runtime-tested.
This does not establish every native-reference address space.

RESOURCE padding aligns the unsigned section stream, excluding its final zero
terminator, to a multiple of 32 bytes. No resource assets, debug/line/symbol
sections, settings, or signing material are generated. Ordinary PRG framing and
its terminator use [the container format](prg-container.md). Signing is separate.

## CLI and saved stages

```sh
cargo run -p ciq-compiler -- --unit --emit prg --app-id \
  3c94c9e5ba2c43fa849d64ecff33264e examples/compiler-application/hello.mc \
  scratch/hello-unsigned.prg
cargo run -p ciq-asm -- sign scratch/hello-unsigned.prg scratch/hello.prg -y \
  keys/developer_key.der
```

`--emit prg` requires `--unit` and source or unit-IR input. `--app-id` requires
exactly 32 hexadecimal digits. `--entry <global-function>` overrides `main`.
Application options are rejected for other outputs; external `--symbols`/`--data`
inputs are rejected for PRG output because the generator owns their layout.
Outputs must be new files.

Saved unit IR resumes complete PRG generation with the same application options
and produces identical unsigned bytes. Raw unit assembly still represents user
bodies without the generated application wrapper. A generated PRG can be
exported and reconstructed through the existing section-project tools.

[unit]: compiler-unit.md
[classes]: data-classes.md
[design]: ../design/console-application.md
[evidence]: ../investigations/application-bootstrap.md
