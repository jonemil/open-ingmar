# View/delegate input counter

SDK 9.2.0 / Forerunner 265, 2026-10-14. The independently authored
[input counter][example] returns a View/delegate pair. Both the SDK-built PRG
and our generated, own-key-signed PRG respond to three simulated touchscreen
selections with counter values 8, 9, 10. Each selection requests a redraw of the
same retained View; both final captures visibly show 10 without an error dialog.
This establishes the bounded callback/retention path, not general GC lifetime,
other event classes, false-result fallback, or other devices/app profiles.

## Public contracts

Official documentation reviewed 2026-10-14:

- [BehaviorDelegate] inherits InputDelegate. `initialize()` exists since API
  1.0.0; `onSelect()` takes no arguments and returns Boolean since API 1.2.0.
  True indicates that selection was handled; Start/Enter or a tap typically
  triggers selection.
- [WatchUi.requestUpdate][WatchUi] requests onUpdate for the current View,
  returns Void, and exists since API 1.0.0.

The example's minimum is 2.1.0 because View construction needs that initializer;
our selected target API is 5.2.0. The SDK probe uses the existing watch-app
manifest/resources under ignored `scratch/native-names/` (manifest minimum 3.0).

## PRG observations

Inspection of our SDK-compiled source PRG establishes these symbol IDs:

| Name | ID |
|---|---|
| BehaviorDelegate | `0x00800064` |
| requestUpdate | `0x008001a2` |
| onSelect | `0x008003f3` |

The delegate record at DATA+`0x4be` has parent globals and extends
`0x400000f8`, matching its BehaviorDelegate import index. Its base field has
symbol `0x00800064`, bits `0x70`, type `0x07`, and the same import reference.
The target reference defaults to null (bits/type/value zero); source initialize
fills it. onSelect is a bits `0x10`, type `0x06` method reference.
Our reduced import table puts BehaviorDelegate at index two, using
`0x40000002` for both extends and the base field. Runtime acceptance establishes
that this remapping works in the specimen.

The exporter preserves SDK CODE as raw binary: decoding stops at CODE+`0xc5`
on an unsupported opcode. We did not need to interpret that opcode to generate
and validate the pair using our existing allocation/element-store instructions.
Our generated CODE decodes completely. Metadata and simulator behavior, rather
than a claimed complete SDK instruction comparison, support these conclusions.

Artifact SHA-256 values from this run (SDK signed PRG, then our unsigned PRG):

```text
01adf563de75338e1a75022c4146f72afdec6ba9415304e8c83ae6439c2f621a
e18c73f0a05e4c6c3f31b4ba94ea57da586f1deb01096e49c81ecc0890f64b7d
```

## Reproduction and observations

Build/sign our PRG using [the example's commands][example]. Build the SDK probe
with `scratch/compiler-input-counter/monkey.jungle`:

```text
project.manifest = ../native-names/manifest.xml
base.sourcePath = ../../examples/compiler-input-counter
base.resourcePath = ../native-names/resources
```

```sh
CIQ_JUNGLE_FILES="$PWD/scratch/compiler-input-counter/monkey.jungle" \
  sh scripts/build-example.sh scratch/compiler-input-counter
cargo run -p ciq-dis -- all \
  scratch/compiler-input-counter/build/fr265/compiler-input-counter.prg \
  scratch/compiler-input-counter/sdk-export
xvfb-run -a -s '-screen 0 1280x1024x24' \
  python3 scripts/check-input-counter.py --prg <program.prg> \
  --output-dir <new-capture-directory>
```

Run the SDK/generated PRGs separately. The helper uses existing X11/XTest
libraries to tap root coordinate `(322, 466)` three times, waiting for initial
`draw / 7` and each selection/redraw. Coordinates assume this device skin at the
simulator's default top-left placement on an unmanaged Xvfb display. The check
returns nonzero on missing output, incorrect selection count, or session failure;
visual review remains separate.

Observed console sequence for both specimens:

```text
start
draw
7
draw
7
select
8
draw
8
select
9
draw
9
select
10
draw
10
```

Initial/periodic redraw count can vary; drawing does not mutate state. Retained
initial probe captures are `sdk-input.{log,png}` and
`generated-input.{log,png}` under `scratch/compiler-input-counter/`.
Source/unit-IR/assembly paths produce equivalent complete unsigned PRGs;
malformed slot order/types, constructor-base claims, callback signatures,
reference initialization, and API availability are checked independently.

[example]: ../../examples/compiler-input-counter/README.md
[BehaviorDelegate]: https://developer.garmin.com/connect-iq/api-docs/Toybox/WatchUi/BehaviorDelegate.html
[WatchUi]: https://developer.garmin.com/connect-iq/api-docs/Toybox/WatchUi.html#requestUpdate-instance_function
