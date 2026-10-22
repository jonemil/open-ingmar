# Monkey C grammar

This grammar describes the accepted frontend profile. `?`, `*`, and `+` mean
optional, repeated, and one-or-more; quoted text denotes source tokens. Named
semantic domains below restrict productions: parsing a path does not authorize
arbitrary native calls or types. Examples in the semantic pages illustrate these
productions rather than defining additional syntax. `? description ?` marks an
explicit lexical character class. Whitespace between grammar symbols is
concatenation, not a required source space; the lexer separates source tokens.

Read from the entry rule through these sections:

- [Declarations](#declarations): `source-input` → files → declarations/functions.
- [Statements](#statements): function `block` → individual statement forms.
- [Expressions](#expressions-and-precedence): statement operands → precedence
  levels → calls, indexing, construction, and literals.
- [Types](#type-syntax): parameter/result/field annotations and expression targets.
- [Lexical rules](#lexical-rules-and-literals): paths, identifiers, and literals.

Every unquoted rule name has a production below. A production such as
`native-base = path` defines its spelling; the accompanying domain restriction
specifies which resolved names it admits. Grammar acceptance alone does not
establish availability, assignability, or executable API support.

## Declarations

```text
source-input = source-file
source-file = native-declaration* declaration*
declaration = function | module | class | typedef
typedef = "typedef" identifier "as" value-type ";"
module = "module" identifier "{" native-declaration* (function | typedef)+ "}"
class = "class" identifier ("extends" native-base)?
        "{" native-declaration* (field | function | typedef)* "}"
native-declaration = "using" native-module ("as" identifier)? ";"
                   | "import" native-module ";"
native-module = "Toybox" "." ("Application" | "System" | "WatchUi"
                             | "Graphics" | "Lang")
function = "function" identifier "(" parameter-list? ")"
           ("as" result-type)? block
parameter-list = parameter ("," parameter)*
parameter = identifier ("as" value-type)?
field = "var" identifier "=" inferred-field-default ";"
      | "var" identifier "as" field-type ("=" field-default)? ";"
inferred-field-default = number-literal | "-" number-literal | symbol-literal
                       | "(" inferred-field-default ")"
field-default = "null" | symbol-literal | "(" field-default ")"
```

`source-file` consumes the complete input. `function` is shared by global
functions, module functions, and class methods. Its `block` expands in
[Statements](#statements).
`native-base`, `field-type`, and `result-type` expand in [Type syntax](#type-syntax).

All source entry points use this file grammar. The fragment entry additionally
requires exactly one global function and no classes or module-owned functions;
this is an input-shape restriction, not a separate parser or language grammar.
Typedef declarations may accompany the fragment. Their targets use `value-type`
below and must resolve before body construction.
A compilation unit requires 1..=256 function/method bodies across its files.
Object-rooted classes may omit all methods; the frontend supplies their default
initializer when absent. Other bases require at least one method. Synthesized
initializers count toward the body limit. Native bases are resolved Object,
AppBase,
View, and BehaviorDelegate identities; omission selects Object. Field types and
permitted defaults are the [storage contracts](README.md#instance-fields).
`field-default` is further restricted by its type: Symbol permits a literal
Symbol or, when nullable, Null, also parenthesized. Other nullable
value contracts, including unions and container references, permit only literal
`null`, without parentheses. A union that canonicalizes to Symbol/nullable Symbol
uses the Symbol default rule; other unions containing Symbol still permit only
Null defaults. Non-null fields have no initializer here except Symbol. Inferred fields accept only the Number/Symbol forms
shown above.
Imports, owner collisions, module reopening, and file-local bindings follow
[declaration resolution](README.md#declarations-and-name-resolution). There are no optional/default
parameters, nested declarations, source superclass chains, or annotations.

## Statements

`function` contains a `block`; conditionals, loops, and switches contain statements.
Each leaf statement below names its expression or target rules.

```text
block = "{" statement* "}"
statement = local-statement | assignment-statement | update-statement
          | call-statement | return-statement | if-statement | while-statement
          | for-statement | do-statement | switch-statement | scoped-statement
          | break-statement | continue-statement
scoped-statement = block statement-terminator?
local-statement = local-clause statement-terminator
assignment-statement = assignment-clause statement-terminator
update-statement = update-clause statement-terminator
call-statement = call-expression statement-terminator
statement-terminator = ";"+
return-statement = "return" expression? ";"
if-statement = "if" "(" expression ")" block
               ("else" (block | if-statement))?
while-statement = "while" "(" expression ")" block
for-statement = "for" "(" for-initializer? ";" expression? ";"
                for-effects? ")" block
for-initializer = local-clause | for-effects
for-effects = loop-effect ("," loop-effect)*
loop-effect = assignment-clause | update-clause
local-clause = "var" local-declarator ("," local-declarator)*
local-declarator = identifier ("=" expression)?
assignment-clause = assign-target ("=" | compound-operator) expression
update-clause = update-target ("++" | "--") | ("++" | "--") update-target
do-statement = "do" block "while" "(" expression ")" statement-terminator
switch-statement = "switch" "(" expression ")" "{" case-group+ "}"
case-group = case-label statement*
case-label = "case" expression ":" | "case" "instanceof" class-target ":"
           | "default" ":"
break-statement = "break" ";"
continue-statement = "continue" ";"
assign-target = identifier | "self" "." identifier | primary "[" expression "]"
              | primary "." identifier
              | "(" assign-target ")"
update-target = assign-target
compound-operator = "+=" | "-=" | "*=" | "/=" | "%="
                  | "&=" | "|=" | "^=" | "<<=" | ">>="
```

Semantic checking restricts each target/operator pair; this syntax does not
permit every assignment form for every type. Locals have no explicit source
annotation; reads require definite assignment. Update operators are statements
only. Calls
include module/self/receiver calls, intrinsic System.println, and supported
native base initialization. Conditional and loop bodies require braces; an
`else` may instead introduce another `if-statement`. Standalone blocks are
statements and introduce local scopes. Arbitrary expression statements,
unbraced bodies and exception statements are unsupported. A switch has at least
one label and at most one `default`, at any position. Repeated value labels are
permitted. Label boundaries end the preceding group's statement sequence; an
explicit block within a group remains one scoped statement. Extra
semicolons are terminators only where `statement-terminator` occurs; there is no
standalone empty statement. Return/break/continue take exactly one semicolon,
and `for` header separators remain single semicolons. An optional terminator
after a standalone scoped block does not permit semicolons after control-flow
body blocks such as an `if` arm or `while` body.
A `for` initializer permits one declaration list sharing a `var`, or an
assignment/update list; its update permits only assignment/update lists.
Standalone calls and trailing commas in either list are rejected. Calls inside
initializer or assignment expressions remain supported. Present
loop conditions must be Boolean. [Statement
contracts](README.md#statements-and-control-flow) define scope, reachability,
loop targets, and evaluation order.

## Expressions and precedence

Statements reach this section through `expression`, `assign-target`, or
`call-expression`. `expression` descends from weakest precedence to `primary`.
`primary` then expands atoms and postfixes; grouping and arguments recurse into
`expression`. Types used by tests/casts expand in [Type syntax](#type-syntax).

```text
expression = logical-or ("?" expression ":" expression)?
logical-or = logical-and (("||" | "or") logical-and)*
logical-and = comparison (("&&" | "and") comparison)*
comparison = arithmetic (("==" | "!=" | "<" | "<=" | ">" | ">=") arithmetic
                       | "instanceof" class-target | "has" arithmetic)?
arithmetic = product (("+" | "-" | "|" | "^") product)*
product = cast (("*" | "/" | "%" | "&" | "<<" | ">>") cast)*
cast = unary ("as" cast-target)*
unary = "!" unary | "~" unary | ("+" | "-") primary | primary
primary = atom postfix*
postfix = member-call | member-access | index-access
member-call = "." identifier "(" argument-list? ")"
member-access = "." identifier
index-access = "[" expression "]"
atom = literal | identifier | "self" | self-field
     | named-call | native-constant
     | "(" expression ")" | array-literal | dictionary-literal | construction
self-field = "self" "." identifier
named-call = call-name "(" argument-list? ")"
call-name = path | "self" "." identifier | root-call-name
root-call-name = "$" "." identifier ("." identifier)?
call-expression = named-call | primary member-call | "(" call-expression ")"
native-constant = path
array-literal = "[" (argument-list ","?)? "]" "b"?
dictionary-literal = "{" (expression "=>" expression
                           ("," expression "=>" expression)* ","?)? "}"
construction = "new" path "(" argument-list? ")"
             | "new" "[" expression "]" "b"?
argument-list = expression ("," expression)*
literal = number-literal | long-literal | float-literal | double-literal
        | string-literal | char-literal | symbol-literal
        | "true" | "false" | "null"
```

`call-expression` selects a call as the final expression, including parenthesized
calls. An indexed result alone is not a call statement. `native-constant` is a
qualified path resolving to a supported API constant, not arbitrary member
access. An unqualified `call-name` resolves through
[owner lookup](README.md#user-calls-and-owner-lookup); qualified calls and native
base initializers follow their respective call contracts.

The levels above bind from weakest to strongest. Binary groups associate left;
conditional arms associate right. Comparison/test operators do not chain. `1 + 2
<< 2` means `1 + (2 << 2)`, and `2 << 1 * 3` means `(2 << 1) * 3`. Casts bind
after postfix/unary and before arithmetic; parenthesize a cast before postfix
access. A cast target may consume `or Null`; other `or` uses logical precedence.
[Operator semantics](README.md#expressions-and-operators) own accepted domains and target restrictions;
paths and arbitrary method names alone do not prove support.

Unary plus/minus accept one primary operand; repeated or mixed prefixes after
these require grouping: `+(-n)` and `-(-n)`, rather than `+-n` or `--n`.
Negation can occur within a `!`/`~` prefix chain. Contiguous `++`/`--` are
update tokens. A negative numeric literal needs parentheses before a member
call: `(-1).abs()` is accepted; `-1.abs()` is rejected due to the observed SDK
literal/postfix discrepancy. See [Number literal evidence][numbers].

Class-test targets are the supported runtime classes in Lang (Number, Long,
Float, Double, Boolean, Char, String, Object, Dictionary, Array, Symbol, Method,
WeakReference, ByteArray), or an exact constructible source class. Interface
names, enum aliases, Null, and dynamic class values are not runtime class tests.
Cast targets accept Number, Long, Float, Double, Boolean, Char, String, Object,
Symbol, Method, WeakReference, ByteArray, Numeric, Integer, Decimal,
NumberFormat, Endian, opaque Dictionary, or exact constructible source classes.
Nullable casts accept the scalar/core-reference, numeric-alias, and exact source
forms specified by [cast semantics](README.md#lexical-casts). Parameterized
containers, general unions, and interfaces remain unsupported. Native spellings
resolve through the import environment.

## Type syntax

Referenced by declaration `parameter`, `function`, and `field`, and by
expression `construction`, `comparison`, and `cast`. The `*-type` and target
rules below define paths; the domains following them restrict name resolution.

```text
value-type = value-atom ("or" value-atom)*
value-atom = named-value-type | "Null" | structural-interface
           | array-type "<" array-element ">"
           | dictionary-type "<" dictionary-key "," dictionary-value ">"
result-type = value-type | "Void"
structural-interface = "interface" "{" interface-member* "}"
interface-member = "var" identifier ("as" value-type)? ";"
                 | "function" identifier "(" interface-parameters? ")"
                   ("as" result-type)? ";"
interface-parameters = interface-parameter ("," interface-parameter)*
interface-parameter = identifier ("as" value-type)?
array-element = value-type
dictionary-key = value-type
dictionary-value = value-type
named-value-type = scalar-or-alias | core-reference | interface-type | enum-type
                 | dictionary-type | view-type | delegate-type | source-class-type
                 | type-alias
type-alias = path
scalar-or-alias = scalar-type | numeric-alias-type
scalar-type = path
numeric-alias-type = path
core-reference = object-type | symbol-type | method-type | weakref-type
               | bytearray-type
interface-type = path
enum-type = path
object-type = path
view-type = path
delegate-type = path
appbase-type = path
source-class-type = identifier
symbol-type = path
method-type = path
weakref-type = path
bytearray-type = path
array-type = path
dictionary-type = path
native-base = object-type | appbase-type | view-type | delegate-type
field-type = value-type
class-target = runtime-class-type | source-class-type
runtime-class-type = path
cast-target = cast-named-type | nullable-cast-type "or" "Null"
cast-named-type = nullable-cast-type | enum-type | dictionary-type | type-alias
nullable-cast-type = scalar-or-alias | core-reference | source-class-type
```

Paths resolve through the native-name environment; canonical `Toybox` paths need
no imports. These semantic type domains mean:

`structural-interface` is a finite lexical member contract, optionally named by
a typedef. Members have no bodies/defaults; omitted annotations mean Any.
Void is permitted only as a method result. Member names are unique across
fields and methods, with at most 256 members and 254 parameters per method.
The [source interface profile](README.md#source-interface-types) restricts member
contracts, proof obligations, and field accesses.

- Scalar: Number, Long, Float, Double, String, Boolean, or Char in Lang.
  Numeric alias: Lang.Numeric, Lang.Integer, or Lang.Decimal.
- Core reference: Lang.Object, Symbol, Method, WeakReference, ByteArray.
- Interface: Lang.Comparable or Lang.Comparator.
- Enum: Lang.NumberFormat or Lang.Endian.
- Object, View, delegate, and AppBase type leaves resolve to Lang.Object,
  WatchUi.View, WatchUi.BehaviorDelegate, and Application.AppBase respectively.
- Source class: an exact constructible source-class name from this unit.
- Type alias: a scoped source typedef. Expansion must fit the destination's
  existing value, element, key, field, or cast contract. Alias names are not
  runtime class targets. Declaration scope and expansion bounds follow
  [type aliases](README.md#type-aliases).
- Dictionary type by itself denotes an opaque Lang.Dictionary; its parameterized
  form specifies the bounded key/value contracts.
- Array element and Dictionary key/value expressions use the shared value-type
  parser. After alias expansion and normalization, they must match the existing
  supported element/key/value contracts; this does not add nested containers,
  exact source-class contents, or arbitrary heterogeneous unions.
  Finite structural interfaces are permitted as elements/keys/values; only
  elements and values may add Null. Member signatures retain their separate
  source-class contract domain.
- Signature unions: up to 254 value atoms, including Null and supported native
  container references and exact source-class atoms. Class-containing unions
  preserve exact identities separately from native alternatives. The normalized
  set is also limited to 254 alternatives, including expanded aliases and Null.
- Fields share signature value atoms and union normalization, including nullable
  container references, native references/enums, exact source sets, and mixed sets.
  Callback-only Graphics.Dc is excluded even inside a union.
- View, Array, Dictionary, Symbol, Method, and ByteArray productions use their
  resolved native identities.
- Native base: Object, AppBase, View, or BehaviorDelegate, as above.
- `field-default` restrictions are specified under Declarations; nullable
  references describe the field itself independently of container contents.
- Class target and cast target: the domains immediately below the expression
  grammar; a parsed path does not admit arbitrary type tests/assertions.

Graphics.Dc is accepted as the native View.onUpdate parameter, not as a general
source value/result contract. Bare Null and either order of native union atoms
are accepted in signatures and fields. Lexical casts retain their separately
checked domains; general union signature/field syntax does not extend casts.
Nested type parameters and parenthesized type unions remain unsupported. Generic containers are invariant; see
[type rules](README.md#types-and-nullability) and the container semantic pages.

## Lexical rules and literals

These are the leaves used by declarations, types, and expressions. Numeric
suffixes, decimal points, exponent signs, and escape sequences are contiguous
within one token. The grammar's character classes exclude the delimiters listed
in their descriptions.

```text
path = identifier ("." identifier)*
identifier = (ascii-letter | "_") (ascii-letter | digit | "_")*
ascii-letter = ? ASCII A-Z or a-z ?
digit = ? ASCII 0-9 ?
digits = digit+
number-literal = integer-magnitude
long-literal = integer-magnitude ("l" | "L")
integer-magnitude = decimal-magnitude | hex-magnitude | octal-magnitude
decimal-magnitude = "0" | nonzero-digit digit*
nonzero-digit = "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9"
hex-magnitude = "0x" hex-digit+
hex-digit = digit | "a" | "b" | "c" | "d" | "e" | "f"
          | "A" | "B" | "C" | "D" | "E" | "F"
octal-magnitude = "0" octal-digit+
octal-digit = "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7"
float-literal = real-magnitude | (decimal-magnitude | real-magnitude) "f" | "NaN"
double-literal = (decimal-magnitude | real-magnitude) "d"
real-magnitude = decimal-magnitude "." digits exponent?
               | "." digits exponent? | decimal-magnitude exponent
exponent = ("e" | "E") ("+" | "-")? digits
symbol-literal = ":" identifier
string-literal = '"' (string-character | literal-escape)* '"'
string-character = ? Unicode scalar except double quote, backslash, CR, LF ?
backslash = ? U+005C ?
literal-escape = backslash ("b" | "f" | "n" | "r" | "t" | "'" | '"'
                         | backslash | "u" hex-digit hex-digit hex-digit hex-digit)
char-literal = "'" (char-character | literal-escape) "'"
char-character = ? BMP Unicode scalar except single quote, backslash, CR, LF ?
```

Minus is a unary source token, not part of the unsigned `number-literal`,
`long-literal`, `float-literal`, or `double-literal` token. The unary rules and
numeric range checks also admit the minimum signed Number/Long values.
Decimal integers require signed range; hexadecimal/octal integers describe
32-bit Number or 64-bit Long patterns. Their literal negation wraps at that
width. Leading-zero integer magnitudes select octal, not decimal; `08`, `01f`,
and `01.5` are rejected. Prefix `0X`, binary prefixes, separators, uppercase
`F`/`D`, and a decimal point without following digits are unsupported.
`NaN` denotes the canonical Float NaN; it has no Double suffix form.

Identifiers use ASCII letters/underscore followed by ASCII letters, digits, or
underscore. The following words cannot be ordinary identifiers, including words
whose associated constructs are not implemented:

```text
alias and as break case catch class const continue default do else enum extends
false finally for function has hidden if import instanceof me module NaN native
new null or private protected public return self static switch throw true try
typedef using var while
```

`self` is accepted as a receiver expression. Source keyword reservation does not
restrict saved IR or assembly names. Additional reserved owner/native names
follow [name
resolution](README.md#declarations-and-name-resolution) and
[class rules](README.md#source-classes). Whitespace, `//` comments, and non-nested
`/* ... */` comments are accepted. Nonempty Array, ByteArray, and Dictionary
literals accept one trailing comma, including comments before the closing
delimiter. It adds no element or entry. Empty literals cannot contain a comma;
missing elements/values and repeated commas are rejected. Other lists, including
parameters, calls, constructors, local declarations, and `for` clauses, reject
trailing commas. See [separator evidence](../../investigations/compiler-array-literals.md#container-separators).

Decimal integer magnitudes fit signed i32/i64, including the minimum values
under unary minus; other radices follow the bit-pattern rules above. Negative
literal forms permit whitespace/comments between minus and magnitude. Floating
parsing rounds to IEEE 754, rejects infinity from overflow, and allows underflow
to zero. Negative floating literals preserve signed zero. Overflow diagnostics
point to the literal start; [numeric semantics](README.md#numbers-and-numeric-aliases)
distinguish literal rounding from arithmetic and conversion behavior.

String and Char literals share escapes for backspace (`\b`), form feed (`\f`),
newline, carriage return, tab, both quotes, backslash, and one UTF-16 code unit
(`\uNNNN`, exactly four hexadecimal digits). Decoding occurs once: escaped
quotes/backslashes do not delimit literals or introduce further escapes.
Neither form accepts `\0`, octal escapes, `\x`, `\U`, braced Unicode escapes,
or backslash-newline continuation. Raw CR/LF and unfinished/unknown escapes
are rejected.

Strings decode to valid UTF-8. A high-surrogate escape must immediately precede
a low-surrogate escape; the pair produces one supplementary scalar. Unpaired
surrogates are rejected rather than replaced. Empty strings and embedded NUL
are allowed. The 65535-byte limit applies to decoded UTF-8, not source spelling
or UTF-16 code-unit count.

Chars contain exactly one raw BMP scalar or one escape. A Unicode escape
preserves its 16-bit value, including NUL and individual surrogates. Raw
supplementary characters and multiple code units are rejected; a surrogate pair
is therefore a String form only. Double quotes may be unescaped in Chars, and
single quotes in Strings. Char values require API 1.3.0 with explicit API
context. IR and assembly retain their separate quoting rules. See
[literal evidence][chars] and [value contracts](README.md#core-lang-values-and-methods).

Expression/control-flow nesting is bounded at 64 and a semantic function at 1024
operations including return. Parameters are limited to 254, parameters plus
locals to 255. These are implementation/encoding limits, not validated device
memory budgets. [API context](../sdk/README.md#api-context-inputs) applies
availability checks separately from grammar acceptance.

[numbers]: ../../investigations/compiler-number-literals.md
[chars]: ../../investigations/compiler-char-literals.md
