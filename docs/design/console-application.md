# Minimal application generation

`ciq-compiler::application` turns a validated compilation unit into a complete
unsigned console watch application. This establishes independent metadata and
lifecycle generation before general class/project compilation. The first
[profile](../specs/console-application.md) is deliberately small and
[simulator-verified](../investigations/application-bootstrap.md).

## Boundaries

The frontend still resolves restricted global/module functions into semantic
IR. Unit validation handles declaration identities and integer call signatures.
The VM backend selects user-function instructions. Application generation then
combines those bodies with typed lifecycle assembly and constructs DATA and
container sections; signing and package/project configuration remain separate.

This avoids embedding application metadata or native import indexes in semantic
function IR. The lifecycle generator uses fixed receiver/object/array/void
operations with recorded evidence, rather than pretending the frontend already
accepts those source constructs. General source classes and typed SDK calls will
need their own semantic operations and validation before replacing the wrapper.

No SDK files, compiled PRG template, copied DATA prefix, or signing key are read
by the generator. Native identities and opaque header/entry profile values are
explicit compatibility constants. Keep unknown meanings visible and extend
profiles only with evidence; they are not universal device/format rules.

## Layout and identity

User symbols are assigned deterministically; module lookup identities remain
separate from plain member names. Existing unit CODE labels bind all methods.
Generated fields are sorted by numeric key because the runtime experiment found
an out-of-order method unresolvable. The shared DATA reader/writer still preserves
existing record order for lossless work.

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

## Next extension

Make lifecycle behavior source-authored: define a minimal AppBase/View class
subset, receiver/context rules, and the required void/null/object/array and SDK
call types. Generate corresponding method records and imports from those
identities instead of fixed wrapper choices. Preserve native/text equivalence
and complete-application simulator tests as each construct is added.

Jungle/manifest resolution should supply application identity, source inputs,
target constraints, and export settings once that source subset can represent
real application lifecycle files. Section-size research remains driven by
actual generated programs and concrete blockers.
