# Supported Monkey C compiler subset

Restricted frontend. Design:
[small semantic IR](../design/compiler-ir.md). Runtime evidence:
[constant returns](../investigations/first-compiler.md),
[arithmetic](../investigations/compiler-arithmetic.md),
[locals](../investigations/compiler-locals.md),
[module calls](../investigations/compiler-calls.md),
[parameters/arguments](../investigations/compiler-arguments.md), and
[strings/printing](../investigations/compiler-strings.md).

Number/String/Boolean/Char/null restrictions describe the current implementation.
They do not define separate primitive types for the full Monkey C language.
The [core type/API model](../design/api-declarations.md) connects values to
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
value-type = "Toybox.Lang.Number" | "Toybox.Lang.String"
           | "Toybox.Lang.Boolean" | "Toybox.Lang.Char"
           | "Toybox.Lang.Array" "<" "Toybox.Lang.Char" ">"
result-type = value-type | "Void"
argument-list = expression ("," expression)*
statement = "var" identifier "=" expression ";"
          | identifier "=" expression ";"
          | "System" "." "println" "(" expression ")" ";"
          | module "." member "(" argument-list? ")" ";"
          | "return" expression? ";"
          | "if" "(" expression ")" block ("else" block)?
          | "while" "(" expression ")" block
          | "break" ";" | "continue" ";"
expression = logical-or
logical-or = logical-and ("||" logical-and)*
logical-and = comparison ("&&" comparison)*
comparison = arithmetic (("==" | ">") arithmetic)?
arithmetic = unary (("+" | "-") unary)*
unary = primary | "-" primary | "!" unary
primary = atom ("." member "(" argument-list? ")" | "[" expression "]")*
atom = signed-decimal-integer | string-literal | char-literal | "true" | "false" | "null"
     | parameter-or-local-identifier
     | module "." member "(" argument-list? ")" | "(" expression ")"
```

Identifiers use ASCII letters/underscore followed by ASCII
letters/digits/underscore. `function`, `module`, `return`, `var`, `if`, `else`,
`true`, `while`, `break`, `continue`, `class`, `extends`, `self`, `false`,
`using`, `import`, `as`, and `null`
are reserved. `true` and `false` produce Boolean literals. Whitespace and
`//` or non-nested `/* ... */` comments are supported. The integer literal domain
is the signed 32-bit range `-2147483648..=2147483647`. Decimal negative literals
are folded into that domain, with ordinary token whitespace/comments permitted.
Unary Number negation also accepts parameters, locals, calls, and grouped
expressions. Positive signs, hex/underscore/suffix forms, other numeric types, unsupported
statements/additional functions in default mode, and other operators are
rejected. A negative literal must be parenthesized before a member call:
`(-1).abs()` is accepted, while `-1.abs()` remains rejected because of an SDK
literal-postfix discrepancy. Repeated negation requires parentheses, as in
`-(-value)`; adjacent `--` is rejected as unsupported decrement.
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

The IR contains integer/string/Boolean/Char constants, binary add/subtract
definitions,
parameter reads, local initialization/read, module calls, print statements, and
comparisons, branches, and typed value returns using value IDs, with optional
source spans. Source
compilation supplies them; IR text may omit them. Without locals, lowering emits
a label from the function name, `argc <parameter-count + 1>`, `ipushz` for zero
or the smallest signed `ipush1`/`ipush2`/`ipush3`/`ipush4` for another Number,
left/right operands followed by
`addv` or `subv` for binary definitions, then `return`. The frame count follows
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
[shared assembly model](../design/assembly-model.md) directly. Existing
assembler/signing commands consume the output after explicitly integrating it
into an application section project.

