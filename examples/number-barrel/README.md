# Number barrel

A code-only Monkey Barrel providing `Numbers.answer()` (`40 + 6`). The source matches the module in [multi-file](../multi-file/README.md). [barrel-app](../barrel-app/README.md) consumes it through either an archive or the source project.

```sh
sh scripts/build-barrel.sh examples/number-barrel
```

Output: `build/number-barrel.barrel`, an ignored archive containing source and build metadata. The builder refuses an existing output, since the SDK can report failure with a successful process exit status. Move an old archive aside before rebuilding.

The helper uses the same `CIQ_SDK_HOME`, `CIQ_DATA_HOME`, and Java discovery as the app builder. A barrel archive does not require a developer signing key. `fr265.personality = resources` overrides automatic device personality input: this plain-number library does not use personality styles, and the barrel packager rejects device resources outside its project root. Do not apply this override blindly to libraries that use styles.
