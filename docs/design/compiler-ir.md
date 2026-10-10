# Compiler representation design

Use a small value-based semantic IR between source parsing and VM assembly. This
separates language meaning from frame/stack choices and keeps each stage
inspectable. The format and APIs remain provisional; this is not Garmin MIR
compatibility or a commitment to LLVM/SSA infrastructure.

## Representation and invariants

A function has a name, parameter count, and ordered operations. Operations
represent integer/string constants, addition/subtraction, positional parameter
reads, local initialization/assignment/reads, user-module calls, print statements,
integer/void returns, comparisons, and explicit block/branch/jump operations. Source
spans are optional diagnostic provenance.

Value IDs name semantic results. Local IDs name stored semantic locals. Neither
chooses a VM slot, symbol ID, or CODE/DATA offset. Parameter indexes name
positions; the backend accounts for the VM context separately.

Each expression value has one use in its defining block and references earlier
definitions. Local initializations execute in order, including unused locals;
reads require initialization on every incoming path. Repeated source reads
create fresh values from stored locals rather than recomputing initializers.

Pure-only expression definitions may be scheduled by dependencies. Functions
containing calls, prints, or assignments must have IR operation order matching
left-to-right
expression evaluation and statement order. Validation rejects schedules the
current backend cannot preserve. Shared-value graphs and object calls require further scheduling/validation work.

The IR is immutable after validation. Source parsing, typed construction, and
[IR text](../specs/compiler-ir-text.md) use the same validator, so no producer
can bypass backend assumptions.

## Layer responsibilities and naming

- **Frontend (`ciq-compiler`)**: source syntax, name resolution, diagnostics,
  and semantic IR construction. Parsing has no machine opcodes.
- **Semantic IR (`ciq-compiler`)**: operations, invariants, and text I/O. A
  future `ciq-ir` may expose this independently of VM lowering/assembly.
- **VM backend (`ciq-compiler::backend`)**: instruction selection and evaluation
  stack/frame/local choices. `Function::lower()` delegates to
  `backend::lower()`. A separate `ciq-codegen` remains optional.
- **Assembly (`ciq-assembly`)**: selected instructions, symbolic operands,
  labels, syntax, layout, binding, and CODE encoding. See
  [the assembly model](assembly-model.md).
- **Instructions (`ciq-mc`)**: concrete opcodes and operand byte encodings.
- **PRG tools (`ciq-asm` / `ciq-dis`)**: reconstruction and inspection/export.
  Keep these paired names and the shared `ciq-assembly` library name.

Each shared library owns a representation and its rules, not miscellaneous
utilities. Extract crates when dependency boundaries or independent consumers
justify it. If IR is extracted, move VM-specific methods out of its types; the
backend consumes IR and produces assembly, never the reverse dependency.

Source project configuration, DATA/resource/module generation, signing, and
packaging remain separate concerns. Raw function integration may use existing
application metadata; the [console application builder](console-application.md)
generates a constrained independent profile from unit IR. Symbol binding resolves module/member names against an
explicit symbol source without generating method records.

## Native and textual pipelines

Source → semantic IR → assembly model → CODE is the native path. IR text and
`.mcasm` are optional serialization boundaries, not mandatory temporary files.
Handwritten, generated, and disassembled assembly use the same language.

Text resumption must reestablish the native model's invariants and use the same
lowering/encoder. Every operation extension updates validation, native lowering,
IR parsing/printing, and parity tests together. Preserve this contract without
freezing the schema or backend algorithm.

The backend traverses expression trees within each block. Blocks and terminators
are explicit operations in the existing ordered representation; they do not
require a new crate or optimizer framework. Keep readable assembly available to
diagnose VM-level issues throughout that evolution.

## Control flow and joins

An unmarked function remains one implicit block. An explicit function begins
with a block marker; the first declared block is entry. Every block terminates
with an integer or void return, Boolean branch, or jump. Targets may name earlier or
later blocks; every block must be reachable from entry. No implicit fallthrough
is accepted.

Integer equality and greater-than produce transient Boolean values, consumed
only by branches. These are semantic operations; the backend selects `eq`/`gt`
and `bf` followed by `goto`. Block labels and branch displacements belong to the
assembly layer. No fallthrough optimization is implemented.

Expression values cannot cross block boundaries. Each block emits complete
expression trees into statement/terminator sinks, leaving an empty evaluation
stack on every edge. Stored locals may cross edges: validation intersects
initialized-local sets from all predecessors at a join, including loop edges.
Local identities remain
unique; assignments reuse initialized locals without changing their type. Frame
slots are allocated across the whole
function, including mutually exclusive paths. This avoids introducing phi values
or a general stack-merge policy before they are needed.

The frontend emits structured `if/else` and `while` statements into the same
graph. Falling-through arms jump to a join declared after both arms. Arms
ending in `return`, `break`, or `continue` have no join edge. An omitted `else`
supplies an empty false arm. Nested arms preserve declaration order. Omit joins
when both arms terminate and reject unreachable following statements. All fallthrough
paths must return an integer when any return has a value; void functions
receive a final return on fallthrough. Mixed returns are rejected.

