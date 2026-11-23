# Object-rooted source construction

2026-10-17, SDK/compiler/simulator 9.2.0, `fr265`, debug, `-O 0`.

**Officially documented:** [Lang.Object][object] is the root of the Monkey C
class hierarchy and dates from API 1.0.0.

**Observed:** an SDK probe accepts an implicit Object base and explicit
`extends Lang.Object`, with source initializers that omit native base calls.
The [tracked two-file specimen][example] additionally uses a Lang module alias,
independent Number fields, local aliases, Object observers, a retained Method,
and an Object-rooted comparator. Replacing all original Counter variables
leaves its Method bound to the original state (`6 + 4 = 10`); the replacement
remains at `100`. The specimen also passes Counter references through cross-file
parameters, module/self results, a retained CounterBox field, replacement, and Object
erasure. SDK/generated programs match the expected output. The original
reference remains independently usable after field replacement (`22`), while
the box refers to the replacement (`42`).

SDK DATA records carry an Object superclass and imported Object field.
The hidden constructor targets for both implicit and explicit classes contain
only `return`; source `initialize` runs separately through construction.
No new instruction or DATA record was needed for structured reconstruction.

## Import identity

The SDK symbol table distinguishes `Toybox_Lang` (`0x00800007`), short `Lang`
(`0x00800020`), and Object (`0x0080004c`). Its Object import at index `0x82`
is the pair `0x00800007 / 0x0080004c`. The reduced generated import table uses
that pair at index 2, or index 3 when a BehaviorDelegate import precedes it.
Class base references select the generated index, independently of SDK layout.

A generated specimen with a return-only hidden constructor but short Lang's
ID in its import pair crashed with `Symbol Not Found` during `newc`. Replacing
the module ID with `Toybox_Lang` made the complete specimen execute. Artifacts:
`scratch/check-root-objects-final/` (failed import) and
`scratch/check-root-objects-verified/` (accepted).
This is evidence for the observed import identity, not a rule for inferring
arbitrary module IDs from their names.

## Reproduction and limits

Build tracked sources through an ignored Jungle using the shared native-names
manifest/resources and `scripts/build-example.sh`. The [console checker][check]
validates saved stages, structured SDK/generated CODE/DATA/SYMBOLS, lossless
reconstruction, and runtime output. The initial SDK probe and its section export
are in `scratch/root-object-probe/`.

The extended reference check is in `scratch/check-root-references/`; it includes
SDK/generated structured round trips and runtime output. These probes do not
establish source superclass chains, Object
casts/refinement, or broader weak targets.
See the [class contract][spec] and [plan](../plan.md).

## Omitted initializers

2026-10-21, SDK/compiler/simulator 9.2.0, `fr265`, debug, `-O 0`.

**Observed:** the SDK accepts Object-rooted classes without `initialize`,
including an empty class, an explicit aliased Object base, and classes with
Number-literal or nullable String fields. The extended [tracked specimen][example]
constructs these classes and uses a comparator whose initializer is omitted.
It checks inherited Object dispatch, initial field values (`7` and Null), and
independent state after mutation (`8` versus `7`). SDK and generated runtime
results are checked against [expected output](../../examples/compiler-root-objects/expected.txt).
Structured export/reassembly needs no additional opcode or DATA record.

Reproduce with the shared console checker using `compiler-root-objects`.
The initial accepted probe and structured SDK export are in
`scratch/compiler-default-initialize/`; the extended comparison uses its `check/`
directory. This does not establish default initialization for other native bases
or source superclass chains.

## Static storage and receivers

**Observed, 2026-11-03:** SDK/compiler/simulator 9.2.0, `fr265`, `-O 0`,
minimum API 3.0.0. The independently authored
[static-members specimen](../../examples/static-members/README.md) reproduces
the successful cases; its README owns build/run commands and expected output.
No compiler or SDK implementation was inspected.

Static fields and methods occupy a separate DATA class record, referenced by
the instance class's `statics` header word and registered under the root statics
record. The instance record contains `own`, `bump`, and `initialize`; the static
record contains `literal`, `value`, `add`, `owner`, and a separate hidden
initializer. In the initial probe, these records were at DATA `0x4fc` and
`0x53e` respectively. Those offsets identify that probe, not a layout rule.

