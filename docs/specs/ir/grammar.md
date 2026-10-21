# IR text grammar

`ciq-ir` serializes one validated semantic function; `ciq-unit` serializes
ownership, class fields, and complete function documents. Neither is Garmin MIR.
This page owns accepted framing, directive operands, and type-tag spellings.
The [IR reference](README.md) defines validation and meaning; its snippets are
examples, not additional syntax.

Notation: `|` selects alternatives; `?`, `*`, and `+` mean optional, repeated,
and one-or-more. Quotes denote literal text; `? description ?` denotes an
explicit lexical character class. Grammar whitespace separates fields unless
the rule is a single token (IDs, metadata, type tags, and numeric literals).
`newline` terminates a logical record; blank/comment lines are ignored between
records, and the final record need not have a physical trailing newline.

Start with `document` and follow these sections:

- [Unit and class framing](#unit-and-class-framing): document entry points,
  ownership, classes, and field records.
- [Text and value definitions](#text-and-value-definitions): function headers,
  metadata, and value/storage IDs.
- [Operation syntax](#operation-syntax): `operation` and every directive body.
- [Type tags](#type-tags): each metadata/operand type domain.
- [Lexical leaves](#lexical-leaves): identifiers, numbers, strings, and newlines.

Every unquoted rule name is defined below. Syntax and validation remain separate:
for example, `value-id*` describes spelling, not dominance or argument arity.

## Unit and class framing

```text
document = function-document | unit-document
function-document = "ciq-ir" newline function-header newline
                    operation+ ".end" newline
unit-document = "ciq-unit" newline unit-record+
unit-record = owner newline function-document | class-record
owner = ".owner" "global" | ".owner" "module" identifier
class-record = ".class" identifier native-base newline
               field-record* function-document+ ".end-class" newline
native-base = "Lang.Object" | "Application.AppBase" | "WatchUi.View"
            | "WatchUi.BehaviorDelegate"
```

The `ciq-unit` header is the first physical line; blank/comment lines may follow
it. Function documents use the `function-header` and `operation` rules in the
next sections. Class records use `field-record` below before their methods.

An owner applies to exactly one following function document. There is no unit
end directive. Class field records precede all methods; class methods require
`receiver=instance`. Canonical output places global/module records first, then
classes, retaining declaration order inside each group. Each nested function is
validated before [unit ownership and call
checks](README.md#compilation-units-and-classes).

```text
field-record = field-body newline
field-body = ".field-number" identifier signed-decimal-i32
           | ".field-scalar" identifier scalar-field-tag
           | ".field-numeric" identifier numeric-alias-tag
           | ".field-object" identifier ("object" | "nullable-object")
           | ".field-array" identifier element-tag
           | ".field-dictionary" identifier dictionary-tag
           | ".field-reference" identifier (source-tag | nullable-source-tag)
           | ".field-interface" identifier interface-tag
           | ".field-symbol" identifier ("symbol" | "nullable-symbol")
             ("default=:" identifier)?
           | ".field-method" identifier ("method" | "nullable-method")
           | ".field-weak-reference" identifier
             ("weak-reference" | "nullable-weak-reference")
           | ".field-byte-array" identifier ("byte-array" | "nullable-byte-array")
```

Scalar field tags are `integer`, `long`, `float`, `double`, `boolean`, `char`,
`string`, and their nullable forms. Alias tags are `integer-union`, `decimal`,
`numeric`, and nullable forms. Interface tags are `comparable`, `comparator`,
and nullable forms. Field records do not carry spans or arbitrary default
expressions. [Field semantics](../frontend/README.md#instance-fields) define
required initialization, metadata defaults, availability, and exact storage
checks.

## Text and value definitions

One directive per nonempty line; whitespace separates fields. Blank lines,
LF/CRLF, and `#` comments outside strings are accepted. Comma separators are
rejected (type tags use their own punctuation). Names use ASCII assembly
identifiers. Quoted strings use the shared assembly string tokenizer: UTF-8,
`\n`, `\r`, `\t`, `\"`, `\\`, `\0`, and `\u{hex}` escapes. A `#` inside a string
is literal text. Unknown escapes and malformed quoting are errors.

```text
function-header = ".function" identifier metadata*
metadata = "parameters=" decimal-usize | "receiver=instance"
         | "parameter-types=" parameter-tag (":" parameter-tag)*
         | "result=" declared-result-tag
value-id = "%" decimal-u32
local-id = "$" decimal-u32
block-id = "^" decimal-u32
builder-id = "&" decimal-u32
span = "span=" decimal-usize ":" decimal-usize
```

Only `newline` marks a record boundary; wrapped productions are not multiline
input. `operation+` occupies one line per operation. A standalone function
document ends at `.end` except for whitespace/comments; an enclosing unit may
continue with its next method, owner, or class record.
Function metadata fields may appear in any order but cannot repeat. Parameter
count defaults to zero; omitted parameter types are Number (`integer`). Explicit
parameter-type counts must match `parameters`. The maximum is 254 parameters;
parameters plus locals may not exceed 255.

An omitted function result is inferred from the validated operations. Canonical
output writes a result tag when needed to preserve a non-inferable declaration,
including any non-Void result when the body has no value returns. Headers accept
the same supported non-Void types as call results, including concrete scalars,
Arrays, native objects, tuples, and source references. Ordinary returning bodies
may omit inferable concrete results. Void is inferred; `result=void` is not
accepted. Metadata does not waive return, call, class, or API validation.

Every operation may append one final `span=` field. Spans use source byte
offsets with start <= end; they are optional diagnostic provenance, not offsets
into saved IR. No filename, source text, API context, or generated PRG debug
information is serialized. Editing text may require changing or omitting spans.

## Operation syntax

`function-document` contains one or more `operation` records. Each consists of
one directive body and an optional trailing source span:

```text
operation = operation-body span? newline
operation-body = value-operation | storage-operation | unary-operation
               | binary-operation | comparison-operation | predicate-operation
               | sink-operation | call-operation | object-operation
               | array-operation | dictionary-operation | control-operation
```

In each result-producing directive the first `value-id` defines its result;
other value IDs consume earlier values under the CFG rules. Repetition is
explicit: `value-id*` permits zero arguments, `value-id+` requires at least one.
Builder counts/indexes use decimal u32; parameter indexes use decimal usize.

### Values and storage

```text
value-operation = ".integer" value-id signed-decimal-i32
                | ".long" value-id signed-decimal-i64
                | ".float" value-id fixed-bits32
                | ".double" value-id fixed-bits64
                | ".char" value-id fixed-bits32
                | ".boolean" value-id boolean
                | (".null" | ".self") value-id
                | ".string" value-id quoted-string
                | ".symbol" value-id identifier
                | ".constant" value-id module-name identifier
storage-operation = ".parameter" value-id decimal-usize
                  | ".assign-parameter" decimal-usize value-id
                  | ".declare-local" local-id local-contract-tag
                  | ".variable" local-id value-id?
                  | ".read-variable" value-id local-id local-contract-tag
                  | (".local" | ".assign") local-id value-id
                  | (".read" | ".take") value-id local-id
                  | ".read-self" value-id identifier call-result-metadata?
                  | ".assign-self" identifier value-id
unary-operation = unary-directive value-id value-id
unary-directive = ".positive" | ".negate" | ".not" | ".complement"
                | ".increment" | ".decrement"
binary-operation = binary-directive value-id value-id value-id
binary-directive = ".add" | ".subtract" | ".multiply" | ".divide" | ".remainder"
                 | ".bit-and" | ".bit-or" | ".bit-xor" | ".shift-left"
                 | ".shift-right"
comparison-operation = comparison-directive value-id value-id value-id
comparison-directive = ".equal" | ".not-equal" | ".less" | ".less-equal"
                     | ".greater" | ".greater-equal"
predicate-operation = ".instanceof" value-id value-id (class-path | source-tag)
                    | ".cast" value-id value-id
                      (cast-tag | source-tag | nullable-source-tag)
                    | ".has" value-id value-id value-id
sink-operation = (".discard" | ".println" | ".return") value-id
               | ".return-void"
```

Integer constants permit optional minus but no plus, hex, suffixes, or
underscores; canonical negative zero is zero. Long constants similarly use
decimal i64. Raw Float/Double/Char constants preserve bits, including values the
source literal grammar cannot produce. Source numeric syntax is independent. See
[values and storage](README.md#values-and-storage), [control flow](README.md#control-flow), and
[core source types](../frontend/README.md#types-and-nullability) for operand domains and validation.

### Calls and objects

```text
call-operation = ".call" value-id module-name identifier
                 call-result-metadata? value-id*
               | ".call-discard" module-name identifier value-id*
               | ".call-global" value-id identifier call-result-metadata? value-id*
               | ".call-global-discard" identifier value-id*
               | ".call-self" value-id identifier call-result-metadata? value-id*
               | ".call-self-discard" identifier value-id*
               | ".call-object" value-id value-id identifier identifier
                 call-result-metadata? value-id*
               | ".call-object-discard" value-id identifier identifier value-id*
               | ".call-native" value-id value-id identifier value-id*
               | ".call-native-discard" value-id identifier value-id*
               | ".call-native-module" value-id module-name identifier value-id*
               | ".call-native-module-discard" module-name identifier value-id*
call-result-metadata = "result=" call-result-tag
object-operation = ".new-native" value-id class-path
                 | ".new-source" value-id identifier native-base value-id*
                 | ".base-initialize" native-base
                 | ".tuple-new" value-id value-id*
```

User-call and field-read result defaults are `integer`; canonical output omits
that default. Native results come from executable API profiles, without a saved
`result=` assertion. Source classes and bases must resolve under unit
validation. Void calls use discard forms; they cannot define a value. See
[calls](README.md#calls-and-objects).

### Containers

```text
array-operation = ".array-new" value-id element-tag
                | ".array-allocate" value-id element-tag value-id
                | ".array-constant" value-id element-tag array-scalar+
                | ".array-begin" builder-id element-tag decimal-u32
                | ".byte-array-new" value-id value-id
                | ".byte-array-constant" value-id byte-payload
                | (".byte-array-begin" | ".byte-options-begin") builder-id decimal-u32
                | ".array-fill" builder-id decimal-u32 value-id
                | ".array-fill-many" builder-id decimal-u32 value-id+
                | ".array-finish" value-id builder-id
                | ".array-read" value-id value-id value-id
                | ".array-write" value-id value-id value-id
dictionary-operation = ".dictionary-new" value-id ("type=" dictionary-tag)?
                     | ".dictionary-constant" value-id dictionary-tag
                       (tagged-scalar tagged-scalar)+
                     | ".dictionary-begin" builder-id dictionary-tag decimal-u32
                     | ".dictionary-fill" builder-id decimal-u32
                       (value-id value-id)+
                     | ".dictionary-finish" value-id builder-id
                     | ".dictionary-read" value-id value-id value-id
                     | ".dictionary-write" value-id value-id value-id
array-scalar = bits32 | bits64 | boolean | quoted-string | "null" | tagged-scalar
tagged-scalar = "null" | ("number" | "char" | "float") bits32
              | ("long" | "double") bits64 | "boolean" boolean
              | "string" quoted-string
```

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

```text
control-operation = ".block" block-id
                  | ".block-value" value-id join-tag
                  | ".branch" value-id block-id block-id
                  | ".jump" block-id
                  | ".jump-value" block-id value-id
```

An implicit single block ends with a return. Explicit blocks require explicit
terminators, declared targets, and the [CFG validation](README.md#control-flow)
rules. Disconnected blocks remain checked. Block parameters do not authorize
arbitrary cross-block value uses.

## Type tags

`metadata`, `call-result-metadata`, `field-body`, and typed operation operands
reach these rules. Tags are single whitespace-free fields: punctuation and
prefixes concatenate inside the field.

```text
parameter-tag = common-tag | array-tag | source-tag | nullable-source-tag
              | "nullable-dictionary"
call-result-tag = common-tag | array-tag | source-tag | nullable-source-tag
                | "one-view" | "view-delegate"
declared-result-tag = call-result-tag
common-tag = scalar-tag | nullable-scalar-tag | core-tag | nullable-core-tag
           | numeric-alias-tag | interface-tag | enum-tag | "dictionary"
           | dictionary-tag | "view" | "dc" | "behavior-delegate"
scalar-tag = "integer" | other-scalar-tag
other-scalar-tag = "long" | "float" | "double" | "string" | "boolean" | "char"
nullable-scalar-tag = "nullable-" scalar-tag
scalar-field-tag = scalar-tag | nullable-scalar-tag
core-tag = "object" | "symbol" | "method" | "weak-reference" | "byte-array"
nullable-core-tag = "nullable-" core-tag
alias-tag = "integer-union" | "decimal" | "numeric"
numeric-alias-tag = alias-tag | "nullable-" alias-tag
interface-name-tag = "comparable" | "comparator"
interface-tag = interface-name-tag | "nullable-" interface-name-tag
enum-tag = "number-format" | "endian"
source-tag = "source(" source-identity ")"
nullable-source-tag = "nullable-source(" source-identity ")"
source-identity = identifier "@" source-base
source-base = "Lang.Object" | "WatchUi.View" | "WatchUi.BehaviorDelegate"
element-tag = "number" | "nullable-number" | other-scalar-tag
            | "nullable-" other-scalar-tag | core-tag | nullable-core-tag
            | numeric-alias-tag | interface-tag | "view"
array-tag = element-tag "-array"
dictionary-tag = "dictionary(" dictionary-key-tag "=>" dictionary-value-tag ")"
dictionary-key-tag = scalar-tag | alias-tag | interface-name-tag
                   | "symbol" | "method" | "byte-array"
dictionary-value-tag = scalar-tag | nullable-scalar-tag | core-tag
                     | nullable-core-tag | numeric-alias-tag | interface-tag
join-tag = scalar-tag | nullable-scalar-tag | core-tag | nullable-core-tag
         | numeric-alias-tag | interface-tag | enum-tag | dictionary-tag
         | "array-" element-tag | "null" | "byte-options"
         | source-tag | nullable-source-tag
local-contract-tag = join-tag | call-result-tag
cast-tag = scalar-tag | nullable-scalar-tag | core-tag | nullable-core-tag
         | numeric-alias-tag | enum-tag | "dictionary"
```

The contexts deliberately differ. Source casts use `source-tag` or
`nullable-source-tag` in `predicate-operation`; `instanceof` accepts only the
non-null source target tag. Field-read result metadata further restricts `call-result-tag`
under [field validation](README.md#calls-and-objects).

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
in parameter, function/call result metadata, and source-reference field
declarations/reads. Field declarations use the same source tags as signatures.
Source tags also serve as exact block-value join contracts; they are not native
API cast tags. `.cast` resolves them as separate source assertion contracts.

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
`null` and `byte-options`; standalone View/Dc/delegate tags are not join tags.
A tag's existence in one context does not imply acceptance in another context.
[Value operations](README.md#values-and-storage) and [container operations](README.md#containers-and-private-construction) own
consumer-specific restrictions.

## Lexical leaves

Used by framing, metadata, operations, and type tags. Except for quoted strings,
each leaf below occupies one field. `fixed-bits32`/`fixed-bits64` require exactly
8/16 hex digits; `bits32`/`bits64` permit variable digit counts within their range.
Canonical output pads bit patterns to the full width. Decimal forms must fit the
named integer width; IDs have no sign. `count`/`index` are not separate grammar
symbols: the relevant directive names its numeric rule directly.

```text
identifier = (ascii-letter | "_") (ascii-letter | digit | "_")*
module-name = identifier ("." identifier)*
class-path = identifier ("." identifier)*
ascii-letter = ? ASCII A-Z or a-z ?
digit = ? ASCII 0-9 ?
hex-digit = ? ASCII 0-9, A-F, or a-f ?
decimal-u32 = digit+
decimal-usize = digit+
signed-decimal-i32 = "-"? digit+
signed-decimal-i64 = "-"? digit+
boolean = "true" | "false"
bits32 = "0x" hex-digit+
bits64 = "0x" hex-digit+
fixed-bits32 = "0x" hex4 hex4
fixed-bits64 = "0x" hex4 hex4 hex4 hex4
hex4 = hex-digit hex-digit hex-digit hex-digit
byte-payload = "0x" (hex-digit hex-digit)*
quoted-string = '"' (string-character | escape)* '"'
string-character = ? Unicode scalar except double quote, backslash, CR, LF ?
backslash = ? U+005C ?
escape = backslash ('"' | backslash | "n" | "r" | "t" | "0"
                 | "u{" hex-digit+ "}")
newline = ? logical line boundary, LF or CRLF; EOF may end the final record ?
```

`\u{...}` must name a valid Unicode scalar. Source keywords do not reserve saved
IR identifiers. Module/class paths remain subject to the corresponding call,
API, and class-test validation; a dotted identifier alone does not authorize an
operation. Function metadata has no repeated keys; unit field declarations have
their own uniqueness and initialization checks.

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