Name resolution gives each arm an inherited environment and discards its local
declarations on exit. Sibling declarations may reuse names but receive distinct
local identities and slots. Shadowing an enclosing local or parameter remains
rejected. The source builder is separate from parsing and VM lowering. Names
and source scopes are resolved there; IR text and assembly retain identities
rather than source environments.

This supports nested statements and early returns without expression value
joins. [The nested-condition experiment][nested] verifies selected prints
and continuing paths in the simulator. Loop construction and initialization
checks extend this representation as described below.
[Condition experiments](../investigations/compiler-conditions.md) verify both
comparison outcomes in the official simulator and native/text CODE parity.

## Mutable locals

`LocalInitialize` declares a unique stored local and fixes its integer/string
type. `LocalAssign` consumes a new value and replaces that local's content,
without declaring another identity or allocating another frame slot. Its target
must already be initialized on every incoming path, and its value must retain
the initializer's type. Assignment cannot make a partly initialized local valid
at a join. Boolean locals and parameter writes remain unsupported.

A fresh local read observes the most recent write on the path being executed.
Common locals can therefore carry selected branch results to a join without
phi values or stack merging. RHS evaluation completes before `lputv` replaces
the existing slot, supporting `n = n + 1`. Source scopes still determine which
local names are visible; assigning an enclosing local uses its inherited ID.

Writes make read timing observable even when a function contains no calls.
Assignment-containing IR must match statement/left-to-right evaluation order,
just like calls and prints. This rejects a read defined before a write but used
afterward: dependency lowering would otherwise execute it after the write and
silently change its meaning. Pure functions without calls, prints, or writes
retain their definition-order flexibility. A future scheduler could preserve
snapshots with temporaries, but that complexity is unnecessary for this subset.

The [assignment experiment][assignments] verifies integer and string writes in
both arms, later reads/increments, and native/text CODE parity. These are fixed
type rules for our supported subset, not a claim that Monkey C locals generally
have fixed types.

## Loops and initialization analysis

A `while` statement jumps to a test block, evaluates its comparison on each
visit, branches to the body or exit, and sends continuing body paths back to
the test. `continue` explicitly targets that test; `break` targets the exit.
The source builder tracks enclosing loop targets and selects the innermost pair.
A terminated body path has no implicit backedge. Body declarations have a nested
source scope and fixed slots; their initializers execute on every iteration.
The exit remains reachable in the source analysis even when the body returns,
since the condition may initially be false. No iteration cap or termination
proof is generated.

Reachability now walks edges from entry, rather than checking whether each
block has a predecessor. This rejects disconnected cycles. Block IDs and textual
order do not determine execution order. The final declared block still returns;
expression definitions and local declarations must precede their textual uses.
Expression values remain block-local, so no stack values cross loop edges.

Definite initialization uses a greatest fixed point over the reachable graph.
For each block, its incoming set is the intersection of predecessor outgoing
sets; its outgoing set adds its local initializers. Start sets at the universe
of declared locals and repeatedly intersect until unchanged. Entry has an empty
external predecessor, preventing a backedge from establishing first-entry
initialization. Within a block, scan operations in order, adding each initializer
before checking subsequent reads/writes. Assignments never establish a local.
This accounts for zero iterations without rejecting common locals that remain
initialized through cycles.

Types remain fixed by unique initializers, and source construction satisfies
the textual declaration-before-use rule. A general type inference or SSA
framework is unnecessary here. [Loop evidence][loops] covers zero/multiple
iterations, reused body slots, nested conditions, and native/text parity.
[Loop-control evidence](../investigations/compiler-loop-control.md) also covers
innermost targets, optional false arms, and mixed terminating paths.

## Rationale and evidence

Direct AST emission is smaller for a literal return, but ties source traversal
to VM stack/frame choices. A semantic boundary makes dependencies, local
identity, and effect order inspectable before selecting instructions. This
supports both native composition and editable stage outputs.

LLVM's frontend → semantic IR → machine representation → MC emission flow
motivated the separation. Its textual assembly is optional. This analogy does
not establish a need for register allocation, a relocatable object format, or an
independent linker for this VM toolchain.

