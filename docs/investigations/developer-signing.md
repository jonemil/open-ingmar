# Developer signing and a same-size instruction edit

Date: 2026-10-08. Target: SDK 9.2.0, Forerunner 265 (`fr265`). Runtime validation of the changed program remains pending.

## Sources and independent observations

Research references were inspected under ignored `references/`:

- [monkeyc-optimizer signer](https://github.com/markw65/monkeyc-optimizer/blob/2920dc88c23caadded9f66ee0613a7de6ab2d1ea/src/readprg/signer.ts), revision `2920dc88c23caadded9f66ee0613a7de6ab2d1ea`, MIT: coverage rules, key fields, optional SHA-256 signature, and store-signing exclusion.
- [MonkeyC Ghidra PRG notes](https://github.com/Gualdimar/MonkeyC-Ghidra-Plugin/blob/12ae55010721165b4ba98254d5e77db22eaa1b07/doc/prg.md), revision `12ae55010721165b4ba98254d5e77db22eaa1b07`, Apache-2.0: signature payload layout.
- Primary cryptographic API references: [RustCrypto RSA](https://docs.rs/rsa/0.9.10/rsa/), [SHA-1](https://docs.rs/sha1/), and [SHA-256](https://docs.rs/sha2/).

The Rust implementation was written independently against [our local specification](../specs/developer-signatures.md). It does not depend on either research checkout. Cryptographic primitives use `rsa` 0.9.10, `sha1` 0.10.7, `sha2` 0.10.9, and `rand_core` 0.6.4 with OS randomness. These are the workspace's first external Rust dependencies; format parsing remains standard-library-only.

Both existing SDK specimens contain a final 1540-byte developer signature with a 512-byte modulus and exponent 65537. We independently checked signatures using Python public-RSA arithmetic: compute `pow(signature, exponent, modulus)`, serialize to 512 bytes, and compare the entire PKCS#1 v1.5 encoded message with the digest's standard DigestInfo and padding. SHA-1 DigestInfo prefix: `3021300906052b0e03021a05000414`; SHA-256 prefix: `3031300d060960864801650304020105000420`. No private-key arithmetic was used for that independent check.

Concatenating ordered non-signature wrappers and payloads, excluding the terminator, verifies both signatures in both specimens. Signed-message lengths are 120632 bytes for Minimal Console and 121112 for Hello World. Original specimen SHA-256 hashes:

| Specimen | PRG length | SHA-256 |
|---|---|---|
| Minimal Console | 122188 | `cd361b76dc29e828d43f0511df3bebd96dc63ce6b7be61b70e2be8e1092d95e7` |
| Hello World | 122668 | `8608ed040f18294a2a9af870f53e26e64a0d2fe3ef8c032d600043b591d182d7` |

Re-signing unchanged Minimal Console with the local developer key reproduces its entire original file byte for byte. This confirms interoperability with the SDK's signing output for this specimen. The private key remains ignored and is not a test fixture.

## Reproduction and modified artifact

Use fresh output paths; the tools refuse to overwrite existing files:

```sh
cargo run -p ciq-asm -- verify examples/minimal-console/build/fr265/minimal-console.prg -y developer_key.der
cargo run -p ciq-asm -- verify examples/hello-world/build/fr265/hello-world.prg -y developer_key.der
cargo run -p ciq-asm -- sign examples/minimal-console/build/fr265/minimal-console.prg scratch/signing/minimal-resigned.prg -y developer_key.der
cmp examples/minimal-console/build/fr265/minimal-console.prg scratch/signing/minimal-resigned.prg
cargo run -p ciq-dis -- all examples/hello-world/build/fr265/hello-world.prg scratch/signing/hello-42-project
```

In the exported CODE assembly (`003-code-c0debabe.mcasm`), change the `ipush1 0x29` at CODE offset `0x3a` to `ipush1 0x2a`. The one-byte operand at CODE offset `0x3b` changes decimal 41 to 42. The enclosing `probe()` function starts at `0x30`. Then:

```sh
cargo run -p ciq-asm -- project scratch/signing/hello-42-project scratch/signing/hello-42.prg -y developer_key.der
cargo run -p ciq-asm -- verify scratch/signing/hello-42.prg -y developer_key.der
```

Observed output: 122668 bytes, 13 sections, SHA-256 `8774c0ab49d7d1aecca35d900ece700bb2ef3efae04e31a21ab8506a94aecc3d`. Rust verification and independent Python public-RSA checks both pass. Comparison against the original establishes:

- CODE changes at exactly one byte, offset `0x3b` (absolute PRG offset `0x33ac`).
- Both 512-byte signature blocks change; modulus and exponent remain identical because the same key was used.
- Every other section is byte identical and section order is preserved.

Ignored `scratch/signing/inspect-signatures.py` and `signature-observations.json` hold the local independent-check script and observations. The algorithm, inputs, and expected hashes above allow reproduction without checking in those artifacts or private keys.

## Automated checks and remaining runtime check

`cargo test --workspace --quiet` passes 52 tests; `cargo clippy --workspace --all-targets -- -D warnings` passes. Signing tests use fresh ephemeral RSA keys, not the user's key. They cover both signature variants, unchanged deterministic re-signing, different-key replacement, tampered code/signatures/exponents, malformed input, store-signature removal, raw-section preservation, project/template workflows, verify, invalid keys, and overwrite refusal. The legacy 1028-byte variant is tested synthetically; no claim of legacy simulator validation is made.

Attempting the simulator launcher here stops because Podman is unavailable. Run on the host:

```sh
sh scripts/run-simulator-container.sh --prg scratch/signing/hello-42.prg
```

Expected: console `Hello from Monkey C!` and `42`, with the captured screen showing `42`. Inspect fresh `scratch/simulator/monkeydo.log`, `simulator.log`, and `screen.png`; older baseline captures showing `41` do not validate this output. For desktop interaction:

```sh
sh scripts/run-simulator-container.sh --interactive --prg scratch/signing/hello-42.prg
```

Until that host run confirms execution, cryptographic validity and preserved layout are established, but changed-bytecode runtime acceptance remains unverified. Do not proceed to length-changing relocation or replacement functions based solely on passing signature checks.
