# Official Garmin documentation

Reviewed: 2026-10-10. Scope: reference navigation and the language, class,
configuration, and lifecycle rules relevant to upcoming compiler work, including
all eight topic subpages linked from the Monkey C guide. This is
a reading map, not an exhaustive review or a claim of implementation support.

## Reading map

- [Monkey C guide][guide]: introductory syntax and linked topic articles.
  The topic map below records where to return for each feature.
- [Monkey C language reference][language]: broader language rules and examples.
- [Jungle reference][jungle]: project configuration and barrel dependencies.
- [Toybox API documentation][api]: module/class contracts, signatures, API
  levels, device support, and links to related types.
- [Core Topics][core]: application, resource, native-service, and tooling
  articles. The scope review below maps these to compiler/build work.

The guide and reference pages may return navigation alone to a browser text
reader. The site's `/page-data/connect-iq/<page>/page-data.json` identifies each
article through `result.pageContext.fileName`. Its body is directly available at
`https://developer.garmin.com/connect-iq/articles/<fileName>`; the earlier review
used article HTML embedded in the site's JavaScript. Downloaded page data and
extracted text are local under ignored `scratch/official-docs/`, not
implementation dependencies.

Live documentation can change independently of our SDK 9.2.0 specimens. Check
API levels and target availability before adding a feature. Documentation
defines source/API behavior; it does not establish bytecode encodings or PRG
layouts. Those still need controlled SDK and simulator experiments.

## Monkey C topic subpages

These pages were reviewed for the topics listed here; their examples and linked
API contracts remain references for feature-specific experiments.

| Page | Useful topics |
| --- | --- |
| [Functions][functions] | Numeric types, operators, control flow, calls, symbols |
| [Objects and Memory][objects] | Construction, inheritance, lookup, imports, weak references |
| [Containers][containers] | Array allocation/literals, dictionaries, typed containers |
| [Monkey Types][types] | Gradual checking, unions, nullability, inference, overrides |
| [Exceptions and Errors][errors] | Recoverable exceptions, fatal errors, runtime limits |
| [Annotations][annotations] | Build selection, execution scopes, extended code |
| [Coding Conventions][conventions] | Naming, source organization, base initialization |
| [Compiler Options][options] | Target/build modes, type checking, optimization, API inputs |

**Officially documented:** [Containers][containers] distinguishes `new [size]`
allocation from `[value, ...]` initialization. Arrays may contain other arrays.
Dictionaries use `key => value` entries; a missing-key lookup returns null.
Typed array/dictionary forms are also documented. These give us source forms
for the next object/container experiments, rather than an encoding contract.

**Officially documented:** [Monkey Types][types] describes `as` declarations,
union types using `or`, nullable types, parameterized containers, and tuples.
`Void` is a return contract; unannotated returns default to `Any`. Local types
can evolve through assignments and branch joins. Overrides without decorations
inherit the parent's parameter/return types when their argument counts match;
explicit decorations must match the parent signature. Checking levels range
from disabled through gradual and informative to strict.

**Interpretation:** callback typing should use resolved native signatures.
Our fixed local types and inferred integer/void returns remain subset rules.
General Monkey C support will need a broader type model and explicit checking
policy, rather than presenting current restrictions as language requirements.

**Officially documented:** [Annotations][annotations] include debug/release and
test selection, background/glance availability, and type-checking controls.
`:extendedCode`, introduced at API level 5.1.0, provides supported devices with
16 MB of code space paged on demand beyond code loaded into the heap.

**Interpretation:** preserve annotation meaning through declaration collection;
build exclusions and runtime scopes are distinct concerns. Extended code merits
a separate specimen when larger programs require it. This documentation does
not settle ordinary CODE address selectors or DATA string offset limits.

**Officially documented:** [Exceptions and Errors][errors] describes typed and
catch-all handlers, `finally`, and `throw`. Fatal API errors cannot be caught.
Its error list states a current limit of 10 method arguments. Confirm that
limit, receiver accounting, and SDK/target applicability before changing our
frame/arity rules. Handler syntax alone does not resolve VM unwinding semantics.

