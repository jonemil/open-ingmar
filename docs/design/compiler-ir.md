# Compiler representation design

Use a value-based semantic IR between source parsing and VM assembly. It makes
language meaning, dependencies, storage identity, and effect order inspectable
before selecting instructions. The representation remains provisional; it does
not require Garmin MIR compatibility or LLVM/SSA infrastructure.

The [source subset][source], [function IR][ir], and [unit IR][unit] specify
exact accepted constructs and validation rules. This page explains the choices
behind those contracts.

## Representation and invariants

Value IDs identify expression results; local IDs identify stored locals;
parameter indexes identify positions. None chooses a VM slot, symbol ID, import
index, or section offset. The backend accounts for hidden context separately.
Source spans provide diagnostic provenance rather than executable meaning.

Expression values have one use within their defining block. Repeated local
reads produce fresh values from stored state, avoiding recomputation of
initializers. This permits tree-based lowering without a general shared-value
scheduler. Calls, prints, and assignments preserve statement and left-to-right
expression order. Pure expressions may be scheduled by dependencies.

Validated IR is immutable. Parsing source, constructing typed IR, and reading
IR text share validation so no producer can bypass backend assumptions.

## Layer responsibilities and naming

The frontend resolves source syntax, names, and scopes into semantic IR.
`ciq-compiler::backend` selects VM instructions and frame/local storage;
`Function::lower()` delegates to `backend::lower()`. Assembly owns selected
instructions, symbolic operands, layout, binding, and encoding through
`ciq-mc`. [Architecture](../architecture.md) defines crate boundaries.

Application generation consumes unit identities and CODE labels to build DATA,
imports, and container metadata. Configuration, signing, and packaging stay
outside the frontend. Extract `ciq-ir` or `ciq-codegen` only when independent
consumers or dependencies justify it; a future IR crate must not depend on its
VM backend. Keep `ciq-asm` and `ciq-dis` as the paired tools.

## API declarations and built-in values

The intended [API declaration model](api-declarations.md) supplies class/member
contracts and availability independently of executable bindings. Current native
contracts remain a bounded set of special cases until that model is implemented.
Built-in values will refer to core `Lang` type identities; operator rules and VM
instruction selection remain separate from the API method catalogue. Object
semantics do not require every IR value to denote a heap allocation.

## Native and textual pipelines

Source → semantic IR → assembly model → CODE is the in-memory path. IR and
`.mcasm` text are optional inspection/resumption boundaries. Text readers
reestablish the same model invariants and reuse native lowering and encoding.
Every extension updates validation, lowering, text I/O, and parity tests
together.

The [shared assembly model](assembly-model.md) serves handwritten source,
compiler output, and disassembly. Retain readable assembly for debugging without
making temporary text files mandatory in the compiler pipeline.

## Control flow and joins

Explicit blocks and terminators describe control flow independently of textual
order. Every block is reachable from entry and has a return, branch, or jump;
there is no implicit fallthrough. Boolean comparisons feed branches.

Expression values remain block-local. Lower each expression into a statement or
terminator sink, leaving an empty evaluation stack on every edge. Stored locals
carry state across edges, with fixed slots across the function. This avoids phi
values and general stack merging while supporting branches, joins, and loops.

The source builder resolves lexical scopes and constructs the graph separately
from parsing and instruction selection. Only paths that continue contribute
join edges. Arm/body declarations disappear from the enclosing source scope;
IR retains their distinct identities. [Source rules][source] define visibility,
shadowing, unreachable statements, and return requirements.

## Mutable locals

Initialization establishes a stored identity and its fixed integer/string type.
Assignment replaces content in the existing slot and cannot establish initial
availability. These restrictions belong to our subset, not all Monkey C.

Reads observe the most recent write on the executed path. Evaluate the RHS
before replacing the slot. Writes make read timing observable, so effectful IR
must preserve operation order even when no calls occur. In particular, a read
cannot be defined before a write but dependency-lowered after it: that would
change the value observed. A broader scheduler would need snapshot temporaries
rather than relaxing this validation without corresponding lowering support.

## Loops and initialization analysis

A loop reevaluates its condition at the test block. Continuing body paths return
there; `continue` selects the innermost test and `break` its exit. Body-local
initializers run on every iteration using fixed slots. Source analysis keeps the
exit reachable because the first condition may be false; it does not prove
termination or impose an iteration cap.

Check graph reachability from entry to reject disconnected cycles. Definite
initialization computes a greatest fixed point: incoming availability is the
intersection of predecessor outgoing sets, and outgoing availability adds local
initializers. An empty external predecessor for entry prevents backedges from
inventing first-entry initialization. Assignments do not add availability.
Within each block, reads/writes require initialization at their operation.

This handles zero iterations and cyclic paths without a general SSA framework.
[IR rules][ir] specify declaration ordering, iteration, and validation precisely.

## Rationale

Direct AST-to-bytecode emission couples source traversal to VM stack and frame
choices. A semantic boundary makes effect order and storage requirements
reviewable before lowering, and supports the same pipeline in memory and text.

The frontend/IR/machine/encoding separation is useful without importing register
allocation, relocatable objects, or a linker. External terminology references
and parity evidence are in [the pipeline investigation][pipeline].

## Compilation units

Collect declarations and signatures before checking calls so forward and
recursive references work across files. Separate ownership from file boundaries.
Unit validation adds identity, arity, and result checks to function validation;
standalone functions can retain explicit external-binding obligations.

Lower all bodies into one assembly layout with owner-qualified labels. Metadata
uses those same identities and labels. Unit text preserves ownership alongside
function IR; native and saved paths share checks and bindings. [Unit
rules][unit] define declaration order, call resolution, labels, and text syntax.

## Literal content and placement

Semantic strings carry UTF-8 content rather than DATA addresses. Lowering
deduplicates content per function and emits unplaced declarations with symbolic
`news` operands. Explicit DATA binding or the application builder supplies
placement, keeping function IR independent of section layout. PRG reconstruction
completes method bindings after layout.

`System.println` is an explicit intrinsic with an established SDK binding;
arbitrary SDK identities and return types are not inferred from spelling.
[String evidence](../investigations/compiler-strings.md) supports this boundary.

## Void results and effect-only calls

Void returns have no value dependency. Statement calls preserve argument order
and define no semantic result, whether their callee returns integer or void.
This keeps effect-only calls outside the expression graph. Unit validation
rejects void callees in value contexts; lowering discards statement-call runtime
results. The application wrapper follows the entry's result contract.

## Class ownership and instance receivers

Classes own instance-method bodies and a native base identity, separately from
global/module declarations. Function IR records its receiver requirement; unit
validation associates it with an owner and resolves explicit self calls within
that class. The receiver is hidden context, not a first-class semantic value.

Owner-qualified labels distinguish same-name bodies. Application generation
binds methods to the source AppBase DATA record and resolves native identity to
imports; IR never stores import indexes. Generated callbacks and View/base setup
remain separate until source native calls and object/null/array values are
supported. [Class rules](../specs/compiler-classes.md) define the supported profile;
[application design](console-application.md) describes the intended extension.

[source]: ../specs/compiler-subset.md
[ir]: ../specs/compiler-ir-text.md
[unit]: ../specs/compiler-unit.md
[pipeline]: ../investigations/compiler-pipeline.md
