# IR text grammar

`ciq-ir` serializes one validated semantic function; `ciq-unit` serializes
ownership, class fields, and complete function documents. Neither is Garmin MIR.
This page owns accepted framing, directive operands, and type-tag spellings.
Operation pages define validation and meaning; their snippets are examples.

## Text and value definitions

One directive per nonempty line; whitespace separates fields. Blank lines,
LF/CRLF, and `#` comments outside strings are accepted. Comma separators are
rejected (type tags use their own punctuation). Names use ASCII assembly
identifiers. Quoted strings use the shared assembly string tokenizer: UTF-8,
`\n`, `\r`, `\t`, `\"`, `\\`, `\0`, and `\u{hex}` escapes. A `#` inside a string
is literal text. Unknown escapes and malformed quoting are errors.

```text
function-document = "ciq-ir" newline function-header newline operation+ ".end"
function-header = ".function" identifier metadata*
metadata = "parameters=" count | "receiver=instance"
         | "parameter-types=" parameter-tag (":" parameter-tag)*
         | "result=" declared-result-tag
value-id = "%" decimal-u32
local-id = "$" decimal-u32
block-id = "^" decimal-u32
builder-id = "&" decimal-u32
span = "span=" decimal-usize ":" decimal-usize
```

Each production line denotes a separate text record; `operation+` occupies one
line per operation. No input may follow `.end` except whitespace/comments.
Function metadata fields may appear in any order but cannot repeat. Parameter
count defaults to zero; omitted parameter types are Number (`integer`). Explicit
parameter-type counts must match `parameters`. The maximum is 254 parameters;
parameters plus locals may not exceed 255.

An omitted function result is inferred from the validated operations. Canonical
output writes a result tag when needed to preserve a non-inferable declaration;
ordinary concrete scalar, Array, native-object, tuple, and Void results are
inferred. Thus `declared-result-tag` is a subset of the value tags below, not a
permission to write every value tag in `.function result=`. It accepts Object,
WeakReference, ByteArray, Method, Symbol, interfaces, numeric aliases,
NumberFormat, Endian, opaque/typed Dictionary, exact source references, and
supported nullable scalar/core/interface/alias tags. Explicit non-null
`integer`, `long`, `float`, `double`, `char`, `string`, `boolean`, Array, native
View/delegate, tuple, and `void` result metadata are not parsed; returns express
those contracts.

Every operation may append one final `span=` field. Spans use source byte
offsets with start <= end; they are optional diagnostic provenance, not offsets
into saved IR. No filename, source text, API context, or generated PRG debug
information is serialized. Editing text may require changing or omitting spans.

## Operation syntax

In the tables, `%r` defines a result; other `%` operands consume values. `$l`,
`^b`, and `&b` denote local, block, and builder IDs. `args...` is zero or more
value IDs unless the operation's semantic contract requires otherwise. `count`,
indexes, and builder lengths use decimal u32. Optional trailing spans are
omitted from tables for readability.

### Values and storage

| Directive | Operands after the directive |
|---|---|
| `.integer` | `%r signed-decimal-i32` |
| `.long` | `%r signed-decimal-i64` |
| `.float` | `%r 0x<8 hex digits>` |
| `.double` | `%r 0x<16 hex digits>` |
| `.char` | `%r 0x<8 hex digits>` |
| `.boolean` | `%r true` or `%r false` |
| `.null`, `.self` | `%r` |
| `.string` | `%r "text"` |
| `.symbol` | `%r identifier` |
| `.constant` | `%r module member` |
| `.parameter` | `%r index` |
| `.local`, `.assign` | `$l %value` |
| `.read`, `.take` | `%r $l` |
| `.read-self` | `%r field [result=call-result-tag]` |
| `.assign-self` | `field %value` |
| `.positive`, `.negate`, `.not`, `.complement` | `%r %operand` |
| `.increment`, `.decrement` | `%r %operand` |
| `.add`, `.subtract`, `.multiply`, `.divide`, `.remainder` | `%r %left %right` |
| `.bit-and`, `.bit-or`, `.bit-xor`, `.shift-left`, `.shift-right` | `%r %left %right` |
| `.equal`, `.not-equal`, `.less`, `.less-equal`, `.greater`, `.greater-equal` | `%r %left %right` |
| `.instanceof` | `%r %operand class-path-or-source-tag` |
| `.cast` | `%r %operand cast-tag-or-source-tag` |
| `.has` | `%r %receiver %symbol` |
| `.discard`, `.println`, `.return` | `%value` |
| `.return-void` | no operands |

