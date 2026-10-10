# Repeated method names

Two independently authored modules in separate source files declare `answer()`:
`Left` returns `21`, and `Right` returns `22`. The app prints the greeting, both
results, and their sum (`43`); the view displays `43`.

```sh
sh scripts/build-example.sh examples/module-names fr265
sh scripts/run-simulator-container.sh --prg examples/module-names/build/fr265/module-names.prg
```

The shared build script selects SDK/device data and the external developer key.
Generated artifacts stay in ignored `build/`. See [the module naming
experiment][ref-the-module-naming-experiment] for full export/import and the
independently signed Left-only edit.

[ref-the-module-naming-experiment]: ../../docs/investigations/module-method-names.md
