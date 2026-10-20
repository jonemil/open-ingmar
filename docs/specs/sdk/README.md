# SDK declarations and profiles

Core Lang value and container behavior lives with the
[frontend semantics](../frontend/README.md). Declarations contain signatures and
availability, not implementations; executable support is checked separately.
Project permissions, scope, and device selection remain
[build requirements](../build/README.md#planned-project-builds-and-targets).

Contents:

- [API declarations](#api-declarations)
- [API context inputs](#api-context-inputs)
- [Native module and UI profiles](#native-module-and-ui-profiles)
- [Application callbacks](#application-callbacks)

## API declarations

`ciq-compiler::api::core()` provides immutable runtime API declarations,
populated in Rust. It owns module/class/interface names, signatures, superclass
paths, introduction levels, callback flags, enum constants, generic parameters,
and exact receiver-result overrides. It contains no runtime method bodies or VM
bindings. No declaration-file grammar is defined.
[Architecture](../../architecture.md) separates declaration lookup, compiler
validation, and executable lowering.

### Declaration model

Types represent named classes/interfaces, Null, Void, unions, invariant Arrays
and Dictionaries, positional tuples, optional Symbol-keyed option fields, and
class type parameters. A declaration describes a runtime-provided API; compiler
analysis state does not belong in it. `Lang.Object` is the root class. Modules
are namespaces and do not inherit Object.

`Catalogue::type_alias()` expands Numeric, Integer, and Decimal into their
[scalar alternatives](../frontend/README.md#numbers-and-numeric-aliases). These typedefs are
neither classes nor enums. NumberFormat and Endian are declared enum types from
API 1.0.0; their published constants require 3.1.0. Executable signatures retain
these enum identities, while `.constant` retains the member's introduction level
and lowers to its integer value. ByteArray conversion methods declare optional
`offset`/`endianness` fields; the
[bounded source option builder](../frontend/README.md#bytearray) checks each
field before exposing a record, independently of general structural
assignability.

Structural interfaces record required method signatures separately from classes.
`Catalogue::interface()` resolves them without class inheritance or
construction; `interface_level()` includes the interface and enclosing module
introduction levels. Named interface types participate in
`ApiContext::check_type()`. Recording an interface alone does not enable source
annotations or executable implementations. Consumer-specific structural checks
must establish the required members before native-call checking can use that
interface type.

`Lang.Comparable` requires `compareTo(Object) as Number` from API 5.0.0.
`CompilationUnit::check_interface(class, interface, context)` checks directly
declared source members against a catalogue interface with exact parameter and
result contracts. It checks the interface, signature types, members, and native
base against the supplied target/minimum API. The query can be repeated on
parsed unit IR; it does not serialize a proof or change the unit. A source-class
parameter cannot satisfy Object merely because its native projection is Object.
This establishes signatures, not comparison behavior. Executable interface
values follow the [source
contract](../frontend/README.md#structural-interfaces).

`Lang.Comparator` requires `compare(Object, Object) as Number` from API 5.0.0.
Array `sort(Comparator or Null)` returns Void from 5.0.0. The executable
[sorting profile](../frontend/README.md#source-comparators) accepts Null or an
exact source-class witness. Unit validation requires a declared `compare` with
exactly matching Object parameters and Number result, following the SDK checks.
The witness is local to the consumer; it does not establish general interface
assignability, inheritance, or interface-typed storage.

Method declarations have a fixed parameter prefix and an optional repeated
argument type. Receiver instantiation substitutes both through generic container
bindings. Fixed declarations still require exact arity; variadic declarations
check the prefix and each additional argument. Only the supported native-value
call profiles authorize execution.

Object `method(Symbol)` returns Method since API 1.0.0. Method declares
`hashCode()` and variadic `invoke` from 1.0.0. Its declared invocation result is
Object; the exact Method receiver uses a conservative Object-or-Null result
because erased callbacks can return Void or Null. Repeated arguments accept
Object or Null. This does not recover a target signature or validate its
introduction level; see the [executable
contracts](../frontend/README.md#method-values-and-invocation).

`Catalogue::method()` follows superclass paths and returns the declaring class,
effective result, and required API level. `Method.result` retains the generic
signature; `ResolvedMethod.result` substitutes an exact receiver override.
Overrides do not propagate to subclasses. `Object.weak` therefore resolves to
the verified numeric, String, Boolean, Char, and Symbol types for those exact
immutable receivers, while other receivers retain WeakReference. Executable
support follows the
[bounded reference profile](../frontend/README.md#ordinary-weak-references).

Assignability follows declared superclass paths without implicit conversion or
acceptance of unknown classes. The executable profile also recognizes bounded
[native Comparable witnesses](../frontend/README.md#structural-interfaces).
Every alternative of an actual union must satisfy the expected type.
`Catalogue::method_for_type()` substitutes Array's `Element` or Dictionary's
`Key`/`Value` throughout argument/result types, including nested containers and
tuples. It retains the resolved declaration/availability and normalizes
substituted unions by flattening and removing duplicate alternatives. Unresolved
signature parameters return None; an opaque named Dictionary cannot invent
key/value contracts. `Catalogue::method()` still returns generic declarations.

Container parameters are invariant: differing Array elements or Dictionary
key/value contracts cannot widen or narrow through assignment. Typed containers
may widen to their opaque class or Object; erasure does not permit assigning
them back to a typed container. Tuples require exact arity and per-slot
assignability; they are not homogeneous Array contracts.

Dictionary declarations specialize `get(K)` to `V or Null`, `put(K, V)` and
`remove(K)` to Void, `hasKey(K)` to Boolean, `keys()` to `Array<K>`, and
`values()` to `Array<V>`. A non-null V rejects Null writes even though the broad
published method accepts Null. These members are API 1.0.0; Dictionary's own
`toString()` override requires 1.0.1. The existing `size`/`isEmpty` declarations
remain 1.0.0. [SDK type
probes](../../investigations/compiler-dictionary.md#typed-keyvalue-contracts)
support specialization and invariant contracts. The compiler still uses the
[bounded executable Dictionary profile](../frontend/README.md#dictionaries),
including typed source/saved IR, lookup/mutation, typed `keys`/`values` Arrays,
Dictionary's own `toString` dispatch, and inherited Object `equals`/`hashCode`
with shared argument and Dictionary identity contracts.

Declaration availability, assignability, and executable support are separate
checks. A broad Object parameter can have a narrower executable profile. Enum
constants retain their declared types even if VM encoding uses a Number. Source
class identity and native projection follow the
[class specification](../frontend/README.md#source-classes).

WeakReference `get()` returns Object or Null and `stillAlive()` returns Boolean;
both members and their class require API 1.2.0. Opaque Object results retain no
concrete contract automatically; callers can apply the supported type tests or
lexical assertions. Parameterized-container recovery remains unsupported.

### Availability

Introduction checks include the requested class, intervening bases, enclosing
modules, methods, and recursively referenced signature/container types. Class
existence does not establish availability of every initializer or method.
Deprecation alone does not mean unavailable; unknown restrictions stay explicit.

`ApiContext` contains target and application minimum API levels. The minimum
must be at least 1.0.0 and no greater than the target. Unguarded references must
fit both levels. Unknown declaration, unavailable target, deployment-minimum
violation, and unsupported lowering require distinct diagnostics.

Context is supplied externally through source/IR `_with_api` entry points or
`validate_api`; ordinary entry points omit availability checks but still
validate supported lowering. Saved IR does not serialize context. These levels
are caller assertions: they neither select a device nor set PRG deployment
metadata. Device, app-type, scope, permission, and guarded optional-API checks
remain planned in
[project builds](../build/README.md#planned-project-builds-and-targets).

### Executable contracts

The [source specification](../frontend/README.md#native-value-calls) owns
supported methods, signatures, API thresholds, and operator behavior. The
[IR specification](../ir/README.md#model-and-validation) owns their serialized representation and
shared validation. Catalogue entries must not silently widen those profiles.
Internal values map to `Lang` API identities; operator rules select result types
before the backend selects instructions. Numeric operations need not be method
calls. API object identity does not establish a heap allocation, tagged layout,
or boxing protocol; do not infer those from the language's heap markers.

Current catalogue inheritance/type references are checked by tests. Any future
external definition reader must establish equivalent closure and availability
invariants. A runtime may later associate these declarations with its own
implementations without coupling the catalogue to VM storage.

### Deferred core methods

This inventory covers outstanding Lang contracts, not just missing method names.
The [source specification](../frontend/README.md) owns implemented behavior and
its bounded argument/storage profiles. Compare each class's own and inherited
members against the reviewed public API before marking it complete. Remaining
operator/type-system dependencies and sequencing belong to the
[core completion gate](../../plan.md#core-completion-gate).

- **Number, Long, Float, Double, Boolean, Char, String:** resolve literal/Unicode
  discrepancies and validate remaining boundaries. Broader Object/interface/enum
  representation and inherited dispatch follow the core type work; existing
  own-method names largely have executable profiles.
- **Object and source objects:** complete source inheritance/overrides, member
  lookup, source-inheritance/general-union type tests and refinement, and broader
  inherited-method dispatch.
- **Array:** exact source-class elements, explicit nested parameters, nullable
  Array references, broader literal inference, and nullable sorting/Comparable
  contracts outside the checked profiles.
- **Dictionary:** general Object/reference keys, exact source/nested value
  contracts, general typed nullable references, and enumeration for those domains.
- **Symbol:** full selector/member-resolution contracts, shared with source
  inheritance and Method lookup work.
- **Method:** constructor and class/module receiver values,
  known-selector checks and concrete callback
  signature/refinement rules where required by consumers.
- **WeakReference and Object.weak:** broader source/Object targets
  and safe concrete recovery from get(); preserve exact
  immutable weak-result behavior.
- **Exception family:** exception construction, message/stack methods, source
  subclassing, and throw/try/catch/finally lowering and unwinding. Distinguish
  fatal runtime errors.
- **ResourceId:** resource-reference value/binding contracts and own/inherited
  methods, using minimal fixtures before broader resource generation.
- **Supporting types:** general unions and broader
  structural option types.

Array default sorting and checked source Comparator instances are implemented
within their [documented profile](../frontend/README.md#source-comparators).
Direct source-instance/self Method binding is likewise implemented; remaining
Method work concerns construction and broader contracts. Neither closes all
container or callable requirements.

This compiler provides checking, binding, and invocation of runtime methods.
Completion does not require implementing native firmware method bodies. The
[public API review](../../investigations/core-lang-completion.md) records inventory
provenance; future discoveries amend this list and the plan.

## API context inputs

`--target-api <major.minor.patch>` and `--min-api <major.minor.patch>` enable
introduction-level checks for source or IR input, in function and unit modes.
Supply both or neither. Each component is decimal `u16`; both levels must be at
least 1.0.0, and the minimum must not exceed the target. Malformed, duplicate,
partial, or inconsistent inputs fail before output creation. Assembly input
rejects these options because semantic API references are no longer available.

Known unguarded references must be available on the target and introduced no
later than the application's minimum API. Errors distinguish unknown members,
target unavailability, an insufficient minimum, and unsupported lowering. For
example, Number's `toChar` requires 1.3.0: target 1.0.0 reports unavailable;
target 1.3.0 with minimum 1.0.0 reports an insufficient minimum; both at 1.3.0
permit the conversion. A `has` predicate does not yet establish an API guard;
calls remain subject to these introduction-level checks.

```sh
cargo run -p ciq-compiler -- --target-api 1.0.0 --min-api 1.0.0 \
  --emit ir input.mc scratch/function.ciqir
cargo run -p ciq-compiler -- --from ir --target-api 1.0.0 --min-api 1.0.0 \
  scratch/function.ciqir scratch/function.mcasm
```

Levels are external build inputs, absent from `ciq-ir`/`ciq-unit` and PRG
deployment metadata. Resupply them when resuming text. Omitting both performs
the existing bounded contract/lowering checks without an API-level claim. These
inputs do not select a device, derive a level from the SDK, or check
permissions, app kind, execution scope, or VM capabilities. See the
[API declaration design](#api-declarations) for coverage.

## Native module and UI profiles

### Native module statements

`System.exit();` requests application termination and has no expression result.
Recognized aliases and `Toybox.System.exit()` use the native module-name rules.
The call requires zero arguments and is validated through its catalogue Void
contract and optional API context before lowering. Unknown module members, wrong
arity, and known members lacking supported lowering are rejected.

Source and saved IR retain this as an ordered native module statement; it is not
an IR return or branch. User module calls and dedicated `System.println`
operations retain their own forms. `WatchUi.requestUpdate()` is the other
supported native module statement: zero arguments, Void, API 1.0.0. It requests
a redraw of the current View and follows the same alias/validation rules.
Runtime exit invokes a supplied source `onStop` callback in the validated
console app; its unguarded Dictionary-or-null state is not a user-call argument.

### Native delegates

`new Toybox.WatchUi.BehaviorDelegate()` constructs a non-null delegate with no
arguments (API 1.0.0). Recognized WatchUi aliases and imported bare names work
as for View construction. Canonical BehaviorDelegate parameter/result types are
supported; delegate arrays and nullable delegates are not. Source subclasses and
the View/delegate callback result follow the
[class profile](#source-behavior-delegate).

### Native Views

`new Toybox.WatchUi.View()` constructs a non-null native View with no arguments.
`new WatchUi.View()` is also accepted; a `using Toybox.WatchUi as Ui` binding
permits `new Ui.View()`, while `import Toybox.WatchUi` permits `new View()`.
Constructor qualifiers cannot be shadowed by locals/parameters. Native View
constructor arguments and bare unimported `View` are unsupported. Unit mode
permits
[declared source View construction](#application-callbacks).
The initializer requires API 2.1.0, independently of the class's API 1.0.0
availability.

Parameters/results use `Toybox.WatchUi.View`; matching non-null references may
be stored, copied, reassigned, passed, or returned. Source result annotations
are required. Nullability, field access, scalar operators, printing, and native
View/Object member calls remain unsupported for these references. A View or
array result cannot serve as the console bootstrap entry result.

`Toybox.Lang.Array<Toybox.WatchUi.View>` is invariant. Literals infer View
elements from construction, typed calls, parameters, locals, or element reads;
direct empty literals use the usual contextual rules. All elements and typed
mutation/search arguments must be non-null Views. `addAll` takes another View
array; `reverse`/`slice` preserve that array type, and indexing returns View.
Array size/string/search/mutation/transform contracts use the shared generic
profile. Search/removal evidence covers repeated references to the same View; it
does not specify equality for arbitrary subclasses or distinct objects.
Signature and empty-allocation availability is 1.0.0; append/search/transform
methods require 1.3.0. Constructing a literal's View elements additionally
requires the initializer's 2.1.0 API level.

Source [getInitialView overrides](#source-initial-view-callback) return a
validated one-view positional result. A generic View array may contain zero or
many Views and cannot replace that shape. Delegate pairs remain planned. See the
[complete example](../../../examples/compiler-native-views/README.md) and
[constructor/runtime evidence](../../investigations/compiler-native-views.md).

### Drawing context

View.onUpdate inherits one non-null `Toybox.Graphics.Dc` parameter. Dc
references may be copied into locals and passed through explicitly typed
parameters. They cannot be constructed, printed, used in scalar operations, or
stored in arrays. The accepted member statements return Void:

```text
dc.clear()
dc.setColor(Number, Number)
dc.drawText(Number, Number, FontDefinition, String, Number)
```

This bounded profile uses integer coordinates/colors/justification and a String
text argument; it does not implement all Numeric, color, font, and null
alternatives in Garmin's API. These methods require API 1.0.0. Void calls cannot
produce expression values.

`Graphics.FONT_MEDIUM` produces FontDefinition with catalogue value 3, since API
1.0.0. The canonical Graphics module spelling, fully qualified
`Toybox.Graphics.FONT_MEDIUM`, and explicit module aliases are accepted.
Qualifiers cannot be shadowed by parameters/locals. A plain Number, including
literal 3, does not satisfy the font argument. Other Graphics constants and
general enum declarations are unsupported. See
[evidence](../../investigations/compiler-visible-view.md).

## Application callbacks

### Source Views and drawing

`new ClassName(arguments)` constructs a declared WatchUi.View subclass.
Arguments must match its source initialize method's arity and types, including
exact source reference contracts. Argument expressions run in order before the
initializer; contextual empty Array arguments use its declared parameter types.
The existing 254-argument/frame limit applies. Native `new WatchUi.View()` still
takes no arguments. Its initializer must satisfy the [native-base
contract](../frontend/README.md#source-initialization). Unit validation rejects
unresolved names, modules, AppBase construction, and missing initializers. The
result has the non-null native View contract; source identity is retained in
construction and source contracts, independently of VM storage. It may supply
the slot in `getInitialView()`. Calls on proven source instances follow the
[source-instance profile](../frontend/README.md#calls-on-constructed-source-objects);
field access on another instance remains unsupported.

View callbacks inherit these contracts, including when annotations are omitted:

| Callback | Parameters | Result |
|---|---|---|
| onShow | none | Void |
| onUpdate | Toybox.Graphics.Dc | Void |
| onHide | none | Void |

All require API 1.0.0. Their signatures are checked against the enclosing native
base; they cannot be invoked as ordinary source callees. Dc is an opaque native
reference with the [bounded drawing profile](#drawing-context). It cannot be
constructed or stored in an Array.

The application builder creates a DATA class for each source View, using native
import index one and binding callbacks to their source CODE. Constructor
lowering looks up the source class through globals, allocates it, invokes
`initialize`, and retains the instance while discarding the initializer result.
See the
[visible example](../../../examples/compiler-visible-view/README.md) and
[runtime evidence](../../investigations/compiler-visible-view.md).

### Source initial View callback

An AppBase class may define `function getInitialView()` with zero parameters.
Every return must be a direct literal with either one non-null View or a View
followed by a non-null BehaviorDelegate. Examples: `return [self.makeView()];`
or `return [view, new CounterDelegate(view)];`. Parentheses are permitted. All
paths must return the same shape: `TupleShape::OneView` or
`TupleShape::ViewDelegate`. The pair's second API slot is InputDelegate;
BehaviorDelegate satisfies it through native inheritance.

The compiler evaluates and stores slot expressions left to right before packing
a fresh positional result. Hidden locals share the normal frame budget. Ordinary
mutable Arrays cannot supply this contract. Wrong cardinality/order, Null,
incompatible slot types/annotations, and ordinary user calls to the callback are
rejected. Tuple source annotations and general tuple-expression inference remain
unsupported.

Saved IR retains the shape through [positional
construction](../ir/README.md#positional-result-construction). Unit validation
checks callback parameters and result alternatives. Application generation binds
native symbol `0x008002a0` to source CODE and omits the default blank-view body.
The existing bootstrap is used when no source callback exists. `getInitialView`
cannot be selected as the startup entry. See the [single-View
example](../../../examples/compiler-initial-view/README.md) and [input
counter](../../../examples/compiler-input-counter/README.md).

### Source behavior delegate

A BehaviorDelegate subclass may supply `function onSelect()` with zero
parameters and a Boolean result on every path, available since API 1.2.0. The
callback may mutate its retained View and call `WatchUi.requestUpdate()`. It
cannot be invoked as an ordinary source call. Other input callbacks and
event-object contracts are outside this profile.

Construction and local copies preserve exact delegate identity for ordinary
source-method dispatch. Canonical `Toybox.WatchUi.BehaviorDelegate` parameters
and results expose only the native base, erasing exact source identity. Named
source-class parameter/result annotations and reference fields retain exact
delegate identity. Delegate and View classes have distinct native bases; one
cannot satisfy the other's contract. Delegate arrays, nullable delegates, and
source inheritance are unsupported. See [input
evidence](../../investigations/compiler-input-counter.md).

### Source startup callback

An AppBase class may define `function onStart(state)` with one parameter and a
void result. Its inherited native contract is Dictionary-or-null, represented by
`ParameterType::NullableDictionary`; ordinary parameters follow the
[source type contracts](../frontend/README.md#types-and-nullability). Callback contracts are resolved independently of VM symbol IDs,
slot numbers, and import indexes. Unit validation rejects wrong callback
signatures and direct user calls to onStart.

`function onStop(state)` uses the same resolved nullable-state/Void contract.
Its method record binds to native symbol `0x008002a1`; the runtime invokes it on
termination. It is neither a bootstrap entry nor an ordinary user-call target.
Supplying it does not change startup-entry selection. Normal shutdown can be
requested with [System.exit](#native-module-statements).

Complete application generation requires `--entry Class.onStart` when that
callback exists. Its DATA method record binds directly to source CODE; the
generated startup wrapper is omitted. Constructor and default blank View remain
generated unless their source overrides are supplied. Without source onStart,
the existing zero-parameter startup-entry workflow remains available.

The state parameter can be read, copied into locals, and compared with null
using `==` in either operand order. Nullable Dictionary locals may receive null
without changing their declared contract. Dictionary access/construction,
printing, numeric operations, returns, and forwarding through integer user calls
remain unsupported. Null locals can only receive null; this subset does not
infer wider local types. Tests preserve these types through saved IR.

These state restrictions also apply to onStop. The tested simulator exit path
supplies null; other termination states remain unverified. Application
generation rejects class names colliding with pinned native symbols before
allocating class IDs, so a name such as `exit` cannot overwrite a native
binding.

Backend self calls select `getselfv @__ciq_member_<name>`, followed by
`invokemz` for zero arguments or `frpush`, ordered arguments, and `invokem
<count + 1>`. Statement calls append `popv`. No explicit receiver operand is
pushed. This sequence is observed in SDK 9.2.0 and verified in generated `fr265`
applications; other device/API profiles and broader inherited lookup remain
unverified.