The static literal Number uses type `0x01`, value `7`; the expression-initialized
static Number starts as type `0x00`, value `0` and is assigned in the static
initializer. Static variable key low bytes are `0x40`; static method low bytes
are `0x50`, compared with `0x10` for ordinary instance methods. Hidden initializer
keys use `0x30`. These observations do not establish every flag bit's meaning.

Reading the class value alone leaves the trace at zero. Reading its static
literal then activates the expression initializer once, changing the trace to
`2`. Two allocations append `3` independently, giving `233`; they do not rerun
the static initializer. Static reads/writes and calls through instances share
the same mutable slot. A nullable uninitialized static defaults to Null.

The SDK emits class selection followed by `getsv` for direct static reads/calls,
and class selection, Symbol, value, `putv` for direct writes. Within the static
method, accesses use `getselfv` / `getself; spush; ...; putv`. Its `self` compares
unequal to the class-definition value. Methods bound to that static `self`, or
through `instance.method(:add)`, invoke successfully and update the shared slot.

Isolated source variants delimit the contract:

- Replacing `Counter.owner()` with `Counter` in
  `new Lang.Method(Counter.owner(), :add)` compiles and constructs, but invocation
  crashes with `Unexpected Type Error`. The failing CODE site is the outer
  `invoke` call, not constructor allocation.
- Printing `Counter.owner()` directly crashes with `Symbol Not Found` in native
  API conversion. The successful specimen compares identity instead; this does
  not establish inherited Object observer methods for static storage.
- Replacing the nullable `absent` declaration with
  `static var absent as Lang.Number;` is rejected as potentially uninitialized
  and not accepting Null.
- Replacing static `owner`'s body with `return own;` is rejected because the
  instance member is absent on static `self`.

