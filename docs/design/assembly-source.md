# Assembly source and layout constraints

Use one assembly language for handwritten code, compiler output, and
disassembly. Explicit metadata and placement constraints may differ; the
underlying declarations and references should not. Native producers can
construct the [shared assembly model](assembly-model.md) directly; text is an
optional inspection and resumption boundary.

## Identity and placement

Keep logical identity separate from storage. A symbol has an authoring alias,
a stored name, a numeric ID, and optional name-record/table placement. Strings
and class records likewise have content/identity and independent placement.
CODE labels identify instruction locations without exposing numeric offsets to
source authors.

Unpinned layout must be deterministic. Explicit pins must be honored or rejected
with a clear conflict; never silently move a pinned record. Omitting placement
does not authorize changing an API symbol ID. Automatic ID allocation requires
known namespaces and complete reference handling.

The [symbol grammar](../specs/symbol-assembly.md),
[DATA strings](../specs/data-strings.md),
[class declarations](../specs/data-classes.md), and
[CODE labels](../specs/code-labels.md) specify supported authoring rules.

## Files and reference ownership

Source-file boundaries do not imply separate linkable objects. Section projects
identify authoritative sources for each payload; inspection listings are not a
second editable source of truth. An include mechanism and its scope/path rules
remain undesigned. Assembly aliases currently occupy a flat namespace.

Use typed references for symbols, strings, classes, and CODE targets where their
meaning is established. Resolve layout-dependent references after their target
layout is known. Numeric fallback preserves unknown or ambiguous references.
Method, branch, line, and exception metadata share CODE labels rather than
maintaining separate numeric addresses.

## Preservation and relocation

Retain raw bytes for unsupported records, including any placement needed to
preserve unknown pointers. Moving known records is unsafe if opaque content may
still reference their original positions. Reject unsupported relocation rather
than emitting inconsistent output.

Retain `at=` as an explicit layout constraint. Ignoring selected pins is an open
idea, not an agreed default. Labels remove known address maintenance without
implying general relocation. Whole-program layout is sufficient for the current
source-library model; a separate linker needs a demonstrated requirement.

Handwritten and printed source use the same parser and model. Validate
reproducibility, binding failures, pin conflicts, and preservation. Exact-byte
checks apply to unchanged reconstruction; intentional edits and own-key signing
need checks of changed content, unrelated payloads, and runtime behavior.
