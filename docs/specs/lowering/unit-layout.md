# Identity and CODE layout

## Identity and CODE layout

Semantic function names remain unqualified; declaration ownership is separate.
Lowering emits all bodies into one assembly model in declaration order, using:

```text
global: __ciq_fn_g_<name-byte-length>_<name>
member: __ciq_fn_m_<module-byte-length>_<module>_<name-byte-length>_<name>
```

Lengths are decimal; source names are ASCII. For example, global `probe` maps to
`__ciq_fn_g_5_probe`, and `Numbers.difference` maps to
`__ciq_fn_m_7_Numbers_10_difference`. Internal block and string aliases use that
label as their prefix, preventing collisions across owners. Method metadata must
bind to the declaration's CODE label, not just its source name.

Calls still select symbolic module/member identities through `getmv` and the
existing context/argument convention. CODE labels identify bodies; they do not
replace module DATA method records. Symbol IDs and DATA literal placements
remain explicit encoder inputs. Strings are deduplicated within each function;
unit lowering does not merge literals across functions.

## Class bodies

Global/module bodies precede class bodies; each group preserves declaration
order, and methods retain class order. Class labels are
`__ciq_fn_c_<class-length>_<class>_<member-length>_<member>` with decimal byte
lengths and ASCII names. Internal aliases use that prefix. Body indexes used in
validation diagnostics follow this same order. All groups together have the
unit's 1..=256-body limit. Serialized spans remain provenance only.
