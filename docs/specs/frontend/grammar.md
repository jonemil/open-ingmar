# Monkey C grammar

This grammar describes the accepted frontend profile. `?`, `*`, and `+` mean
optional, repeated, and one-or-more; quoted text denotes source tokens. Named
semantic domains below restrict productions: parsing a path does not authorize
arbitrary native calls or types. Examples in the semantic pages illustrate these
productions rather than defining additional syntax.

## Declarations

```text
single-file = native-declaration* function
unit-file = native-declaration* declaration*
declaration = function | module | class
module = "module" identifier "{" native-declaration* function+ "}"
class = "class" identifier ("extends" native-base)?
        "{" native-declaration* (field | function)+ "}"
native-declaration = "using" native-module ("as" identifier)? ";"
                   | "import" native-module ";"
native-module = "Toybox.Application" | "Toybox.System" | "Toybox.WatchUi"
              | "Toybox.Graphics" | "Toybox.Lang"
function = "function" identifier "(" parameter-list? ")"
           ("as" result-type)? block
parameter-list = parameter ("," parameter)*
parameter = identifier ("as" value-type)?
field = "var" identifier "=" inferred-field-default ";"
      | "var" identifier "as" field-type ("=" field-default)? ";"
inferred-field-default = signed-number-literal | ":" identifier
                       | "(" inferred-field-default ")"
```

A compilation unit requires 1..=256 function/method bodies across its files.
Classes require at least one method. Native bases are resolved Object, AppBase,
View, and BehaviorDelegate identities; omission selects Object. Field types and
permitted defaults are the [storage contracts](fields.md). Imports, owner
collisions, module reopening, and file-local bindings follow
[declaration resolution](declarations.md). There are no optional/default
parameters, nested declarations, source superclass chains, or annotations.

## Type syntax

```text
value-type = named-value-type | nullable-type "or" "Null"
           | array-type "<" array-element ">"
           | dictionary-type "<" dictionary-key "," dictionary-value ">"
result-type = value-type | "Void"
array-element = scalar-or-alias | core-reference | interface-type
              | (scalar-or-alias | core-reference | interface-type) "or" "Null"
              | view-type
dictionary-key = scalar-or-alias | symbol-type | method-type | bytearray-type
               | interface-type
dictionary-value = scalar-or-alias | core-reference | interface-type
                 | (scalar-or-alias | core-reference | interface-type) "or" "Null"
path = identifier ("." identifier)*
```

Paths resolve through the native-name environment; canonical `Toybox` paths need
no imports. These semantic type domains mean:

- Scalar/alias: Number, Long, Float, Double, String, Boolean, Char, Numeric,
  Integer, Decimal in Lang.
- Core reference: Lang.Object, Symbol, Method, WeakReference, ByteArray.
- Interface: Lang.Comparable or Lang.Comparator.
- Named value: the scalar/alias, core reference, and interface domains, plus
  Lang.NumberFormat, Lang.Endian, opaque Lang.Dictionary, WatchUi.View,
  WatchUi.BehaviorDelegate, or an exact constructible source-class name.
- Nullable: scalar/alias, core reference, and interface domains. Nullable exact
  source-class, Array, and typed Dictionary references are unsupported.
- View, Array, Dictionary, Symbol, Method, and ByteArray productions use their
  resolved native identities.

Graphics.Dc is accepted as the native View.onUpdate parameter, not as a general
source value/result contract. Nullable opaque Dictionary is a lifecycle IR
contract; it is not an ordinary source annotation. Fields have narrower,
independently checked [type/default domains](fields.md). Nested type parameters,
arbitrary unions, reversed `Null or T`, bare Null, and parenthesized type unions
are unsupported. Generic containers are invariant; see [type rules](types.md)
and the container semantic pages.

## Statements

```text
block = "{" statement* "}"
statement = "var" identifier "=" expression ";"
          | assign-target "=" expression ";"
          | assign-target compound-operator expression ";"
          | update-target ("++" | "--") ";"
          | ("++" | "--") update-target ";"
          | call-expression ";"
          | "return" expression? ";"
          | "if" "(" expression ")" block ("else" block)?
          | "while" "(" expression ")" block
          | "break" ";" | "continue" ";"
assign-target = identifier | "self" "." identifier | primary "[" expression "]"
update-target = assign-target
compound-operator = "+=" | "-=" | "*=" | "/=" | "%="
                  | "&=" | "|=" | "^=" | "<<=" | ">>="
```