**Officially documented:** [Compiler Options][options] distinguishes device
executables from IQ packaging, debug from release output, and type checking
from optimization. It lists `--disable-v2-opcodes` and identifies `api.db` as
API linking input and `api.mir` as API type input. These are useful independent
axes for differential specimens; we need not copy the SDK's CLI or formats.

The [coding conventions][conventions] recommend one class per source file and
calling the superclass initializer first. These are guidance, not evidence of
parser restrictions. Keep source organization separate from declaration
ownership in our multi-file compiler.

## Planning review: language, API, and project gaps

Reviewed 2026-10-11 against the current compiler subset. Re-read the functions,
objects, containers, types, exceptions, annotations, language/Jungle references,
and core topics for manifests, resources, build configuration, and application
lifecycle. Article snapshots and a URL list are local under
`scratch/official-docs/plan-review/`. No new SDK/runtime evidence was collected.

**Officially documented:** [Functions][functions] and the [language reference][language]
cover numeric types, operators/precedence, constants/enums, symbols, additional
loop/switch forms, member lookup, and method-reference callbacks. The
[objects guide][objects] also covers fields, static members, visibility,
construction, inheritance/dispatch, and weak references. These are broader than
our integer/string, explicit-module/self-call subset; implementing one opcode
does not establish the corresponding complete language feature.

**Officially documented:** [Manifest and Permissions][manifest] defines an
application entry as its AppBase class, with identity, app type, products, API
requirements, permissions, languages, and dependencies. Application name and
launcher icon refer to resources. Our CLI's selected startup method is a
bootstrap convenience, not the project's manifest entry contract.

**Officially documented:** [Resources][resources] describes XML input compiled
into a resource database and generated `Rez` identifiers, including strings,
bitmaps, fonts, menus, and JSON data. Resource scopes and target/localization
selection affect what is available. [Build Configuration][configuration]
describes device/family/language qualifiers; [application lifecycle][lifecycle]
extends beyond launching one console entry. These contracts do not specify
resource binary encodings or all PRG entry fields.

**Interpretation:** separate the backlog into language semantics, native API
descriptors/binding, lifecycle/UI, resources/containers, and project resolution.
Choose language increments through complete app examples. Project resolution
can be tested on selected inputs before complete language/resource support.
Packing an exported tree does not implement a resource compiler or Jungle
evaluator. [The plan](../plan.md) owns the order and acceptance criteria.

## Core Topics planning review

Reviewed 2026-10-11: all 44 linked article headings and overviews, with closer
reading of manifest, lifecycle, background/glance, resources/configuration,
input, properties/storage, styles, libraries, debugging, and unit testing.
This is a planning review, not an exhaustive API or example audit. Public
article snapshots and the navigation/article URL map are ignored under
`scratch/official-docs/core-topics-review/`. No new runtime evidence was collected.

The tree adds several distinct contracts:

- **Project and distribution:** manifest, build configuration, shareable
  libraries, security, publishing, beta/trial apps, and review requests.
  The first three affect resolved build inputs; signing already has a separate
  component. Store workflows do not require compiler implementations.
- **Application state and execution:** lifecycle, persistence, backgrounding,
  glances, properties/settings, intents, and notifications. These combine
  callbacks, compiled metadata, execution scopes, and native services.
- **UI and resources:** Views, layouts, graphics, input, native controls,
  resources, Monkey Style, attention APIs, and on-device watch-face editing.
  Source bindings, generated resource identities, and device capabilities must
  agree; a drawable example does not establish every resource format.
- **Native feature families:** HTTPS/authentication, mobile messaging/content,
  ANT/BLE/pairing, sensors/location, activity recording/control/prompts, user
  metrics, and complications. Grow descriptors and callback support through
  selected app examples. Companion Android/iOS SDKs are host integrations.
- **Validation tools:** debugging, unit testing, ERA, and profiling. Test builds
  and source/debug metadata affect compiler output; reproducing all SDK tools
  is outside the current plan.

**Officially documented:** [Manifest and Permissions][manifest] lists
`watchface`, `datafield`, `widget`, `watch-app`, and
`audio-content-provider-app`. Entry identifies an AppBase class; application
name/icon identify resources. API availability depends on app type as well as
target/API level and permissions, with operation-specific exceptions.
Barrel dependencies use their declared namespace and optional version
requirements: exact, minimum (`>=`), pessimistic (`~>`), or unrestricted.
The documented pessimistic example keeps major/minor fixed and allows later
micro versions. These external library contracts are distinct from our
current-only provisional format grammars.