External comparisons consulted on 2026-10-10:
[LLVM code generation](https://llvm.org/docs/CodeGenerator.html),
[Clang toolchains](https://clang.llvm.org/docs/Toolchain.html), [LLVM
IR][ref-llvm-ir], [LLVM MC][ref-llvm-mc],
[llvm-mc](https://llvm.org/docs/CommandGuide/llvm-mc.html),
[GIMPLE](https://gcc.gnu.org/onlinedocs/gccint/GIMPLE.html),
[RTL](https://gcc.gnu.org/onlinedocs/gccint/RTL.html),
[GNU as](https://sourceware.org/binutils/docs/as/), and [Cargo target
names][ref-cargo-target-names]. These informed terminology; they are not
compatibility requirements.

Local evidence:
[initial replacement](../investigations/function-replacement.md),
[instruction semantics](../investigations/instruction-semantics.md),
[MIR/module observations](../investigations/module-method-names.md),
[pipeline parity](../investigations/compiler-pipeline.md),
[locals](../investigations/compiler-locals.md),
[module calls](../investigations/compiler-calls.md), and
[parameters/arguments](../investigations/compiler-arguments.md).

[ref-llvm-ir]: https://github.com/llvm/llvm-project/blob/main/llvm/lib/IR/CMakeLists.txt
[ref-llvm-mc]: https://github.com/llvm/llvm-project/blob/main/llvm/lib/MC/CMakeLists.txt
[ref-cargo-target-names]: https://doc.rust-lang.org/cargo/reference/cargo-targets.html

## Compilation units

`CompilationUnit` owns declarations with separate global/module ownership and
immutable `Function` bodies. It collects all signatures before checking qualified
calls, allowing forward and recursive references across files. Existing function
validation supplies the current integer argument/result contract; unit validation
adds identity, exact arity, and integer/void result checks. Single-function compilation still permits
external callees for bounded scaffold experiments.

Lowering combines all bodies into one assembly model with length-prefixed,
owner-qualified labels. Internal block/string aliases use the same identity,
preventing collisions between similarly named members. Metadata generation must
consume those labels and module/member identities together. No linker, separate
IR crate, or new call opcode is needed for this step.

Unit IR wraps unchanged function IR documents with ownership records. Source,
typed construction, and text resumption share checks and produce equivalent
assembly/CODE with the same bindings. Strings remain deduplicated per function;
the separate console application builder supplies module DATA and symbol
allocation for its supported profile. See
[the unit profile](../specs/compiler-unit.md) and
[simulator evidence](../investigations/compiler-unit.md).

## Literal content and placement

Semantic string values retain UTF-8 content rather than DATA addresses. Local
reads retain their initializer's integer/string type. The current validator
restricts arithmetic, parameters, and user-call arguments to integers;
expression calls and valued returns yield integers; print statements accept either type and define no result.

Lowering deduplicates content within a function and emits unplaced assembly
string declarations plus symbolic `news` operands. An explicit DATA source
supplies placement at encoding, keeping semantic IR independent of application
layout. DATA layout can be validated before its method references are resolved;
PRG reconstruction completes those bindings. This supports both native and saved
textual stages without allocating application metadata in the frontend.

`System.println` is an explicit intrinsic. Its dedicated SDK alias and discarded
VM result follow observed SDK output; arbitrary SDK names and return types are
not inferred. [The experiment](../investigations/compiler-strings.md) documents
this boundary and generated UTF-8 output in the simulator.

[nested]: ../investigations/compiler-nested-conditions.md

[assignments]: ../investigations/compiler-assignments.md

[loops]: ../investigations/compiler-loops.md

## Void results and effect-only calls

Functions infer one common integer or void result contract from validated
return terminators. A void return has no value dependency. Statement calls
consume ordered integer arguments and define no value, whether the callee
returns an integer or void. Unit checking rejects void callees in expressions;
standalone expression calls retain an external integer-result obligation.
This keeps effect-only calls out of the expression graph and preserves the
existing one-use, block-local value model.

The backend emits a bare VM return and discards each statement call's runtime
result. The application layer selects its startup wrapper from the entry
contract and avoids printing void results. [Source and IR rules][void-source]
and [simulator evidence][void-evidence] establish this increment before source
class ownership, receiver types, null/object/array values, or native SDK calls.

[void-source]: ../specs/compiler-subset.md#void-functions-and-statement-calls
[void-evidence]: ../investigations/compiler-void.md

## Class ownership and instance receivers

Class declarations own named instance-method bodies and a native base identity.
The unit keeps them separate from its existing global/module declaration list,
so callers cannot mistake class members for static module functions. Collection
and layout group ordinary functions before class methods, retaining order within
each group. Owner-qualified labels keep same-name bodies distinct.

Function IR independently records whether it requires an instance receiver.
Ordinary constructors reject self calls; method construction enables them.
Unit validation pairs this receiver contract with class ownership and resolves
self callees only within that class. It checks integer arguments and integer/void
results before lowering. The receiver remains the hidden context rather than a
first-class value, sufficient for explicit self calls without object operations.

The backend selects getselfv and the existing counted-call convention.
Application generation binds the source methods into the selected AppBase DATA
record and invokes the chosen startup on that instance. Native base identity is
resolved there; semantic IR never stores import indexes. Generated callbacks and
View/base setup remain a separate layer until their native-call and value types
are supported. See [the class profile][classes] and [runtime evidence][class-run].

[classes]: ../specs/compiler-classes.md
[class-run]: ../investigations/compiler-class.md