Integer constants permit optional minus but no plus, hex, suffixes, or
underscores; canonical negative zero is zero. Long constants similarly use
decimal i64. Raw Float/Double/Char constants preserve bits, including values the
source literal grammar cannot produce. Source numeric syntax is independent. See
[values and storage](README.md#values-and-storage), [control flow](README.md#control-flow), and
[core source types](../frontend/README.md#types-and-nullability) for operand domains and validation.

### Calls and objects

| Directive | Operands after the directive |
|---|---|
| `.call` | `%r module member [result=call-result-tag] args...` |
| `.call-discard` | `module member args...` |
| `.call-self` | `%r member [result=call-result-tag] args...` |
| `.call-self-discard` | `member args...` |
| `.call-object` | `%r %receiver class member [result=call-result-tag] args...` |
| `.call-object-discard` | `%receiver class member args...` |
| `.call-native` | `%r %receiver member args...` |
| `.call-native-discard` | `%receiver member args...` |
| `.call-native-module` | `%r module member args...` |
| `.call-native-module-discard` | `module member args...` |
| `.new-native` | `%r canonical-native-class` |
| `.new-source` | `%r class native-base args...` |
| `.base-initialize` | `native-base` |
| `.tuple-new` | `%r args...` |

User-call and field-read result defaults are `integer`; canonical output omits
that default. Native results come from executable API profiles, without a saved
`result=` assertion. Source classes and bases must resolve under unit
validation. Void calls use discard forms; they cannot define a value. See
[calls](README.md#calls-and-objects).

### Containers

| Directive | Operands after the directive |
|---|---|
| `.array-new` | `%r element-tag` |
| `.array-allocate` | `%r element-tag %length` |
| `.array-constant` | `%r element-tag scalar-payload...` |
| `.array-begin` | `&b element-tag length` |
| `.byte-array-new` | `%r %length` |
| `.byte-array-constant` | `%r 0x<hex pairs>` |
| `.byte-array-begin`, `.byte-options-begin` | `&b length` |
| `.array-fill` | `&b index %value` |
| `.array-fill-many` | `&b start-index values...` |
| `.array-finish` | `%r &b` |
| `.array-read` | `%r %array %index` |
| `.array-write` | `%array %index %value` |
| `.dictionary-new` | `%r [type=dictionary-tag]` |
| `.dictionary-constant` | `%r dictionary-tag (key-scalar value-scalar)...` |
| `.dictionary-begin` | `&b dictionary-tag length` |
| `.dictionary-fill` | `&b start-index (%key %value)...` |
| `.dictionary-finish` | `%r &b` |
| `.dictionary-read` | `%r %dictionary %key` |
| `.dictionary-write` | `%dictionary %key %value` |

ByteArrays reuse Array read/write/fill/finish operations with distinct validated
types. Bare `0x` is an empty ByteArray payload. Fill groups require at least one
value/pair. Builder identity is not an expression value. See
[container validation](README.md#containers-and-private-construction) for initialization, escape, constant,
allocation, and option-record restrictions.

Concrete Array constant payloads omit per-value tags: Number/Char use eight hex
digits after `0x`, Long/Double sixteen, Float eight, Boolean `true|false`,
String quoted text, and nullable elements also accept `null`. Numeric-alias,
Object, and interface element categories instead use tagged scalar tokens, as
Dictionary constants do: `null`, `number 0x<u32>`, `char 0x<u32>`, `long
0x<u64>`, `float 0x<u32>`, `double 0x<u64>`, `boolean true|false`, `string
"text"`. There is no Symbol, Method, weak-handle, source-reference, or
nested-container constant token. Validator domains decide which scalar tokens
satisfy a contract; syntactic framing does not broaden those domains.

### Control flow

| Directive | Operands after the directive |
|---|---|
| `.block` | `^b` |
| `.block-value` | `%r join-tag` |
| `.branch` | `%condition ^true-block ^false-block` |
| `.jump` | `^target` |
| `.jump-value` | `^target %value` |

An implicit single block ends with a return. Explicit blocks require explicit
terminators, reachable targets, and the [CFG validation](README.md#control-flow)
rules. Block parameters do not authorize arbitrary cross-block value uses.

## Type tags

The spelling `integer` means Number; `integer-union` means Lang.Integer.
`numeric` and `decimal` are the other numeric aliases. Common tags are `long`,
`float`, `double`, `string`, `boolean`, `char`, `object`, `symbol`, `method`,
`weak-reference`, `byte-array`, `comparable`, and `comparator`. Each supports a
`nullable-` form where the consumer allows it. `number-format` and `endian`
retain enum identity; `dictionary` is opaque. User-call tuple results also use
`one-view` and `view-delegate`. Native reference tags include `view`,
`behavior-delegate`, and callback-only `dc`/`nullable-dictionary`.

Exact source tags use `source(ClassName@Native.Base)` or
`nullable-source(ClassName@Native.Base)` with ASCII class names.
Bases use `Lang.Object`, `WatchUi.View`, or `WatchUi.BehaviorDelegate` for
constructible references. AppBase is a declaration/base identity, not a
constructible ordinary source-reference type. Nullable source tags are accepted
in parameter and function/call result metadata; field reads retain their
separate non-null source profile. Source tags are not join or native cast tags.

Array element tags use `number`, not `integer`, for Number; otherwise they use
the shared scalar/core/interface/alias tags, nullable forms, or `view`. A
parameter or call-result Array tag appends `-array` to its element tag; a join
tag prepends `array-`. Nullability belongs to elements, not the Array reference.
The Number parameter/call tag is `number-array`.

Typed Dictionary tags are `dictionary(key=>value)` without whitespace inside the
token. Key/value domains follow the [source
contracts](../frontend/README.md#dictionaries), with additional supported
core/interface/alias domains documented in their semantic pages. Keys exclude
Null. Typed Dictionary tags work for signatures, user-call results, fields, and
matching conditional joins; casts do not accept them. Join tags also admit
`null` and the supported native references. A tag's existence in one context
does not imply it is accepted in every other context.
[Value operations](README.md#values-and-storage) and [container operations](README.md#containers-and-private-construction) own
consumer-specific restrictions.

## Unit and class framing

```text
unit-document = "ciq-unit" newline unit-record+
unit-record = owner newline function-document | class-record
owner = ".owner global" | ".owner module" identifier
class-record = ".class" identifier native-base newline
               field-record* function-document+ ".end-class"
native-base = "Lang.Object" | "Application.AppBase" | "WatchUi.View"
            | "WatchUi.BehaviorDelegate"
```

An owner applies to exactly one following function document. There is no unit
end directive. Class field records precede all methods; class methods require
`receiver=instance`. Canonical output places global/module records first, then
classes, retaining declaration order inside each group. Each nested function is
validated before [unit ownership and call
checks](README.md#compilation-units-and-classes).

| Field directive | Operands after the directive |
|---|---|
| `.field-number` | `name signed-decimal-i32` |
| `.field-scalar` | `name scalar-tag` |
| `.field-numeric` | `name numeric-alias-tag` |
| `.field-object` | `name object` or `name nullable-object` |
| `.field-array` | `name element-tag` |
| `.field-dictionary` | `name dictionary-tag` |
| `.field-reference` | `name ClassName@Native.Base` |
| `.field-interface` | `name interface-tag` |
| `.field-symbol` | `name symbol-or-nullable-symbol [default=:identifier]` |
| `.field-method` | `name method` or `name nullable-method` |
| `.field-weak-reference` | `name weak-reference` or `name nullable-weak-reference` |
| `.field-byte-array` | `name byte-array` or `name nullable-byte-array` |

Scalar field tags are `integer`, `long`, `float`, `double`, `boolean`, `char`,
`string`, and their nullable forms. Alias tags are `integer-union`, `decimal`,
`numeric`, and nullable forms. Interface tags are `comparable`, `comparator`,
and nullable forms. Field records do not carry spans or arbitrary default
expressions. [Field semantics](../frontend/README.md#instance-fields) define
required initialization, metadata defaults, availability, and exact storage
checks.

## Examples

```text
ciq-ir
.function probe
.integer %0 41 span=120:122
.return %0 span=113:123
.end
```

```text
ciq-unit
.owner global
ciq-ir
.function probe
.call %0 Numbers answer
.return %0
.end
.owner module Numbers
ciq-ir
.function answer
.integer %0 41
.return %0
.end
```

```text
ciq-unit
.class Counter Application.AppBase
ciq-ir
.function start receiver=instance
.call-self-discard announce
.return-void
.end
ciq-ir
.function announce receiver=instance
.integer %0 41
.println %0
.return-void
.end
.end-class
```
