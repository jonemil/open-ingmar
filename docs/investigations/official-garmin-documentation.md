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

The guide and reference pages load their article bodies from JavaScript. On
this review the browser text reader returned navigation alone. The site's
`/page-data/connect-iq/<page>/page-data.json` identifies each article; the
linked JavaScript bundle contains its HTML. Reading those article bodies
provided the findings below. Downloaded page data and extracted text are local
under ignored `scratch/official-docs/`, not implementation dependencies.

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
