# ciq-asm

CLI for reconstructing PRGs from section projects, assembling individual
symbol/DATA sections, replacing template CODE, and signing/verifying PRGs.

From the workspace root (input paths are placeholders):

```sh
cargo run -p ciq-asm -- project scratch/project scratch/output.prg
cargo run -p ciq-asm -- sign scratch/output.prg scratch/signed.prg -y keys/developer_key.der
cargo run -p ciq-asm -- verify scratch/signed.prg -y keys/developer_key.der
```

Output files must be new. Reconstruction preserves signatures unless `-y` is
supplied. Create section projects with [ciq-dis](../ciq-dis/README.md).
See [project rules](../../docs/specs/formats/README.md#section-project),
[signing](../../docs/specs/formats/developer-signatures.md), and the
[CLI source](src/main.rs) for section/template commands.