The compiler CLI supports `--from source|ir|assembly` and `--emit
ir|assembly|code`. The default two-path invocation still emits assembly from
source. CODE output is a raw payload, not a PRG. See
[pipeline commands and limits](compiler-ir-text.md#command-line-stages).

## Arithmetic and lowering contract

The source subset supports binary `+` and `-` with equal precedence and left
associativity, and parentheses for grouping. Decimal literals cover the signed
32-bit range. Unary minus binds more tightly than binary arithmetic and after
postfix calls: `-value.abs()` means `-(value.abs())`; `(-value).abs()` calls
`abs` on the negated value. Boolean `!` uses the same precedence, supports
repeated prefixes (`!!value`), and binds before comparisons:
`!a == b` means `(!a) == b`. Unary plus, other operators, and general
object calls remain unsupported; the bounded native calls below are separate. Parse nesting is
bounded at 64; the semantic function is limited to 1024 operations including
return. These are implementation limits.

### Operator contracts

Validation resolves operand API types to these result contracts:

| Operands | Operators | Result |
|---|---|---|
| Number / Number | `+`, `-` | Number |
| Number | unary `-` | Number |
| Boolean | unary `!` | Boolean |
| Char / Number, Number / Char | `+` | Char |
| Char / Char | `+` | String |
| Exact String / supported scalar or null, either order | `+` | String |
| Number / Number, Char / Char, Number / Char, Char / Number | `==`, `>` | Boolean |
| Boolean / Boolean | `==` | Boolean |
| Exact String / String, Number, Char, or Null, either order | `==` | Boolean |

Mixed Char/Number addition changes the numeric character value, including
negative Number offsets. Char + Char concatenates the two characters.
Comparison uses numeric values: `'A' == 65` is true on the validated profile,
while `'A'.equals(65)` is false. Subtraction involving Char is unsupported.
With one exact String operand, concatenation accepts Number, String, Boolean,
Char, Null, or nullable Number/String, returning String. The VM supplies scalar
conversion and formats Null as `null`. Equality accepts the same profile except
Boolean, using String identity: copied references compare true, while separately
created equal-content strings compare false. Use `.equals()` for content.
Nullable alternatives are accepted only with an exact String on the other side;
this does not establish a non-null proof for subsequent use. String ordering,
subtraction, and other object/container operands are unsupported. See the
[String operator evidence](../investigations/compiler-string-operators.md).
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

Logical negation requires a non-null Boolean and produces Boolean. It evaluates
its operand once. Value-producing negation lowers to the operand, `bfpush`,
then `eq`; condition negation reverses the true/false destinations. Other types
are rejected without truth-value conversion. Postfix calls bind first:
`!value.equals(1)` inverts the method result, while `(!value).equals(1)` calls
the method on the inverted value. See the
[Boolean negation evidence](../investigations/compiler-boolean-negation.md).

Semantic IR contains typed add/subtract definitions with explicit
left/right value IDs. Each initializer/return consumes one expression tree:
definitions are unique, operands refer to earlier definitions, every defined
value has exactly one use, and initialization statements, branches, prints, or
returns consume expression results within the same block. General shared-value
graphs and unused definitions remain rejected until their scheduling policy is
implemented. For pure-only functions, definition order need not be VM stack
order: the backend traverses each initializer/return expression and emits left
operand, right operand, then
`addv`/`subv`, preserving subtraction order. No constant folding is performed.
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

## Function type contracts

Ordinary source parameters default to Number. `as` annotations accept the fully
qualified names `Toybox.Lang.Number`, `Toybox.Lang.String`,
`Toybox.Lang.Boolean`, and `Toybox.Lang.Char`; result annotations also accept
`Void`. Short names, module aliases in annotations, and general unions are
outside this subset.
Number and String annotations additionally accept the exact suffix `or Null`
for both parameters and results. Nullable Boolean/Char, reversed alternatives,
parenthesized unions, and other type syntax remain unsupported. These
declarations require no `using` statement.

Unannotated functions with value returns require Number results; those with no
value returns have a Void contract. String, Boolean, Char, and nullable source
results require explicit annotations. Each return must be assignable to its
contract:
a nullable result accepts its scalar type, Null, or the same union. Unrelated
types, value/void mixtures, and missing returns are errors. Typed IR constructors
infer uniform scalar/Void returns unless an explicit signature is supplied.
Native callbacks retain
their independently resolved signatures; annotations cannot override them.

## User module calls

The supported call form is `<module>.<member>(<arguments>?)` in expressions
and initializers. The module is a top-level user module provided by the
surrounding PRG scaffold. Value calls return supported scalar or nullable types.
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
initializations and accept one Number, String, Boolean, or Char value. String types
propagate through locals and annotated function parameters/results; incompatible
uses are rejected. Addition follows the operator table above, including Char
concatenation and supported scalar conversions. Arbitrary
SDK calls, and using
`System.println` as an expression remain unsupported. A local/parameter named
`System` cannot be used as the print receiver. The function's scaffold must
provide the SDK module. The native name profile below resolves its supported
spellings; Jungle resolution remains unimplemented.

## Short-circuit expressions

Boolean `&&` and `||` support conditions and value-producing expressions,
including grouping and negation. Comparison binds before `&&`, which binds
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
native-module = "Toybox.Application" | "Toybox.System"
```

`using` introduces a module binding, optionally aliased. `import` introduces
the module name and its supported class names: currently only Application's
`AppBase`. Therefore `import Toybox.Application;` permits `extends AppBase`,
while `using Toybox.Application;` requires `extends Application.AppBase`.
Aliased modules permit `extends App.AppBase` and `Sys.println(value)`.
Fully qualified `Toybox.Application.AppBase` and `Toybox.System.println(value)`
are also accepted. Existing `Application.AppBase` and `System.println` spellings
remain accepted without declarations.

Bindings apply to following declarations in their file and are inherited by
class/module bodies. Owner-local bindings do not escape their owner; bindings
are not shared between files, including reopened source modules. Imports in
the body cannot resolve that class's superclass clause. This is a bounded
frontend profile, not a specification of all Monkey C namespace behavior.

Reject duplicate bindings, shadowing by parameters/locals/functions/owners,
aliases of reserved identities, import aliases, declarations after functions,
and native modules outside this profile. General source namespace aliases and
general native type/member lookup remain unsupported. Number conversion uses
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

A function may end with `if (<condition>) { return <integer-expression>; } else
{ return <integer-expression>; }`. Braces are required; `else` is optional.
An omitted false arm continues to the next statement, which must eventually
return. Arms may also contain local declarations, print statements, and nested
conditions, as
described below. Loop syntax is specified separately below.

`==` and `>` compare Number/Char expressions and produce Booleans; `==` also
accepts Boolean pairs and the String identity profile above. Dictionary/null
equality follows the callback rules below.
Arithmetic has higher precedence, so `value + 1 > 2` compares the sum. At most
one comparison is accepted per expression level; parentheses retain their grouping
behavior. The condition must produce a Boolean: a literal, typed parameter or
local read, supported call, or comparison. Boolean locals, assignments, and print arguments are
accepted. Annotated Boolean parameters/results and user-call arguments use
these same rules. Boolean arithmetic, integer truthiness, mixed Boolean/Number
equality, Boolean ordering, other comparison operators, and short-circuit operators are unsupported.

Lowering emits an entry block, condition evaluation, and branch, then separate
arm blocks. Both arms may read parameters and enclosing locals. The
[IR rules](compiler-ir-text.md#control-flow) validate
block-local values and initialization on every path. See
[conditions.mc](../../examples/compiler-probe/conditions.mc) and
[initial simulator evidence](../investigations/compiler-conditions.md).

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
The target must be a bare identifier. Parameters, fields, unknown/forward names,
and arm locals used outside their scope are rejected. Assignment is a statement,
not an expression; compound assignment and increment/decrement are unsupported.

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
Number, String, Boolean, or Char result and discards a Void entry's result.
The [example][void-example] and
[simulator evidence][void-evidence] verify both void return forms and caller
continuation after discarded calls. Source class/receiver syntax follows the
[restricted class profile](compiler-classes.md).

[void-example]: ../../examples/compiler-void/README.md
[void-evidence]: ../investigations/compiler-void.md

## Instance class extension

Unit mode also accepts [AppBase-derived class
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
operations retain their own forms. No other native module statement is enabled
by this profile. Runtime exit invokes a supplied source `onStop` callback in the
validated console app; its Dictionary-or-null state is not a user-call argument.

## Native value calls

A Number-valued expression may receive these catalogue-resolved methods:

| Method | Declaring class | Result |
|---|---|---|
| `toString()` | `Lang.Object` | String |
| `format(String)` | `Lang.Number` | String |
| `abs()` | `Lang.Number` | Number |
| `toNumber()` | `Lang.Number` | Number |
| `compareTo(Number, Boolean, or Char)` | `Lang.Number` | Number |
| `equals(scalar or nullable scalar)` | `Lang.Object` | Boolean |
| `hashCode()` | `Lang.Object` | Number |

Number `compareTo` requires API 5.0.0; the other listed Number methods require
1.0.0. Its declaration takes Object, but current lowering admits Number or
Boolean or Char only. Unchecked nullable comparison arguments are rejected.
`equals`
accepts Number, String, Boolean, Char, Null, or nullable Number/String; it returns
Boolean. Dictionary arguments remain outside this lowering profile.
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
the same scalar/nullable scalar arguments as Number equality, since API 1.0.0.
`toNumberWithBase(Number)` returns Number or Null, since API 1.4.1, and follows
the existing null-check rules. The base is 0 for prefix detection or 2..=36 for
an explicit radix. Numeric ranges and conversion content are passed to the
runtime without compiler validation. See the
[String group evidence][string-group].

Boolean receivers support `toString()` returning String, `hashCode()` returning
Number, and `equals` returning Boolean with the same scalar/nullable argument
profile as Number/String equality. These inherited methods date from API 1.0.0.
Boolean `compareTo` returns Number and accepts Number/Boolean/Char arguments
since API 5.0.0. Unchecked nullable arguments are rejected. This native-call support
does not enable Boolean arithmetic or ordering operators. See the
[Boolean group evidence](../investigations/compiler-boolean-methods.md).

Char values originate from literals or Number's zero-argument `toChar()` and are distinct
from Number. They support `toLower()`/`toUpper()` returning Char, `toNumber()`
returning Number, and `toString()` returning String. Char's direct `compareTo`
returns Number for Number/Boolean/Char arguments, since API 5.0.0. Inherited
`equals` accepts the scalar/nullable scalar profile above and returns Boolean;
`hashCode()` returns Number. Char itself and its conversions require API 1.3.0,
including when it appears only in a function signature. Typed arguments,
returns, local initialization/assignment, printing, and chained calls retain
the exact Char type. Char operators follow the operand table above; nullable Char
remains unsupported.
Conversion ranges and case/String behavior are runtime obligations;
the compiler adds no Unicode scalar-range validation. See the
[Char group evidence](../investigations/compiler-char-methods.md).

Number, String, and Boolean receivers also support zero-argument `weak()` since
API 1.2.0. Its effective result is the exact receiver type, not WeakReference.
It remains an effectful native call; the compiler does not replace it with its
receiver. Nullable Number/String receivers require a direct non-null proof
before the call. Other classes and ordinary object weak references are outside
this lowering profile. See the [weak-result evidence][weak-evidence].

String results may initialize/replace a String local or be printed. Number
results support integer locals, arithmetic, comparisons, user-call
arguments, integer returns, and further Number calls. For example,
`(0 - 41).abs().format("%03d")` produces `"041"` on the validated profile.
Existing user-call arguments and return restrictions remain unchanged.

Postfix calls bind more tightly than addition/subtraction; parenthesize a
computed arithmetic receiver, for example `(n + 1).format("%03d")`. Literal,
local, Number/String/Boolean/Char parameter, and supported call-result receivers
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
contract; discarding does not enable `Array.sort` or unknown members. See the
[statement evidence](../investigations/compiler-native-statements.md).

Null/Dictionary receivers remain unsupported. Chained calls
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

## Char arrays

String `toCharArray()` returns a non-null `Array<Char>` since API 1.3.0. The
supported source annotation is `Toybox.Lang.Array<Toybox.Lang.Char>`, with ordinary
token whitespace permitted. Array parameters and results retain the element
contract; source array results require an explicit annotation. Locals may store,
copy, and reassign matching Char-array values, and user calls may pass/return
them. Raw Array annotations, other element types, and nullable arrays are
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
concatenation, comparisons, Boolean conditions, scalar equality methods, or
console entry results. Array literals and construction, `sort`, and ordinary
object weak references remain unsupported. Native Array calls may be values or
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
observable. String/dictionary indexing, element writes, nullable arrays, and
other element contracts remain unsupported. See the
[array-read evidence](../investigations/compiler-array-reads.md).

## Nullable results

String `find(String)` and `toNumber()` are available since API 1.0.0 and return
Number or Null; `toNumberWithBase(Number)` has the same result since 1.4.1. String `substring` and user calls may also return nullable
values. Compare nullable parameters or stored results with null.
A direct `if (value == null)` or `if (null == value)` proves that parameter or
local non-null on the false path. Boolean negation reverses this proof edge;
`if (!(value == null))` proves non-null on the true path. Fresh reads there have
the non-null Number or String type
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
