# IR control flow

## Control flow

Explicit control flow uses block declarations and terminators:

```text
ciq-ir
.function choose parameters=1
.block ^0
.parameter %0 0
.integer %1 0
.equal %2 %0 %1
.branch %2 ^1 ^2
.block ^1
.integer %3 47
.return %3
.block ^2
.integer %4 49
.return %4
.end
```

- `.block` starts a block; IDs are decimal `u32`, distinct from `%` values
  and `$` locals. Block IDs must be unique; the first block is entry.
- `.equal`, `.not-equal`, `.less`, `.less-equal`, `.greater`, and `.greater-equal`
  each take `%<result> %<left> %<right>`. They follow the [source comparison domains](../frontend/operators.md#operator-contracts) and
  define Boolean. Equality/inequality also accept the String identity profile,
  Boolean pairs, and supported dictionary/null checks. Ordering rejects String,
  Boolean, null, and unproved nullable operands. Lowering selects `eq`, `ne`,
  `lt`, `lte`, `gt`, and `gte` respectively; operand evaluation remains left to
  right. See [comparison evidence](../../investigations/compiler-number-comparisons.md).
- `.branch` consumes a Boolean and
  terminates the block. The two targets may be the same block.
- `.jump` terminates the block unconditionally.

All new directives accept the existing optional source span. A body containing
block/branch/jump operations must begin with `.block`. Every block must end in
`.return`, `.branch`, `.jump`, or `.jump-value`, with no operations after its
terminator until the next block. Targets may name earlier or later blocks. Every
block must be reachable from entry; missing targets, duplicate blocks,
disconnected cycles, and implicit fallthrough are rejected. The final block
returns. A body without these operations uses an implicit single block.

Expression definitions still precede their uses, are globally unique, and have
exactly one use, restricted to their defining block. Locals can cross edges, but
reads and assignments require the unique initializer to execute on every
incoming path. Its declaration must also precede the textual use. Availability
is the fixed-point intersection described below. A local initialized in only one
arm cannot be read or assigned in a sibling or join. Typed [expression
joins](values.md#expression-joins) carry incoming values through fresh block
parameters. Other cross-block value IDs remain unsupported. Local slots are
allocated across the entire function, including mutually exclusive paths.

The 1024-operation limit includes block markers and terminators. Effectful
functions retain left-to-right expression and statement order within each block;
block markers and jumps participate in schedule validation. Branches consume
their comparisons. Ordinary edges carry no evaluation-stack values; value edges
carry exactly their parameter. Source parsing, typed construction, and text
resumption use these same checks. Source conditions use these same forward joins
with `.jump` for continuing arms, including nested statements and early returns.
Source local names/scopes are resolved before serialization; the IR only retains
globally unique local identities. See [the source
subset](../frontend/README.md).

## Cyclic control flow

Backward branch/jump targets are permitted. The graph is checked for
reachability by walking from the first block, including cycles. Having a
predecessor alone does not make a block reachable. The final declared block must
still end in `.return`; this does not prove every runtime path terminates.

Definite-local sets are computed to a greatest fixed point. Start incoming and
outgoing sets at the universe of declared local IDs. For every block, intersect
its predecessor outgoing sets to obtain its incoming set; its outgoing set adds
all its `.local` initializers. Entry additionally has an empty external
predecessor, so its incoming set is always empty. Repeat until sets stop
changing. Scan each block's operations from its final incoming set, adding
initializers only when encountered. Reads/assignments require membership at
their position and an earlier textual declaration; assignments do not add
initialization.

This accepts common initialized locals carried around a loop and rejects values
initialized only on its backedge. Local initializer IDs remain globally unique;
one declaration may execute repeatedly, resetting its existing slot each visit.
Types retain their initializer's supported classification. Expression values
still refer to earlier definitions in the same block and have one static use;
runtime execution may visit that definition/use repeatedly. No expression values
or evaluation-stack state cross an edge.

Source loops lower to these existing graph operations. Test comparisons/calls
execute again on every visit; body statements retain evaluation order. Native
construction, text resumption, and source compilation use the same validator.
See [loop source rules](../frontend/statements.md#while-loops) and
[the runtime experiment](../../investigations/compiler-loops.md).
