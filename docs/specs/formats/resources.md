# RESOURCES: string indexes and remaining scope

Section ID: `0xf00d600d`. Evidence:
[DATA and resource investigation](../../investigations/data-and-resources.md).
Resource strings are independently indexed assets; they are not `news` literals
or symbol-name records.

## Read-only string inspection

A supported index consists of a big-endian u16 entry count followed by count ×
8-byte entries: u32 ID and u32 offset relative to the RESOURCES payload. Strings
use three index levels:

1. Top-level category `0x008000a2` points to the string-language index.
2. A language key points to that language's resource-ID index.
3. A resource ID points to a string container.

The observed string container has tag `01`, big-endian u16 UTF-8 byte length,
contents, and a zero terminator. Its format matches the observed DATA string
record, but its address belongs to RESOURCES. IDs and language keys are not
offsets.

```sh
cargo run -p ciq-dis -- resource-strings examples/minimal-console/build/fr265/minimal-console.prg
```

Output for the current Minimal Console specimen:

```text
language=0x008002e7 resource=0x00000083 @0x00001e46 "Minimal Console"
```

The parser validates table bounds, duplicate IDs within each table, string
bounds/encoding/termination, and cyclic or shared language-table offsets. Shared
language indexes are unsupported by the reader. It inspects the string category
only and leaves other categories uninterpreted. String offsets are currently
limited to the same zero-prefix 24-bit range as the shared record decoder.

The language key remains numeric. Our specimens use `0x008002e7`, whose SYMBOLS
name is `<valyrian>`; the source reference's English example uses `0x008002eb`,
named `<eng>` locally. We have not established the compiler's
default-language/fallback rules and do not label the observed key as English.

## Preservation and unsupported scope

RESOURCES remains raw binary in section projects. There is no resource string
assembler or resource relocation yet. The read-only reader can validate the
current string indexes without changing any resource bytes.

Resource-ID binding, generated `Rez` declarations, language selection/fallback,
and lossless resource assembly remain unspecified. Bitmap/font/JSON/animation
containers, background/glance indexes, and compressed/device-specific assets are
outside this inspection profile. [The plan](../../plan.md) owns research
priorities.
