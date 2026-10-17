# Console application bootstrap profile

Generate a complete unsigned watch-app PRG from a [compilation unit][unit]. This
profile is simulator-verified with SDK 9.2.0 and Forerunner 265. It uses fixed
native identities and constrained container fields; compatibility with other
API/device profiles is not established. No SDK file or original PRG is an input.
The application builder supplies metadata/lifecycle code separately from semantic
function IR and uses the shared symbolic assembly model. See
[runtime evidence][evidence] and [architecture](../architecture.md).

## Input and lifecycle

The caller supplies sixteen application-ID bytes and a global entry function
name. The entry must have zero parameters and a Number, String, Boolean, Char, or
Void result under the existing function rules. CLI default: `main`. The ID identifies the
application; it is independent of the signing key.
Nullable entries are rejected because the generated wrapper prints valued
results without a source-authored null guard.

Generated lifecycle behavior:

- A synthesized class extends native `Application.AppBase`. Its constructor and
  default `initialize` invoke the corresponding native base methods. A validated
  source initializer replaces the initialize method's DATA target.
- `onStart(state)` ignores state, calls the global entry once, prints its value
  result using `System.println`, discards the print result, and returns void.
  A void entry is invoked without that print; its VM result is discarded.
- `getInitialView()` returns a one-element array containing an initialized native
  `WatchUi.View`. This is a blank view; greeting/result output is in the console.
- Globals and user modules have empty initializer methods. A leading empty
  bootstrap body occupies CODE offset zero before lifecycle and user code.

User functions may print strings/integers, call declared module members, and use
existing locals/control flow. The [class extension](compiler-classes.md) adds source ownership and self calls;
The source, class, and IR specs define supported values and construction;
this profile does not introduce a separate language subset. Void functions and qualified statement calls follow
[the source rules][void-source]. Generated lifecycle assembly supplies those
fixed operations separately from semantic function IR.

Modules named `globals`, `statics`, or `Toybox`, and global functions named
`Toybox`, collide with bootstrap fields and are rejected. The unit's existing
`System` restriction also applies. Source diagnostics and signature checks
precede application generation.

## Symbol and DATA layout

Synthesized application-class symbol: `1`. Distinct user plain names, including
module names, function names, and literal Symbol names, receive IDs from `2`
in ASCII lexicographic order. The following predefined member names reuse their native IDs:
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
| exit | `0x0080006d` |
| format | `0x0080009c` |
| abs | `0x008000a6` |
| equals | `0x008000b3` |
| hashCode | `0x008000ad` |
| weak | `0x00800418` |
| compareTo | `0x00800e50` |
| toNumber | `0x008000a7` |
| toChar | `0x008004c6` |
| toCharArray | `0x008004c4` |
| toUtf8Array | `0x008004c5` |
| size | `0x008000ae` |
| indexOf | `0x008004bf` |
| add | `0x0080014e` |
| addAll | `0x008004be` |
| remove | `0x008000b7` |
| removeAll | `0x008004c0` |
| reverse | `0x008004c1` |
| slice | `0x008004c2` |
| toNumberWithBase | `0x008004c3` |
| toString | `0x008000af` |
| length | `0x008000bb` |
| find | `0x008000bc` |
| substring | `0x008000bd` |
| toLower | `0x008000be` |
| toUpper | `0x008000bf` |
| View | `0x0080019f` |
| BehaviorDelegate | `0x00800064` |
| requestUpdate | `0x008001a2` |
| onSelect | `0x008003f3` |
| onShow | `0x00800203` |
| onUpdate | `0x00800204` |
| onHide | `0x00800205` |
| setColor | `0x0080020b` |
| clear | `0x00800239` |
| drawText | `0x00800246` |
| AppBase | `0x0080029c` |
| onStart | `0x0080029f` |
| getInitialView | `0x008002a0` |
| onStop | `0x008002a1` |
| globals | `0x008002e6` |

DATA contains [version-2 records][classes] in this order: statics, globals,
synthesized app class, source Views in declaration order, user modules in
lexicographic order, then CODE string/constant-array literals in assembly order.
Positions are calculated from record lengths; no SDK
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

Unsigned sections, in order:

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
   Source BehaviorDelegate subclasses add a third pair,
   `Toybox_WatchUi/BehaviorDelegate`, with count `3`.