**Interpretation:** class definitions, static storage, and source instances must
remain distinct receiver contexts. A static declaration must not acquire
instance-field proofs, and a class-definition Method binding must not be given
a proven static callable signature merely because the selector exists.
The [plan](../plan.md#core-language-worklist) owns the
remaining source/IR/lowering work. This specimen does not establish inheritance,
cross-class static cycles, or access-control behavior.

## Static constants

**Observed, 2026-11-03:** SDK/simulator 9.2.0, `fr265`, `-O 0`, minimum API
3.0.0, target 5.2.0. The [static-constant
specimen](../../examples/compiler-static-constants/README.md) produces matching
[SDK/generated console output](../../examples/compiler-static-constants/expected.txt)
through the shared specimen checker. Static Array/Method references survive
two allocations and retain shared contents; conditional initialization runs
once. The trace is `235` after first access and `23544` after the two instance
allocations. The SDK also accepts an added unannotated `static const EMPTY;`
declaration without an initializer.

A nested `Nested.Settings` initializer calling `Nested.select()` exposed the
statics record's lexical parent requirement. SDK DATA puts both the instance
definition and statics record under the runtime `Nested` module. Generated
statics initially parented to globals failed with `Symbol Not Found` during
that helper lookup. Using the containing module as parent made the generated
application match the SDK, including the final `10`. This applies to the
shared class-enum storage path too.

The SDK uses String DATA defaults directly for `MESSAGE`; our current profile
writes it in the statics hook before runtime expressions. The matched trace
establishes the specimen's observable order and identity, not identical SDK
instruction selection or static-initialization cycle behavior. Current
contracts and conservative readiness restrictions belong in the
[frontend spec](../specs/frontend/README.md#static-constants).

## Mutable static fields

**Observed, 2026-11-04:** SDK/simulator 9.2.0, `fr265`, `-O 0`, minimum API
3.0.0, target 5.2.0. The [mutable-static
specimen](../../examples/compiler-static-fields/README.md) passes the shared
SDK/generated console comparison and structured reconstruction. Its
[expected output](../../examples/compiler-static-fields/expected.txt) captures
the distinctive ordering and lifetime observations.

`Cold.VALUE = Trace.mark(1)` evaluates the RHS before first static initialization:
the trace becomes `12`, and the stored value is `1`. In contrast,
`Warm.VALUE += Trace.mark(7)` initializes/read-captures the old `6` first,
yielding `13` and trace `1267`. A later compound RHS overwrites `Cold.VALUE`
with `50`, but the compound result still uses the captured old value and becomes
`4`. Subsequent instance reads/writes see the same slot. Two allocations append
their independent `4` markers without rerunning statics. Replacing a static
Array through a branching assignment expires its old weak reference. An
unannotated Number-initialized static accepts String replacement; nullable and
nested-module fields also match the SDK output.

These observations establish the tracked assignment profile, not static-method
receiver behavior or unrestricted initialization cycles. Current contracts and
conservative readiness checks belong in the
[frontend spec](../specs/frontend/README.md#mutable-static-fields).

## Static method contexts

**Observed, 2026-11-04:** SDK/simulator 9.2.0, `fr265`, `-O 0`, minimum API
3.0.0, target 5.2.0. The [static-method
specimen](../../examples/compiler-static-methods/README.md) matches all 17
[expected SDK/generated output lines](../../examples/compiler-static-methods/expected.txt)
through the shared specimen checker, including structured reconstruction.

An initial `new Counter()` only marks instance allocation: the trace becomes
`4`. First static method access then runs the statics hook, which marks `2`
and calls static `seed()` to mark `3`; later allocations produce trace `42344`
without rerunning statics. This isolates a limit the earlier specimens did not:
allocation alone does not activate static storage. Instance initialization can
activate it indirectly by accessing a static member.

For static `owner(){ return self; }`, calls through two instances return
different receivers. `first.owner() == first` is true, while comparison with
the class value and with the statics hook's stored `OWNER=self` is false.
The earlier direct `Counter.owner()` probe instead returns static storage.
Thus a static declaration alone does not determine the physical receiver;
lookup/invocation context matters. The compiler's opaque receiver view retains
identity without supplying instance-member evidence.

Instance `method(:add)`, a Method constructed from a static body's `self`, and
a Method constructed from the hook's stored receiver all update the same
static slot. A method-only class needs a separate statics record even without
shared fields. A static method in `Nested.Settings` reaches an enclosing-module
helper through the statics record's parent. These cases require existing member
lookup/invocation and `getself`, not a new opcode or DATA record kind.

The [frontend contract](../specs/frontend/README.md#static-methods) defines the
checked member scope. The matched specimen does not establish Object inheritance
for every possible static receiver.

The [class-call specimen](../../examples/compiler-class-calls/README.md) matches
all 19 SDK/generated lines with structured reconstruction under the same profile.
`Cold.add(Trace.mark(3))` first initializes static storage (trace `2`), then
runs the argument (trace `23`); captures preserve this order across conditional
and constructed arguments. Aliases/root paths reach the same static declarations.
Direct `owner()` equals the hook's stored receiver, but differs from the class
definition; calling through an instance instead returns that instance.

Separate authored probes returned static `self` with Object and source-class
annotations, both implicitly and via explicit casts. SDK compilation and
execution succeed. On direct class calls, `instanceof` the annotated type is
false; through instances it is true. Thus accepted annotations do not prove
physical instance identity. The checked specimen exercises implicit Object/source-class returns through a
local receiver alias and the stored `OWNER` constant. Returning stored `OWNER`
fails `instanceof Cold` for both class and instance dispatch, while returning the
current `self` passes for instance dispatch only. Origin-aware opaque contracts
allow these annotations without treating an unannotated alias as an instance.

## Member visibility

**Officially documented, reviewed 2026-11-04:** the [Monkey C reference][hiding]
restricts hiding to class members. Default/explicit public access is unrestricted;
private access belongs to the declaring class; protected includes subclasses,
and hidden is its synonym. Modules and classes themselves remain public.

**Observed:** SDK/simulator 9.2.0, `fr265`, minimum API 3.0.0, target 5.2.0,
`-O 0`. The [visibility specimen](../../examples/compiler-visibility/README.md)
matches all nine SDK/generated output lines after structured reconstruction.
Private instance/static fields and methods, protected/hidden direct calls,
public callbacks, and private construction from a same-class factory work.
Outside reads/calls and outside construction with private/protected initializers
are rejected by the SDK compiler. Module-private declarations are syntax errors.

Protected/hidden field keys add `0x20`; method keys are `0x30`. Private selectors
are mangled, for example `<globals/Vault/<>VALUE>` and
`<globals/Vault/<statics>/<>STATIC>`, with ordinary low flags `0x00`/`0x40`.
Private methods use `0x10`/`0x50`; private instance constant K uses `0x10`.
Our generated selectors use an independent reserved identifier namespace.

Reflection is narrower than direct access. In an own-class method, `self has
:VALUE` and `self has :secret` are false for private members. SDK compilation
accepts `method(:secret)` but invocation fails with Invalid Value. Binding a
protected method also compiles, but its invocation fails with Symbol Not Found.
These probes justify public-only checked bindings; erased runtime lookup remains
separate.

An SDK variant combining `other.VALUE + other.secret()` inside
`read(other as Vault)` crashes compiler code generation with a critical-error
report. This does not establish an object-only privacy rule: the documented
contract is class-level access. Our source/IR tests allow same-class explicit
receivers; the SDK failure leaves that combined variant without runtime
comparison. Source-subclass access/override rules remain in the inheritance plan.

## Source superclass identities

**Observed, 2026-11-05:** SDK/simulator 9.2.0, `fr265`, minimum API 3.0.0,
target 5.2.0, `-O 0`. The [superclass specimen](../../examples/compiler-inheritance-identities/README.md)
matches all eight SDK/generated output lines after structured reconstruction.
Leaf precedes Middle and Root in source. Upcast arguments/results retain the
same object; runtime tests recognize all three source classes and Lang.Object.
A Leaf guard on a Root parameter permits Leaf's own field read. Contextual
Array<Root> literals accept a Leaf element without adding a conversion.

The SDK DATA records place Root at `0x0a19`, Leaf at `0x0a45`, and Middle at
`0x0a7a`. Leaf's `extends=0x00000a7a` agrees with its `Middle` class field
(key bits `0x70`, type 7); Middle's `extends=0x00000a19` agrees with its Root
field. Root instead uses a native import word `0x40000082`. Thus a source
superclass is a relocatable DATA class address, distinct from native imports
and Symbol-valued header roles. No new VM opcode or DATA record kind is needed.

This specimen has no inherited instance state or parent initializer effects.
It does not establish their order, default forwarding, protected subclass
access, or override compatibility. Those remain explicit construction/member
restrictions and work in the inheritance plan.

## Inherited instance methods

**Observed, 2026-11-07:** SDK/simulator 9.2.0, `fr265`, API 3.0.0–5.2.0,
`-O 0`. The [inherited-method specimen](../../examples/compiler-inherited-methods/README.md)
matches all ten SDK/generated output lines. Both PRGs pass structured CODE/DATA
export and exact unsigned reconstruction. It covers a three-level hierarchy,
overrides through a Root parameter, protected virtual self calls, inherited
Method bindings, and independent private helpers with different result types.

Focused authored variants establish these checking rules:

- Undecorated override parameter/result slots inherit independently. Explicit
  Number → String parameter/result changes fail; widening a Number parameter
  to Object also fails. A typed Number override of an undecorated parent slot
  compiles, whether only the parameter, only the result, or both are untyped.
- A private child helper may differ from the parent's private helper signature.
  The parent's body retains its own helper; private names fail ordinary `has`.
- Public → private/protected and instance → static redeclarations compile.
  A Root-typed call still returns Root's result (8), whereas an ordinary public
  override returns the child's result (108). These are distinct lookup profiles;
  treating every same-named child method as a virtual override is incorrect.
- Inherited field reads and public field shadowing compile/run. Inherited static
  reads return 10, but `Leaf.owner() == Root.owner()` is false: subclass static
  context differs from the parent's. These observations do not establish the
  complete storage/default initialization order.

The implemented profile supports same-access instance overrides and separate
private helpers. Access/kind shadowing, inherited fields/static storage, and
parent initialization remain explicit restrictions in the frontend contract.

## Inherited field allocation

**Observed, 2026-11-07:** SDK/simulator 9.2.0, `fr265`, API 3.0.0–5.2.0,
`-O 0`. The [field specimen](../../examples/compiler-inherited-fields/README.md)
matches all 23 SDK/generated lines and reconstructs structured CODE/DATA exactly.
Root's expressions print 1, 2, 3, Middle prints 4 after reading Root's field,
and Leaf prints 5. A second Leaf repeats that order with independent array
storage. Protected mutation and parent-bound reads address inherited public
storage. Root/Leaf private fields have different types and remain independent.

A separate authored probe gives Root defaults `a=mark(1)` and private
`secret=mark(2)`, Leaf default `b=mark(3)`, and ordinary initializers printing
10 and 20 respectively. `new Leaf()` prints 1, 2, 3, 20, followed by 4 for
`a+b` and 2 for Root's private helper. It never prints 10: parent allocation
work runs without an implicit parent ordinary-initializer call.

The SDK's Leaf allocation CODE explicitly uses `getselfv Root`, `getsv <init>`,
`invokemz` before its own default expressions. Root and Leaf have separate
allocation CODE entries; existing DATA class links and VM instructions suffice.
Our reserved default-body selector must likewise retain its declaring owner;
a shared virtual selector would select Leaf's body from Root's hook.

This establishes default allocation order, not general constructor forwarding,
public field shadowing, or safe receiver escape from partially initialized
subclasses. Those remain explicit frontend restrictions.

## Field shadowing

**Observed, 2026-11-07:** SDK/simulator 9.2.0, `fr265`, API 3.0.0–5.2.0,
`-O 0`. The [shadowing specimen](../../examples/compiler-field-shadowing/README.md)
matches all 34 SDK/generated output lines and reconstructs both PRGs' CODE/DATA
exactly. Focused variants establish the following:

| Parent default | Child default | Visible result after allocation |
|---|---|---|
| Number literal 11 | Number literal 22 | 22 |
| `mark(11)` | Number literal 22 | 11 |
| Number literal 11 | `mark(22)` | 22 |
| `mark(11)` | `mark(22)` | 22, after traces 11 then 22 |

Public Number → String redeclaration compiles. A parent-declared Number read
returns `"leaf"`; a parent self store of 30 then makes the child's String-declared
read return 30. These are static contracts, not runtime conversion guarantees.

With public Root.value=11 and protected Leaf.value=22, a global function reading
`x.value` for `x as Root` returns 11; inherited Root self lookup returns 22.
A Root self store updates the protected value while the explicit parent-bound
read remains 11. A private child field instead has an independent selector.
The SDK rejects redeclaring a protected parent field with any of public,
protected, or private access.

A visible constant child rejects direct `value=...` and `self.value=...` stores
in the parent body, even without a call to that body. A private child constant
does not. An explicit parent-typed parameter or local alias of self permits the
store; in the runtime probe, the visible child constant remains 22 afterwards.
Parent expression defaults with constant child shadows also compile/run.
A variable child may shadow a parent constant; the child store is accepted.
These observations justify separate direct-self and explicit-receiver checks,
while retaining metadata-driven runtime lookup. Existing instructions and DATA
records represent these cases without new encodings.

## Inherited static lookup and receivers

**Observed, 2026-11-07:** SDK/simulator 9.2.0, `fr265`, API 3.0.0–5.2.0,
`-O 0`. The [inherited-statics specimen](../../examples/compiler-inherited-statics/README.md)
matches all 17 SDK/generated lines; both PRGs reconstruct through structured
CODE/DATA exactly. `Leaf.owner()` visits Leaf, Middle, then Root statics, printing
30, 20, 10 before the receiver comparison. Each statics hook runs once. Empty's
inherited `owner()` receives a distinct receiver despite no own static members.
Root's value and Array remain parent-owned: a Root mutation is visible through
Leaf, and appending through Leaf changes Root's Array. Inherited static calls
through a Leaf instance receive that instance and can read/mutate the shared
parent field. Mere allocation adds no statics-init traces.

Separate two-class probes declare Root.value=`mark(10)` and Leaf.child=`mark(20)`.
Class-qualified Leaf.value reads 10 after traces 20 then 10. `Leaf.read()` with
Root's body `return value;`, `Leaf.bump()` with `value+=1`, and direct
`Leaf.value=30` each compile but fail with Symbol Not Found for value. The
inherited static receiver has no own value slot. SDK DATA places Root.value in
Root's separate statics record (key bits `0x40`) and Leaf.child in Leaf's record;
the records have lexical globals parents, not static superclass links.

Our generated negative read probe also fails at the same self lookup. Its
console reports numeric selector `0000000b` because generated symbols lack names;
CODE `getselfv 0x0000000b` and the Root static field key `0x00000b40` identify value.
No parent-storage rewrite is justified by successful inherited field reads.

A variant redeclaring Root's public static value in Leaf compiles in the SDK.
Its shadowed storage/access/initialization behavior remains the explicit next
static-field task; this variant has only compile evidence here.

## Static-field redeclarations

**Observed, 2026-11-07:** SDK/simulator 9.2.0, `fr265`, API 3.0.0–5.2.0,
`-O 0`. The [static-shadowing specimen](../../examples/compiler-static-shadowing/README.md)
matches all 29 SDK/generated lines; both PRGs reconstruct with structured
CODE/DATA. Parent and child slots mutate independently, including differently
typed and readonly/variable redeclarations. Separate Array initializers do not
share contents. Accessing a child's shadow does not run the parent's default;
a later parent expression default leaves the child's value unchanged.

Protected ancestor static fields reject public/protected/private redeclarations.
Private ancestor slots allow independent child declarations. A parent static
body may store its mutable field despite a visible constant child; the same
store in an instance body is rejected as a write to the child's constant.
A separate enum probe prints 11, 22, 11, 23 for parent enum ITEM and child static
variable ITEM before/after the child store.

Inherited `readRoot()` containing `return value` fails through the Leaf class
receiver even when both classes declare a public static value (11 and 22).
SDK and generated negative probes first print 22, 11, then Symbol Not Found.
Both SDK records use the same value Symbol, with separate `0x40` field entries;
no distinct public selector or shared storage follows from the spelling.

Mixed storage-kind probes accept instance access in both directions. External
`Leaf.value` is rejected when public parent/child fields change static/instance
kind. A static child body accepts bare/self access and `Leaf.value` within Leaf,
while the same qualified expression in Root, Grandchild, or an unrelated class
is rejected. A private instance ancestor does not cause that external rejection.
These context-dependent views remain an explicit compiler restriction; they
cannot safely use one closest-field contract for every receiver category.

## Mixed field lookup views

**Observed, 2026-11-07:** SDK/simulator 9.2.0, `fr265`, API 3.0.0–5.2.0,
`-O 0`. The [mixed-field specimen](../../examples/compiler-mixed-fields/README.md)
matches all 24 SDK/generated lines; both PRGs reconstruct through structured
CODE/DATA. Root instance Number/child static String declarations produce these
contracts: instance/bare instance access accepts `abs()` and rejects `length()`;
a static child body accepts `length()` and rejects `abs()`. Reversing storage
kinds gives String instance access and rejects either access in a static child
body. Eighteen controlled acceptance probes agree with the compiler.

Within Leaf, `Leaf.slot` is checked under that body's category but emits
`getselfv Leaf; getsv slot`. A Number-checked instance method can return the
static String `"leaf"`. With a private static child slot it returns `"private"`;
a bare instance-body read is Number-checked but also binds that private selector.
An external instance read remains 11. Symbols in the SDK confirm the private
selector is scoped to Leaf's statics.

External class qualification is rejected where an inherited instance field stays
visible. Same-class qualification compiles; Root, Grandchild, and unrelated
class scopes do not receive that exception. Structural witnesses use the closest
declaration: the static String field satisfies a mutable String field witness,
while a Number witness is rejected despite instance access returning a Number.
Direct/scoped stores also honor a lexical constant; an explicit instance store
can still update the inherited mutable instance slot. Private binding effects
must retain the static slot's initialization dependencies.

When an instance field hides the only inherited static member, SDK Leaf DATA
has no statics reference. Effective members, rather than any static ancestor,
therefore determine whether generated Leaf needs a statics record.

An unresolved metadata case is `Leaf.slot` for an initialized instance String.
SDK DATA stores a String reference in the class field; class-qualified access
in the probe compiles but yields System Error when printed. Our allocation-only
default initially returned Null. The compiler now rejects that class-metadata
read explicitly until default representation is implemented; ordinary instance
allocation/access remains supported. Numeric metadata defaults already round
trip through the same qualified directive.

## Instance field metadata defaults

**Observed, 2026-11-08:** SDK/simulator 9.2.0, `fr265`, minimum API 3.0.0,
target API 5.2.0, `-O 0`. The
[field-metadata specimen](../../examples/compiler-field-metadata/README.md)
matches SDK/generated console output and reconstructs both PRGs with structured
CODE/DATA. Literal String defaults use type `0x03` DATA references; Float,
Boolean, and Char defaults use inline `0x02`, `0x09`, and `0x13` payloads.
Readonly String constants use the same reference with field flag `0x10`.
An added `classFrozen()` method returning `Defaults.frozen` compiles to
`getselfv Defaults; getsv frozen`, retaining class lookup even for readonly fields.
Long/Double literals have type `0x0e`/`0x0f` with zero payloads and allocation
CODE writes the actual values. Grouped literals, including Number and String,
have Null metadata and allocation writes. The fixture distinguishes metadata
reads from instance values and verifies a parent expression overwrites a child's
literal default without replaying that literal afterward.

Class-qualified heap-backed reads expose a runtime limitation, not a usable
zero/default value. To reproduce the String case, insert
`System.println(a.classTextIsNull());` in the specimen after its existing class
reads. Both SDK and generated programs raise `Error: System Error` in that
method, even though the String is only compared with Null. Inserting
`System.println(a.classLong() == 0l);` instead raises `Error: Invalid Value` in
both programs. These isolated failures differ from the successful instance
reads and the Null result of `classExpr()`. They supersede the compiler's
previous blanket rejection: preserving the metadata and ordinary `getsv`
operation reproduces the SDK behavior without manufacturing an instance value.

## Method access shadowing

**Observed, 2026-11-08:** SDK/simulator 9.2.0, `fr265`, minimum API 3.0.0,
target API 5.2.0, `-O 0`. The
[method-access specimen](../../examples/compiler-method-access/README.md)
matches all 16 SDK/generated lines and reconstructs both PRGs through structured
CODE/DATA. An external explicit receiver retains the public Root method behind
a protected/private child method. Protected calls within the child invoke its
body; Root's inherited self call can also dispatch to that protected body.
A private child self call invokes its mangled selector. Its explicit receiver
call is checked against the private String signature but invokes the public
Number method, returning 11. SDK CODE confirms plain `getlocalv ... pick` for
that explicit call and a scoped private Symbol in the bare self call.
A separate class with only `private pick() as Number { return 99; }` and
`through(x as OnlyPrivate) as Number { return x.pick(); }` compiles, but invoking
`through` raises `Error: Symbol Not Found Error` in both SDK and generated
programs when no ordinary parent slot exists. SDK CODE still uses the plain `pick` selector. This distinguishes absent
runtime lookup from a private body's effects.

Acceptance probes used `Root.pick(Number) as Number` and a child
`pick(String) as String`, varying public/protected/private and instance/static
modifiers. For each combination, test no call, external Number/String calls,
instance-body Number/String calls, static-body Number/String calls, and an
own-class qualified String call: 48 cases. Public/protected redeclarations fail
override signature checks. Private instance declarations compile independently;
external calls retain the Number contract, while calls in the private child's
instance scope use String. A public grandchild skips the private intermediate
method when inheriting the Root signature. Separate probes confirm the same
private String contract for an explicit receiver inside its owning class.

The pending static profile differs: a private static child behind an instance
parent accepts Number in an instance body and String in a static body. Public
and protected static children still undergo parent signature checks. These are
checked lookup observations; this batch does not establish their runtime profile.

Method interface witnesses match declarations independently of public access.
PrivateLeaf satisfies a String-method shape and fails the Number-method shape,
although ordinary runtime calls reach the public Number slot. A private-only
String method also satisfies the String shape, and a private `compare(Object,
Object) as Number` satisfies Lang.Comparator. Protected methods satisfy matching
method shapes. The specimen's erased calls retain ordinary selectors and return
11 through both String and Number witnesses. These results concern method
witnesses; structural field access keeps its separate public/mutable checks.

## Static and instance method redeclarations

**Observed, 2026-11-13:** SDK/simulator 9.2.0, `fr265`, minimum API 3.0.0,
target API 5.2.0, `-O 0`. The
[static-method specimen](../../examples/compiler-static-method-shadowing/README.md)
matches all 19 SDK/generated lines and reconstructs both PRGs with structured
CODE/DATA. Instance calls retain Root's instance method behind a static child.
An own private static selector can still execute behind an instance-checked
Number signature, returning String. Own-class qualification retains that class
lookup. External `StaticLeaf.pick(0)` and `PrivateLeaf.pick(0)` fail SDK checking
because the instance view selects an instance method, despite the static child.

A separate declaration probe confirms that a static parent imposes no override
signature on static or instance children: `SharedRoot.pick(Number) as Number`
accepts either child `pick(String) as String`. By contrast, a nonprivate static
child behind an instance parent must retain the instance signature. With
Root.instance(Number), Middle.static(Number), and Leaf.instance(String), the
SDK accepts all three: the nearest static ancestor stops older override checks.
Private child signatures remain independent. These observations extend the
previous access-shadowing probes; they are not inferred from field behavior.

Initialization markers keep ordinary instance methods cold behind static
children. The private static self call activates its statics; class-qualified
calls do likewise. Allocating InstanceLeaf activates SharedRoot's statics through
the source-parent allocation hook, producing 300 before its instance calls.
The compiler's effect summaries therefore include source-parent static hooks.

Add `System.println(SharedLeaf.forward());` before exit to reproduce the inherited
static limitation: both SDK and generated programs report
`Error: Symbol Not Found Error` for `pick`, even though SharedLeaf declares it. The ordinary hidden
receiver and lookup instructions are preserved; no parent-qualified workaround
or artificial value conversion is inserted.

[hiding]: https://developer.garmin.com/connect-iq/reference-guides/monkey-c-reference/#datahiding

## Qualified instance methods and initializers

**Observed, 2026-11-23:** SDK/simulator 9.2.0, `fr265`, minimum API 3.0.0,
target API 5.2.0, `-O 0`. The
[qualified-call specimen](../../examples/compiler-qualified-instance-calls/README.md)
matches all 21 SDK/generated lines and reconstructs both PRGs with structured
CODE/DATA. Build the SDK source with the minimal-console scaffold and use the
[console checker][check] for the recorded expected output.

Within Leaf, `Leaf.pick(...)` selects its override, while `M.Root.pick(...)`
selects the parent method; both read the current Leaf object's field. A parent
`forward()` calling `virtual()` reaches Leaf's override. Root is in a separate
module, and the branched argument checks preservation of the callable/context
across staging. Explicit parent and own-class initializer calls change the same
object without repeating the allocation markers 10 and 20.

Separate SDK probes reject `Root.pick(5)` from an unrelated AppBase instance,
the same qualified instance calls from a static Leaf body, and private parent
methods. Protected parent methods and a module alias are accepted. Own private
qualification executes in the tracked specimen. Module aliases and root-qualified
parent paths also preserve the same output in a separate run.

Own-class `initialize` in a static body is an SDK exception to the ordinary
instance-method restriction: `static function run() { Root.initialize(); }`
compiles while the corresponding `Root.pick()` does not. The tracked specimen's
field-free initializer executes with that supplied static receiver. With
`var field as Number = 10` and `initialize() { System.println(field); }`, the SDK
instead raises `Symbol Not Found Error` for `field`; generated output reproduces
the same error with the numeric selector because it lacks SDK debug names.
Other-class and ancestor-qualified instance initializers in static bodies are
rejected.

SDK accepts `class Root { function initialize(n as Lang.Number) { ... } }`
with `class Leaf extends Root {}` and `new Leaf()`. A marker in Root's
initializer does not run: the default child initializer is empty, rather than
an inherited invocation with missing arguments. A private unused parent
initializer also compiles. These observations justify removing ancestor
initializer body/arity/access restrictions independently of the remaining
constructor-required ancestor-field restriction.

[object]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/Object.html
[example]: ../../examples/compiler-root-objects/README.md
[check]: ../../scripts/README.md#console-specimen-check
[spec]: ../specs/frontend/README.md#object-rooted-classes
