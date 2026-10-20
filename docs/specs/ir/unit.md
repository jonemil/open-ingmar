# IR compilation units and classes

`ClassDeclaration` records a name, native `ClassBase`, ordered fields, and
ordered methods. Bases are identities rather than VM import indexes.
`Function::method` constructs receiver-bearing IR; ordinary function
constructors reject receiver operations. Unit validation requires instance
bodies in classes and ordinary bodies in global/module owners. A method with no
self operations still retains its receiver contract.

`SelfCall` defines a result under an explicit contract; `SelfCallStatement`
consumes arguments without a result. Both share single-use, block-local,
control-flow, type, and effect-schedule checks with other calls. The hidden
receiver is not an explicit parameter or semantic value. Source ownership is
checked separately from member Symbol IDs.

The [unit/class grammar](grammar.md#unit-and-class-framing) serializes global,
module, and class owners, including Lang.Object bases and every supported field
kind. [Source class contracts](../frontend/classes.md) define constructibility,
receiver provenance, and ordinary versus callback invocation; [field
contracts](../frontend/fields.md) define types, defaults, required
initialization, and snapshot/null proofs. Unit reload rechecks these same
contracts. A result tag cannot forge a field or source-class identity merely
because a native projection matches.

## Unit IR text

Each owner record wraps exactly one complete function. Class containers wrap
fields followed by complete receiver-bearing method documents. Inner function
validation precedes ownership and cross-body call checking. Reject malformed
framing, duplicate owners/members, unresolved references, mismatched receiver
contracts, wrong arity, incompatible arguments/results, invalid fields, and
ordinary calls to native lifecycle callbacks.

The unit contains 1..=256 bodies including methods. Error body indexes follow
all global/module bodies, then class methods. [Unit
layout](../lowering/unit-layout.md) owns this grouped order and generated CODE
labels. Multiple supported helper classes may accompany the application class;
unit validity alone does not select an application entry or establish the
complete-app profile.

Spans remain optional per-function provenance. Filenames and source-file indexes
are not serialized. Native source compilation and saved unit IR produce the same
assembly/CODE with identical bindings. Standalone function IR remains supported,
with unresolved declaration checks retained as caller obligations. Examples and
record syntax are in the [IR grammar](grammar.md#examples).
