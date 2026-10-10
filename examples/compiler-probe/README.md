# Compiler probe

A standalone source input for our restricted Rust Monkey C frontend:

```sh
mkdir -p scratch/compiler-example
cargo run -p ciq-compiler -- examples/compiler-probe/probe.mc scratch/compiler-example/probe.mcasm
```

This emits a labeled five-byte function returning `41`. It is not a complete watch-app project and does not have a Jungle/manifest. [The compiler experiment](../../docs/investigations/first-compiler.md) documents explicit integration with Hello World's application scaffold, own-key signing, and simulator runs for `41` and a changed source returning `42`.

See [the supported grammar](../../docs/specs/compiler-subset.md) and [IR design](../../docs/design/compiler-ir.md). Keep generated assembly and binaries under ignored `scratch/`.