**Officially documented:** [Backgrounding][background] selects annotated code
and scoped resources for a smaller execution environment, reached through
AppBase/service callbacks. [Glances][glances] similarly select code and have
device-dependent update lifecycles. Glance support begins at API 3.1.0; the
article distinguishes behavior before and after 4.0.0. Neither is simply a
different archive flag, and a glance is not an additional manifest app type.

**Officially documented:** [Input Handling][input] uses delegate callbacks
whose Boolean result reports whether input was handled; capabilities differ by
app type/device. [Properties and Settings][settings] defines resource-backed
defaults and settings references. [Persisting Data][persistence] distinguishes
those properties from runtime `Application.Storage` data.
[Monkey Style][styles] introduces a separate personality language, Jungle
selection, resource references, and generated `Rez.Styles` bindings.

**Officially documented:** [Unit Testing][tests] runs in the simulator. Test
functions use `:test`, accept a `Test.Logger`, return a Boolean verdict, and
must be static when members. They are excluded from ordinary builds and
included by a test build. Release assertions are removed.
[Shareable Libraries][libraries] includes barrel testing; [Debugging][debugging]
requires the executable, target, and debug XML for command-line debugging.
These document output requirements, not the runner/debug binary encodings.

**Interpretation:** supply manifest/target context while resolving native
contracts, then validate a visible and interactive source app. Extend project
resolution with mode/scope selection, resource references, and library
requirements. Settings, styles, tests, and other app profiles are explicit
extensions rather than prerequisites for the first app. The
[plan](../plan.md) owns sequencing; [project-build design][build-design] owns
the proposed build boundary. Scope partitioning, test discovery, resource
encoding, and device budgets still require controlled specimens.

## Language and class findings

**Officially documented:** the [language reference][language] describes optional
type checking over a dynamically typed language. `Number` is signed 32-bit;
arrays and dictionaries are object containers. Our fixed integer/string subset
is a deliberate restriction, not the complete type system.

**Officially documented:** [Objects and Memory][objects] describes `initialize`
as the constructor invoked by `new`, `self` as the current instance, and
superclass calls qualified with the superclass name. It also distinguishes
namespace declarations: `import` introduces the module name and its classes
into the type namespace; `using` introduces the module name and permits an
`as` alias. `using` is scoped to its containing class/module. These constructs
must not be implemented as interchangeable textual substitutions.

**Interpretation:** native-base calls need an explicit resolved call target;
ordinary module calls and superclass calls can have similar source spelling.
Name binding must determine ownership before instruction selection. Confirm
dispatch and constructor behavior with minimal SDK specimens before expanding
our class profile.

## Core Lang types and API declarations

Reviewed 2026-10-11: [language reference][language], [Object][lang-object], and
[Number][lang-number]. This review adds public-contract evidence, not new
SDK/simulator observations.

**Officially documented:** basic numeric/Boolean values are objects with methods.
The type table marks Long, Double, String, Array, and Dictionary as requiring
heap allocation; Number, Float, Boolean, Char, and Symbol are unmarked. Object
is the class-hierarchy root; modules do not inherit it.

Availability applies to individual members. Number exists since API 1.0.0,
`toChar()` since 1.3.0, and `compareTo()` since 5.0.0. Object's `weak()` starts
at 1.2.0. A class's introduction level does not establish all member availability.

The language reference reserves `native` for internal use. This review found no
publicly documented external/prototype declaration mechanism suitable for our
catalogue; it does not establish what internal SDK declaration forms exist.

**Interpretation:** core type identities and operator semantics should be read
alongside API contracts. Arithmetic need not be lowered as a method call or
preceded by heap allocation. Declarations and executable bindings are separate.

**Unverified hypothesis:** tagged values with immediate payloads or heap
references could explain these semantics. The documentation does not establish
that layout, allocation timing, or a boxing protocol. Mixed numeric behavior,
string addition, equality, and guarded optional API binding still need focused
specimens before extending our supported profile. See [API design][api-design].

## Lifecycle contracts

