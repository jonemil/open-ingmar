# Native module and UI profiles

## Native module statements

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

## Native delegates

`new Toybox.WatchUi.BehaviorDelegate()` constructs a non-null delegate with no
arguments (API 1.0.0). Recognized WatchUi aliases and imported bare names work
as for View construction. Canonical BehaviorDelegate parameter/result types are
supported; delegate arrays and nullable delegates are not. Source subclasses and
the View/delegate callback result follow the
[class profile](callbacks.md#source-behavior-delegate).

## Native Views

`new Toybox.WatchUi.View()` constructs a non-null native View with no arguments.
`new WatchUi.View()` is also accepted; a `using Toybox.WatchUi as Ui` binding
permits `new Ui.View()`, while `import Toybox.WatchUi` permits `new View()`.
Constructor qualifiers cannot be shadowed by locals/parameters. Native View
constructor arguments and bare unimported `View` are unsupported. Unit mode
permits
[declared source View construction](callbacks.md#application-callback-profiles).
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

Source [getInitialView overrides](callbacks.md#source-initial-view-callback)
return a validated one-view positional result. A generic View array may contain
zero or many Views and cannot replace that shape. Delegate pairs remain planned.
See the [complete example](../../../examples/compiler-native-views/README.md)
and
[constructor/runtime evidence](../../investigations/compiler-native-views.md).

## Drawing context

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
