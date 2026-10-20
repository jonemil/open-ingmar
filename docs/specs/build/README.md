# Builds and applications

[SDK callbacks](../sdk/README.md#application-callbacks) define the bounded source override contracts;
[formats](../formats/README.md) define generated binary containers and packages.

Contents:

- [Compiler commands and saved stages](#compiler-commands-and-saved-stages)
- [Complete application generation](#complete-application-generation)
- [Planned project builds and targets](#planned-project-builds-and-targets)

## Compiler commands and saved stages

### Command-line stages

```sh
mkdir -p scratch/pipeline-example
# Source → saved IR
cargo run -p ciq-compiler -- --emit ir examples/compiler-probe/probe.mc scratch/pipeline-example/probe.ciqir
# Saved IR → saved assembly
cargo run -p ciq-compiler -- --from ir scratch/pipeline-example/probe.ciqir scratch/pipeline-example/probe.mcasm
# Saved assembly → raw CODE bytes
cargo run -p ciq-compiler -- --from assembly --emit code scratch/pipeline-example/probe.mcasm scratch/pipeline-example/text.bin
# Source → raw CODE bytes through the native models
cargo run -p ciq-compiler -- --emit code examples/compiler-probe/probe.mc scratch/pipeline-example/native.bin
cmp scratch/pipeline-example/text.bin scratch/pipeline-example/native.bin
```

Defaults: `--from source`, `--emit assembly`. `--from` accepts `source`, `ir`,
or `assembly`; `--emit` accepts `ir`, `assembly`, `code`, or `prg`. PRG requires
unit mode and the
[application inputs](#complete-application-generation). Source/IR can emit IR, assembly, or CODE. Assembly can emit canonical
assembly or CODE, but cannot be raised to semantic IR. Duplicate options/unknown
stages are errors. Validate and encode before creating a new output file;
existing files are never overwritten.

`--emit code` emits an ordinary CODE payload, **not a complete PRG or
executable**. Use unit-mode `--emit prg` for an independently generated complete
application; signing follows generation. For CODE emission, `--symbols
<symbol-source.mcasm>` supplies a metadata-only `.symbol` file, including an
exported PRG symbol section. Its declared aliases resolve ordinary assembly
references. Compiler aliases `__ciq_module_<name>` / `__ciq_member_<name>`
resolve by unique stored names `globals/<name>` / `<name>`, independently of the
file's aliases. Missing/ambiguous names, duplicate aliases/IDs, conflicting
generated bindings, and non-symbol items are errors before output creation.
Literal aliases `__ciq_symbol_<name>` likewise resolve by the exact stored name.
Native aliases `__ciq_native_<name>` follow that same short-name rule; method
authorization belongs to IR validation rather than a second binding whitelist.
`--symbols` with IR/assembly emission is rejected; those stages remain symbolic.
No DATA/SYMBOLS section is written by raw CODE encoding.

```sh
cargo run -p ciq-compiler -- --emit code --symbols scratch/call-project/010-symbols-5717b015.mcasm examples/compiler-probe/calls.mc scratch/calls.bin
```

`symbol_bindings(&assembly, &symbols)` exposes the same resolution to library
consumers. `Program::encode_with_references` consumes the resulting map; callers
may also supply explicit maps. `--data <data-source.mcasm>` supplies DATA string
bindings for CODE emission only. `Source::parse_layout` validates placement
while retaining unresolved class symbol/method fields; final PRG assembly must
resolve those fields. `string_bindings(&assembly, &data)` binds each unplaced
literal declaration by unique content, independently of the DATA alias.
Missing/ambiguous content or a conflicting alias is an error. Pinned
declarations must match both content and offset; ordinary undeclared `&alias`
operands use DATA aliases directly. Neither option writes or allocates DATA.
Both options are rejected for IR/assembly emission. Pinned self-contained
assembly still encodes without `--data`; symbolic stages can be generated
without either option.

### CLI

`--unit` selects compilation-unit source or IR mode. With source input it
accepts multiple input paths, with the final path always the new output:

```sh
cargo run -p ciq-compiler -- --unit examples/compiler-unit/entry.mc examples/compiler-unit/numbers.mc scratch/unit.mcasm
cargo run -p ciq-compiler -- --unit --emit ir examples/compiler-unit/entry.mc examples/compiler-unit/numbers.mc scratch/unit.ciqir
cargo run -p ciq-compiler -- --unit --from ir scratch/unit.ciqir scratch/resumed.mcasm
```

`--from ir` and `--from assembly` require one input. Assembly is the existing
shared format, so `--unit` is unnecessary when resuming it. `--emit code`
accepts the existing explicit `--symbols`/`--data` bindings and emits raw CODE,
not a PRG. All outputs must be new files. Without `--unit`, the original
single-function CLI behavior is unchanged.

See [the example](../../../examples/compiler-unit/README.md) and
[simulator evidence](../../investigations/compiler-unit.md).

For complete unsigned application output, use `--emit prg --app-id <id>` with
source or saved unit IR. The [console application
profile](#complete-application-generation) specifies the generated lifecycle,
metadata, entry selection, and restrictions.

Unit signatures include explicit parameter types and the validated result
contract. Expression `.call` carries the expected supported value or nullable
result; unit validation requires it to match the callee. Statement
`.call-discard` permits any supported result, including Void, and discards it.
Both check argument types and exact arity. Inner functions permit [void-return
IR](../ir/README.md#void-returns-and-statement-calls). See [void source
rules](../frontend/README.md#void-functions-and-statement-calls).

## Complete application generation

### Application and VM binding

Complete PRG generation permits exactly one declared AppBase class plus optional
Object-rooted classes or WatchUi.View/BehaviorDelegate subclasses and requires
`--entry Class.method`. The selected method must have zero parameters, except
source onStart follows the [callback
contract](../sdk/README.md#source-startup-callback). Value results are printed
by generated onStart; void results are discarded. Class symbol `1` identifies
the application class. Plain names matching the class reuse that identity; other
names retain deterministic allocation. Class methods are sorted DATA fields
bound to their CODE labels, separate from globals/modules even when member names
share one symbol ID.

The `getInitialView` override follows the [positional
contract](../sdk/README.md#source-initial-view-callback). Methods named
`AppBase` remain unsupported. The [source initializer
contract](../frontend/README.md#source-initialization) permits `initialize`; the
bootstrap supplies missing AppBase lifecycle methods and a blank View when no
source initial-view callback exists. Constructed source Views require an
initializer. Native imports and empty resources retain the bounded application
profile. Object/native calls must satisfy their [source](../frontend/README.md)
and [SDK profiles](../sdk/README.md).

### Console application bootstrap profile

Generate a complete unsigned watch-app PRG from a [compilation
unit](../frontend/README.md#declarations-and-name-resolution). This profile is
simulator-verified with SDK 9.2.0 and Forerunner 265. It uses fixed native
identities and constrained container fields; compatibility with other API/device
profiles is not established. No SDK file or original PRG is an input. The
application builder supplies metadata/lifecycle code separately from semantic
function IR and uses the shared symbolic assembly model. See
[runtime evidence](../../investigations/application-bootstrap.md) and [architecture](../../architecture.md).

### Input and lifecycle

The caller supplies sixteen application-ID bytes and a global entry function
name. The entry must have zero parameters and return Void or a checked printable
non-null scalar: Number, Long, Float, Double, String, Boolean, Char, Numeric,
Integer, or Decimal. Other result contracts, including nullable aliases, Object,
Symbol, containers, and references, are rejected before generation. CLI default:
`main`. The ID identifies the application; it is independent of the signing key.
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
existing locals/control flow. The [class
extension](../frontend/README.md#source-classes) adds source ownership and self
calls; The source, class, and IR specs define supported values and construction;
this profile does not introduce a separate language subset. Void functions and
qualified statement calls follow
[the source rules](../frontend/README.md#void-functions-and-statement-calls). Generated lifecycle assembly supplies those
fixed operations separately from semantic function IR.

Modules named `globals`, `statics`, or `Toybox`, and global functions named
`Toybox`, collide with bootstrap fields and are rejected. The unit's existing
`System` restriction also applies. Source diagnostics and signature checks
precede application generation.

### Symbol and DATA layout

Synthesized application-class symbol: `1`. Distinct user plain names, including
module names, function names, and literal Symbol names, receive IDs from `2` in
ASCII lexicographic order. The following predefined member names reuse their
native IDs: `initialize`, `println`, `View`, `AppBase`, `onStart`,
`getInitialView`. Module lookup identities then receive separate IDs in
module-name order. A module's plain field identity and its lookup identity are
distinct.

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
| Object | `0x0080004c` |
| ByteArray | `0x008007a2` |
| decodeNumber | `0x00800a25` |
| encodeNumber | `0x00800a26` |
| offset | `0x00800aa9` |
| endianness | `0x00800aae` |
| Toybox_Lang (native import module) | `0x00800007` |
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

DATA contains [version-2 records](../formats/data.md#classes-and-modules) in
this order: statics, globals, synthesized app class, constructible source
classes in declaration order, user modules in lexicographic order, then CODE
string/constant-array literals in assembly order. Positions are calculated from
record lengths; no SDK DATA prefix or opaque spans are transplanted.

Statics maps module lookup IDs, including globals, to numeric DATA offsets.
Globals binds the synthesized app class, global methods, user module fields, and
a `Toybox` field with type `0x00` and value zero. User modules have parent
`globals`, their own module lookup ID, member methods, and an empty initializer.
Method values resolve their declaration's unit CODE label with the supported
ordinary-CODE prefix. Parent/module headers retain symbol identities; they are
not converted to DATA addresses.

Generated fields are sorted by full numeric key. This is a generator invariant
supported by the lookup experiment, not a rule that changes lossless handling of
existing records. Fields use observed bits: `0x10` for methods, `0x30` for
initializers, `0x50` for application classes/module fields, and `0x70` for the
native base-class field. App flags remain `0x007f`; broader flag semantics are
not inferred.

Class/string references and CODE aliases resolve through the shared layout and
encoder. Strings remain deduplicated per function. Existing encoder limits,
including the constrained DATA range and ordinary method-offset encoding, still
apply. Device memory budgets are not checked by this profile.

### Generated sections

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
   `Toybox_WatchUi/BehaviorDelegate`, with count `3`. Object-rooted source
   classes append a `Toybox_Lang/Object` pair after those entries. Their base
   reference selects that pair's index (2 without a delegate, 3 with one);
   it is not the SDK reference's fixed index. Lang's native import module is
   `0x00800007`, distinct from the short `Lang` symbol `0x00800020`.
6. RESOURCES: u16 count zero and deterministic trailing zero padding.
7. PERMISSIONS (`0x6000db01`): u16 count zero.
8. EXCEPTIONS: u16 count zero.
9. SYMBOLS, when Symbol literals occur: their names and bound IDs, using the
   shared [symbol format](../formats/metadata.md#symbols). This retains development names for
   runtime Symbol string conversion.

The native `AppBase` class reference is `0x40000000`, selecting import index
zero. The SDK reference's `0x40000119` selected index `0x119` in its larger
imports table; the reduced table and remapped base reference are runtime-tested.
This does not establish every native-reference address space.

RESOURCE padding aligns the unsigned section stream, excluding its final zero
terminator, to a multiple of 32 bytes. No resource assets, debug/line sections,
settings, or signing material are generated. Ordinary PRG framing and its
terminator use [the container format](../formats/README.md#prg-container).
Signing is separate.

### CLI and saved stages

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

### Source class startup

A unit containing [source classes](../frontend/README.md#source-classes) must
contain exactly one AppBase class, optionally with Object/View/BehaviorDelegate
subclasses, and select a zero-parameter startup method with `Class.method`, or
select `Class.onStart` when the source supplies that native callback. The global
application field and ENTRY_POINTS class identity remain symbol `1`,
representing the named source class. Plain member names equal to the class reuse
`1`; other user/native names retain the allocation rules above.

Generated onStart selects the startup using `getselfv`, then the existing call
and supported value/Void result policy. Source class methods extend application
DATA record with sorted fields bound to their owner-qualified CODE labels.
Existing global/module methods stay in their own DATA owners. Class names,
native base identity, and method ownership survive saved unit IR.

Source `initialize` follows the
[initializer contract](../frontend/README.md#source-initialization) and owns its
single method record when supplied. Source onStart similarly follows its native
callback contract and replaces the startup wrapper. Source getInitialView
replaces the generated callback with its validated View or View/delegate result.
Source onStop binds the shutdown callback; methods named AppBase remain
unsupported.

Source Views receive distinct symbols and DATA records with parent globals,
native extends word `0x40000001` (WatchUi.View import index one), and a View
base field with that same reference. Their native constructor wrappers invoke
View's constructor; `initialize`, onShow, onUpdate, and onHide fields point to
source CODE where supplied. Source construction selects the class through
globals. Source BehaviorDelegate subclasses use the same record/wrapper
structure with extends/base-field reference `0x40000002` (import index two),
base symbol `0x00800064`, and an onSelect method record bound to `0x008003f3`
when supplied. Their reference fields hold initialized View targets. See
[input evidence](../../investigations/compiler-input-counter.md).
Number instance fields follow the
[class field contract](../frontend/README.md#instance-fields),
with type `0x01`, key bits zero, and literal initial values. General
object-member dispatch beyond constructor-proven source calls remains
unsupported. See
[visible View evidence](../../investigations/compiler-visible-view.md).

Source-reference fields use null defaults (type `0x00`, key bits zero, value
zero) and mandatory source initialization. Parameterized source View initialize
methods receive validated constructor arguments; the native base constructor
still takes none. See the [reference-field
profile](../frontend/README.md#source-reference-fields).

## Planned project builds and targets

Project builds are planned; no Rust Jungle parser or project build driver
exists. Resolve real application and barrel projects into explicit compiler and
packaging inputs. [The plan](../../plan.md) owns implementation priorities.

### Inputs and boundaries

Read the project's existing Jungle configuration, commonly `monkey.jungle`,
together with its manifest. A future build layer resolves configuration into
explicit source files, resources, library dependencies, target settings, and
exclusions. The Monkey C compiler consumes those resolved inputs; archive
writing remains in `ciq-package`, and signing remains in `ciq-sign`. Decide
crate names and public APIs when implementation begins.

Resolve inputs before the
[compilation-unit](../frontend/README.md#declarations-and-name-resolution)
declaration pass. File boundaries do not establish module ownership; project
selection must retain source provenance and the supported name-visibility rules.

Support two project workflows:

- **Application:** resolve inputs, compile source and resources into a target
  PRG, sign with the supplied key, and optionally export an IQ distribution
  using the package layer.
- **Barrel:** resolve inputs and collect the selected source/resources and
  metadata into the package layout, generating the archive's `barrel.jungle` and
  manifest copies as required by the supported export profile. The observed
  barrel is distributed as source; executable bytecode is produced when a
  consuming application is compiled.

The project Jungle and exported Jungle are distinct inputs/outputs. In our SDK
specimen, export rewrites `manifest.xml` to `content/manifest.xml` and `source`
to `content/source`; see [the barrel
profile](../formats/packages.md#barrel-format). The current `pack-barrel` only
consumes an already exported canonical tree and preserves its Jungle bytes. It
does not perform this project-to-package transformation.

### Manifest and build context

The resolved manifest identifies the AppBase-derived entry class, application
UUID/type, supported products/languages, minimum API, permissions, resource
references, and barrel namespace/version requirements. The console CLI's startup
method is not a manifest entry. Preserve these identities without assigning
binary fields until the relevant target profile has evidence.

Native API validation needs the selected device, app type, API level,
permissions, and execution scope. Permission checks can depend on individual
operations, not just modules. Resource validation must resolve name/icon and
settings references against the selected resources. An unsupported manifest
feature must be diagnosed rather than silently omitted.

The [API catalogue](../sdk/README.md#api-declarations) owns class/member
availability. Keep catalogue coverage, selected-device capabilities, application
minimum API, and VM encoding capabilities distinct. Target resolution supplies
constraints; it does not duplicate the API catalogue for each device.

The resolver must supply [API context](../sdk/README.md#availability) from
verified target/manifest data, not infer device capability from the SDK version.

Build mode (debug/release/test) and execution scope (foreground/background/
glance) are separate inputs. Preserve declaration annotations and resource
scopes, with provenance, for selection and reference checking. A test build
requires test entry discovery and runner metadata; a barrel test needs a
runnable application rather than an archive alone. Exact rules and encodings
remain to be specified from public contracts and controlled specimens.

### Target constraints and section layout

Proposed boundary: resolve a device and app type for a complete application
build, and supply their constraints to target-dependent selection, resource
generation, and final layout/validation. Shared source, semantic IR, and generic
instruction encoding need not acquire device-specific offsets or require a
device profile for every operation. API validation or conditional input
selection may still need the target before layout.

Keep VM format rules separate from device capacity. Operand widths, address
selectors, and record framing describe encoding; app memory allowances describe
whether a particular target can execute the result. A small memory allowance
must not become an assumed address-field width. The
[string-boundary experiment](../../investigations/string-address-boundary.md)
demonstrates this distinction: the same `news 0x01000000` encoding runs after
raising only the simulated watch-app memory allowance.

A shared high-nibble region model remains a hypothesis, not a verified 28-bit
offset rule; see the boundary investigation. Do not invent per-device regions.
Retain full u32 addresses, distinguish selectors from offsets, and preserve
unknown numeric addresses until additional bits/selector behavior are verified.

Device profiles should initially supply capacity and capability constraints,
rather than redefining section regions. Add VM format variants only when
observations establish them. After resolving the shared DATA address rule,
replace the current 24-bit checks consistently across string parsing, DATA
layout, and CODE binding. Keep CODE/method-address changes dependent on their
own evidence.

Layout uses section-relative offsets, rather than placing objects at physical
watch RAM addresses. Two devices with compatible formats and identical selected
inputs may use the same section layout; a smaller budget can cause validation to
reject it instead of requiring different offsets. Target-dependent inputs or
resource encodings can change the layout itself.

Memory accounting needs its own investigation before enforcing precise build
limits. PRG file size is not established as peak runtime memory: loaded DATA,
resources, frame/stack storage, and dynamic objects may have different costs.
Report known static costs and distinguish them from unbounded or estimated
runtime allocation; do not promise that passing a static check guarantees all
execution fits.

Keep low-level assembly/container tools usable without a device profile for
inspection, lossless reconstruction, and format experiments. A complete target
build should validate against its resolved profile and record explicit
experimental capacity overrides. Exact profile fields, accounting rules, and
compatibility checks remain to be specified from evidence.

### Resolution and export requirements

The [official documentation
review](../../investigations/official-garmin-documentation.md) records
configuration evaluation, path bases, precedence, and direct barrel-project
dependencies. Define the supported Jungle grammar/resolution rules from those
contracts and controlled SDK experiments. Unsupported constructs must produce
diagnostics with configuration file/location, rather than silently changing
selected inputs.

The resolved build description must retain input provenance and deterministic
selection. Its rules need to cover variables/substitution, lists, multiple
Jungle files, defaults/inheritance, device overrides, annotations, resource
personalities/localization, and dependencies. These semantics are not yet
specified locally; they remain separate from Monkey C parsing and archive I/O.

Project-to-barrel export must define file selection, archive path mapping,
generated Jungle content, manifest handling, and dependency policy. Application
builds use the same description to supply compiler/resource inputs and IQ export
settings as the supported target profile grows. Keep direct assembly and
canonical-tree packing usable independently. [The plan](../../plan.md) owns the
implementation order.

Acceptance examples should cover a multi-file app, code-only and resource
barrels, an app consuming multiple barrels, device-specific selection, and
different project working directories. Compare resolved inputs and observable
results against Garmin's SDK; archive byte identity is not required.
Shared-source variants and multiple barrel outputs from one repository need
explicit configurations and experiments before promising a particular project
model.

Crate names and public build APIs remain undecided.
