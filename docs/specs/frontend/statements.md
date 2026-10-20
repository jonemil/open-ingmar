# Statements, scopes, and control flow

## Local declarations and reads

`var <identifier> = <expression>;` initializes a local with a supported value
contract. Expressions may read it repeatedly after initialization. Duplicate
declarations in one scope, shadowing an enclosing local or parameter,
self/forward/unknown reads and uninitialized declarations are rejected. An arm's
locals are visible within that arm and nested arms, but not in a sibling or
after the condition. Disjoint arms may reuse a name; each declaration receives a
distinct semantic local and VM slot. A later outer declaration may also reuse a
name that has gone out of scope. Standalone braced blocks are unsupported.
Reassignment follows the fixed-type rules below.

Locals retain stored state; repeated expression reads produce distinct
single-use values. The [IR storage rules](../ir/values.md#local-operations) own
initialization/availability checking, and [frame
lowering](../lowering/README.md) assigns slots independently of semantic IDs.
Unused initializers still execute.
[Controlled SDK evidence](../../investigations/compiler-locals.md) verifies ordinary
local bodies; the frame limit is structural, not an established device maximum.

## Terminal conditions

Runtime comparisons follow the [Number-comparison
evidence](../../investigations/compiler-number-comparisons.md).

A function may end with `if (<condition>) { return <expression>; } else { return
<expression>; }`. Braces are required; `else` is optional. An omitted false arm
continues to the next statement, which must satisfy the function return
contract. Arms may also contain local declarations, print statements, and nested
conditions, as described below. Loop syntax is specified separately below.

Comparisons follow the [operator](operators.md) and [numeric](numeric-values.md)
contracts, producing Boolean results. Equality/inequality also accept Boolean
pairs and the String identity profile; `!=` negates the existing equality
contract, not String content equality. Dictionary/null checks follow the
[nullable contracts](types.md#nullable-results).
Arithmetic has higher precedence, so `value + 1 > 2` compares the sum. At most
one comparison is accepted per expression level; parentheses retain their
grouping behavior. The condition must produce a Boolean: a literal, typed
parameter or local read, supported call, or comparison. Boolean locals,
assignments, and print arguments are accepted. Annotated Boolean
parameters/results and user-call arguments use these same rules. Boolean
arithmetic, integer truthiness, mixed Boolean/Number equality and Boolean/String
ordering are unsupported. Short-circuit operators follow the [short-circuit
rules](operators.md#short-circuit-expressions).

[VM lowering](../lowering/control-flow.md#control-flow-lowering).

## Statement arms and joins

Conditions may appear between statements; combined `if`/`while` nesting is
limited to 64 levels. Empty arms are allowed; `else if` without braces is not
accepted. Each arm has its own local scope. Every value-returning path must
satisfy the declared result contract; Void fallthrough inserts a return.
Statements after `return`, `break`, `continue`, or a conditional whose arms both
terminate are rejected as unreachable. A value-returning path with no return is
rejected at the function's closing brace.

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

See
[nested-conditions.mc](../../../examples/compiler-probe/nested-conditions.mc)
and [nested/continuing-arm simulator
evidence](../../investigations/compiler-nested-conditions.md). Full language
scoping, other loop forms and standalone blocks remain future work.

## Local reassignment

`<identifier> = <expression>;` replaces an earlier initialized, in-scope local.
The local target must be a bare identifier. Parameters, unknown/forward names,
and arm locals used outside their scope are rejected. Assignment is a statement,
not an expression. Local compound and increment/decrement statements are
specified below. Class methods additionally support [explicit self field
assignment](fields.md).

The initializer fixes the local's supported type. Each assignment must preserve
it, including Boolean locals; a nullable Dictionary local may receive null.
General type changes are rejected. RHS evaluation finishes before the write, so
`n = n + 1` reads the old value. Later reads observe the new content on that
path. Assigning an enclosing local inside an arm is allowed; assignment does not
create a declaration.

A write consumes one semantic value and defines no result. It reuses the local's
existing slot and does not contribute to the 255-local count. The operation
count still includes each write and its RHS definitions. Both arms may update a
common local, or one arm may leave it unchanged; it remains available at the
join because its initializer executes before the condition.

Source, typed IR, and text IR share type, availability, and effect-order checks.
Assignment-containing IR uses the conservative schedule specified in
[IR text](../ir/values.md#local-assignment). See
[assignments.mc](../../../examples/compiler-probe/assignments.mc) and
[integer/string simulator evidence](../../investigations/compiler-assignments.md).

## Increment and decrement

`target++;` and `++target;` add one to a numeric scalar or Char, preserving its
type. `target--;` and `--target;` subtract one from a numeric scalar; Char
decrement is rejected. The two operator characters must be contiguous. Accepted
targets are bare initialized numeric/Char locals, explicit `self` numeric/Char
fields, and supported numeric/Char indexed elements. Storage write-back must
preserve the field/element contract. Char requires API 1.3.0, as for other Char
expressions. Parameters, grouped local targets, other value/element types, and
unknown or out-of-scope names are rejected. A nullable operand needs an
established non-null proof at the read. Existing assignment contracts still
govern its write.

Both prefix and postfix forms are statements with identical supported behavior;
they cannot be arguments, conditions, initializers, return values, or parts of
larger expressions. Source lowering retains a checked step and separate write,
without assigning expression-result semantics to either spelling.

For indexed targets, evaluate/capture array then index once, read the selected
element, add/subtract one, and write back to that captured location. Bounds and
allocation failures remain runtime obligations. Captures share the ordinary
frame budget; local updates reuse their existing slot. The tested Number
boundaries wrap from maximum to minimum on increment and minimum to maximum on
decrement. See [SDK/runtime
evidence](../../investigations/compiler-increments.md).

## Local compound assignment

A bare in-scope initialized local supports `+=`, `-=`, `*=`, `/=`, `%=`, `&=`,
`|=`, `^=`, `<<=`, and `>>=` as statements. Operator characters must be
contiguous. Parameters, grouped local targets, and unknown or out-of-scope names
are rejected. Field/indexed compound targets are specified below. Assignment
expressions, logical compounds, and unsigned-right-shift compounds remain
unsupported.

`n op= rhs;` reads the existing local once, evaluates `rhs` once, applies the
corresponding supported binary operator, then writes the result to the existing
slot. The whole RHS is parsed before applying the implicit operator: `n *= 1 +
2` means `n = n * (1 + 2)`. Existing binary contracts and fixed local types
apply: all ten operators accept non-null Number pairs; String `+=` uses
supported scalar concatenation, and Char `+=` accepts Number/Long. Number `+=
Char` and Char `+= Char` fail because their results change the local type. No
new coercion, folding, or runtime guard is introduced.

The read/binary/write sequence uses ordinary IR operations and source spans. A
RHS needing control-flow staging snapshots the old local before RHS effects;
hidden locals share the existing frame budget. Plain compounds need no hidden
slots. A nullable local requires an existing non-null proof for its old read,
and the write invalidates that proof as ordinary reassignment does. A later
non-null read needs a fresh check. See [compound
evidence](../../investigations/compiler-compound-locals.md).

## While loops

`while (<condition>) { <statements> }` evaluates its condition before each
iteration. The condition must produce a Boolean, with the same Boolean/type
rules as `if`. Braces are required; the body may be empty or contain locals,
assignments, prints, nested conditions/loops, and early returns. `for` and
`do/while` are unsupported.

Body locals are visible within that body and nested scopes, but not after the
loop or in its condition. Their existing slots are initialized anew on each
iteration; new slots are not allocated per iteration. Enclosing local writes
persist into later tests and after exit. Fixed local types still apply.

The body may execute zero times. Its declarations cannot establish a local
needed at the test or exit. A function ending with a loop still needs a return
for its false-condition path, even if its body always returns. The compiler does
not prove termination or condition outcomes and does not insert an iteration
limit. "Bounded subset" refers to supported syntax and compilation limits.

[VM lowering](../lowering/control-flow.md#while-loops).

See [loops.mc](../../../examples/compiler-probe/loops.mc) and
[zero/multiple-iteration simulator evidence](../../investigations/compiler-loops.md).

## Loop control

`break;` exits the innermost enclosing `while`; `continue;` jumps to its test,
reevaluating the condition before another iteration. Both require semicolons and
an enclosing loop, including when nested within conditional arms. They terminate
their current path and carry no expression values across the edge. A nested
loop's controls never target an outer loop. Labeled controls are unsupported.

A loop may still run zero times, so its body cannot satisfy the function's
fallthrough return requirement. Mixed returning/breaking/continuing arms omit a
join when all paths terminate, but the loop's false-condition exit remains.
Skipped declarations stay scoped to their body/arm; initialized enclosing locals
retain their slots and fixed types. These statements emit existing `.jump`
operations.

See [loop-control.mc](../../../examples/compiler-probe/loop-control.mc) and
[simulator evidence](../../investigations/compiler-loop-control.md).

## Void functions and statement calls

A value-returning function must return its declared type on every path. A Void
function's `return;` terminates immediately; reaching the final closing brace
inserts a void return. Value and void return forms cannot be mixed, including
across branches. Unreachable statements after either form are rejected. Loops
and scoped locals retain their existing rules. See [function type
contracts](types.md#types-and-value-contracts) for annotations and defaults.

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
arguments and discard any supported result, including Void. Void calls cannot
initialize locals, provide arguments, or appear in any other expression. Unit
mode checks the callee's result type and arity after collecting all signatures.
Single-function mode still leaves external callee contracts to the caller;
expression calls there assume integers. No void expression value is created.
Statement calls preserve argument evaluation and statement order.

Typed IR adds `ReturnVoid` and `ModuleCallStatement`; `Function::return_type()`
reports the validated scalar, nullable, or Void contract. Saved text uses
[IR](../ir/calls.md#void-returns-and-statement-calls).
Both forms share type, control-flow, use, and effect-schedule validation. A void
function may contain only its return, so the operation bound is now 1..=1024.
The backend emits bare `return`; statement calls use the existing call sequence
and discard the VM result with `popv`.

The [console application profile](../build/application.md) prints a value
entry's Number, Long, Float, Double, String, Boolean, or Char result and
discards a Void entry's result. The
[example](../../../examples/compiler-void/README.md) and
[simulator evidence](../../investigations/compiler-void.md) verify both void return forms and caller
continuation after discarded calls. Source class/receiver syntax follows the
[restricted class profile](classes.md).

## Array element writes

`array[index] = value;` replaces an element of a non-null supported array. The
index must be non-null Number; the value must match the invariant Number, Char,
String, or native View element contract. Guarded fresh nullable reads are
accepted for Number indices and Number/String values. View values may be native
or source View instances; storing/reading through a View array erases exact
source identity. Null values, delegates, and drawing contexts cannot fill View
slots. Receiver, index, and value evaluate once, in that order. The assignment
produces no value and does not change the array's length. Aliases observe the
changed element. Calls in the value expression may mutate array contents or
length before the write checks its index.

[VM lowering](../lowering/values.md#array-element-writes).

Nested arrays and assignments used as expressions remain unsupported. Indexed
compound targets follow the contract below. See [Number write
evidence](../../investigations/compiler-array-writes.md) and
[Char/View write evidence](../../investigations/compiler-typed-array-writes.md).

## Indexed compound assignment

`array[index] op= rhs;` supports Number/Long elements with all ten compound
operators, Float/Double elements with the four arithmetic operators, and Char
elements with `+= Number/Long`, and String elements with `+=` using the shared
String concatenation operands. Existing binary contracts and invariant array
element types apply; View elements have no supported compound operator. The
index must be non-null Number, including a fresh guarded nullable read.
String/dictionary receivers, nested arrays, and assignment expressions remain
unsupported.

Evaluate array and index once, in that order, then read the old element before
RHS effects. Evaluate RHS once, apply the binary operation, and write through
the captured array/index. Changes to contents or length during RHS evaluation do
not change the old value used by the operation. An invalid initial index fails
at the read before RHS effects; a RHS that shrinks the array can make the final
write fail after those effects. Both accesses retain runtime bounds checks,
without clamping or static range rejection.

Array and index are captured in two typed hidden locals. Plain RHS expressions
need no additional old-value/result slots. A branching RHS stages the old read
and result before defining fresh final write operands, keeping expression IDs
block-local. These temporaries share the ordinary frame budget; loop iterations
reuse their static slots. Saved IR uses existing locals, `.array-read`, binary
operations, and `.array-write`, with no compound directive. See [target
evidence](../../investigations/compiler-compound-targets.md).
