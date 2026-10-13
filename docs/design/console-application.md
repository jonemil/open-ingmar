# Minimal application generation

`ciq-compiler::application` turns a validated compilation unit into a complete
unsigned console watch application. This keeps metadata and lifecycle generation
separate from source compilation. The
[profile](../specs/console-application.md) is deliberately small and
[simulator-verified](../investigations/application-bootstrap.md).

## Boundaries

The frontend resolves restricted global/module functions and source AppBase/View
class methods into semantic IR. Unit validation handles declaration identities,
typed calls, callbacks, and source constructors. The VM backend selects instructions.
Application generation then combines those bodies with typed lifecycle assembly
and constructs DATA and container sections; signing and package/project
configuration remain separate.

This avoids embedding application metadata or native import indexes in semantic
function IR. The lifecycle generator supplies default bodies using the same
symbolic assembly model. Validated source lifecycle methods replace their
corresponding defaults; source Views receive separate native-derived DATA records.

The frontend's bounded native name environment resolves supported Toybox
module bindings and imported class names before IR construction. These use
native identities independently of VM symbols/import indexes. Aliases and
qualified spellings select the same superclass/print operation. Native base
initialization now has its own ordered semantic operation with an implicit
instance and no result. Unit validation constrains it to the first statement of
a zero-parameter void source initializer. The application builder replaces the
default initializer's DATA target with the source method, avoiding duplicate
method fields. Catalogue callback descriptors supply AppBase and View signatures
according to the enclosing native base. Function IR preserves nullable Dictionary
state, one-view positional results, and opaque Dc parameters independently of
VM slots. Source construction retains the class identity for binding while its
value uses the native View contract.

No SDK files, compiled PRG template, copied DATA prefix, or signing key are read
by the generator. Native identities and opaque header/entry profile values are
explicit compatibility constants. Keep unknown meanings visible and extend
profiles only with evidence; they are not universal device/format rules.

## Layout and identity

User symbols are assigned deterministically; module lookup identities remain
separate from plain member names. Existing unit CODE labels bind all methods.
Generated fields are sorted by numeric key because the runtime experiment found
an out-of-order method unresolvable. The shared DATA reader/writer still
preserves existing record order for lossless work.

The builder lays out class/module records first, calculates numeric statics
references required by the current DATA model, then appends literal records.
CODE labels and string placements resolve through existing typed encoders.
This keeps the native path in memory and makes saved-unit-IR resumption use the
same builder. A linker or new crate is unnecessary for the current profile.

Debug tables, launcher assets, named views, and resource-language tables are
omitted in this profile. The default native View stays blank; a source
getInitialView can instead construct a source View with drawing callbacks.
Entry prints and, for integer entries, its result provide console observations.
Void entries run once with their result discarded and no wrapper print;
[the void-call experiment](../investigations/compiler-void.md) verifies this path.

## Extension boundary

Non-Number fields, source superclass dispatch, and general source-object member calls
require a broader object model. Method records and imports should follow those
identities rather than fixed wrapper choices. [The plan](../plan.md) defines
the sequence and runnable acceptance examples. Preserve native/text equivalence
and complete-application simulator tests as each construct is added.

Jungle/manifest resolution should supply application identity, source inputs,
target constraints, and export settings once that source subset can represent
real application lifecycle files. Section-size research remains driven by
actual generated programs and concrete blockers.
