# Supported Monkey C compiler subset

Restricted frontend. Semantic model:
[small semantic IR](compiler-ir-text.md#semantic-model). Runtime evidence:
[constant returns](../investigations/first-compiler.md),
[arithmetic](../investigations/compiler-arithmetic.md),
[locals](../investigations/compiler-locals.md),
[module calls](../investigations/compiler-calls.md),
[parameters/arguments](../investigations/compiler-arguments.md), and
[strings/printing](../investigations/compiler-strings.md).

The scalar, container, and null restrictions describe the current implementation.
They do not define separate primitive types for the full Monkey C language.
The [core type/API model](api-declarations.md) connects values to
`Lang` identities for operator validation and the bounded native calls below.
General member lookup remains unsupported. Explicit API-level checks cover the
known native references; see [API context inputs][api-context].
No API declaration-file syntax is specified here.

[api-context]: compiler-ir-text.md#api-context-inputs

The default mode accepts exactly one top-level function.
[Compilation-unit mode](compiler-unit.md) adds global/module declarations and
cross-function validation across files. The per-function grammar is:

```text
function = "function" identifier "(" parameter-list? ")" ("as" result-type)? block
block = "{" statement* "}"
parameter-list = parameter ("," parameter)*
parameter = identifier ("as" value-type)?
value-type = "Toybox.Lang.Number" | "Toybox.Lang.Long" | "Toybox.Lang.Float"
           | "Toybox.Lang.Double" | "Toybox.Lang.String"
           | "Toybox.Lang.Boolean" | "Toybox.Lang.Char" | "Toybox.Lang.Dictionary"
           | "Toybox.WatchUi.View" | "Toybox.WatchUi.BehaviorDelegate"
           | source-view-class-name
           | nullable-scalar "or" "Null"
           | "Toybox.Lang.Array" "<" array-element ">"
           | "Toybox.Lang.Dictionary" "<" dictionary-key "," dictionary-value ">"
nullable-scalar = "Toybox.Lang.Number" | "Toybox.Lang.Long" | "Toybox.Lang.Float"
                | "Toybox.Lang.Double" | "Toybox.Lang.String"
                | "Toybox.Lang.Boolean" | "Toybox.Lang.Char"
array-element = nullable-scalar ("or" "Null")? | "Toybox.WatchUi.View"
dictionary-key = nullable-scalar
dictionary-value = nullable-scalar ("or" "Null")?
result-type = value-type | "Void"
argument-list = expression ("," expression)*
statement = "var" identifier "=" expression ";"
          | identifier "=" expression ";"
          | identifier compound-operator expression ";"
          | update-target ("++" | "--") ";"
          | ("++" | "--") update-target ";"
          | primary "[" expression "]" "=" expression ";"
          | primary "[" expression "]" compound-operator expression ";"
          | "System" "." "println" "(" expression ")" ";"
          | module "." member "(" argument-list? ")" ";"
          | "return" expression? ";"
          | "if" "(" expression ")" block ("else" block)?
          | "while" "(" expression ")" block
          | "break" ";" | "continue" ";"
update-target = identifier | "self" "." identifier | primary "[" expression "]"
compound-operator = "+=" | "-=" | "*=" | "/=" | "%=" | "&=" | "|=" | "^=" | "<<=" | ">>="
expression = logical-or ("?" expression ":" expression)?
logical-or = logical-and (("||" | "or") logical-and)*
logical-and = comparison (("&&" | "and") comparison)*
comparison = arithmetic (("==" | "!=" | "<" | "<=" | ">" | ">=") arithmetic)?
arithmetic = product (("+" | "-" | "|" | "^") product)*
product = unary (("*" | "/" | "%" | "&" | "<<" | ">>") unary)*
unary = primary | ("-" | "+") primary | ("!" | "~") unary
primary = atom ("." member "(" argument-list? ")" | "[" expression "]")*
atom = "{" "}" | signed-decimal-integer | string-literal | char-literal | "true" | "false" | "null"
     | parameter-or-local-identifier
     | module "." member "(" argument-list? ")" | "(" expression ")"
     | "[" argument-list? "]"
     | "new" native-class-path "(" ")"
```

Identifiers use ASCII letters/underscore followed by ASCII
letters/digits/underscore. `function`, `module`, `return`, `var`, `if`, `else`,
`true`, `while`, `break`, `continue`, `class`, `extends`, `self`, `false`,
`using`, `import`, `as`, `null`, `new`, `and`, and `or`
are reserved. `true` and `false` produce Boolean literals. Whitespace and
`//` or non-nested `/* ... */` comments are supported. The integer literal domain
is the signed 32-bit range `-2147483648..=2147483647`. Decimal negative literals
are folded into that domain, with ordinary token whitespace/comments permitted.
Unary Number negation also accepts parameters, locals, calls, and grouped
expressions. Unary plus accepts non-null Number expressions as specified below.
Hex/underscore/suffix forms, other numeric types, unsupported
statements/additional functions in default mode, and other operators are
rejected. A negative literal must be parenthesized before a member call:
`(-1).abs()` is accepted, while `-1.abs()` remains rejected because of an SDK
literal-postfix discrepancy. Repeated negation requires parentheses, as in
`-(-value)`; adjacent `--` denotes a statement update and is not unary negation.
Literal overflow is diagnosed at the literal's source start. See the
[Number literal evidence](../investigations/compiler-number-literals.md).

Diagnostics identify the input byte offset and one-based line/column (columns
count Unicode scalar values). The caller supplies the filename. Unsupported
input never silently generates a partial function. Name resolution currently
covers parameters and earlier initialized locals in the current arm and its
enclosing scopes. There is no general type inference, global/member resolver,
whole-program analysis, or optimizer yet.

Char literals use single quotes around exactly one BMP Unicode scalar or one
of the escapes `\n`, `\r`, `\t`, `\'`, and `\\`. A double quote may appear
unescaped inside single quotes. Raw single quotes, backslashes, and newlines are invalid;
empty/multiple-character literals and supplementary characters are rejected.
`\0`, `\u`, and `\x` escapes are outside this subset. Literals produce Char,
retain source byte spans, and require API 1.3.0 with explicit context. See the
[literal evidence](../investigations/compiler-char-literals.md).

The IR contains integer/string/Boolean/Char constants, binary arithmetic
definitions,
parameter reads, local initialization/read, module calls, print statements, and
comparisons, branches, and typed value returns using value IDs, with optional
source spans. Source
compilation supplies them; IR text may omit them. Without locals, lowering emits
a label from the function name, `argc <parameter-count + 1>`, `ipushz` for zero
or the smallest signed `ipush1`/`ipush2`/`ipush3`/`ipush4` for another Number,
left/right operands followed by
`addv`, `subv`, `mulv`, `divv`, `modv`, `andv`, `orv`, `xorv`, `shlv`, or
`shrv` for binary definitions, then `return`.
The frame count follows
the observed hidden context slot; omitting SDK global-resolution instructions is
limited to this context-independent body. It does not establish the frame layout
or valid setup for arbitrary functions.

```sh
cargo run -p ciq-compiler -- examples/compiler-probe/probe.mc scratch/probe.mcasm
```

CLI output must be a new file; existing output is never overwritten. Output is
ordinary assembly, not a complete PRG. Library `compile(source)` returns an
inspectable immutable `Function` IR; `Function::assembly()` lowers/prints it.
Public IR types are preliminary. `Function::new` validates explicitly
constructed operations, `parse_ir` resumes [IR text](compiler-ir-text.md), and
`Function::lower()` constructs the
[shared assembly model](assembly.md) directly. Existing
assembler/signing commands consume the output after explicitly integrating it
into an application section project.

The compiler CLI supports `--from source|ir|assembly` and `--emit
ir|assembly|code`. The default two-path invocation still emits assembly from
source. CODE output is a raw payload, not a PRG. See
[pipeline commands and limits](compiler-ir-text.md#command-line-stages).

## Arithmetic and lowering contract

The higher binary group is `*`, `/`, `%`, `&`, `<<`, and `>>`. The lower
group is `+`, `-`, `|`, and `^`. Each group associates left to right;
parentheses override grouping. Both groups bind before comparisons. This is
Monkey C precedence: `1 + 2 << 2` means `1 + (2 << 2)`, while `2 << 1 * 3`
means `(2 << 1) * 3`. Decimal literals cover the signed 32-bit range. Unary minus binds more tightly than binary arithmetic and after
postfix calls: `-value.abs()` means `-(value.abs())`; `(-value).abs()` calls
`abs` on the negated value. Unary `!` uses the same precedence, supports
repeated prefixes (`!!value`), and binds before comparisons:
`!a == b` means `(!a) == b`. Integer complement `~` shares that unary
precedence and allows repeated prefixes. Unary plus binds after postfix calls
and before binary operators; it returns a non-null numeric value unchanged
after evaluating it once. It accepts literals, parameters, locals, calls, and
grouped expressions. Repeated or mixed prefixes require grouping, as in
`+(+n)`, `+(-n)`, and `+(~n)`; `+ +n`, `+-n`, and `+~n` are rejected.
Contiguous `++`/`--` denote statement updates, described below. Compound assignments
support bare locals as specified below. Unsigned `>>>` and general object
calls remain unsupported; the bounded native calls below are separate. Parse
nesting is bounded at 64; the semantic function is limited to 1024 operations
including return. These are implementation limits.

Number division truncates toward zero; remainder has the dividend's sign.
Multiplication uses the Number VM instruction; representative overflow probes
wrap to signed 32-bit values. Zero divisors are runtime failures, including
literal zero; the compiler does not fold arithmetic or insert exception guards.
The SDK 9.2.0 simulator exits when the minimum Number is divided by, or has
remainder taken against, -1. This boundary does not establish a portable result
or a catchable Monkey C exception. See [product evidence][products].

Number-only bitwise operators use non-null signed 32-bit bit patterns.
Mixed Long operations follow the [Long contracts](#long-values). Floating,
Char, Boolean, String, and nullable operands without a proof are rejected by
bitwise operations. `>>` is an arithmetic right shift in the tested VM. Complement retains
a unary IR operation and lowers to `invv`, evaluating its operand once.

For Number-only shifts, counts are non-null Numbers. Mixed Long shifts follow
the [Long contracts](#long-values). The compiler passes counts unchanged,
without range checks or masking. SDK/generated probes for -1, 32, and 33 agree
with selecting the low five count bits; this is evidence for the tested VM,
not a portable guarantee for other runtimes. See [bitwise evidence][bits].

[bits]: ../investigations/compiler-number-bits.md

Unary plus is retained as a checked identity operation in IR and emits no VM
instruction. It does not parse text, convert types, or relax null proofs. The
SDK probe passes other value types through too; non-numeric types remain outside
our current profile. See [unary-plus evidence][positive].

[positive]: ../investigations/compiler-number-plus.md

### Operator contracts

Validation resolves operand API types to these result contracts:

| Operands | Operators | Result |
|---|---|---|
| Number / Number | `+`, `-`, `*`, `/`, `%`, `&`, `|`, `^`, `<<`, `>>` | Number |
| Number | unary `+`, `-`, `~`, `!` | Number |
| Boolean | unary `!` | Boolean |
| Char / Number or Long, either order | `+` | Char |
| Char / Char | `+` | String |
| Exact String / supported scalar or null, either order | `+` | String |
| Number / Number, Char / Char, Number / Char, Char / Number | `==`, `!=`, `<`, `<=`, `>`, `>=` | Boolean |
| Boolean / Boolean | `==`, `!=` | Boolean |
| Char / Long, Float, or Double, either order | `==`, `!=` | Boolean |
| Exact String / String, numeric, Char, or Null, either order | `==`, `!=` | Boolean |

Mixed Char/integer addition changes the numeric character value, including
negative Number/Long offsets. Char + Char concatenates the two characters.
Comparison uses numeric values: `'A' == 65` is true on the validated profile,
while `'A'.equals(65)` is false. Char ordering with Long/Float/Double is
unsupported, as is Char addition with Float/Double and all Char subtraction.
With one exact String operand, concatenation accepts numeric scalars, String,
Boolean, Char, Null, or their nullable scalar forms, returning String. The VM
supplies scalar
conversion and formats Null as `null`. Equality accepts the same profile except
Boolean (including Boolean-or-Null), using String identity: copied references
compare true, while separately created equal-content strings compare false. Use `.equals()` for content.
Nullable alternatives are accepted only with an exact String on the other side;
this does not establish a non-null proof for subsequent use. String ordering,
subtraction, and other object/container operands are unsupported. See the
[String operator evidence](../investigations/compiler-string-operators.md) and
[wider operand evidence](../investigations/compiler-wide-operators.md).
Unchecked nullable operands require a non-null proof before numeric use.
Result contracts remain exact in assignments, calls, and returns. Overflow and
invalid Unicode ranges are runtime responsibilities. See the
[operator evidence](../investigations/compiler-char-operators.md).

Supported null checks are specified below. These contracts are independent of
method lookup and VM instruction selection. API declarations do not broaden accepted
operator combinations or cause heap allocations to be emitted.

Unary negation requires a non-null Number; fresh reads under a validated direct
null guard are accepted. It evaluates the operand once and lowers to
`ipushz`, operand evaluation, then `subv`. Negating the i32 minimum retains
`-2147483648` on the validated profile. Other arithmetic overflow behavior is
not specified by that observation. Signed decimal literals remain constants;
general negation uses a distinct IR operation and counts toward the nesting
bound. See the [negation evidence](../investigations/compiler-number-negation.md).

Unary `!` inverts a Boolean or complements the 32/64 bits of Number/Long, preserving
its operand type and evaluating it once. Thus `!7` is Number `-8`, not Boolean;
`!!7` is Number `7`. Value-producing inversion lowers directly to `invv`.
Boolean condition negation reverses the true/false destinations. Other types
are rejected without truth-value conversion; numeric results cannot be used as
Boolean conditions. A nullable Number needs a non-null proof. Postfix calls bind
first: `!value.equals(1)` inverts the method result, while `(!value).equals(1)`
calls the method on the inverted value. See the
[Boolean evidence](../investigations/compiler-boolean-negation.md) and
[operator review](../investigations/compiler-operator-coverage.md).

Semantic IR contains typed arithmetic definitions with explicit
left/right value IDs. Each initializer/return consumes one expression tree:
definitions are unique, operands refer to earlier definitions, every defined
value has exactly one use, and initialization statements, branches, prints, or
returns consume expression results within the same block. General shared-value
graphs and unused definitions remain rejected until their scheduling policy is
implemented. For pure-only functions, definition order need not be VM stack
order: the backend traverses each initializer/return expression and emits left
operand, right operand, then
`addv`/`subv`/`mulv`/`divv`/`modv`, preserving operand order. Division and
remainder activate strict evaluation-order checks because they may fail.
No constant folding is performed.
VM selection lives in `ciq_compiler::backend::lower`; `Function::lower()`
delegates to it. Source parsing, IR validation/text I/O, and assembly encoding
share the existing stage boundaries.

## Local declarations and reads

`var <identifier> = <expression>;` initializes an integer or string local.
Expressions may read it repeatedly after initialization. Duplicate declarations
in one scope, shadowing an enclosing local or parameter, self/forward/unknown
reads and uninitialized declarations are rejected. An arm's
locals are visible within that arm and nested arms, but not in a sibling or
after the condition. Disjoint arms may reuse a name; each declaration receives
a distinct semantic local and VM slot. A later outer declaration may also reuse
a name that has gone out of scope. Standalone braced blocks are unsupported.
Reassignment follows the fixed-type rules below.

IR distinguishes semantic `LocalId` from `ValueId` and physical VM slots.
`LocalInitialize` consumes an expression value and declares a unique local;
`LocalRead` defines a fresh value from a previously initialized local. Every
expression value still has exactly one use; repeated source reads create
separate read definitions. Initializations execute in source/IR order, including
unused locals. The backend assigns local slots after parameters in
initialization order, preserving slot 0, uses `argcincsp <parameter-count + 1>
<local-count>`, and emits `lputv`/`lgetv`. Without locals it uses `argc
<parameter-count + 1>`. Parameters plus locals are limited to 255 and functions
to 1024 total operations. [Controlled SDK output and simulator
results][ref-controlled-sdk-output-and-simulator-resu] confirm two/three-local
bodies; the 255-local limit is checked structurally in tests, not established as
a device runtime limit.

IR text represents locals with `.local $<decimal-u32-id> %<value-id>` and
`.read %<result-id> $<local-id>`, each with an optional span. Semantic
IDs need not be contiguous; their numeric values never select physical slots.

## Runtime object predicates

`value instanceof Lang.Class` produces Boolean for supported concrete classes:
Object, Number, Long, Float, Double, Boolean, Char, String, Array, Dictionary,
Symbol, Method, and WeakReference. Canonical `Toybox.Lang.Class` and Lang module
aliases also work. The class must satisfy the selected API requirements.
[Imported type names](#native-name-declarations) and qualified aliases resolve to
the same targets. Interfaces, unions, and parameterized class operands remain
outside this profile. Null is not an instance of these
classes. This is a runtime identity/inheritance test, not a conversion or an
exact source-class assertion.

`receiver has selector` produces Boolean for member existence. The receiver
must be non-null; a preceding supported null guard can establish that fact.
The selector must be a non-null Symbol, including a parameter or computed result.
Its identity is shared with ordinary field/method binding. The receiver is
computed before the selector, once each. The tested member is not invoked.
Source fields/methods and inherited native methods can be tested.

Both predicates have the existing comparison precedence below arithmetic and
above logical AND/OR; chained comparisons are unsupported. Ordinary short-circuit
and conditional evaluation rules apply. `has` establishes no member-signature
or API-availability proof. Guarded newer APIs, general union refinement, and
broader cast targets remain planned.

### Object refinement

A positive `instanceof` branch refines fresh reads of the tested local or
parameter. Object and Object-or-Null can narrow to the supported concrete scalar,
Dictionary, Method, Symbol, or WeakReference contract. An Object test establishes
non-nullness. An Array test also establishes non-nullness, but cannot infer an
Array element contract or enable indexed access on an erased Object. Existing
concrete storage contracts retain their types, with Null removed when proven.

```text
using Toybox.Lang;
function length(o as Toybox.Lang.Object) as Toybox.Lang.Number {
    if (o instanceof Lang.String) { return o.length(); }
    else { return 0; }
}
```

The tested read and predicate must directly feed the branch in the same block.
Boolean negation reverses the proof edge; AND/OR and conditional expressions
carry facts through their emitted branches. False tests do not subtract types.
Entry has no facts. Joins retain only facts shared by every incoming edge;
loops include their entry and backedges. A local write invalidates its facts,
including a write between the tested read and branch. Unrelated writes and
calls preserve facts about local/parameter reference values.

Aliases and stored Boolean conditions do not transport proofs. New copies made
inside a proven branch receive the narrowed value type; storage that existed
before the test keeps its original assignment contract. Fields and arbitrary
expressions are not proof-bearing storage locations. Source superclass chains,
container elements, method signatures, and API guards require separate contracts.
Source and saved IR share these rules. See
[runtime evidence](../investigations/compiler-object-tests.md).

### Source-class targets

`value instanceof ClassName` tests against a declared Object-, View-, or
BehaviorDelegate-derived source class. Targets resolve across all unit input
files, independently of declaration order. A successful test gives fresh reads
of the tested local/parameter the exact source identity and its native projection,
using the same branch, join, and write-invalidation rules above. This enables
source calls, exact signature/field contracts, and structural comparator checking.
Different classes sharing a native base remain distinct. Null does not match.

`value as ClassName` is a lexical assertion of that same exact identity and
projection. It needs no proof and leaves runtime identity unchanged, including
for false assertions. A copy initialized from a proven/asserted source value
retains its exact class requirement. An opaque Object local narrowed only for a
branch keeps its original assignment contract; a later write invalidates its
proof.

Class operands are named declarations, not dynamic class values. The bounded
profile rejects local/parameter shadowing of a source target. Qualified/nested
source names, source inheritance, AppBase targets, and nullable source-class
assertions remain unsupported. Unit validation checks target identity and base;
standalone function IR retains that declaration-resolution obligation.

## Lexical casts

`expression as Lang.Type` explicitly asserts a value's static type. Supported
targets are Number, Long, Float, Double, Boolean, Char, String, Object, Symbol,
Method, WeakReference, and opaque Dictionary. Canonical `Toybox.Lang.Type` and
Lang module aliases are accepted. Number, Long, Float, Double, Boolean, Char,
String, Symbol, and Object targets can include `or Null`. Target types still satisfy
selected API introduction requirements.
[Source-class targets](#source-class-targets) additionally retain exact identity.
Imported native names use the same cast profile. Parameterized containers,
interfaces, general unions, Void, and Null alone remain outside this profile.

```text
using Toybox.Lang;
function length(o as Toybox.Lang.Object) as Toybox.Lang.Number {
    return (o as Lang.String).length();
}
```

The operand is evaluated once. The assertion produces the same runtime value,
without conversion, allocation, class lookup, or a check. It can remove Null or
assert a type unrelated to the operand's static type. The caller is responsible
for that assertion; a falsely asserted value can fail when subsequently used.
Runtime `instanceof` still sees the original value's class. A source object's
assertion to an opaque native type erases exact source-class provenance.

Casts bind below unary/member/index operations and above binary arithmetic and
comparisons. Chains associate left to right and share the 64-level expression
nesting limit. `o as Lang.Number + 1` asserts the left arithmetic operand;
negating an Object requires `-(o as Lang.Number)`. Parenthesize a cast before
member/index access. `or Null` belongs to its target; other `or` expressions use
ordinary logical precedence. Assertions in conditional arms, staged operands,
and native/user call arguments preserve existing evaluation and ownership rules.
A cast does not change a storage declaration or establish a branch/API proof.

See [SDK/runtime evidence](../investigations/compiler-object-tests.md#lexical-casts).

## Function type contracts

Ordinary source parameters default to Number. `as` annotations accept the fully
qualified names `Toybox.Lang.Number`, `Toybox.Lang.Long`, `Toybox.Lang.Float`,
`Toybox.Lang.Double`, `Toybox.Lang.String`,
`Toybox.Lang.Boolean`, and `Toybox.Lang.Char`; result annotations also accept
`Void`. Imported bare native names and qualified module aliases follow the
[native name profile](#native-name-declarations), including supported container
arguments. General unions remain outside this subset. Unqualified constructible
source-class names additionally provide [exact source contracts](compiler-classes.md#source-class-signatures).
Numeric, String, Boolean, and Char annotations additionally accept `or Null`
for both parameters and results. Reversed alternatives, parenthesized unions,
and other type syntax remain unsupported. Canonical Toybox paths are available
without native-name declarations.

Unannotated functions with value returns require Number results; those with no
value returns have a Void contract. Long, Float, Double, String, Boolean, Char, View, array, and
nullable source results require explicit annotations. Each return must be
assignable to its contract:
a nullable result accepts its scalar type, Null, or the same union. Unrelated
types, value/void mixtures, and missing returns are errors. Typed IR constructors
infer uniform scalar/Void returns unless an explicit signature is supplied.
Native callbacks retain
their independently resolved signatures; annotations cannot override them.

## User module calls

The supported call form is `<module>.<member>(<arguments>?)` in expressions
and initializers. The module is a top-level user module provided by the
surrounding PRG scaffold. Value calls return supported scalar, object, array, or
nullable types.
Arguments must satisfy the declared types and positional arity: a nullable
parameter accepts Null, its scalar type, or the same union; scalar parameters
require exact types. Global unqualified
calls, arbitrary local/object receivers, and nested modules remain unsupported.
Void module calls are supported as statements, as described below.
The separate `System.println` statement is described below. Module/member names
stay symbolic until encoding; neither IDs nor DATA method records are generated
here.

IR contains `ModuleCall { result, result_type, module, member, arguments }`.
[Call text](compiler-ir-text.md#module-calls-and-operation-order) preserves the
result contract. Calls may have
effects. For any function containing a call, IR operations must appear in the
backend's left-to-right expression evaluation order, with initialization and
print statements in order. Reject incompatible schedules rather than silently
reordering effects; pure-only IR keeps its existing ordering flexibility. This
conservative restriction can change when a more general scheduling policy is
implemented.

Backend lowering selects `getmv @__ciq_module_<module> @__ciq_member_<member>;
invokemz`, for zero arguments. Nonempty calls additionally use `frpush`,
argument expressions in order, and `invokem <argument-count + 1>`. Module
bindings must identify `globals/<module>`; the short `<module>` symbol is a
distinct field identity. The compiler CLI supports `--symbols
<symbol-source.mcasm>` for CODE emission: resolve generated aliases by unique
stored names in the provided `.symbol` declarations, reject absent/ambiguous
targets, and preserve ordinary aliases for assembly inputs. Library consumers
can supply bindings through the assembly encoder. [Runtime
evidence](../investigations/compiler-calls.md) confirms return values and
observable call order; the helper functions and their DATA module/method records
are still supplied by the SDK scaffold.

## Parameters and positional arguments

The frontend accepts comma-separated untyped integer parameters and positional
integer expressions in user-module calls. Parameter identities are zero-based
positions, distinct from locals/value IDs; repeated reads define fresh values.
Duplicate names and local/parameter shadowing are rejected. Caller arity and
callee types are still obligations of the supplied scaffold.

Function IR records a parameter count (default zero); `ParameterRead` defines a
value by positional index. `ModuleCall` carries an ordered argument-value list.
Existing `.function <name>` and zero-argument `.call` text remain accepted;
optional `parameters=<count>` on `.function`, `.parameter %<result> <index>`,
and argument IDs after `.call %<result> <module> <member>` represent the
extension. Argument dependencies and nested calls participate in the existing
evaluation-order validation.

Backend prologue counts parameters plus one context slot; parameter index 0
loads slot 1, and local slots start after all parameters. Nonempty calls select
the callable, emit `frpush`, evaluate arguments left to right, then `invokem
<argument-count + 1>`. Empty calls keep `invokemz`. Function/call parameter
counts are limited to 254 and parameters plus locals to 255, so existing u8
fields/slot indexes fit. The 1024-operation and 64-level expression nesting
limits remain. These encoding limits do not claim runtime validation at maximum
counts.
[SDK and runtime evidence](../investigations/compiler-arguments.md) verifies two
parameters, a following local, nested calls, and generated caller/callee
execution with observable argument order.

[ref-controlled-sdk-output-and-simulator-resu]: ../investigations/compiler-locals.md

## Strings and printing

Double-quoted literals accept UTF-8 characters and source escapes `\n`, `\r`,
`\t`, `\"`, and `\\`. Raw newlines, unfinished/unknown escapes, and values
longer than 65535 UTF-8 bytes are rejected. Empty strings are allowed. Source
Unicode escape syntax is not supported; IR/assembly use their own documented
escape syntax.

String literals, native conversion results, and string-valued local reads can
initialize locals or supply
`System.println(expression);`. Print statements may occur between local
initializations and accept one Number, Long, Float, Double, String, Boolean, or Char value.
String types
propagate through locals and annotated function parameters/results; incompatible
uses are rejected. Addition follows the operator table above, including Char
concatenation and supported scalar conversions. Arbitrary
SDK calls, and using
`System.println` as an expression remain unsupported. A local/parameter named
`System` cannot be used as the print receiver. The function's scaffold must
provide the SDK module. The native name profile below resolves its supported
spellings; Jungle resolution remains unimplemented.

## Short-circuit expressions

Boolean `&&`/`and` and `||`/`or` support conditions and value-producing expressions,
including grouping and negation. Word spellings require complete identifier
tokens and have identical short-circuit behavior and precedence; type annotations
still use `or` for unions. Comparison binds before `&&`, which binds
before `||`; each binary level is left associative. Evaluate the left operand
once. `&&` evaluates the right only when the left is true; `||` evaluates it
only when the left is false. Every operand must be Boolean, even when a constant
left operand would skip it at runtime. No truth-value conversion, bitwise
behavior, or `and`/`or` keyword aliases are supported.

The builder emits explicit branches and intermediate blocks, with no eager
binary operation or cross-block expression values. Condition negation swaps destinations.
Nullable facts follow the selected edges; joins retain only facts established
on every incoming path. For example:

```monkeyc
if (!(value == null) && value > 0) { System.println(value); }
if (value == null || value > 0) { return 0; }
return value;
```

The right operand and true arm of the first condition know `value` is non-null.
The false path of the second condition knows the same. In contrast,
`!(value == null) || flag` does not prove non-null on its true path.

Logical results are Boolean and may initialize/assign locals, feed calls,
returns, printing, comparisons, concatenation, and native receiver calls under
those consumers' normal type contracts. Value results initialize a temporary
Boolean local to false, assign true on the selected outcome, then read it at
the join. Earlier operands and receivers are evaluated and stored before later
operands branch; fresh temporary reads feed the final operation in source order.
This applies to expression and statement-call arguments alike. Temporaries have
distinct local identities and count toward the existing combined frame-slot
and operation limits. Slots are not reused.

Storing a logical result carries no null-proof relation to the checked source
storage. Guarded right operands may use non-null reads, but a later branch on
the result local does not narrow the original. Existing nesting/operation
limits apply to parsing and generated blocks. See the
[condition evidence](../investigations/compiler-short-circuit.md) and
[value-result evidence](../investigations/compiler-logical-values.md).

## Native name declarations

Default and unit source modes accept these declarations before functions or
owners at the start of a file, and before methods at the start of a
class/module:

```text
native-declaration = "using" native-module ("as" identifier)? ";"
                   | "import" native-module ";"
native-module = "Toybox.Application" | "Toybox.System" | "Toybox.WatchUi"
              | "Toybox.Graphics" | "Toybox.Lang"
```

`using` introduces a module binding, optionally aliased. `import` introduces
the module name and its directly declared type names known to the API catalogue:
classes, interfaces, and enums. Bare native names require that import; qualified
module names and aliases resolve through the same catalogue. For example,
`import Toybox.Lang;` permits `Number` in annotations, casts, and runtime class
tests, while `using Toybox.Lang as L;` permits `L.Number`.

Resolution also covers supported Array element and Dictionary key/value type
arguments. Each consumer retains its own executable profile: recognizing
Comparator does not make it a runtime class, and recognizing Array does not
enable an unparameterized cast. API introduction checks use canonical types;
imports neither filter names by API level nor bypass those checks.

The same environment resolves base clauses and supported constructors:
`import Toybox.Application;` permits `extends AppBase`; module aliases permit
`extends App.AppBase`. Existing canonical short module paths such as
`Application.AppBase` and `System.println` remain available without declarations.
Fully qualified Toybox paths are also accepted.

Bindings apply to following declarations in their file and are inherited by
class/module bodies. Owner-local bindings do not escape their owner; bindings
are not shared between files, including reopened source modules. Imports in
the body cannot resolve that class's superclass clause. This is a bounded
frontend profile, not a specification of all Monkey C namespace behavior.

Reject duplicate bindings, shadowing by parameters/locals/functions/owners,
aliases of reserved identities, import aliases, declarations after functions,
and native modules outside this profile. Imported type names also conflict with
module aliases in either declaration order. Bare imported module functions and
constants, general source namespace aliases, and types outside the catalogue
remain unsupported. Number conversion uses
inherited lookup only as specified below. Calls on ordinary user modules
and `self` keep their existing rules. Supported native spellings resolve to the
same semantic operation or superclass identity before IR construction; aliases
are not serialized as VM symbols or executable import indexes.

`String { result, value }` defines a semantic string; `Println { value }` is an
effectful sink without a result. Printing participates in the same conservative
IR scheduling check as user calls. Lowering uses `news` for literals and `getmv;
frpush; <argument>; invokem 2; popv` for printing. The dedicated
`__ciq_sdk_System` alias binds only to `Toybox_System`; the member alias binds
to `println`. This does not establish a general SDK naming convention.

Lowering deduplicates literal content within each function, naming declarations
`__ciq_string_<function>_<first-occurrence-index>`. These CODE declarations omit
`at`; they describe required content without assigning DATA placement. Raw CODE
needs `--data <data-source.mcasm>` in addition to `--symbols`. Matching and
integration are described in [IR pipeline commands](compiler-ir-text.md) and
[the runtime experiment](../investigations/compiler-strings.md). Application
DATA allocation and method metadata remain explicit scaffold work.

## Terminal conditions

Runtime comparisons follow the [Number-comparison evidence][comparisons].

A function may end with `if (<condition>) { return <integer-expression>; } else
{ return <integer-expression>; }`. Braces are required; `else` is optional.
An omitted false arm continues to the next statement, which must eventually
return. Arms may also contain local declarations, print statements, and nested
conditions, as
described below. Loop syntax is specified separately below.

Comparisons follow the operand table above and numeric-width contracts below,
producing Boolean results. Equality/inequality also accept Boolean pairs and
the String identity profile; `!=` negates the existing equality
contract, not String content equality. Dictionary/null checks follow the
callback rules below.
Arithmetic has higher precedence, so `value + 1 > 2` compares the sum. At most
one comparison is accepted per expression level; parentheses retain their grouping
behavior. The condition must produce a Boolean: a literal, typed parameter or
local read, supported call, or comparison. Boolean locals, assignments, and print arguments are
accepted. Annotated Boolean parameters/results and user-call arguments use
these same rules. Boolean arithmetic, integer truthiness, mixed Boolean/Number
equality and Boolean/String ordering are unsupported. Short-circuit operators
follow their separate rules above.

Lowering emits an entry block, condition evaluation, and branch, then separate
arm blocks. Both arms may read parameters and enclosing locals. The
[IR rules](compiler-ir-text.md#control-flow) validate
block-local values and initialization on every path. See
[conditions.mc](../../examples/compiler-probe/conditions.mc) and
[initial simulator evidence](../investigations/compiler-conditions.md).

## Conditional expressions

`condition ? whenTrue : whenFalse` binds below logical OR and associates to the
right. The condition must be Boolean. Evaluate it once, then evaluate only the
selected arm. Both arms must type-check even with a constant condition. Preserve
operand/argument order when an enclosing expression has earlier effects.

Supported arm types match exactly: numeric scalars, Char, Boolean, String, Null,
or an
invariant Number/Char Array. Scalar alternatives may also join with Null
or their existing nullable type, producing the corresponding scalar-or-Null. No
Char-to-Number promotion, heterogeneous union, nullable Array, class join,
Void arm, or non-Boolean truthiness is supported. Expected Array element context
from a declaration, call, or return reaches direct empty literals in both arms;
no element contract is inferred solely from a sibling arm.

Join inference uses checked incoming operand types after the complete graph's
null-flow analysis. A direct nullable read under a non-null proof contributes
Number or String, so `n != null ? n : 0` has type Number. Nested joins and copies
of proven reads retain those checked types. Unproven reads remain nullable;
stored Boolean tests do not establish proofs, and local writes invalidate them.
Nullable results still require proof at non-null consumers. Source construction
and saved IR share these validation rules.

Lower through [typed IR joins](compiler-ir-text.md#expression-joins). Stage
results and earlier operands when a consumer needs call/field/array stack setup;
these locals count toward the existing frame limit. A direct conditional return
carries its selected value on the VM stack without a staging local. Existing
nesting/node/operation bounds apply, including nested arms, calls, and arrays.
See the [example](../../examples/compiler-conditionals/README.md) and
[SDK/runtime evidence](../investigations/compiler-conditionals.md).

## Statement arms and joins

Conditions may appear between statements; combined `if`/`while` nesting is
limited to 64 levels. Empty arms are
allowed; `else if` without braces is not accepted. Each arm has its own local
scope. The function must return an integer on every fallthrough path. Statements
after `return`, `break`, `continue`, or a conditional whose arms both terminate
are rejected as unreachable. A path with no return is rejected at the function's
closing brace.

Continuing arms jump to a shared block after both arms. Returning or
loop-control arms do not contribute an edge to that join; when both arms
terminate, no join is generated. Only enclosing locals are visible after the
condition. Even if both arms declare the same name, those distinct declarations
do not define a result at the join. Enclosing locals can carry results using
assignment; no phi values or shared stack values are introduced.

The semantic 1024-operation limit includes generated block markers and
terminators. The 255 parameter/local slots count all declarations across the
function, including mutually exclusive arms; slots are not reused. Local
initializers, prints, comparison operands, and module call arguments retain
source evaluation order within the selected path. Common initializers run once
before the branch. These forms use the control-flow IR directives.

See [nested-conditions.mc](../../examples/compiler-probe/nested-conditions.mc)
and [nested/continuing-arm simulator evidence][nested]. Full language scoping,
other loop forms and standalone blocks remain future work.

[nested]: ../investigations/compiler-nested-conditions.md

## Local reassignment

`<identifier> = <expression>;` replaces an earlier initialized, in-scope local.
The local target must be a bare identifier. Parameters, unknown/forward names,
and arm locals used outside their scope are rejected. Assignment is a statement,
not an expression. Local compound and increment/decrement statements are
specified below.
Class methods additionally support [explicit self field
assignment](compiler-classes.md#number-instance-fields).

The initializer fixes the local's supported type. Each assignment must preserve
it, including Boolean locals; a nullable Dictionary local may receive null.
General type changes are rejected. RHS evaluation finishes before the write,
so `n = n + 1` reads the old
value. Later reads observe the new content on that path. Assigning an enclosing
local inside an arm is allowed; assignment does not create a declaration.

A write consumes one semantic value and defines no result. It reuses the local's
existing slot and does not contribute to the 255-local count. The operation
count still includes each write and its RHS definitions. Both arms may update a
common local, or one arm may leave it unchanged; it remains available at the
join because its initializer executes before the condition.

Source, typed IR, and text IR share type, availability, and effect-order checks.
Assignment-containing IR uses the conservative schedule specified in
[IR text](compiler-ir-text.md#local-assignment). See
[assignments.mc](../../examples/compiler-probe/assignments.mc) and
[integer/string simulator evidence][assignments].

[assignments]: ../investigations/compiler-assignments.md

## Increment and decrement

`target++;` and `++target;` add one to a numeric scalar or Char, preserving its type.
`target--;` and `--target;` subtract one from a numeric scalar; Char decrement
is rejected.
The two operator characters must be contiguous. Accepted targets are bare
initialized numeric/Char locals, explicit `self` Number fields, and Number/Char
array elements. Char requires API 1.3.0, as for other Char expressions.
Parameters, grouped local targets, other value/element types, and unknown or
out-of-scope names are rejected. A nullable Number local needs an established
non-null proof at the read. Existing assignment contracts still govern its write.

Both prefix and postfix forms are statements with identical supported behavior;
they cannot be arguments, conditions, initializers, return values, or parts of
larger expressions. Source lowering retains a checked step and separate
write, without assigning expression-result semantics to either spelling.

For indexed targets, evaluate/capture array then index once, read the selected
element, add/subtract one, and write back to that captured location. Bounds and
allocation failures remain runtime obligations. Captures share the ordinary
frame budget; local updates reuse their existing slot. The tested Number
boundaries wrap from maximum to minimum on increment and minimum to maximum on
decrement. See [SDK/runtime evidence](../investigations/compiler-increments.md).

## Local compound assignment

A bare in-scope initialized local supports `+=`, `-=`, `*=`, `/=`, `%=`,
`&=`, `|=`, `^=`, `<<=`, and `>>=` as statements. Operator characters must be
contiguous. Parameters, grouped local targets, and unknown or out-of-scope
names are rejected. Field/indexed compound targets are specified below. Assignment expressions, logical compounds, and unsigned-right-shift compounds
remain unsupported.

`n op= rhs;` reads the existing local once, evaluates `rhs` once, applies the
corresponding supported binary operator, then writes the result to the existing
slot. The whole RHS is parsed before applying the implicit operator:
`n *= 1 + 2` means `n = n * (1 + 2)`. Existing binary contracts and fixed local
types apply: all ten operators accept non-null Number pairs; String `+=`
uses supported scalar concatenation, and Char `+=` accepts Number/Long. Number
`+= Char` and Char `+= Char` fail because their results change the local type.
No new coercion, folding, or runtime guard is introduced.

The read/binary/write sequence uses ordinary IR operations and source spans.
A RHS needing control-flow staging snapshots the old local before RHS effects;
hidden locals share the existing frame budget. Plain compounds need no hidden
slots. A nullable local requires an existing non-null proof for its old read,
and the write invalidates that proof as ordinary reassignment does. A later
non-null read needs a fresh check. See [compound evidence][compound-locals].

[compound-locals]: ../investigations/compiler-compound-locals.md

## While loops

`while (<condition>) { <statements> }` evaluates its condition before each
iteration. The condition must produce a Boolean, with the
same Boolean/type rules as `if`. Braces are required; the body may be empty or
contain locals, assignments, prints, nested conditions/loops, and early returns.
`for` and `do/while` are unsupported.

Body locals are visible within that body and nested scopes, but not after the
loop or in its condition. Their existing slots are initialized anew on each
iteration; new slots are not allocated per iteration. Enclosing local writes
persist into later tests and after exit. Fixed local types still apply.

The body may execute zero times. Its declarations cannot establish a local
needed at the test or exit. A function ending with a loop still needs a return
for its false-condition path, even if its body always returns. The compiler does
not prove termination or condition outcomes and does not insert an iteration
limit. "Bounded subset" refers to supported syntax and compilation limits.

Lowering uses explicit test/body/exit blocks and a symbolic backward jump from
continuing body paths. Conditions are evaluated in the test block each visit;
body effects stay in the body. Every edge has an empty evaluation stack. These
use branch/jump directives; cyclic initialization checks
are specified in [IR text](compiler-ir-text.md#cyclic-control-flow).

See [loops.mc](../../examples/compiler-probe/loops.mc) and
[zero/multiple-iteration simulator evidence][loops].

[loops]: ../investigations/compiler-loops.md

## Loop control

`break;` exits the innermost enclosing `while`; `continue;` jumps to its test,
reevaluating the condition before another iteration. Both require semicolons
and an enclosing loop, including when nested within conditional arms. They
terminate their current path and carry no expression values across the edge.
A nested loop's controls never target an outer loop. Labeled controls are
unsupported.

A loop may still run zero times, so its body cannot satisfy the function's
fallthrough return requirement. Mixed returning/breaking/continuing arms omit
a join when all paths terminate, but the loop's false-condition exit remains.
Skipped declarations stay scoped to their body/arm; initialized enclosing
locals retain their slots and fixed types. These statements emit existing
`.jump` operations.

See [loop-control.mc](../../examples/compiler-probe/loop-control.mc) and
[simulator evidence](../investigations/compiler-loop-control.md).

## Void functions and statement calls

A value-returning function must return its declared type on every path. A Void
function's `return;` terminates immediately; reaching the final closing brace
inserts a void return. Value and void return forms cannot be mixed, including
across branches. Unreachable statements after either form are rejected. Loops
and scoped locals retain their existing rules. See [function type
contracts](#function-type-contracts) for annotations and defaults.

```monkeyc
function main() {
    Startup.greet(41);
}
module Startup {
    function greet(number) {
        System.println(number);
        return;
    }
}
```

Qualified user calls may be expression statements. They take typed positional
arguments and discard any supported result, including Void. Void calls
cannot initialize locals, provide arguments, or appear in any other expression.
Unit mode checks the callee's result type and arity after collecting all
signatures. Single-function mode still leaves external callee contracts to the
caller; expression calls there assume integers. No void expression value is
created. Statement calls preserve argument evaluation and statement order.

Typed IR adds `ReturnVoid` and `ModuleCallStatement`; `Function::return_type()`
reports the validated scalar, nullable, or Void contract. Saved text uses
[IR](compiler-ir-text.md#void-returns-and-statement-calls).
Both forms share type, control-flow, use, and effect-schedule validation.
A void function may contain only its return, so the operation bound is now
1..=1024. The backend emits bare `return`; statement calls use the existing call
sequence and discard the VM result with `popv`.

The [console application profile](console-application.md) prints a value entry's
Number, Long, Float, Double, String, Boolean, or Char result and discards a Void entry's result.
The [example][void-example] and
[simulator evidence][void-evidence] verify both void return forms and caller
continuation after discarded calls. Source class/receiver syntax follows the
[restricted class profile](compiler-classes.md).

[void-example]: ../../examples/compiler-void/README.md
[void-evidence]: ../investigations/compiler-void.md

## Instance class extension

Unit mode also accepts [AppBase- and View-derived class
declarations](compiler-classes.md). Their methods may call `self.<member>(...)`
as typed value expressions or statements; class ownership and receiver contracts
remain explicit in typed/saved IR. Default single-function source mode does not
accept self calls. Source initialization and onStart use the
[native lifecycle contracts](compiler-classes.md#source-startup-callback).
Null literals and opaque nullable callback-state locals support equality checks;
general object values/fields, source inheritance,
and general native calls remain open. See [runtime
evidence](../investigations/compiler-class.md).

## Native module statements

`System.exit();` requests application termination and has no expression result.
Recognized aliases and `Toybox.System.exit()` use the native module-name rules.
The call requires zero arguments and is validated through its catalogue Void
contract and optional API context before lowering. Unknown module members,
wrong arity, and known members lacking supported lowering are rejected.

Source and saved IR retain this as an ordered native module statement; it is
not an IR return or branch. User module calls and dedicated `System.println`
operations retain their own forms. `WatchUi.requestUpdate()` is the other
supported native module statement: zero arguments, Void, API 1.0.0. It requests
a redraw of the current View and follows the same alias/validation rules.
Runtime exit invokes a supplied source `onStop` callback in the
validated console app; its unguarded Dictionary-or-null state is not a user-call argument.

## Floating-point values

Float and Double are distinct numeric scalars, available since API 1.0.0.
Decimal literals with a fractional part or decimal exponent default to Float;
a lowercase `d` suffix selects Double, including an integer mantissa such as `1d`.
A fractional part requires digits on both sides of the point; an exponent uses
`e`/`E`, optional sign, and at least one digit. Literal parsing rounds to IEEE
754 binary32/binary64. Values rounding to infinity are rejected; underflow may
round to zero. A negative literal preserves signed zero. Parenthesize a negative
literal before calling a member, as for Number/Long.

Explicit Float/Double parameters/results, local storage, module/instance calls,
printing, conditional joins, and `or Null` contracts use the shared scalar rules.
Assignments, arguments, returns, and conditional arms do not implicitly convert
numeric widths. Arrays and Dictionaries follow their shared scalar contracts;
source fields of these types are not supported yet. String concatenation and
Char equality accept these widths; Char ordering/addition with floating values
remain unsupported.

The four arithmetic operators `+`, `-`, `*`, and `/` accept Number, Long, Float,
and Double pairs. Double dominates; Long mixed with Float also produces Double;
otherwise Float dominates integer operands. Integer-only pairs follow their
existing rules. All six numeric comparison operators accept these pairs and
return Boolean. Floating `%`, bitwise/shift operations, `!`, and `~` are rejected.
Unary plus preserves type/value. Negation subtracts from a same-type positive
zero; this differs from a negative-zero literal at zero. Statement `++`/`--`
and arithmetic compound writes preserve the local's type under these promotion
rules. Floating arithmetic and exceptional values are runtime operations;
the compiler inserts no conversions or constant folding. The tested SDK 9.2.0 /
fr265 runtime rejects the tested Float/Double zero-divisor cases with
`Invalid Value`; these operations do not supply infinity or NaN values.
See [isolated boundary evidence][numeric-boundaries].

Float/Double receivers support `abs()` returning the receiver type,
`format(String)` and `toString()` returning String, and numeric conversions
`toNumber()`, `toLong()`, `toFloat()`, and `toDouble()` returning their named
scalar types, since API 1.0.0. Number and Long also support both floating
conversions since 1.0.0. In-range floating-to-integer conversions truncate toward
zero. Tested extreme finite and non-finite values convert to the minimum signed
Number/Long on the SDK 9.2.0 / fr265 simulator; this is not a portable conversion
guarantee or a rule to fold into generated code. See
[inputs and limits][numeric-boundaries].

Inherited `hashCode()` returns Number, since 1.0.0; immutable `weak()` returns
the effective receiver type since 1.2.0. Float inherits Object `equals` since
1.0.0; Double overrides it since 1.3.0. Equality accepts the
[shared Object argument profile](#object-equality-and-container-observers)
and distinguishes floating widths and integer classes. Do not infer a
cross-instance Double hash contract from numeric equality. `compareTo` accepts
non-null numeric/Boolean/Char arguments and returns Number since 5.0.0.
String `toFloat()` returns Float or Null since 1.0.0; `toDouble()` returns Double
or Null since 3.1.0. Guard nullable results before scalar use. Conversion content
and formatting are passed through to the runtime.

See the [grouped example](../../examples/compiler-float-values/README.md) and
[encoding/runtime evidence](../investigations/compiler-floating-values.md).

## Long values

Long is a signed 64-bit scalar, distinct from Number, available since API 1.0.0.
Decimal literals end in `l` or `L`; values must fit i64, including
`-9223372036854775808l` and `9223372036854775807l`. An unsuffixed token retains
Number's signed 32-bit limit. A minus immediately applied to a Long token forms
a negative literal under the same spacing/postfix restrictions as Number.
Unary plus and negation also accept Long expressions, preserving their width.

Explicit `Toybox.Lang.Long` parameters/results, local copies/reassignment,
module/instance calls, discarded calls, repeated construction, and
`System.println` preserve all 64 bits. Long and Number contracts do not
implicitly convert. Unannotated source returns still require Number; inferred
IR returns can carry Long.

Long/Long and mixed Number/Long pairs support `+`, `-`, `*`, `/`, and `%`.
Either Long operand makes the result Long; Number/Number keeps Number. Promotion
belongs to these operators only: arguments, returns, and assignments still
require their declared storage type. Long locals support the corresponding
compound assignments with Number/Long right operands; a Number local cannot
store a promoted Long result. Nullable numeric operands require a non-null proof.

These pairs also support all six numeric comparisons, returning Boolean. Long
mixing with Char admits equality/inequality and addition returning Char; ordering
remains unsupported. String mixing follows the shared String profile. Boolean
mixing remains unsupported; nullable Long equality with Null supports proofs.
Unary plus preserves the value; unary minus, addition, subtraction, and multiplication
wrap to signed 64-bit results. Integer division truncates toward zero, and
remainder follows the dividend's sign. On the SDK 9.2.0 / fr265 simulator,
zero divisors report `Invalid Value`; minimum Long divided by or reduced modulo
-1 terminates the simulator process. No portable boundary result or exception
recovery is established. Keep native instructions without guards or folding;
see [isolated boundary evidence][numeric-boundaries].

Long/Long and mixed Number/Long pairs also support `&`, `|`, `^`, `<<`, and
`>>`, producing Long whenever either operand is Long. This includes a Number
left operand and Long shift count. The current runtime uses the low six bits
of the count for a promoted Long shift; negative counts and counts above 63
therefore wrap. Right shift is arithmetic, retaining the sign. Pure Number
pairs retain their existing 32-bit contracts; the compiler inserts no count
conversion or masking code.

`!` and `~` complement all 64 Long bits and return Long. `!` does not make a
Long into Boolean; conditions still require Boolean. Prefix/postfix `++` and
`--` accept Long as statements, preserve its type, and wrap at the signed
boundaries. Bitwise/shift compound assignments follow operator promotion and
must preserve the local's storage type. Updates remain invalid as expressions.

Long receivers support these native methods:

| Method | Result | Since API |
|---|---|---|
| `abs()`, `toLong()` | Long | 1.0.0 |
| `toNumber()` | Number | 1.0.0 |
| `toString()`, `format(String)` | String | 1.0.0 |
| inherited `hashCode()` | Number | 1.0.0 |
| inherited `weak()` | Long | 1.2.0 |
| `equals(other)` | Boolean | 1.3.0 |
| `compareTo(numeric, Boolean, or Char)` | Number | 5.0.0 |

`toNumber()` retains the low 32 bits as a signed Number. Number's `toLong()`
sign-extends its value, since API 1.0.0. `abs()` of minimum Long wraps to minimum.
Equality does not equate Long and Number values; comparison supports numeric
cross-types. Native argument profiles and null-proof requirements follow the
[shared rules](#native-value-calls).

String `toLong()` and `toLongWithBase(Number)` return Long or Null, since API
3.1.0. The runtime parses a numeric prefix and returns Null for an invalid prefix
or signed 64-bit overflow. Base 0 detects prefixes; 2..=36 specifies a radix.
The compiler passes conversion content and bases through without validation.

`Long or Null` parameters/results, locals, and conditional joins use the shared
nullable rules. A direct non-null proof is required before numeric operations,
printing, or receiver dispatch. Matching Long arms join without conversion;
mixed Number/Long conditional arms remain invalid.

Long `toFloat()` and `toDouble()` are available since API 1.0.0, as described
below. Long arrays and Dictionaries follow their shared scalar contracts; fields
remain deferred. See the
[example](../../examples/compiler-long-values/README.md) and
[encoding/runtime evidence](../investigations/compiler-long-values.md).

## Dictionary values

`{}` creates a fresh empty `Toybox.Lang.Dictionary` (API 1.0.0). Whitespace and
comments may separate the braces. Dictionary references may initialize/copy or
replace Dictionary locals and pass through explicitly annotated non-null
Dictionary parameters/results. Copies preserve reference identity; reassignment
does not copy contents. Calls and construction retain source evaluation order.

`Toybox.Lang.Dictionary<K, V>` accepts invariant non-null numeric, String,
Boolean, Char, or Symbol keys and values of those types, optionally `or Null`.
It also accepts [comparison-interface keys and values](#comparison-interface-dictionaries).
The Dictionary itself is non-null. Char keys or values require API 1.3.0;
the other contracts require 1.0.0. `DictionaryType` carries the bounded contracts through source,
semantic IR, locals, function arguments/results, and API checks.

A direct `{}` receives its key/value contracts from an annotated return,
call parameter, or reassigned typed local. Without that context it stays opaque;
later uses do not retrospectively type an opaque local. Different key or value
contracts cannot widen or narrow through assignment. Typed references may pass
to opaque Dictionary parameters/results, but erasure cannot be reversed.

Nonempty `{key => value, ...}` literals accept expressions producing homogeneous
numeric, String, Boolean, Char, or Symbol keys and homogeneous values with optional
Null. Without context, a non-null value establishes the value type; any Null
makes it nullable. Null-only values require an expected nullable contract.
Typed contexts check every entry without changing invariant key/value types.
Null keys, mixed key kinds without an expected interface contract,
mixed non-null scalar value kinds without an expected
interface contract, container-valued entries,
and trailing commas are unsupported. Nested Dictionary literal syntax is also
unsupported, including inside entry call arguments. Ordinary calls and Array
reads may produce scalar entries.

Literal-only entries use `hpush` and a [scalar DATA constant](data-dictionaries.md).
Signed Number tokens use the existing 32-bit range. Constant strings are UTF-8;
their local pool length, including the zero terminator, must fit u16. Every entry
is type-checked before repeated scalar keys retain their last value. Numeric
constant keys match exact class and bits: signed zeros stay distinct, and
Double NaN keys never coalesce. String keys match text. These rules preserve
the tested Dictionary behavior rather than ordinary numeric operator equality.
Runtime hashing/lookups remain native operations; no key conversions are emitted.
See [wider numeric key evidence](../investigations/compiler-numeric-dictionaries.md).
Parentheses
around literals do not introduce effects or require computed construction.

Computed literals allocate a private Dictionary before entry expressions. Each
pair evaluates key, then value, then inserts before the next pair. Every
overwritten expression still executes. The `newd` argument is the source entry
count, including duplicates; its capacity/allocation effect is not specified.
No partially constructed reference escapes. Branching operands use private
locals under the existing frame limit. Every evaluation returns a fresh mutable
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
[Object equality and hash observers](#object-equality-and-container-observers).

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
Arrays, and Array edits do not change the Dictionary. Nullable V remains nullable
in the returned Array; non-null V does not acquire lookup's missing-key Null.
Array argument/result invariance, element checks, null proofs, and method API
minimums still apply. Opaque Dictionary references cannot supply K or V.

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

Chaining and discarded results use existing native-call rules.
Native callback Dictionary-or-Null state supports these opaque methods under a
non-null proof; unguarded access is rejected. Its nullable declaration/assignment rules
remain unchanged. Ordinary source nullable Dictionary annotations are not yet
supported.

Dictionary `==`/`!=`, conditional Dictionary joins, direct printing,
`new Dictionary()`, and the other methods remain unsupported. No key
or value contract is inferred from an empty literal without context. See
[the example](../../examples/compiler-dictionary/README.md) and
[SDK evidence](../investigations/compiler-dictionary.md).

### Planned Dictionary extensions

These are intended contracts, not executable support yet. Do not infer key/value
contracts from later writes or reinterpret an opaque callback Dictionary as a
typed one.

General heterogeneous unions, Null keys, and nested containers need separate
contracts before acceptance. Object `method` follows the
[bounded Method profile](#method-values-and-invocation);
ordinary `weak` follows the [bounded reference profile](#ordinary-weak-references).


## Object equality and container observers

Supported numeric, String, Boolean, Char, Symbol, Array, Dictionary, opaque Object,
Method, and WeakReference receivers accept `equals(other)` → Boolean. Its bounded
argument profile includes those scalar
types, all supported invariant Array contracts, opaque or typed Dictionary
references, opaque Object/Method/WeakReference values, Null, and supported nullable
forms. Comparing containers does not require matching element/key/value
contracts and does not convert or erase
their types. Nullable callback Dictionary arguments require no proof; nullable
receivers still require a non-null proof. Arbitrary source/native object
arguments and nested containers remain deferred. Opaque Object storage and its
limits follow the [weak-reference contracts](#ordinary-weak-references).

All supported Arrays and Dictionaries inherit `equals` and `hashCode()` →
Number from Object, since API 1.0.0. Element/key/value availability is checked
separately. Container equality tests reference identity: aliases compare equal,
while separately allocated containers with equal contents compare unequal.
Hashes remain stable across mutations and agree for aliases. The runtime
supplies their values; distinct containers need not have different hashes.

Both calls preserve receiver-before-argument evaluation and support discarded
results. Native dispatch retains runtime semantics for scalar equality,
including String content comparison and numeric class distinctions; no result
or hash is folded. Scalar override/API requirements remain in their respective
sections. This profile does not enable Array/Dictionary equality operators,
or general callback consumers. See the
[observer evidence](../investigations/compiler-object-observers.md).

## Native value calls

A Number-valued expression may receive these catalogue-resolved methods:

| Method | Declaring class | Result |
|---|---|---|
| `toString()` | `Lang.Object` | String |
| `format(String)` | `Lang.Number` | String |
| `abs()` | `Lang.Number` | Number |
| `toNumber()` | `Lang.Number` | Number |
| `toLong()` | `Lang.Number` | Long |
| `compareTo(numeric, Boolean, or Char)` | `Lang.Number` | Number |
| `equals(other)` | `Lang.Object` | Boolean |
| `hashCode()` | `Lang.Object` | Number |

Number `compareTo` requires API 5.0.0; the other listed Number methods require
1.0.0. Its declaration takes Object, but current lowering admits Number or
Long, Float, Double, Boolean, or Char only. Unchecked nullable comparison arguments are rejected.
`equals` uses the [shared argument profile](#object-equality-and-container-observers).
Dictionary arguments to Number `compareTo` remain unsupported.
Object arguments use catalogue-declared inheritance, not implicit conversions
or acceptance of unknown classes. Scalar user-function contracts remain exact.
Comparison/equality/hash behavior is supplied by the runtime; comparison signs
and hash consistency are the contracts, not particular magnitudes or hashes.
See the [Number group evidence](../investigations/compiler-number-methods.md).

String-valued expressions support these zero-argument methods, declared by
`Lang.String`: `length()` and `hashCode()` return Number; `toLower()`,
`toUpper()`, and `toString()` return String. These methods are available since
API 1.0.0. Runtime implementations determine character counts and case mapping;
the compiler does not evaluate these calls at compile time.

String `compareTo(String)` returns Number and requires API 5.0.0. Its Object
argument declaration is narrowed to String by the executable profile; unchecked
nullable arguments are rejected. String `equals` returns Boolean and accepts
the shared Object equality arguments, since API 1.0.0.
`toNumberWithBase(Number)` returns Number or Null, since API 1.4.1, and follows
the existing null-check rules. The base is 0 for prefix detection or 2..=36 for
an explicit radix. Numeric ranges and conversion content are passed to the
runtime without compiler validation. See the
[String group evidence][string-group].

Boolean receivers support `toString()` returning String, `hashCode()` returning
Number, and `equals` returning Boolean with the shared Object argument
profile as Number/String equality. These inherited methods date from API 1.0.0.
Boolean `compareTo` returns Number and accepts numeric/Boolean/Char arguments
since API 5.0.0. Unchecked nullable arguments are rejected. This native-call support
does not enable Boolean arithmetic or ordering operators. See the
[Boolean group evidence](../investigations/compiler-boolean-methods.md).

Char values originate from literals or Number's zero-argument `toChar()` and are distinct
from Number. They support `toLower()`/`toUpper()` returning Char, `toNumber()`
returning Number, and `toString()` returning String. Char's direct `compareTo`
returns Number for numeric/Boolean/Char arguments, since API 5.0.0. Inherited
`equals` accepts the shared Object argument profile and returns Boolean;
`hashCode()` returns Number. Char itself and its conversions require API 1.3.0,
including when it appears only in a function signature. Typed arguments,
returns, local initialization/assignment, printing, and chained calls retain
the exact Char type. Char operators follow the operand table above; nullable Char
requires a non-null proof for scalar operators and method receivers.
Conversion ranges and case/String behavior are runtime obligations;
the compiler adds no Unicode scalar-range validation. See the
[Char group evidence](../investigations/compiler-char-methods.md).

Numeric, String, Boolean, and Char receivers also support zero-argument `weak()` since
API 1.2.0, with Char receivers additionally requiring 1.3.0. Its effective result
is the exact receiver type, not WeakReference.
It remains an effectful native call; the compiler does not replace it with its
receiver. Nullable scalar receivers require a direct non-null proof
before the call. Ordinary references follow the separate profile below.
See the [weak-result evidence][weak-evidence].

String results may initialize/replace a String local or be printed. Number
results support integer locals, arithmetic, comparisons, user-call
arguments, integer returns, and further Number calls. For example,
`(0 - 41).abs().format("%03d")` produces `"041"` on the validated profile.
Existing user-call arguments and return restrictions remain unchanged.

Postfix calls bind more tightly than addition/subtraction; parenthesize a
computed arithmetic receiver, for example `(n + 1).format("%03d")`. Literal,
local, numeric/String/Boolean/Char parameter, and supported call-result receivers
are accepted. Evaluate the receiver first, then arguments left to right, then invoke
the member.

The frontend distinguishes visible local/parameter receivers from module calls.
Supported native value calls may also be statements: `n.abs();`,
`chars.remove('a');`, and `factory.make().add('x').size();` evaluate the call and
discard its final result. Receiver/argument order, API checks, and nullable
receiver proofs are the same as in value contexts. A discarded result does not
contribute to function return inference. Nullable results need no guard when
discarded. Bare arithmetic, literals, and element reads are not source expression
statements. Native Void methods still require their own supported statement
contract; discarding does not enable unsupported members or receiver profiles. See the
[statement evidence](../investigations/compiler-native-statements.md).

Null receivers remain unsupported; Dictionary receivers follow their bounded
[profile](#dictionary-values). Chained calls
must resolve each member against the preceding result's type, for example
`41.toString().toUpper().length().format("%03d")`. String and Boolean user
parameters/results require the function annotations defined above.
String `find(String)`, `toNumber()`, and `toNumberWithBase(Number)` return
Number or Null;
`substring(Number or Null, Number or Null)` returns String or Null. These follow the
[null-check rules](#nullable-results). Unknown catalogue
members, wrong arity/types, and known members lacking lowering are distinct
errors. Explicit [API context inputs][api-context] check introduction levels;
general native dispatch remains unsupported.
Format content is passed to the runtime; type checking does not establish that
a String contains a valid format pattern.

[Native call IR](compiler-ir-text.md#native-value-calls) preserves receiver/member
identity separately from executable IDs. Runtime evidence and reproduction are
in the [native call experiment](../investigations/compiler-native-calls.md).

## Symbol values

`:name` produces a non-null `Toybox.Lang.Symbol` since API 1.0.0. Names use
the ASCII identifier grammar. A Symbol is distinct from Number and String;
matching names denote the same identity within a build. Numeric IDs are assigned
during application/assembly binding and need not agree between builds or
toolchains. Do not treat them as persistent identifiers.

Symbols support annotated parameters/results, exact local assignment, matching
conditional arms, `==`/`!=` between Symbols, and the shared native `equals`
argument profile. They support `toString()` → String, inherited `hashCode()` →
Number and `equals` → Boolean since 1.0.0, immutable `weak()` → Symbol since
1.2.0, and `toNumber()` → Number since 2.3.0. Equality and conversions remain
native operations; no numeric ID or hash is folded into semantic IR.

Complete applications retain names for literal Symbols in a SYMBOLS section,
so their development `toString()` results use those names. This does not promise
names for arbitrary runtime-created values or release builds without reflection
metadata. Raw CODE output needs an explicit symbol source; it cannot allocate
an independent identity space.

`Symbol or Null` supports annotated parameters/results, local storage, and
Symbol/Null conditional joins from API 1.0.0. Forwarding and returns preserve the
declared union even when the current value is a Symbol. Local writes may replace
it with Symbol, Null, or the same union. `==`/`!=` with literal Null support fresh
local/parameter proofs in either operand order. Guarded reads may compare with
other Symbols, invoke methods, or act as `has`/`method` selectors. Existing
[null-proof rules](#nullable-results) apply across branches, short circuit
expressions, and loops; writes invalidate the local's proof. Copies and stored
Boolean guards do not transfer it. Nullable Symbols cannot satisfy a non-null
Symbol consumer without proof. Native `equals` arguments explicitly permitting
Null retain their shared argument profile. Method API requirements still apply
after a guard, including 2.3.0 for `toNumber`.

Symbol arithmetic, ordering, truthiness, and direct printing remain unsupported.
Classes follow the [Symbol field contract](compiler-classes.md#symbol-fields).
Method lookup follows its
separate contract below.
See the [specimen and binding evidence](../investigations/compiler-symbols.md).

### Symbol containers

Arrays support invariant Symbol and Symbol-or-Null elements from API 1.0.0.
Symbol Dictionary keys are non-null; values may be Symbol or Symbol-or-Null,
combined with other supported key/value contracts. An expected contract types
empty and nullable literals. Non-null Symbol literals can infer homogeneous
Array/Dictionary contracts; arbitrary scalar/Symbol mixtures do not infer a
broader union. Existing mutation, collection, Object observer, and enumeration
profiles apply. `reverse` and `slice` return independent Arrays. Symbol is not a
Comparable witness; sorting these Arrays remains outside the sorting profile.

Indexed reads, `get`, `keys`, and `values` preserve their declared types.
Dictionary lookup always adds missing-key Null; `values()` does not add it.
Snapshot a nullable element and guard the local before Symbol consumers. Repeated
indexed reads and `hasKey` do not establish a proof. Nullable key arguments need
a fresh non-null proof; nullable key contracts and nullable container references
remain unsupported.

Symbol-bearing literals use ordered private construction with symbolic `spush`
operands. Raw CODE resolves names through the supplied symbol source; complete
applications share the normal name/ID map, including pinned native member IDs.
No numeric Symbol ID is placed in semantic IR or treated as Number. Flat DATA
constants do not accept Symbol tokens. A Null-only payload under a nullable
Symbol element/value contract can still use the existing Null constant encoding.
Pooled Symbol encodings require separate evidence/contracts.

## Method values and invocation

Supported scalar, Array, Dictionary, opaque Object, WeakReference, and Method
values and supported native/source-class receivers support `method(Symbol)` →
non-null `Toybox.Lang.Method`, since API 1.0.0. Receiver class availability and
construction retain their own API requirements. Method lookup and missing-member errors remain runtime operations.
The Method retains its bound receiver; replacing the original variable does
not release that receiver while a Method still owns it.

Method supports annotated parameters/results, local storage/assignment,
matching conditional arms, and inherited `toString`/`equals` plus its own
`hashCode`, since 1.0.0. `weak()` returns WeakReference from 1.2.0. Native
observers retain runtime identity/hash behavior: repeated bindings need not
compare equal even for the same target/member.

`Method or Null` supports explicit parameter/result contracts, locals,
assignment, and conditional joins with Method or Null alternatives. Nullable
locals retain their declared union when assigned a Method or Null. Equality
and inequality with literal Null establish proofs for fresh reads of that
storage; invocation and every Method member require a non-null proof. Writes
invalidate local proofs; aliases and stored Boolean tests do not transfer them.
Releasing the last Method alias also releases its bound receiver. Lexical
`as Method or Null` is an unchecked type assertion, with no runtime conversion
or signature recovery. [Method fields](compiler-classes.md#method-fields) support
required/nullable storage. Arrays follow the [Method Array profile](#method-arrays);
Method Dictionary keys/values remain unsupported.

`invoke(arguments...)` accepts up to 254 supported Object/Null values and
returns opaque `Object or Null`. This erased result covers scalar/container
values, nullable returns, and Void callbacks. Save and guard it before Object
member calls; it cannot become a Number, typed container, or Method without a
separate refinement contract. Statement invocation discards the result.
Receiver evaluation precedes arguments, left to right, and each runs once.

The compiler checks the Method API and VM argument-count limit. The target's
existence, arity, accepted types, introduction level, and result behavior remain
runtime/caller obligations; dynamic lookup does not recover its signature.
Source instances support `instance.method(selector)` and instance bodies support
`self.method(selector)` or bare `method(selector)`. Bare calls resolve to the
current instance and use the same semantic operations as explicit self calls.
A declared source member named `method` keeps
ordinary source-call priority. Bare `self` produces opaque Object and requires
an instance body; it does not recover a source-class type or concrete callback
signature. Existing `self.member(...)` source calls retain checked signatures.
Bare `method(...)` requires an instance body; a local, parameter, or field named
`method` prevents implicit lookup. Calling such lexical values remains
unsupported; use explicit `self.method(...)` for the instance member. Other
unqualified function/member calls are outside this profile. Receiver capture
still requires all required instance fields to be initialized.

Unlike the SDK checker, this erased lookup profile does not statically resolve
a literal selector to a source target. A missing target or incompatible
invocation remains a runtime/caller obligation even for literal selectors.
Constructors and typed signatures remain deferred.
Array sorting uses a [separate Comparator contract](#source-comparators). See the
[binding/lifetime evidence](../investigations/compiler-methods.md).

### Method Arrays

`Array<Method>` and `Array<Method or Null>` preserve invariant element contracts
through parameters, results, locals, indexed reads/writes, and matching Array
conditional arms. The value types require API 1.0.0; native Array methods retain
their existing API requirements. A non-null-element Array cannot widen to a
nullable-element Array. Empty/nullable literals need a concrete expected contract;
otherwise a first non-null Method can establish the literal element type.

Method-bearing literals allocate privately and fill in source order at runtime.
They do not embed Method objects or addresses in DATA constants. Null-only
nullable literals can use the existing Null constant encoding. Each construction
produces a fresh mutable Array.

The supported `size`, `toString`, `add`, `addAll`, `indexOf`, `remove`, `removeAll`,
`reverse`, and `slice` profiles apply. Element arguments match the declared
nullability; `addAll` requires the same invariant Array contract. Reverse/slice
produce independent Arrays while retaining the same Method references. Search
and removal use runtime Method equality; separately created bindings need not
compare equal. Method sorting remains outside the supported ordering profiles.

Nullable indexed reads need guarded local snapshots before invocation or member
use; a test of one indexed read does not refine another. Replacing/removing an
element preserves saved aliases. Releasing the last Method reference releases
its bound receiver. Array storage does not recover a callback signature or a
concrete invocation result. See the [specimen](../../examples/compiler-methods/README.md)
and [ownership evidence](../investigations/compiler-methods.md#nullable-ownership).

## Ordinary weak references

Supported Arrays, opaque/typed Dictionaries, Method, and WeakReference receivers support
`weak()` → `Toybox.Lang.WeakReference`, since API 1.2.0. WeakReference is a
non-null value with `get()` → `Toybox.Lang.Object or Null` and `stillAlive()` →
Boolean, both since 1.2.0. It also inherits `toString`, `equals`, and `hashCode`;
calls retain runtime behavior rather than synthesizing wrapper identity.
There is no `clear()` method. Release a strong reference through assignment or
frame exit; the runtime determines when the target ceases to exist.

Source signatures accept `Toybox.Lang.WeakReference`, `Toybox.Lang.Object`, and
`Toybox.Lang.Object or Null`. They survive locals, function calls/results, saved
IR, and ordinary branch/loop null proofs. A retrieved Object is a strong value;
saving it can keep the target alive. Save and guard that value before calling
its supported Object methods (`toString`, `equals`, `hashCode`). Neither
`stillAlive()` nor testing a separate `get()` call proves a later result non-null.

Object contracts erase concrete type information without changing runtime
values. Compatible values may pass to Object parameters/results and replace
opaque Object locals; a nullable Object local also accepts Null. Erasure cannot
be reversed into a typed Array/Dictionary or a scalar. Object joins accept
matching opaque contracts and Object/Null alternatives; matching WeakReference
arms also join. These rules do not introduce heterogeneous container joins.

The declared Object type does not establish a WeakReference result from
`Object.weak()`: the runtime value might be an immutable scalar. That receiver
path, other native/source weak targets, nullable WeakReference contracts,
constructors, casts, and concrete recovery remain deferred. The new types do
not acquire arithmetic, truthiness, direct printing, or general equality
operators; nullable Object/Null tests use the existing proof rules.

Hidden reference temporaries transfer ownership on their final read using IR
`.take`; private container builders release their frame reference on finish.
Source locals retain their assignment/frame lifetimes. Backend staging must not
silently extend a target's strong lifetime. See the
[lifetime specimen and evidence](../investigations/compiler-weak-references.md).

## Char arrays

Nonempty Char-array literals accept Char-valued expressions, for example
`['a', 'b']`, `[65.toChar(), 'z']`, and `[Chars.make(), 'x']`. They produce a
fresh non-null `Array<Char>` and may receive postfix reads/calls. Number literals
follow the [Number-array contract](#number-arrays); View literals follow the
[native object contract](#native-views); String literals follow the
[String-array contract](#string-arrays). Other element types, mixed
types, nested arrays, and trailing commas are rejected. Empty literals follow
the [contextual construction rules](#empty-array-literals).

Homogeneous nonempty lists of Number, Char, or String tokens become typed IR
constants and DATA array records loaded with `apush`. Each execution creates a
fresh mutable array. Computed expressions are not folded into these records.

Other nonempty construction allocates the final length before evaluating elements
left to right, once each, and stores each result with VM array instructions.
A private builder prevents unfilled slots from escaping; branching elements
use a reusable staging local. Builder and ordinary slots share the normal frame
budget. Operation/nesting limits still apply. Allocation and store failures
remain runtime obligations. See
[literal evidence](../investigations/compiler-array-literals.md).

String `toCharArray()` returns a non-null `Array<Char>` since API 1.3.0. The
supported source annotation is `Toybox.Lang.Array<Toybox.Lang.Char>`, with ordinary
token whitespace permitted. Array parameters and results retain the element
contract; source array results require an explicit annotation. Locals may store,
copy, and reassign matching Char-array values, and user calls may pass/return
them. [Number elements](#number-arrays) have a separate supported profile.
Raw Array annotations, other element types, and nullable arrays are
unsupported. Element contracts are invariant; Char arrays are not widened to
mutable arrays of Object.

Char-array receivers support `size()` returning Number, `toString()` returning
String, `indexOf(Char)` returning Number, `add(Char)` returning `Array<Char>`,
`addAll(Array<Char>)` returning `Array<Char>`, `remove(Char)` and `removeAll(Char)`
returning Boolean, `reverse()` returning `Array<Char>`, and
`slice(Number or Null, Number or Null)` returning `Array<Char>`.
Search returns the first matching index or `-1` when
absent. Generic API parameters and results substitute the receiver's element
type; executable array values remain limited to Char elements. Array methods
dispatch through the Array class. Explicit context checks both the Array class
and Char element, requiring API 1.3.0 for this profile.

`add` appends one element; `addAll` appends the argument's elements. Both mutate
the receiver and return that same array. `remove` deletes the lowest-index match,
while `removeAll` deletes every match. Each returns true if it removed an
element, otherwise false. Remaining elements retain their relative order, and
aliases observe the mutation. Calls evaluate receiver before arguments, once
each; native mutation uses the existing ordered-call validation and lowering.
See [mutation evidence](../investigations/compiler-array-mutation.md).

`reverse` returns the elements in reverse order and leaves
the receiver's contents unchanged. `slice` uses a start-inclusive, end-exclusive
range. Null start/end select the beginning/end; negative bounds offset from the
end, and out-of-range bounds are clipped. A start after the end returns an empty
array.

Convert arrays to String before printing. Arrays cannot feed arithmetic,
concatenation, comparison operators, Boolean conditions, or
console entry results. Native Array constructors and ordinary weak targets
outside the bounded profile remain unsupported. Default sorting follows its
[separate contract](#default-array-sorting). Native Array
calls may be values or
statements that discard the result.
Guard nullable String receivers before conversion; that proof does not imply
support for nullable Array results. See the
[Char-array evidence](../investigations/compiler-char-arrays.md).

`array[index]` reads a Char from a non-null Char array. The index must be a
non-null Number; a fresh nullable Number read under an existing null proof is
accepted. Indexing and member calls share the postfix level and may interleave:
`array[0].toUpper()` and `text.toCharArray()[0]` are supported. Array evaluates
before index, once each. Logical subexpressions use the ordinary temporary
snapshot path to preserve this order across joins. The combined postfix/index
nesting limit is 64, and diagnostics retain the complete indexed-expression span.

The compiler emits `agetv`, with no clamping, negative-index adjustment, or
static bounds analysis. An index outside `0..array.size()` remains a runtime
obligation; the tested VM reports a fatal bounds error. Reads are order-sensitive
in IR validation even without other calls/writes, because failure order is
observable. Indexing a String itself and nullable Array receivers remain
unsupported; Dictionary indexing follows its [own contract](#dictionary-values).
See the
[array-read evidence](../investigations/compiler-array-reads.md).

## Array element writes

`array[index] = value;` replaces an element of a non-null supported array.
The index must be non-null Number; the value must match the invariant Number,
Char, String, or native View element contract. Guarded fresh nullable reads are
accepted for Number indices and Number/String values. View values may be native
or source View
instances; storing/reading through a View array erases exact source identity.
Null values, delegates, and drawing contexts cannot fill View slots. Receiver, index, and value evaluate once, in that order.
The assignment produces no value and does not change the array's length.
Aliases observe the changed element. Calls in the value expression may mutate
array contents or length before the write checks its index.

Lowering evaluates the three operands and emits `aputv`. Logical expressions
or array construction in any operand stage all three operands through hidden
locals in order; these locals use the ordinary frame budget. Plain operands
need no hidden slots. Bounds are a runtime obligation, with no clamping or
static range rejection. The tested VM reports a fatal bounds error after all
operand effects, including for an index equal to the length.

Nested arrays and assignments used as expressions remain unsupported.
Indexed compound targets follow the contract below. See [Number write evidence][array-writes] and
[Char/View write evidence](../investigations/compiler-typed-array-writes.md).

[array-writes]: ../investigations/compiler-array-writes.md

## Indexed compound assignment

`array[index] op= rhs;` supports Number/Long elements with all ten compound
operators, Float/Double elements with the four arithmetic operators, and
Char elements with `+= Number/Long`, and String elements with `+=` using the
shared String concatenation operands. Existing binary contracts and invariant
array element types apply; View elements have no supported compound operator.
The index must be non-null Number, including a fresh guarded nullable read.
String/dictionary receivers, nested arrays, and assignment expressions remain
unsupported.

Evaluate array and index once, in that order, then read the old element before
RHS effects. Evaluate RHS once, apply the binary operation, and write through
the captured array/index. Changes to contents or length during RHS evaluation
do not change the old value used by the operation. An invalid initial index
fails at the read before RHS effects; a RHS that shrinks the array can make the
final write fail after those effects. Both accesses retain runtime bounds
checks, without clamping or static range rejection.

Array and index are captured in two typed hidden locals. Plain RHS expressions
need no additional old-value/result slots. A branching RHS stages the old read
and result before defining fresh final write operands, keeping expression IDs
block-local. These temporaries share the ordinary frame budget; loop iterations
reuse their static slots. Saved IR uses existing locals, `.array-read`, binary
operations, and `.array-write`, with no compound directive.
See [target evidence][compound-targets].

[compound-targets]: ../investigations/compiler-compound-targets.md

## Number arrays

String `toUtf8Array()` returns a non-null `Array<Number>` since API 1.3.0.
Its initial elements are UTF-8 bytes. The Array element contract is Number,
so later appends need not stay within byte range. Source parameters/results use
`Toybox.Lang.Array<Toybox.Lang.Number>`; non-Number results require explicit
source annotations as usual.

Number arrays use the same storage, native-method, and evaluation-order rules
as Char arrays. `add`, `indexOf`, `remove`, and `removeAll` require Number;
`addAll` requires `Array<Number>`. `reverse` and `slice` retain `Array<Number>`.
Indexing produces Number, permitting ordinary arithmetic and native Number
calls. Number and Char arrays are invariant and cannot substitute for each
other; no implicit numeric/Char conversion is performed.

The Array/Number classes require API 1.0.0; conversion and the mutation/search/
transform methods above require 1.3.0. Context checking keeps signature-type
availability separate from method availability. Nullable String receivers need
proof before conversion. Nullable arrays and unsupported element types remain
rejected.

Nonempty literals such as `[1, 2]`, `[n, -n]`, and `[M.make(), 3]` produce a fresh
`Array<Number>`. The first element selects the supported element type from its
expression contract; every element must have that same non-null type. Parameters, locals,
user/native calls, arithmetic, and element reads can supply the first element.
Nullable values require the usual branch proof, including when used first.
Allocation precedes element effects; evaluation and typed stores proceed left
to right as for Char literals. Construction requires Array/Number availability
(1.0.0), without an incidental native `add` requirement. Element calls and
other Array methods retain their own API constraints.
Empty construction uses [contextual types](#empty-array-literals). See the
[literal evidence][number-array-literals].

Arrays must be converted to String before
printing, and cannot be console entry results. See the
[Number-array evidence](../investigations/compiler-number-arrays.md).

[number-array-literals]: ../investigations/compiler-number-array-literals.md

## Numeric arrays

`Array<Long>`, `Array<Float>`, and `Array<Double>` use fully qualified scalar
names in source and are invariant. Parameters/results, locals, reads/writes,
empty construction, and the supported generic Array methods preserve the exact
element contract. No Number/Long/Float/Double substitution or implicit widening
is permitted at array boundaries. Indexes remain non-null Number.

Each element also supports `or Null` under the shared nullable-element rules.
Guard a saved nullable read before scalar use; guarding one read does not narrow
a later read of the same array. Non-null arrays reject Null values/arguments.

Literal-only homogeneous arrays use typed DATA constants, preserving signed
Long and floating bit patterns. Nullable constants may contain Null. Computed
literals use ordered private construction and may include branching elements;
expected contracts reach empty and nullable literals. Repeated constant loads
produce independent mutable arrays.

`size`, `toString`, `indexOf`, `add`, `addAll`, `remove`, `removeAll`, `reverse`,
and `slice` use the existing generic Array profile. `addAll` takes an invariant
Array of the same element contract; `removeAll` takes an element. Native search
and removal semantics remain runtime supplied. Allocation, indexing, `size`,
and `toString` require API 1.0.0; the other listed methods require 1.3.0.

Non-null indexed updates/compounds follow scalar operator promotion and must
retain the element type. For example, `Float += Number` is valid but
`Float += Long` would produce Double and is rejected. Nullable elements cannot
be compound targets without a supported direct non-null read contract.
Non-null elements support [default sorting](#default-array-sorting) and
[source comparators](#source-comparators). Broader callback consumers, nested
containers, nullable Array references, and whole-array expression joins outside
the existing Number/Char profile remain deferred. See the [example](../../examples/compiler-numeric-arrays/README.md)
and [record/runtime evidence](../investigations/compiler-numeric-arrays.md).

## String arrays

`Toybox.Lang.Array<Toybox.Lang.String>` is an invariant non-null String-element
contract. Parameters/results preserve that type; indexed reads produce String
and writes require String. Nullable source values require a non-null proof
before construction, writes, or native arguments. Mixed types, Null elements,
nested arrays, and nullable Array receivers remain unsupported for this profile.
Nullable String elements have a [separate contract](#nullable-array-elements).

Homogeneous literals such as `["one", "two"]` use pooled DATA constants.
Repeated text shares a local pool entry; repeated execution loads independently
mutable arrays. Computed literals, including conditional and guarded elements,
use the same ordered private construction as Char/Number arrays. An expected
String-array contract supplies the type of a direct `[]`.

`size`, `toString`, `indexOf`, `add`, `addAll`, `remove`, `removeAll`, `reverse`,
and `slice` use the existing Array contracts with String substituted for the
element type. Search/removal compare String text. Array/String types,
construction, indexing, `size`, and `toString` require API 1.0.0; the other
methods require 1.3.0. [Default sorting](#default-array-sorting) requires 5.0.0.
Source comparators follow their [separate contract](#source-comparators);
broader callback consumers remain deferred.
Arrays must be converted to String before printing.

See the [example](../../examples/compiler-string-arrays/README.md) and
[runtime evidence](../investigations/compiler-string-arrays.md).

## Boolean arrays

`Toybox.Lang.Array<Toybox.Lang.Boolean>` is an invariant non-null Boolean-element
contract. Literals infer Boolean; reads produce Boolean for conditions, logical
operators, and native Boolean methods. Writes and element-taking Array methods
require Boolean, with no Number conversion. Null elements remain unsupported.
Typed parameters/results and direct contextual `[]` preserve the contract.

Literal-only arrays use `apush` constants; computed elements use ordered private
construction, including conditional/short-circuit expressions. Copies share
mutation; repeated constant loads are independently mutable. The supported
Array methods and API levels match the String/Number profile, including
[default sorting](#default-array-sorting) from 5.0.0. Source comparators follow their
[separate contract](#source-comparators); broader callback consumers remain deferred. See the
[example](../../examples/compiler-boolean-arrays/README.md) and
[evidence](../investigations/compiler-boolean-arrays.md).

## Default Array sorting

`array.sort(null)` sorts a non-null Array of Number, Long, Float, Double,
Boolean, Char, or String elements in ascending order. It mutates the receiver,
so aliases observe the new order; separately created copies retain their own
contents. Empty/singleton Arrays are accepted, and duplicates are retained.
The invariant element contract does not change. Sorting requires API 5.0.0.

The result is Void: use a call statement, not an initializer, return expression,
or chained value. Evaluate the receiver once before the Null argument, then
invoke the runtime method and discard its VM result slot. The compiler does
not implement a sorting algorithm or promise sort stability, NaN ordering,
locale-sensitive String collation, or atomic behavior if comparison fails.

Garmin's full parameter type is `Lang.Comparator or Null`. Comparator is a
structural interface requiring `compare(Object, Object) as Number`; it is not
Method or an instantiable native class. Null selects default sorting; supported
source comparator instances follow the contract below. Non-null
[Comparable elements](#comparison-interface-arrays) also support default sorting.
Nullable/general Object elements remain unsupported, even if current contents
happen to be comparable.

See the [example](../../examples/compiler-array-sort/README.md) and
[evidence](../investigations/compiler-array-sort.md).

## Source comparators

`array.sort(comparator)` accepts an instance of a declared source class that
supplies `compare(a as Toybox.Lang.Object, b as Toybox.Lang.Object) as
Toybox.Lang.Number`. Both parameter types, arity, and result must exactly match
the interface declaration. Narrower scalar parameters, nullable Object
parameters/results, and missing/extra parameters are rejected. Untyped source
parameters still infer Number and therefore do not establish this interface.
Matching is structural: no Comparator superclass or `implements` syntax is used.

Supported receivers are the scalar-element Arrays above and non-null
[comparison-interface Arrays](#comparison-interface-arrays), from API 5.0.0.
A negative/zero/positive Number orders the first element before,
equal to, or after the second. The runtime calls the source `compare` body on
the supplied instance, preserving its fields. It controls comparison count and
order; callers must supply a consistent ordering and must not depend on a
particular sorting algorithm. Custom comparators may choose an order different
from the default, including ordering Object string conversions.

Construction, aliases, exact source-class parameters/results, and source
reference fields can supply a comparator while retaining class identity.
Whole-unit validation resolves the class and rechecks the required method;
an erased View/Object or Method cannot establish the structural contract.
The [comparison-interface profile](#comparison-interface-values) also accepts
Comparator parameters/results and locals, including Comparator-or-Null values.
Bare `self` as a comparator remains unsupported. Object-rooted source classes
support construction, exact source signatures/reference fields, and comparator use
through their [class contract](compiler-classes.md#object-rooted-classes).
The existing source class/base and constructor requirements still apply.

Sort uses ordinary native-call lowering with receiver-before-argument order.
The runtime callback member `compare` uses its pinned symbol ID; it remains a
source method body. Constructor checks propagate known comparator shutdown
requests through sort and source wrappers, rejecting them while reference fields
are pending. These checks do not prove callback termination, exception safety,
or comparator consistency.

See the [example](../../examples/compiler-comparators/README.md) and
[evidence](../investigations/compiler-comparators.md).

## Comparison interface values

`Toybox.Lang.Comparable` and `Toybox.Lang.Comparator`, optionally `or Null`, are
supported parameter and result contracts from API 5.0.0. Module aliases and
imports resolve them like other API types. Comparable exposes `compareTo(Object)
as Number`; Comparator exposes `compare(Object, Object) as Number`. Calls use
ordinary receiver/member dispatch, evaluate receiver before arguments once, and
preserve the implementing instance. Comparator values can also supply Array
`sort`.

User calls, returns, and interface-local assignments accept the same interface,
a verified native scalar witness, or an exact source-class witness. Unit
validation rechecks every required source member, arity, parameter, and result
contract. Opaque Object/View values, a `has` test, and source classes with
narrowed parameters do not establish the interface. A standalone function can
retain unresolved source witnesses; assembling a complete unit requires proof.

Number, Long, Float, Double, Boolean, Char, and String satisfy Comparable. The
bounded native witness check requires the catalogue's exact method signature and
an effective introduction level no later than Comparable's API 5.0.0
requirement. Checking the receiving interface therefore covers these native
members as well. No runtime conversion, allocation, or wrapper is added. Other
native types do not establish an interface witness. A nullable scalar needs a
non-null/concrete proof before non-null Comparable use; a positive concrete
class test on an Object read can likewise establish a scalar witness. Comparison
still follows the receiver's runtime method; a common interface does not
guarantee that arbitrary operand domains can be ordered or that a call cannot
fail.

An interface parameter or call result retains its contract through inferred
locals, aliases, and same-interface assignment and conditional joins. A
Comparable local also accepts a verified scalar assignment. A conditional with a
Comparable arm and a verified scalar arm retains Comparable in either arm order;
two different scalar types do not implicitly infer an interface. There is no
implicit conversion between Comparable and Comparator. Direct source assignments
defer their signature proof to unit validation, including after IR reload; an
incompatible source class cannot enter either interface local.

A nullable interface accepts Null and compatible non-null values. Nullable
scalars can widen to Comparable-or-Null. Nullable locals keep their declared
contract across writes and require a fresh non-null read proof before member
calls or non-null forwarding. Equality/inequality with Null works in either
operand order. The shared branch rules cover early returns, conditional arms,
and loops; writes invalidate local proofs, and aliases/stored Boolean guards do
not transfer them. A conditional combining an interface and Null produces the
corresponding nullable interface. Existing nullable-interface arms can also
accept compatible scalar/interface alternatives; this does not enable general
union inference.

`Array.sort` accepts Comparator-or-Null directly: Null selects default ordering,
and a non-null instance supplies custom ordering. This does not permit calling
`compare` on a nullable receiver without a non-null proof.

Source classes also support [comparison-interface fields](compiler-classes.md#comparison-interface-fields)
with the same write contracts and checked initialization. Arrays follow the
[interface element contract](#comparison-interface-arrays). Dictionaries follow
their [interface value contract](#comparison-interface-dictionaries).
Unchecked `as` assertions and interface runtime type targets remain
unsupported. A non-null interface can widen to
Object without proving additional members. A nullable interface can widen to
Object-or-Null; Object requires a non-null proof. Ordinary Object casts and
concrete type tests retain their existing rules.

Erased comparison calls, including sorting through a Comparator value, may
request shutdown. Constructor checks conservatively propagate that effect
through source wrappers and require reference fields to be initialized first.
This does not establish exception safety or callback termination.

## Comparison interface Arrays

Non-null Arrays support Comparable and Comparator elements, optionally `or Null`,
from API 5.0.0. Parameter/result annotations use `Array<Toybox.Lang.Comparable>`
and the corresponding Comparator/nullable forms. Array contracts stay invariant:
`Array<Number>` cannot widen to `Array<Comparable>`, and non-null element Arrays
cannot widen to nullable-element Arrays.

An expected Array annotation or argument contract supplies literal element
typing, including empty and mixed native/source literals. Otherwise the first
element must already establish a supported element contract; arbitrary mixtures
do not infer an interface. Scalar-token literals use tagged DATA constants,
preserving Number/Char distinctions and each native runtime tag. Nullable
contracts also permit Null tokens, including Null-only literals. Each evaluation
produces a fresh mutable Array. Source-object and computed literals allocate
privately before evaluating and filling elements in order. The same declared
element contract applies to either construction path.

Indexed reads retain the declared interface, including Null where allowed;
they do not recover an exact source class. Member calls on nullable elements
require a guarded local snapshot. A test of one indexed read does not refine
another read. Writes, private literal fills, and typed native element arguments
accept compatible native/interface values or an exact source witness. Unit
validation checks required source members at every such ingress after IR reload.
Opaque Object values and incompatible signatures cannot bypass that proof.

The existing `size`, `toString`, `add`, `addAll`, `indexOf`, `remove`, `removeAll`,
`reverse`, and `slice` profiles apply; reverse/slice preserve the element type.
`addAll` requires the same invariant Array contract, though a direct argument
literal can receive that context. Inherited Object methods retain their existing
profiles. Collection operations that may call erased element methods
(`toString`, lookup, and removal) conservatively propagate possible shutdown
effects through constructor checking.

`Array<Comparable>.sort` accepts Null, a checked exact source Comparator, or a
Comparator(-or-Null) value. Null dispatches through element `compareTo` methods;
an explicit Comparator supplies `compare` instead. `Array<Comparator>.sort`
requires a non-null Comparator: its element contract alone does not prove
`compareTo` for default ordering. A guarded nullable Comparator snapshot can
satisfy this requirement. Exact source arguments retain their non-null proof
while unit checking verifies their comparison signature.

Callers must provide mutually comparable elements or a callback that handles
every element pair. The interface proves a signature, not a common comparison
domain: native scalar witnesses and unrelated source implementations need not
accept each other. Sorting mutates shared contents and retains the invariant
element type. Erased element/callback calls conservatively propagate possible
shutdown effects through constructor checking, including default sorting.
Runtime comparison failures, sort stability, and callback count/order are not
guaranteed. Existing [sort evaluation/lowering](#default-array-sorting) applies.

Sorting nullable elements, nested/general Object containers, and Array field
declarations remain outside this profile.
Comparator values read from an Array can supply an ordering callback. See the
[example](../../examples/compiler-comparators/README.md) and
[evidence](../investigations/compiler-comparators.md#interface-arrays).

## Comparison interface Dictionaries

`Dictionary<K, Comparable>` and `Dictionary<K, Comparator>`, optionally `or Null`
on the value type, support scalar keys and the interface-key profile below. Interfaces
require API 5.0.0. Key/value parameters stay invariant across assignments and
calls; scalar-valued Dictionaries do not widen to interface-valued ones.
General Object values remain unsupported.

An expected Dictionary contract types direct literals and empty construction.
Values accept compatible native/interface witnesses or exact source instances;
unit validation checks required source members for every private literal fill,
indexed write, and `put` argument, including after IR reload. Opaque Object
values do not establish an interface. Computed literals retain ordered evaluation
and private construction. Scalar-only literals can use the existing Dictionary
constant payload, which distinguishes Number and Char tags under an erased
Comparable contract. Implicit inference of interface-valued literals is deferred.

Lookup always produces the value type or Null, including for a non-null value
contract. Calls require a guarded local snapshot; `hasKey` or a guard on another
read does not refine the lookup. Indexed writes and `put` accept Null only for
nullable value contracts. `values()` returns `Array<V>`, preserving value
nullability without adding lookup's missing-key Null. `keys()` retains the
key contract. Enumeration snapshots membership, sharing element references
under the existing [Dictionary rules](#dictionary-values).

Other supported Dictionary methods retain their profiles. Formatting may call
erased element methods, so constructor checking treats `toString` as possibly
requesting shutdown while required reference fields are pending. Opaque
Dictionary/Object string conversions retain this conservative effect: erasing
the container must not bypass initialization checks. No guarantees about callback
sequence, exception recovery, or element string representations follow.

See the [example](../../examples/compiler-comparators/README.md) and
[evidence](../investigations/compiler-comparators.md#interface-dictionaries).

### Interface Dictionary keys

`Dictionary<Comparable, V>` and `Dictionary<Comparator, V>` support non-null
interface keys from API 5.0.0, combined with any supported scalar/interface value
contract. Container parameters remain invariant. `keys()` produces an Array of
the declared interface; lookup's result type and missing-key Null are unchanged.
Nullable or general Object key contracts remain unsupported. A nullable key
argument requires a fresh non-null proof.

Every literal fill, indexed read/write, and native `get`, `put`, `hasKey`, or
`remove` argument requires an interface witness. Unit validation checks exact
source members after IR reload as well as source compilation. Key identity uses
runtime hashing/equality, not `compareTo` or `compare`. Source instances may use
inherited identity or compatible [Object overrides](compiler-classes.md#object-rooted-classes).
Callers must keep key hashes stable while stored and make equal keys hash
consistently; signature checking does not establish these behavioral laws.

Interface-key literals use private runtime construction and ordered insertion.
They do not become Dictionary DATA constants, even with scalar-token contents;
saved IR rejects `.dictionary-constant` with interface keys. All-scalar-token
literals require a single key token kind. Mixed scalar-token literals are
rejected because SDK constant construction can conflate Number and Char keys
that runtime insertion distinguishes. Heterogeneous native witnesses can enter
through explicit `put` calls or computed/runtime key operands. No compile-time
key normalization or deduplication is applied to this path.

Construction fills and key lookups/writes/removal may invoke erased key methods.
Constructor checking conservatively propagates possible shutdown through those
operations and source wrappers while reference fields are pending. Formatting
may also invoke key methods. Enumeration does not itself compare keys.
Callback count/order, mutation during callbacks, and exception recovery are
not guaranteed. See [evidence](../investigations/compiler-comparators.md#interface-keys).

## Nullable Array elements

`Toybox.Lang.Array<T or Null>` supports numeric, String, Boolean, Char, or Symbol T
with invariant element contracts and a non-null Array reference. Indexed reads produce the base type or Null. Stores and native
`add`, `indexOf`, `remove`, and `removeAll` accept either alternative; `addAll`
requires the same Array contract. `reverse` and `slice` retain it. API levels
match the base-element Array profile.

Direct literals acquire a nullable element contract from an annotated return,
a typed user/native argument, or assignment to an existing nullable-element
Array local. This includes empty, Null-only, scalar-only, and computed literals.
For example, a function returning `Array<Number or Null>` may return `[1, null]`.
Literal tokens use typed DATA constants; other expressions allocate before
ordered fills. Branching nullable literals use a separate staging local per
element so differing operand types do not change an existing local's contract.

Existing Array references cannot widen or narrow their element contracts.
Unannotated literals retain the non-null inference rules; `[null]` and
`[1, null]` without nullable context remain rejected. Nested arrays and nullable
Array receivers remain deferred.

Store a nullable read in a local and guard a fresh local read before scalar
operators, methods, or printing. A present false Boolean is distinct from Null. Writes and `a[index] != null` do not prove that a
later indexed read is non-null. A scalar saved before Array mutation retains
its earlier value. Reassigning that scalar invalidates its null proof.
See the [example](../../examples/compiler-nullable-arrays/README.md) and
[evidence](../investigations/compiler-nullable-arrays.md).

## Empty array literals

A direct `[]` requires a concrete expected `Array<Number>`, `Array<Char>`,
`Array<String>`, `Array<Boolean>`, `Array<Symbol>`, `Array<View>`, or a supported
[nullable element contract](#nullable-array-elements).
Parentheses preserve direct context. Supported contexts are:

- An explicitly annotated array return: `return [];`.
- A typed user-call argument, including module, cross-file, and self calls.
- Assignment to an initialized array local: `var a = [1]; a = [];`.
- A native argument whose resolved contract is an array, such as `a.addAll([])`.

The literal allocates a fresh empty array at its position in evaluation order.
Reassignment replaces the local reference; existing aliases retain the old
array. No `add` call or element-storage temporary is emitted. Empty Number/String/View
allocation requires API 1.0.0; Char element availability requires 1.3.0.
Enclosing calls retain their own API requirements.

Context applies only to the direct empty literal, not through arbitrary
operators or calls. `var a = []`, `[].size()`, and `return [].reverse()` are
rejected, including the last case under an array return annotation. Local type
annotations, nested arrays, and inference from later mutations are unsupported.
Nonempty literals keep their independently inferred contract; argument/return
context cannot coerce mixed elements or substitute Number and Char arrays.
Saved IR records the concrete allocation type and rechecks all uses. See the
[complete example][empty-example] and [evidence][empty-evidence].

[empty-example]: ../../examples/compiler-empty-arrays/README.md
[empty-evidence]: ../investigations/compiler-empty-arrays.md

## Native delegates

`new Toybox.WatchUi.BehaviorDelegate()` constructs a non-null delegate with no
arguments (API 1.0.0). Recognized WatchUi aliases and imported bare names work
as for View construction. Canonical BehaviorDelegate parameter/result types
are supported; delegate arrays and nullable delegates are not. Source subclasses
and the View/delegate callback result follow the
[class profile](compiler-classes.md#source-behavior-delegate).

## Native Views

`new Toybox.WatchUi.View()` constructs a non-null native View with no arguments.
`new WatchUi.View()` is also accepted; a `using Toybox.WatchUi as Ui` binding
permits `new Ui.View()`, while `import Toybox.WatchUi` permits `new View()`.
Constructor qualifiers cannot be shadowed by locals/parameters. Native View
constructor arguments and bare unimported `View` are unsupported. Unit mode permits
[declared source View construction](compiler-classes.md#source-views-and-drawing).
The initializer requires API 2.1.0, independently of the
class's API 1.0.0 availability.

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
profile. Search/removal evidence covers repeated references to the same View;
it does not specify equality for arbitrary subclasses or distinct objects.
Signature and empty-allocation availability is 1.0.0; append/search/transform
methods require 1.3.0. Constructing a literal's View elements additionally
requires the initializer's 2.1.0 API level.

Source [getInitialView overrides][initial-view-callback] return a validated
one-view positional result. A generic View array may contain zero or many Views
and cannot replace that shape. Delegate pairs remain planned.
See the [complete example][native-views-example] and
[constructor/runtime evidence][native-views-evidence].

[native-views-example]: ../../examples/compiler-native-views/README.md
[native-views-evidence]: ../investigations/compiler-native-views.md
[initial-view-callback]: compiler-classes.md#source-initial-view-callback

## Drawing context

View.onUpdate inherits one non-null `Toybox.Graphics.Dc` parameter. Dc references
may be copied into locals and passed through explicitly typed parameters. They
cannot be constructed, printed, used in scalar operations, or stored in arrays.
The accepted member statements return Void:

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
`Toybox.Graphics.FONT_MEDIUM`, and explicit module aliases are accepted. Qualifiers
cannot be shadowed by parameters/locals. A plain Number, including literal 3,
does not satisfy the font argument. Other Graphics constants and general enum
declarations are unsupported. See [evidence](../investigations/compiler-visible-view.md).

## Nullable results


String `find(String)` and `toNumber()` are available since API 1.0.0 and return
Number or Null; `toNumberWithBase(Number)` has the same result since 1.4.1.
String `toLong()`/`toLongWithBase(Number)` return Long or Null since 3.1.0.
String `substring` and user calls may also return nullable values. Compare
nullable parameters or stored results with null.
A direct `if (value == null)` or `if (null == value)` proves that parameter or
local non-null on the false path. Direct `value != null` or `null != value`
proves non-null on the true path. Boolean negation reverses this proof edge;
`if (!(value == null))` proves non-null on the true path. Fresh reads there have
the non-null numeric, String, Boolean, Char, or Symbol type
and support its consumers, including ordinary user arguments/returns. For example:

```monkeyc
function parse(text as Toybox.Lang.String) as Toybox.Lang.Number {
    var value = text.toNumber();
    if (value == null) { return 0; }
    return value + 1;
}
```

A join retains a proof only if all incoming paths provide it. The false exit of
`while (value == null)` also provides a proof, unless another exit such as
`break` lacks it. Assignments to that local invalidate the proof, including
assignments of non-null values. Nullable locals accept Null, their non-null
type, or the same union; their declared storage type stays nullable.

Only direct parameter/local-read comparisons with literal null used by the
branch establish facts. A stored Boolean condition, a null-valued local used
as the comparison operand, or a check of a different storage identity does not
narrow the target. Boolean negation chains around the direct comparison reverse
its edge once per negation; storing the Boolean result loses the proof relation.
Checking one local or parameter does not narrow aliases, copies, or later
native-call results. Unchecked nullable values cannot be printed or used as non-null operands/receivers. They may be
passed to matching nullable parameters and returned under matching nullable
result contracts. Scalar parameters/results still require a non-null value.
Fresh parameter reads on the non-null path use the scalar type directly.
Parameter slots cannot be assigned in this subset. Local writes do not
invalidate parameter facts, even when their numeric local/parameter IDs match.
Other unions remain outside this profile. Conversion, search, and slicing
behavior is supplied by the runtime, not folded by the compiler. See
[Number evidence](../investigations/compiler-nullable-numbers.md) and
[substring evidence](../investigations/compiler-substring.md).

`substring` takes zero-based start and exclusive end indices. Number indices
are accepted from API 1.0.0. Null or Number-or-Null indices require target and
minimum API at least 3.3.2 when an API context is supplied: Null selects the
respective beginning/end boundary. Negative indices are relative to the end
from API 3.3.2. Numeric ranges are not analyzed, so callers using negative
values must select a compatible minimum themselves. No API context means these
compatibility checks are omitted. Index normalization/error behavior remains
a runtime obligation; the declared result remains nullable.

[string-group]: ../investigations/compiler-string-methods.md

[weak-evidence]: ../investigations/compiler-immutable-weak.md

[products]: ../investigations/compiler-number-products.md

[numeric-boundaries]: ../investigations/compiler-numeric-boundaries.md

[comparisons]: ../investigations/compiler-number-comparisons.md
