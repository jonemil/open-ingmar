# ciq-sign

Library for PRG developer signing/verification, detached signatures, and
RSA-4096 developer/public keys. Re-signing replaces publisher signing material
with the supplied developer identity.

Start with `DeveloperKey`, `PublicKey`, `resign`, and `verify` in the
[API source](src/lib.rs). The [assembler CLI](../ciq-asm/README.md) exposes PRG
signing commands; `ciq-package` handles archive-level signature workflows.

See the [signature profile](../../docs/specs/formats/developer-signatures.md) for
coverage, key encodings, supported variants, and limits. Keep private keys in
ignored `keys/`, outside exports and packages.