6. RESOURCES: u16 count zero and deterministic trailing zero padding.
7. PERMISSIONS (`0x6000db01`): u16 count zero.
8. EXCEPTIONS: u16 count zero.
9. SYMBOLS, when Symbol literals occur: their names and bound IDs, using the
   shared [symbol format](symbols.md). This retains development names for
   runtime Symbol string conversion.

The native `AppBase` class reference is `0x40000000`, selecting import index
zero. The SDK reference's `0x40000119` selected index `0x119` in its larger
imports table; the reduced table and remapped base reference are runtime-tested.
This does not establish every native-reference address space.

RESOURCE padding aligns the unsigned section stream, excluding its final zero
terminator, to a multiple of 32 bytes. No resource assets, debug/line
sections, settings, or signing material are generated. Ordinary PRG framing and
its terminator use [the container format](prg-container.md). Signing is
separate.

## CLI and saved stages

```sh
cargo run -p ciq-compiler -- --unit --emit prg --app-id \
  3c94c9e5ba2c43fa849d64ecff33264e examples/compiler-application/hello.mc \
  scratch/hello-unsigned.prg
cargo run -p ciq-asm -- sign scratch/hello-unsigned.prg scratch/hello.prg -y \
  keys/developer_key.der
```

`--emit prg` requires `--unit` and source or unit-IR input. `--app-id` requires
exactly 32 hexadecimal digits. `--entry <global-function>` overrides `main`;
class units require `--entry Class.method` under the profile below. Application
options are rejected for other outputs; external `--symbols`/`--data` inputs are
rejected for PRG output because the generator owns their layout. Outputs must be
new files.

Saved unit IR resumes complete PRG generation with the same application options
and produces identical unsigned bytes. Raw unit assembly still represents user
bodies without the generated application wrapper. A generated PRG can be
exported and reconstructed through the existing section-project tools.

[unit]: compiler-unit.md
[classes]: data-classes.md
[design]: console-application.md
[evidence]: ../investigations/application-bootstrap.md

[void-source]: compiler-subset.md#void-functions-and-statement-calls

## Source class startup

A unit containing [source classes](compiler-classes.md) must contain exactly one
AppBase class, optionally with View/BehaviorDelegate subclasses, and select a zero-parameter
startup method with `Class.method`,
or select `Class.onStart` when the source supplies that native callback.
The global application field and ENTRY_POINTS class identity remain symbol `1`,
representing the named source class. Plain member names equal to the class
reuse `1`; other user/native names retain the allocation rules above.

Generated onStart selects the startup using `getselfv`, then the existing call
and supported value/Void result policy. Source class methods extend application DATA
record with sorted fields bound to their owner-qualified CODE labels. Existing
global/module methods stay in their own DATA owners. Class names, native base
identity, and method ownership survive saved unit IR.

Source `initialize` follows the
[initializer contract](compiler-classes.md#source-initialization) and owns its
single method record when supplied. Source onStart similarly follows its native
callback contract and replaces the startup wrapper. Source getInitialView
replaces the generated callback with its validated View or View/delegate result. Source
onStop binds the shutdown callback; methods named AppBase remain unsupported.

Source Views receive distinct symbols and DATA records with parent globals,
native extends word `0x40000001` (WatchUi.View import index one), and a View base
field with that same reference. Their native constructor wrappers invoke View's
constructor; `initialize`, onShow, onUpdate, and onHide fields point to source
CODE where supplied. Source construction selects the class through globals.
Source BehaviorDelegate subclasses use the same record/wrapper structure with
extends/base-field reference `0x40000002` (import index two), base symbol
`0x00800064`, and an onSelect method record bound to `0x008003f3` when supplied.
Their reference fields hold initialized View targets. See
[input evidence](../investigations/compiler-input-counter.md).
Number instance fields follow the
[class field contract](compiler-classes.md#number-instance-fields),
with type `0x01`, key bits zero, and literal initial values. General object-member
dispatch beyond constructor-proven source calls remains unsupported. See
[visible View evidence](../investigations/compiler-visible-view.md).

Source-reference fields use null defaults (type `0x00`, key bits zero, value zero)
and mandatory source initialization. Parameterized source View initialize methods
receive validated constructor arguments; the native base constructor still takes
none. See the [reference-field profile](compiler-classes.md#source-reference-fields).
