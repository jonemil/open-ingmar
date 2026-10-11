# Minimal application generation

`ciq-compiler::application` turns a validated compilation unit into a complete
unsigned console watch application. This keeps metadata and lifecycle generation
separate from source compilation. The
[profile](../specs/console-application.md) is deliberately small and
[simulator-verified](../investigations/application-bootstrap.md).

## Boundaries

The frontend resolves restricted global/module functions and source AppBase
class methods into semantic IR. Unit validation handles declaration identities
and integer call signatures. The VM backend selects user-function instructions.
Application generation then combines those bodies with typed lifecycle assembly
and constructs DATA and container sections; signing and package/project
configuration remain separate.

This avoids embedding application metadata or native import indexes in semantic
function IR. The lifecycle generator uses fixed receiver/object/array/void
operations with recorded evidence, rather than pretending the frontend already
accepts those source constructs. Source classes supply method ownership and self
calls; typed SDK calls and object/null/array operations still need validation
before replacing the remaining wrapper.

The frontend's bounded native name environment resolves Application/System
module bindings and imported AppBase names before IR construction. These use
native identities independently of VM symbols/import indexes. Aliases and
qualified spellings select the same superclass/print operation. Native base
initialization now has its own ordered semantic operation with an implicit
instance and no result. Unit validation constrains it to the first statement of
a zero-parameter void source initializer. The application builder replaces the
default initializer's DATA target with the source method, avoiding duplicate
method fields. A small native callback descriptor supplies AppBase.onStart
parameter/result
contracts. The function IR preserves its nullable Dictionary parameter
separately
from VM slots; source onStart replaces the generated wrapper and owns its method
record. Broader native member descriptors remain upcoming work.

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
omitted in this console profile. The generated native View stays blank; entry
prints and, for integer entries, its result provide the runtime observation.
Void entries run once with their result discarded and no wrapper print;
[the void-call experiment](../investigations/compiler-void.md) verifies this path.

## Extension boundary

Source class ownership, instance receiver calls, and integer/void startup are
[implemented](../investigations/compiler-class.md). Source-authored lifecycle
requires broader constructor/receiver semantics, callback contracts,
and null/object/array values. Method records and imports should follow those
identities rather than fixed wrapper choices. [The plan](../plan.md) defines
the sequence and runnable acceptance examples. Preserve native/text equivalence
and complete-application simulator tests as each construct is added.

Jungle/manifest resolution should supply application identity, source inputs,
target constraints, and export settings once that source subset can represent
real application lifecycle files. Section-size research remains driven by
actual generated programs and concrete blockers.
