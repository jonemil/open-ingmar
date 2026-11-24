# Nested classes

[app.mc](app.mc) constructs a nested class through a named outer instance,
a subclass instance, the current receiver, and the qualified class. The inner
method reads its module variable even though the outer class has a field of the
same name. See [expected.txt](expected.txt).

```sh
mkdir -p scratch/compiler-nested-classes
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 3.0.0 \
  --app-id b67c1691c9a5422a9d3202a0c016870c --entry MinimalConsoleApp.onStart \
  examples/compiler-nested-classes/app.mc \
  scratch/compiler-nested-classes/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-nested-classes/unsigned.prg \
  scratch/compiler-nested-classes/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-nested-classes/generated.prg
```

SDK reference: 9.2.0, `fr265`, minimum API 3.0.0, `-O 0`, default checking
policy. Use the [minimal console scaffold](../minimal-console/README.md)
with this source file.
