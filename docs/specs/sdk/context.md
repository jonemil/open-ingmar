# API checking inputs

## API context inputs

`--target-api <major.minor.patch>` and `--min-api <major.minor.patch>` enable
introduction-level checks for source or IR input, in function and unit modes.
Supply both or neither. Each component is decimal `u16`; both levels must be at
least 1.0.0, and the minimum must not exceed the target. Malformed, duplicate,
partial, or inconsistent inputs fail before output creation. Assembly input
rejects these options because semantic API references are no longer available.

Known unguarded references must be available on the target and introduced no
later than the application's minimum API. Errors distinguish unknown members,
target unavailability, an insufficient minimum, and unsupported lowering. For
example, Number's `toChar` requires 1.3.0: target 1.0.0 reports unavailable;
target 1.3.0 with minimum 1.0.0 reports an insufficient minimum; both at 1.3.0
permit the conversion. A `has` predicate does not yet establish an API guard;
calls remain subject to these introduction-level checks.

```sh
cargo run -p ciq-compiler -- --target-api 1.0.0 --min-api 1.0.0 \
  --emit ir input.mc scratch/function.ciqir
cargo run -p ciq-compiler -- --from ir --target-api 1.0.0 --min-api 1.0.0 \
  scratch/function.ciqir scratch/function.mcasm
```

Levels are external build inputs, absent from `ciq-ir`/`ciq-unit` and PRG
deployment metadata. Resupply them when resuming text. Omitting both performs
the existing bounded contract/lowering checks without an API-level claim. These
inputs do not select a device, derive a level from the SDK, or check
permissions, app kind, execution scope, or VM capabilities. See the
[API declaration design](declarations.md) for coverage.
