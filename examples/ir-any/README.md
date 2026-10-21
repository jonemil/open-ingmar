# Dynamic semantic IR

[program.ciqir](program.ciqir) is a complete hand-authored unit using `any`
parameter/result contracts. It exercises dynamic calls, arguments, arithmetic,
indexed reads/writes, conditions, and class-test narrowing. A source-class
member checks that dynamic and declared dispatch share symbol identities.

[app.mc](app.mc) is the SDK runtime comparison. Omitted source parameter/result annotations select Any in our frontend.
The source and hand-authored IR are independent inputs to the same pipeline.
The [IR specification](../../docs/specs/ir/README.md#dynamic-values) defines the
contracts. Expected console text is in [expected.txt](expected.txt).

```sh
mkdir -p scratch/ir-any
cargo run -p ciq-compiler -- --unit --from ir --emit prg --app-id 123456789abcdef0123456789abcdef0 --entry MinimalConsoleApp.onStart examples/ir-any/program.ciqir scratch/ir-any/unsigned.prg
cargo run -p ciq-compiler -- --unit --emit prg --app-id 123456789abcdef0123456789abcdef0 --entry MinimalConsoleApp.onStart examples/ir-any/app.mc scratch/ir-any/source.prg
cargo run -p ciq-asm -- sign scratch/ir-any/unsigned.prg scratch/ir-any/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/ir-any/generated.prg
```

For an SDK reference build, point a local Jungle file's `base.sourcePath` at
this directory and use the ordinary console-app manifest/resources, as in the
[SDK helper instructions](../../scripts/README.md#build-an-example).
