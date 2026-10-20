# Application callback profiles

## Source Views and drawing

`new ClassName(arguments)` constructs a declared WatchUi.View subclass.
Arguments must match its source initialize method's arity and types, including
exact source reference contracts. Argument expressions run in order before the
initializer; contextual empty Array arguments use its declared parameter types.
The existing 254-argument/frame limit applies. Native `new WatchUi.View()` still
takes no arguments. Its initializer must satisfy the [native-base
contract](../frontend/classes.md#source-initialization). Unit validation rejects
unresolved names, modules, AppBase construction, and missing initializers. The
result has the non-null native View contract; source identity is retained in
construction and source contracts, independently of VM storage. It may supply
the slot in `getInitialView()`. Calls on proven source instances follow the
[source-instance profile](../frontend/classes.md#calls-on-constructed-source-objects);
field access on another instance remains unsupported.

View callbacks inherit these contracts, including when annotations are omitted:

| Callback | Parameters | Result |
|---|---|---|
| onShow | none | Void |
| onUpdate | Toybox.Graphics.Dc | Void |
| onHide | none | Void |

All require API 1.0.0. Their signatures are checked against the enclosing native
base; they cannot be invoked as ordinary source callees. Dc is an opaque native
reference with the [bounded drawing profile](profiles.md#drawing-context). It
cannot be constructed or stored in an Array.

The application builder creates a DATA class for each source View, using native
import index one and binding callbacks to their source CODE. Constructor
lowering looks up the source class through globals, allocates it, invokes
`initialize`, and retains the instance while discarding the initializer result.
See the
[visible example](../../../examples/compiler-visible-view/README.md) and
[runtime evidence](../../investigations/compiler-visible-view.md).

## Source initial View callback

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
construction](../ir/calls.md#positional-result-construction). Unit validation
checks callback parameters and result alternatives. Application generation binds
native symbol `0x008002a0` to source CODE and omits the default blank-view body.
The existing bootstrap is used when no source callback exists. `getInitialView`
cannot be selected as the startup entry. See the [single-View
example](../../../examples/compiler-initial-view/README.md) and [input
counter](../../../examples/compiler-input-counter/README.md).

## Source behavior delegate

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

## Source startup callback

An AppBase class may define `function onStart(state)` with one parameter and a
void result. Its inherited native contract is Dictionary-or-null, represented by
`ParameterType::NullableDictionary`; ordinary parameters follow the
[source type contracts](../frontend/types.md#types-and-value-contracts). Callback contracts are resolved independently of VM symbol IDs,
slot numbers, and import indexes. Unit validation rejects wrong callback
signatures and direct user calls to onStart.

`function onStop(state)` uses the same resolved nullable-state/Void contract.
Its method record binds to native symbol `0x008002a1`; the runtime invokes it on
termination. It is neither a bootstrap entry nor an ordinary user-call target.
Supplying it does not change startup-entry selection. Normal shutdown can be
requested with [System.exit](profiles.md#native-module-statements).

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
