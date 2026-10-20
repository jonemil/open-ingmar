# Object containers

[app.mc](app.mc) exercises Object and nullable Object Array elements and
Dictionary values. Mixed scalar constants retain their runtime classes;
computed literals store source objects, nested containers, ByteArrays, Methods,
WeakReferences, and Symbols. It checks guarded recovery, mutation, snapshots,
generic container methods, enum erasure, and strong ownership through removal
and `values()` enumeration.

API minimum: 3.1.0; target: 5.2.0; SDK/compiler/simulator: 9.2.0 / fr265.

From the repository root, using unused output paths:

```sh
mkdir -p scratch/compiler-object-containers
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 3.1.0 \
  --app-id 9c77dc1202384df19b7be6ddc92a48bc --entry MinimalConsoleApp.onStart \
  examples/compiler-object-containers/app.mc scratch/compiler-object-containers/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-object-containers/unsigned.prg \
  scratch/compiler-object-containers/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh \
  --prg scratch/compiler-object-containers/generated.prg
```

Compare the complete console with [expected.txt](expected.txt). See the
[container contracts](../../docs/specs/frontend/README.md#object-containers) and
[evidence](../../docs/investigations/compiler-object-containers.md).