**Officially documented:** [AppBase][appbase] is the application base class.
Its example initializes the base explicitly. `onStart(state)` and `onStop(state)`
accept a dictionary or null and return void. `getInitialView()` returns an
array containing a view, optionally with an input delegate. The documented
sequence is startup, initial view, then termination cleanup.

**Officially documented:** [View][view] has its own constructor and callbacks.
`onLayout(dc)` and `onUpdate(dc)` receive a graphics context; `onShow()` and
`onHide()` have no parameters. Callback sequences vary by app type.

**Interpretation:** source-authored lifecycle needs null and object/container
values beyond today's integer parameters. Keep the current generated lifecycle
until these contracts can be expressed and runtime-tested independently.

## Jungle findings

**Officially documented:** [Jungle][jungle] uses newline-terminated assignments,
hash comments, semicolon-separated values, quoted strings, and `$(...)`
dereferences. File-local variables resolve after their file; qualifiers resolve
after all project files. Self-references in assignments are an immediate
evaluation exception, allowing an inherited path to be extended.

The default Jungle applies before supplied files; later files take precedence.
Relative paths use their Jungle's directory; default-Jungle paths use the
manifest's directory. The configuration identifies one project manifest.
Resource paths are processed left to right, allowing later definitions to
override earlier ones. Barrel paths can name archives or project Jungle files,
including bracketed groups; dependencies also appear in the manifest.

**Interpretation:** a parser alone is insufficient. The planned resolver needs
file provenance, ordered evaluation, inherited settings, and manifest-aware
path bases. Compare resolved inputs against the SDK using two Jungle files,
an inherited-path extension, and a direct barrel-project dependency before
specifying our supported subset.

[guide]: https://developer.garmin.com/connect-iq/monkey-c/
[language]: https://developer.garmin.com/connect-iq/reference-guides/monkey-c-reference/
[jungle]: https://developer.garmin.com/connect-iq/reference-guides/jungle-reference/
[api]: https://developer.garmin.com/connect-iq/api-docs/
[objects]: https://developer.garmin.com/connect-iq/monkey-c/objects-and-memory/
[appbase]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Application/AppBase.html
[view]: https://developer.garmin.com/connect-iq/api-docs/Toybox/WatchUi/View.html
[functions]: https://developer.garmin.com/connect-iq/monkey-c/functions/
[containers]: https://developer.garmin.com/connect-iq/monkey-c/containers/
[types]: https://developer.garmin.com/connect-iq/monkey-c/monkey-types/
[errors]: https://developer.garmin.com/connect-iq/monkey-c/exceptions-and-errors/
[annotations]: https://developer.garmin.com/connect-iq/monkey-c/annotations/
[conventions]: https://developer.garmin.com/connect-iq/monkey-c/coding-conventions/
[options]: https://developer.garmin.com/connect-iq/monkey-c/compiler-options/
[manifest]: https://developer.garmin.com/connect-iq/core-topics/manifest-and-permissions/
[resources]: https://developer.garmin.com/connect-iq/core-topics/resources/
[configuration]: https://developer.garmin.com/connect-iq/core-topics/build-configuration/
[lifecycle]: https://developer.garmin.com/connect-iq/core-topics/application-and-system-modules/
[core]: https://developer.garmin.com/connect-iq/core-topics/
[background]: https://developer.garmin.com/connect-iq/core-topics/backgrounding/
[glances]: https://developer.garmin.com/connect-iq/core-topics/glances/
[input]: https://developer.garmin.com/connect-iq/core-topics/input-handling/
[settings]: https://developer.garmin.com/connect-iq/core-topics/properties-and-app-settings/
[persistence]: https://developer.garmin.com/connect-iq/core-topics/persisting-data/
[styles]: https://developer.garmin.com/connect-iq/core-topics/monkey-style/
[tests]: https://developer.garmin.com/connect-iq/core-topics/unit-testing/
[libraries]: https://developer.garmin.com/connect-iq/core-topics/shareable-libraries/
[debugging]: https://developer.garmin.com/connect-iq/core-topics/debugging/
[build-design]: ../design/project-builds.md

[lang-object]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/Object.html
[lang-number]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/Number.html
[api-design]: ../design/api-declarations.md
