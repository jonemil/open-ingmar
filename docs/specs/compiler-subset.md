# Supported Monkey C compiler subset

Restricted frontend. Design:
[small semantic IR](../design/compiler-ir.md). Runtime evidence:
[constant returns](../investigations/first-compiler.md),
[arithmetic](../investigations/compiler-arithmetic.md),
[locals](../investigations/compiler-locals.md),
[module calls](../investigations/compiler-calls.md),
[parameters/arguments](../investigations/compiler-arguments.md), and
[strings/printing](../investigations/compiler-strings.md).

Integer/string/Boolean/null restrictions describe the current implementation.
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
value-type = "Toybox.Lang.Number" | "Toybox.Lang.String" | "Toybox.Lang.Boolean"
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
expression = arithmetic (("==" | ">") arithmetic)?
arithmetic = primary (("+" | "-") primary)*
primary = atom ("." member "(" argument-list? ")")*
atom = decimal-integer | string-literal | "true" | "false" | "null"
     | parameter-or-local-identifier
     | module "." member "(" argument-list? ")" | "(" expression ")"
```

Identifiers use ASCII letters/underscore followed by ASCII
letters/digits/underscore. `function`, `module`, `return`, `var`, `if`, `else`,
`true`, `while`, `break`, `continue`, `class`, `extends`, `self`, `false`,
`using`, `import`, `as`, and `null`
are reserved. `true` and `false` produce Boolean literals. Whitespace and
`//` or non-nested `/* ... */` comments are supported. The integer literal domain
is `0..=127`; computed results may be negative or exceed 127. Unary signs,
larger literals, other literal forms, other types, unsupported
statements/additional functions in default mode, and other operators are
rejected. This is a frontend limit, not a Monkey C language or VM integer limit.
The domain deliberately avoids unresolved high-bit one-byte immediate behavior.

Diagnostics identify the input byte offset and one-based line/column (columns
count Unicode scalar values). The caller supplies the filename. Unsupported
input never silently generates a partial function. Name resolution currently
covers parameters and earlier initialized locals in the current arm and its
enclosing scopes. There is no general type inference, global/member resolver,
whole-program analysis, or optimizer yet.

The IR contains integer/string/Boolean constants, binary add/subtract definitions,
parameter reads, local initialization/read, module calls, print statements, and
comparisons, branches, and typed value returns using value IDs, with optional
source spans. Source
compilation supplies them; IR text may omit them. Without locals, lowering emits
a label from the function name, `argc <parameter-count + 1>`, `ipushz` for zero
or `ipush1` for a positive supported literal, left/right operands followed by
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
associativity, and parentheses for grouping. Literals remain decimal `0..=127`;
subtraction results may be negative. Unary signs, other operators, and general
object calls remain unsupported; the bounded native calls below are separate. Parse nesting is
bounded at 64; the semantic function is limited to 1024 operations including
return. These are implementation limits.

Arithmetic validation uses the semantic API type `Toybox.Lang.Number` for both
operands and the result. Comparisons produce `Toybox.Lang.Boolean`; supported
null checks are specified below. These contracts are independent of method
lookup and VM instruction selection. API declarations do not broaden accepted
operator combinations or cause heap allocations to be emitted.

Semantic IR contains pure integer add/subtract definitions with explicit
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
qualified names `Toybox.Lang.Number`, `Toybox.Lang.String`, and
`Toybox.Lang.Boolean`; result annotations also accept `Void`. Short names,
module aliases in annotations, unions, nullable types, and other type syntax
are outside this subset. These declarations require no `using` statement.

Unannotated functions with value returns require Number results; those with no
value returns have a Void contract. String and Boolean source results require
explicit annotations. Every value-returning path must return the same declared
type; mixed types, value/void mixtures, and missing returns are errors. Typed IR
construction infers the uniform result from its returns. Native callbacks retain
their independently resolved signatures; annotations cannot override them.

## User module calls

