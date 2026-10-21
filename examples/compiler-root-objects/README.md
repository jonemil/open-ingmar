# Object-rooted source classes

[objects.mc](objects.mc) defines an implicit Object-rooted Counter and an
explicit `Lang.Object` comparator through a module alias. [app.mc](app.mc)
checks independent fields, aliases, inherited observers, retained Method
receivers, custom comparator dispatch, and exact cross-file source references
through parameters, returns, retained fields, and replacement. EmptyToken,
DefaultFields, and the comparator omit explicit initializers, checking default
construction, nullable field defaults, and independent mutable fields.
The [expected output](expected.txt)
matches SDK/generated `fr265` programs.

From the repository root:

```sh
mkdir -p scratch/compiler-root-objects
cargo run -p ciq-compiler -- --unit --emit prg --target-api 5.2.0 --min-api 5.0.0 --app-id 949ff3758b28492cb54821459b9c90fa --entry MinimalConsoleApp.onStart examples/compiler-root-objects/app.mc examples/compiler-root-objects/objects.mc scratch/compiler-root-objects/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-root-objects/unsigned.prg scratch/compiler-root-objects/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-root-objects/generated.prg
```

Object initialization has no native base call; omitted initializers become
zero-argument Void methods in IR. Exact source annotations/reference fields retain
class identity
and its native base; opaque Object erasure loses source-member access. See the
[class contract](../../docs/specs/frontend/README.md#object-rooted-classes).
