# Monkey C frontend

The frontend parses a restricted Monkey C profile, resolves declarations and
native names, and constructs validated semantic IR. This is a supported subset,
not a replacement definition of the complete Monkey C language.

- [Grammar](grammar.md): accepted syntax, literals, and precedence.
- [Declarations](declarations.md): imports, compilation units, and name lookup.
- [Types](types.md) and [numeric values](numeric-values.md): contracts,
  assignability, nullability, and scalar domains.
- [Expressions and operators](operators.md): result types, tests, and casts.
- [Statements](statements.md): scopes, updates, control flow, and returns.
- [Classes](classes.md) and [fields](fields.md): receiver identity and storage.
- [Core Lang values](lang-values.md), [Arrays](containers.md),
  [Dictionaries](lang-dictionary.md), [ByteArrays](lang-bytearray.md), and
  [interfaces](interfaces.md): executable core-library contracts.

The default source entry point accepts one function. Unit mode collects multiple
files and owners before checking cross-function references. Neither mode
resolves Jungle projects. Diagnostics identify input byte offsets and one-based
line and column; columns count Unicode scalar values. Callers supply filenames.
Unsupported input is diagnosed rather than silently producing partial functions.

Source contracts are independent of [IR serialization](../ir/grammar.md),
[VM lowering](../lowering/README.md), and [application generation](../build/application.md).
Native API declarations and higher-level application callbacks belong in the
[SDK specifications](../sdk/README.md). The [plan](../../plan.md) owns remaining
language and library work, including exceptions and source inheritance.
