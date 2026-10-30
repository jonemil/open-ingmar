# Global and module variables

[app.mc](app.mc) and [later.mc](later.mc) check shared mutable bindings,
dynamic and annotated contracts, Null defaults, assignment/update forms,
local/parameter shadowing, and reopened modules. Initialization markers show
that a write activates its target module before evaluating either a simple or
conditional RHS. See [expected.txt](expected.txt).

```sh
mkdir -p scratch/compiler-shared-variables
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 5.0.0 \
  --app-id 9dc7a27b1eb34adba087012221feb6f6 --entry MinimalConsoleApp.onStart \
  examples/compiler-shared-variables/app.mc examples/compiler-shared-variables/later.mc \
  scratch/compiler-shared-variables/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-shared-variables/unsigned.prg \
  scratch/compiler-shared-variables/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-shared-variables/generated.prg
```

SDK reference: 9.2.0, `fr265`, minimum API 5.0.0, `-O 0`, default checking policy.
Use the [minimal console scaffold](../minimal-console/README.md) with both source
files and minimum API 5.0.0.
