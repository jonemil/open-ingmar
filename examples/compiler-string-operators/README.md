# String operators

[app.mc](app.mc) and [strings.mc](strings.mc) exercise concatenation with
supported scalar/null values, nullable arguments, and String identity equality.
The Char examples require API 1.3.0; String/Number/Boolean/Null forms use 1.0.0.

From the repository root:

```sh
mkdir -p scratch/compiler-string-operators
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 1.3.0 \
  --app-id dda08a66fa6e4a00b7a19870f7a38943 --entry start \
  examples/compiler-string-operators/app.mc \
  examples/compiler-string-operators/strings.mc \
  scratch/compiler-string-operators/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-string-operators/unsigned.prg \
  scratch/compiler-string-operators/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-string-operators/generated.prg
```

Expected output, observed on SDK 9.2.0 / fr265:

```text
ab
n=7
7n
b=true
falseb
c=A
Ac
z=null
nullz
false
true
true
true
false
true
false
false
false
false
false
false
n=7
n=null
nulln
false
[same]
false
[null]
false
same!
false
HELLO WORLD
left
right
v=7
```

Use `.equals()` to compare String content. `==` compares identity on the
validated profile: shared references compare true, but separately created
equal-content strings compare false. See the
[contracts](../../docs/specs/frontend/operators.md#operator-contracts) and
[experiment](../../docs/investigations/compiler-string-operators.md).
