# Static member SDK reference

This SDK-only specimen establishes static storage and receiver behavior before
compiler support is added. Our source compiler currently rejects `static`.
It reuses the [minimal console](../minimal-console/README.md) manifest/resources;
all source in [App.mc](source/App.mc) is independently authored.

```sh
sh scripts/build-example.sh examples/static-members
sh scripts/run-simulator-container.sh --prg examples/static-members/build/fr265/static-members.prg
```

Environment: SDK/simulator 9.2.0, `fr265`, minimum API 3.0.0, `-O 0`.
Compare the complete console with [expected.txt](expected.txt).

The initial zeroes show that reading `Counter` as a class value does not run its
static initializer. Reading `Counter.literal` returns its metadata default `7`
and activates the runtime initializer (`Trace.LOG` becomes `2`). A nullable
uninitialized static defaults to Null. Static mutation is shared by both
instances; their allocation expressions run separately (`Trace.LOG` becomes
`233`), while static initialization runs once.

Static `self` differs from the class value: `Counter.owner() == Counter` prints
false. Both `new Lang.Method(Counter.owner(), :add)` and `instance.method(:add)`
invoke the static target. To reproduce the isolated failure, replace the first
receiver with `Counter`: construction succeeds, but invocation crashes with
`Unexpected Type Error`. Do not include that change when comparing normal
expected output. See [receiver and encoding evidence](../../docs/investigations/compiler-root-objects.md#static-storage-and-receivers).
