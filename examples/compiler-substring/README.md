# Nullable substring results

[app.mc](app.mc) and [slices.mc](slices.mc) exercise guarded String returns,
empty ranges, reversed/oversized ranges, negative and nullable indices,
reassignment, and loop guards. API 3.3.2 is required for the index forms used.

From the repository root:

```sh
mkdir -p scratch/compiler-substring
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 3.3.2 \
  --app-id a0715dcac3134a0186061b29872aef56 --entry start \
  examples/compiler-substring/app.mc examples/compiler-substring/slices.mc \
  scratch/compiler-substring/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-substring/unsigned.prg \
  scratch/compiler-substring/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-substring/generated.prg
```

Expected output, observed on SDK 9.2.0 / fr265:

```text
bananas
0
ll
hello
lo
HELLO
became null
hello
retry
ell
```

The tested runtime normalizes reversed and oversized ranges. Keep the nullable
result contract: these observations do not prove all inputs succeed. See the
[guard and index rules](../../docs/specs/compiler-subset.md#nullable-results)
and [experiment](../../docs/investigations/compiler-substring.md).
