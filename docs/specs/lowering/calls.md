# Call and object lowering

## Calls on constructed source objects

Lowering evaluates the receiver, selects its method with `getsv`, prepares the
argument context when needed, invokes it, and discards statement results. Native
and text paths use identical lowering and ordinary member-symbol binding. See
the [object-call example](../../../examples/compiler-object-calls/README.md) and
[runtime evidence](../../investigations/compiler-object-calls.md).

## User module calls

Backend lowering selects `getmv @__ciq_module_<module> @__ciq_member_<member>;
invokemz`, for zero arguments. Nonempty calls additionally use `frpush`,
argument expressions in order, and `invokem <argument-count + 1>`. Module
bindings must identify `globals/<module>`; the short `<module>` symbol is a
distinct field identity. The compiler CLI supports `--symbols
<symbol-source.mcasm>` for CODE emission: resolve generated aliases by unique
stored names in the provided `.symbol` declarations, reject absent/ambiguous
targets, and preserve ordinary aliases for assembly inputs. Library consumers
can supply bindings through the assembly encoder. [Runtime
evidence](../../investigations/compiler-calls.md) confirms return values and
observable call order. Complete application generation creates module/method
DATA from the unit; raw CODE consumers must provide those records themselves.

## Native object construction

Lowering emits `getmv @__ciq_sdk_WatchUi @__ciq_native_View; newc; dup 0; getsv
@__ciq_native_initialize; dup 1; invokem 1; popv`. The instance remains on the
stack after the initializer result is popped. Symbol sources bind those aliases
to exactly one `Toybox_WatchUi`, class, and `initialize` declaration
respectively; the class alias is `__ciq_native_BehaviorDelegate` for delegate
construction. See [constructor
evidence](../../investigations/compiler-native-views.md).

## Positional result construction

Lowering allocates length one or two with `ipush1 <length>; newa`, then emits
`dup 0; ipush1 <index>; <stored slot read>; aputv` for each slot. The result is
published after all slots are filled. Array/View/InputDelegate exist at API
1.0.0; View construction requires its initializer's 2.1.0 level. No allocation
failure timing or memory equivalence with SDK lowering is implied. See
[input evidence](../../investigations/compiler-input-counter.md).

## Native module values

Lowering selects `getmv @__ciq_sdk_Lang @__ciq_native_format`, `frpush`, the
arguments in order, and `invokem` with receiver-inclusive arity. Explicit symbol
sources bind `Toybox_Lang` and `format`; application generation supplies their
native IDs. Concrete Array types survive the erased parameter boundary. See the
[source contract](../sdk/profiles.md#native-module-and-ui-profiles) for literal context and
callback effects.

## Native module statements

Lowering selects `getmv @__ciq_sdk_<module> @__ciq_native_<member>`, `invokemz`,
and `popv`. Explicit symbol sources must contain one matching module/member
identity; application generation supplies the observed native IDs. The operation
is an ordered statement, not a CFG terminator. IR still requires its ordinary
returns and branches; exit has no expression value. Wrong arity, unknown
declarations, and unsupported native statement contracts are errors.

## Native value calls

Lowering emits `spush @__ciq_symbol_<name>`. Raw CODE binding resolves that
alias by a unique stored name in the supplied symbol source. Complete
application generation allocates the name in the shared identity map and retains
its development name in SYMBOLS. Known native member names reuse their pinned
IDs. These are existing assembly/SYMBOLS formats, not a new bytecode encoding.

## Computed Dictionary construction

Begin lowers to a Number push, `newd`, and private frame storage before entry
effects. A fill evaluates/stores each pair before the next; non-final pairs use
`aputvdup`, and the last uses `aputv`. Finish transfers the private reference
and clears its frame slot using `lgetv; npush; lputv`. Source control flow
stages key/value operands into locals and emits separate fill groups, preserving
empty-stack ordinary edges and incoming-value consumption. Candidate inference
is provisional; final IR checking validates each entry and its API/null proofs.
Shared checks follow the [builder
rules](../ir/containers.md#private-container-checks).