Semantic checking restricts each target/operator pair; this syntax does not
permit every assignment form for every type. Locals require initializers and
have no explicit source annotation. Update operators are statements only. Calls
include module/self/receiver calls, intrinsic System.println, and supported
native base initialization. Arbitrary expression statements, standalone nested
blocks, unbraced arms, `else if`, `for`, `do`, `switch`, and exception
statements are unsupported. [Statement contracts](statements.md) define scope,
reachability, loop targets, and evaluation order.

## Expressions and precedence

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
primary = atom ("." identifier "(" argument-list? ")"
               | "[" expression "]")*
atom = literal | identifier | "self" | "self" "." identifier
     | path "(" argument-list? ")" | native-constant
     | "(" expression ")" | array-literal | dictionary-literal | construction
array-literal = "[" argument-list? "]" "b"?
dictionary-literal = "{" (expression "=>" expression
                           ("," expression "=>" expression)*)? "}"
construction = "new" path "(" argument-list? ")"
             | "new" "[" expression "]" "b"?
argument-list = expression ("," expression)*
literal = signed-number-literal | long-literal | float-literal | double-literal
        | string-literal | char-literal | ":" identifier
        | "true" | "false" | "null"
```

The levels above bind from weakest to strongest. Binary groups associate left;
conditional arms associate right. Comparison/test operators do not chain. `1 + 2
<< 2` means `1 + (2 << 2)`, and `2 << 1 * 3` means `(2 << 1) * 3`. Casts bind
after postfix/unary and before arithmetic; parenthesize a cast before postfix
access. A cast target may consume `or Null`; other `or` uses logical precedence.
[Operator semantics](operators.md) own accepted domains and target restrictions;
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
Nullable casts are limited to the scalar/core-reference and numeric-alias forms
specified by [cast semantics](operators.md#lexical-casts). Parameterized
containers, general unions, interfaces, and nullable source-class casts are
unsupported. Native spellings resolve through the import environment.

## Lexical rules and literals

Identifiers use ASCII letters/underscore followed by ASCII letters, digits, or
underscore. Declaration keywords, `self`, literal keywords, `new`, `as`,
`instanceof`, `has`, `and`, and `or` cannot be ordinary identifiers. Additional
reserved owner/native names follow [name resolution](declarations.md) and
[class rules](classes.md). Whitespace, `//` comments, and non-nested `/* ... */`
comments are accepted. Commas separate lists; trailing commas are unsupported.

Number literals are decimal signed i32, including -2147483648. Long literals end
in `l`/`L` and fit signed i64. Fractional/exponent literals default to Float;
`d` selects Double, including `1d`. A fraction needs digits on both sides of the
point; `e`/`E` exponents allow an optional sign and require digits. Negative
literal tokens permit whitespace/comments between minus and digits. Floating
parsing rounds to IEEE 754, rejects infinity from overflow, and allows underflow
to zero. Negative literals preserve signed zero. Hexadecimal/underscore numeric
forms and other suffixes are unsupported. Overflow diagnostics point to the
literal start; [numeric semantics](numeric-values.md) distinguish literal
rounding from arithmetic and conversion behavior.

String literals use double quotes, UTF-8, and `\n`, `\r`, `\t`, `\"`, `\\`. Raw
newlines, unknown/unfinished escapes, and lengths above 65535 UTF-8 bytes are
rejected; empty strings are allowed. Char literals use single quotes around
exactly one BMP Unicode scalar, with escapes `\n`, `\r`, `\t`, `\'`, `\\`.
Double quotes may be unescaped inside Char literals. Empty/multiple-character
literals, raw single quotes/backslashes/newlines, and supplementary characters
are rejected. Neither form accepts `\0`, `\u`, or `\x`. IR and assembly have
separate quoting rules. Char values require API 1.3.0 with explicit API context.
See [Char evidence][chars] and [string
contracts](lang-values.md#core-lang-values-and-methods).

Expression/control-flow nesting is bounded at 64 and a semantic function at 1024
operations including return. Parameters are limited to 254, parameters plus
locals to 255. These are implementation/encoding limits, not validated device
memory budgets. [API context](../sdk/context.md) applies availability checks
separately from grammar acceptance.

[numbers]: ../../investigations/compiler-number-literals.md
[chars]: ../../investigations/compiler-char-literals.md