The supported call form is `<module>.<member>(<arguments>?)` in expressions
and initializers. The module is a top-level user module provided by the
surrounding PRG scaffold. Value calls return Number, String, or Boolean;
arguments must match the declared types and positional arity. Global unqualified
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
initializations and accept one Number, String, or Boolean value. String types
propagate through locals and annotated function parameters/results; incompatible
uses are rejected. Arithmetic remains Number-only. Concatenation, arbitrary
SDK calls, and using
`System.println` as an expression remain unsupported. A local/parameter named
`System` cannot be used as the print receiver. The function's scaffold must
provide the SDK module. The native name profile below resolves its supported
spellings; Jungle resolution remains unimplemented.

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

`==` and `>` compare integer expressions and produce Booleans; `==` also accepts
two Booleans. Dictionary/null equality follows the callback rules below.
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
reports the validated Number/String/Boolean/Void contract. Saved text uses
[IR](compiler-ir-text.md#void-returns-and-statement-calls).
Both forms share type, control-flow, use, and effect-schedule validation.
A void function may contain only its return, so the operation bound is now
1..=1024. The backend emits bare `return`; statement calls use the existing call
sequence and discard the VM result with `popv`.

The [console application profile](console-application.md) prints a value entry's
Number, String, or Boolean result and discards a Void entry's result. The [example][void-example] and
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

String-valued expressions support these zero-argument methods, declared by
`Lang.String`: `length()` returns Number; `toLower()`, `toUpper()`, and
`toString()` return String. All methods in this profile are available since
API 1.0.0. Runtime implementations determine character counts and case mapping;
the compiler does not evaluate these calls at compile time.

String results may initialize/replace a String local or be printed. Number
results support integer locals, arithmetic, comparisons, user-call
arguments, integer returns, and further Number calls. For example,
`(0 - 41).abs().format("%03d")` produces `"041"` on the validated profile.
Existing user-call arguments and return restrictions remain unchanged.

Postfix calls bind more tightly than addition/subtraction; parenthesize a
computed arithmetic receiver, for example `(n + 1).format("%03d")`. Literal,
local, Number/String parameter, and supported call-result receivers are
accepted. Evaluate the receiver first, then arguments left to right, then invoke
the member.

The frontend distinguishes visible local/parameter receivers from module calls.
Only these value-producing native calls are supported. Null/Dictionary/Boolean
receivers and discarded method statements remain unsupported. Chained calls
must resolve each member against the preceding result's type, for example
`41.toString().toUpper().length().format("%03d")`. String and Boolean user
parameters/results require the function annotations defined above.
String `find(String)` and `toNumber()` return Number or Null and follow the
[null-check rules](#nullable-number-results). Unknown catalogue
members, wrong arity/types, and known members lacking lowering are distinct
errors. Explicit [API context inputs][api-context] check introduction levels;
general native dispatch remains unsupported.
Format content is passed to the runtime; type checking does not establish that
a String contains a valid format pattern.

[Native call IR](compiler-ir-text.md#native-value-calls) preserves receiver/member
identity separately from executable IDs. Runtime evidence and reproduction are
in the [native call experiment](../investigations/compiler-native-calls.md).

## Nullable Number results

String `find(String)` and `toNumber()` are available since API 1.0.0 and return
Number or Null. Store their results in locals and compare them with null.
A direct `if (value == null)` or `if (null == value)` proves that local non-null
on the false path. Fresh reads there have Number type and support arithmetic,
Number methods, printing, and ordinary user arguments/returns. For example:

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
assignments of Number values. Nullable Number locals accept Number, Null, or
Number-or-Null replacements; their declared storage type stays nullable.

Only direct local-read/literal-null comparisons used by the branch establish
facts. A stored Boolean condition, a null-valued local used as the comparison
operand, or a check of a different local does not narrow the target. Checking
one local does not narrow aliases or later native-call results. Unchecked nullable
values cannot be printed, used as Number operands/receivers, passed as ordinary
user arguments, or returned. Nullable user signatures remain unsupported.
Other unions and String-result methods such as `substring` remain outside this
profile. Conversion and search behavior is supplied by the runtime, not folded
by the compiler; see [evidence](../investigations/compiler-nullable-numbers.md).
