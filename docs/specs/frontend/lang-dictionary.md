# Core Lang Dictionary

## Dictionary values

`{}` creates a fresh empty `Toybox.Lang.Dictionary` (API 1.0.0). Whitespace and
comments may separate the braces. Dictionary references may initialize/copy or
replace Dictionary locals and pass through explicitly annotated non-null
Dictionary parameters/results. Copies preserve reference identity; reassignment
does not copy contents. Calls and construction retain source evaluation order.

`Toybox.Lang.Dictionary<K, V>` accepts invariant non-null numeric, String,
Boolean, Char, or Symbol keys and values of those types, optionally `or Null`.
It also accepts [comparison-interface keys and
values](interfaces.md#comparison-interface-dictionaries). The Dictionary itself
is non-null. Char keys or values require API 1.3.0; the other contracts require
1.0.0. `DictionaryType` carries the bounded contracts through source, semantic
IR, locals, function arguments/results, and API checks.

A direct `{}` receives its key/value contracts from an annotated return, call
parameter, or reassigned typed local. Without that context it stays opaque;
later uses do not retrospectively type an opaque local. Different key or value
contracts cannot widen or narrow through assignment. Typed references may pass
to opaque Dictionary parameters/results, but erasure cannot be reversed.

[Numeric alias containers](numeric-values.md#numeric-alias-containers) additionally admit bounded
mixed numeric classes under explicit or already established alias contracts.

Nonempty `{key => value, ...}` literals accept expressions producing homogeneous
numeric, String, Boolean, Char, or Symbol keys and homogeneous values with
optional Null. Without context, a non-null value establishes the value type; any
Null makes it nullable. Null-only values require an expected nullable contract.
Typed contexts check every entry without changing invariant key/value types.
Null keys, mixed scalar kinds without an expected interface, numeric alias, or
Object contract, container-valued entries outside the Object value profile, and
trailing commas are unsupported. Nested Dictionary literal syntax is also
unsupported, including inside entry call arguments. Ordinary calls and Array
reads may produce scalar entries.

Literal-only entries use `hpush` and a [scalar DATA
constant](../formats/data-dictionaries.md). Signed Number tokens use the
existing 32-bit range. Constant strings are UTF-8; their local pool length,
including the zero terminator, must fit u16. Every entry is type-checked before
repeated scalar keys retain their last value. Numeric constant keys match exact
class and bits: signed zeros stay distinct, and Double NaN keys never coalesce.
String keys match text. These rules preserve the tested Dictionary behavior
rather than ordinary numeric operator equality. Runtime hashing/lookups remain
native operations; no key conversions are emitted. See [wider numeric key
evidence](../../investigations/compiler-numeric-dictionaries.md). Parentheses
around literals do not introduce effects or require computed construction.

Computed literals allocate a private Dictionary before entry expressions. Each
pair evaluates key, then value, then inserts before the next pair. Every
overwritten expression still executes. The `newd` argument is the source entry
count, including duplicates; its capacity/allocation effect is not specified. No
partially constructed reference escapes. Branching operands use private locals
under the existing frame limit. Every evaluation returns a fresh mutable
Dictionary; String keys compare by text.

Computed value inference retains candidate scalar nullability, including
nullable call results. An expected contract instead checks actual values using
the full IR null proofs. A guarded nullable key still needs a non-null proof;
selecting its non-null class during inference does not authorize an unsafe key.

Opaque and typed references support `size()` → Number, `isEmpty()` → Boolean,
and `toString()` → String. Dictionary's `toString` override requires API 1.0.1;
it does not fall back to the older Object declaration. Conversion reflects the
current contents and produces a String unaffected by later Dictionary mutation.
Entry order is unspecified. Conversion uses ordinary native dispatch and String
result contracts, including chaining and discarded results.

Opaque and typed references also inherit the
[Object equality and hash observers](lang-values.md#object-equality-and-container-observers).

Typed references additionally support the following API 1.0.0 contracts:

| Form | Result |
|---|---|
| `d[key]`, `d.get(key)` | V or Null |
| `d[key] = value`, `d.put(key, value)` | Statement only |
| `d.hasKey(key)` | Boolean |
| `d.remove(key)` | Statement only |
| `d.keys()` | Array<K> |
| `d.values()` | Array<V> |

Enumeration returns independent mutable Arrays containing the current keys or
values, with unspecified order. Later Dictionary changes do not update these
Arrays, and Array edits do not change the Dictionary. Nullable V remains
nullable in the returned Array; non-null V does not acquire lookup's missing-key
Null. Array argument/result invariance, element checks, null proofs, and method
API minimums still apply. Opaque Dictionary references cannot supply K or V.

Reads require a key of K; writes require K and V. Null is permitted only when V
includes Null. Inserting Null retains the key; removal deletes it, and removing
an absent key leaves the Dictionary unchanged. Copies share mutations. Receiver,
key, and value expressions execute once in that order. Indexed access uses
`agetv`/`aputv`; explicit methods retain native dispatch.

Every lookup is nullable, including reads from non-null V and `hasKey` branches.
Store the result and use the existing non-null proofs before arithmetic, String
methods, or printing. A stored scalar result retains its value across later
Dictionary mutations. `hasKey` does not establish a proof for a later lookup.
Guarded nullable keys use the existing proof rules; there is no implicit key
conversion. Indexed compound assignments and increments remain unsupported.

Chaining and discarded results use existing native-call rules. Native callback
Dictionary-or-Null state supports these opaque methods under a non-null proof;
unguarded access is rejected. Its nullable declaration/assignment rules remain
unchanged. Ordinary source nullable Dictionary annotations are not yet
supported.

Matching typed Dictionary conditional arms retain their exact invariant
key/value contract and selected reference. Different contracts, opaque
Dictionary joins, and Dictionary/Null joins remain unsupported. Conditional
literals receive an available declared contract in each arm.

Dictionary `==`/`!=`, direct printing, `new Dictionary()`, and the other methods
remain unsupported. No key or value contract is inferred from an empty literal
without context. See
[the example](../../../examples/compiler-dictionary/README.md) and
[SDK evidence](../../investigations/compiler-dictionary.md).

### Planned Dictionary extensions

These are intended contracts, not executable support yet. Do not infer key/value
contracts from later writes or reinterpret an opaque callback Dictionary as a
typed one.

General source unions, Null keys, and explicit nested parameter contracts need
separate rules before acceptance. Nested references stored as
[Object values](containers.md#object-containers) remain erased. Object `method` follows the
[bounded Method profile](lang-values.md#method-values-and-invocation);
ordinary `weak` follows the [bounded reference
profile](lang-values.md#ordinary-weak-references).
