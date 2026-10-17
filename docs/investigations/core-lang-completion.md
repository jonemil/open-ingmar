# Core Lang completion review

Reviewed 2026-10-17 against current source contracts and the public API.
This is a documentation/coverage review; no new simulator evidence was collected.

**Officially documented:** the [Lang index][lang] lists ByteArray, ResourceId,
and the Exception family alongside the scalar/container/reference classes.
It also defines Comparable/Comparator, numeric aliases, NumberFormat/Endian,
and the module-level format function. These belong in the core inventory rather
than being implicitly postponed with broader SDK modules.

Own-method headings on the public Number, Long, Float, Double, Boolean, Char,
String, Symbol, Array, Dictionary, Method, and WeakReference pages were compared
with the compiler's declaration/executable profiles. The scalar own-method
names largely have implementations; their bounded argument/storage contracts
and inherited methods prevent a claim of complete compatibility. Array sorting
and source/self Method binding are present; the previous deferred-method notes
were stale on those points.

**Interpretation:** remaining work is dominated by shared object/type/container
machinery and untouched Lang facilities, rather than a long list of unimplemented
numeric methods. Method constructor support still requires class/module receiver
values. ByteArray encoding options require constants and Symbol-keyed option
contracts. Exception methods require construction and source exception handling.
ResourceId needs a minimal resource-reference fixture; it does not justify
starting full resource/project generation.

The [remaining contract inventory][inventory] and [completion gate][gate] own
current requirements. Full completion needs a final public-API/operator-table
reconciliation and SDK/generated evidence; this review is not an exhaustive
operand-pair or exception-behavior audit. Keep SDK-rejected constructs distinct
from unsupported valid language/API forms.

[lang]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang.html
[inventory]: ../specs/api-declarations.md#deferred-core-methods
[gate]: ../plan.md#core-completion-gate
