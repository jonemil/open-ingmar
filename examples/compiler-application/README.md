# Independently generated console application

[hello.mc](hello.mc) prints a greeting and returns `41`. Our compiler generates
its application class, lifecycle CODE, module/method DATA, native imports, and
container fields. No SDK-generated PRG or section payload is used.

From the repository root, with fresh outputs:

```sh
mkdir -p scratch/compiler-application
cargo run -p ciq-compiler -- --unit --emit prg --app-id \
  3c94c9e5ba2c43fa849d64ecff33264e examples/compiler-application/hello.mc \
  scratch/compiler-application/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-application/unsigned.prg \
  scratch/compiler-application/hello.prg -y keys/developer_key.der
cargo run -p ciq-asm -- verify scratch/compiler-application/hello.prg -y \
  keys/developer_key.der
sh scripts/run-simulator-container.sh --prg \
  scratch/compiler-application/hello.prg
```

Expected console output:

```text
Hello from the Rust compiler!
41
```

The application opens a blank native View. Its startup wrapper invokes `main()`
once and prints the returned integer. Editing the source return to `42` and
recompiling changes that result. This bootstrap profile is tested with SDK 9.2.0
and `fr265`; it does not yet compile source-authored lifecycle classes.

To generate an application from the existing two-file unit, select `probe`:

```sh
cargo run -p ciq-compiler -- --unit --emit prg --app-id \
  3c94c9e5ba2c43fa849d64ecff33264e --entry probe \
  examples/compiler-unit/entry.mc examples/compiler-unit/numbers.mc \
  scratch/compiler-application/unit-unsigned.prg
```

After signing/running, it prints `1`, `2`, `47`, then result `48` with generated
module metadata. Source and saved unit IR produce identical unsigned PRGs with
the same application options. See [CLI/profile rules][profile] and
[runtime evidence][evidence]. Signing keys remain under ignored `keys/`.

[profile]: ../../docs/specs/build/application.md
[evidence]: ../../docs/investigations/application-bootstrap.md
