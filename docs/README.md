# Documentation conventions

The [root README](../README.md) introduces the project and indexes its
documentation. This page defines how to write and organize that documentation.

## Where information belongs

- **Root README**: project overview, getting started, and links to further
  reading. Keep detailed contracts, architecture, and research procedures in
  their own pages; brief summaries should link to those authoritative pages.
- **Crate READMEs** (`crates/<name>/README.md`): purpose, API entry points or
  brief CLI usage, and links to detailed contracts. Keep API documentation in
  Rustdoc; do not duplicate format specifications here.
- **Architecture** (`architecture.md`): crate responsibilities, dependencies,
  shared pipelines, and implementation boundaries.
- **Plan** (`plan.md`): where we are heading, next work, newly discovered needs,
  and deferred directions. Do not keep a list of completed steps.
- **Specifications** (`specs/`): authoritative subsystem contracts, including
  design intent, invariants, rationale, languages/formats, supported behavior,
  limits, and errors. Mark planned requirements separately from implemented rules.
  Define enough detail to implement independently without investigation notes.
- **Investigations** (`investigations/`): compact evidence records: question,
  environment/input, procedure, observed outcome, provenance, and uncertainty.
  Link to current contracts instead of repeating implementation descriptions.
- **Research conventions** (`research.md`): evidence categories, attribution,
  and requirements for reproducible investigations.
- **Rustdoc**: what an API does, what it produces, and its requirements and
  restrictions. Include relevant failure conditions and guarantees.

Architecture describes intended component relationships; specs own detailed
contracts and the reasons needed to understand them. There is no separate design
layer. The plan owns upcoming work; do not maintain completed-step lists or
extension sequences in other pages. Examples own runnable commands/expected
output; crate Rustdoc owns concrete API use.

The [specifications index](specs/README.md) groups contracts by frontend, IR,
lowering, assembly, VM, SDK, build, and container formats. Keep accepted Monkey
C, IR, and assembly syntax in their respective grammar files. Semantic pages own
behavior and validation, with links to grammar productions; binary-format pages
own encoded records rather than another copy of editable assembly syntax.

Within each specification category, start with one general reference
(`README.md`) and a separate grammar where there is a textual language. Use
headings and a short contents list to keep the general reference searchable.
File length, an implementation helper module, or a new feature alone does not
justify another document. Split only for an independently used contract or
reader workflow with little shared material; keep the reason clear in the
category overview. Consolidate shared rules before creating parallel files.

Give each rule one authoritative home. Other pages may introduce it briefly and
link there. Shared source/IR validation contracts should be defined once, with
representation-specific syntax and lowering documented where they belong. Keep
provisional formats current-only; Git history retains superseded designs.

Specs describe supported profiles, not complete Garmin compatibility. Preserve
unknowns and distinguish intended requirements from experimentally established
rules. API/SDK/device versions scope compatibility evidence, not our local
format history. Investigations are supporting evidence, not required
implementation references or a parallel feature inventory. They can be retired
when the spec, tests, and retained provenance adequately capture their value.

## Writing conventions

Wrap prose and list items at about 80 columns. Keep paragraphs focused on one
idea. Use short headings and small tables; use lists when table rows become too
wide for a terminal. Leave commands, code, hashes, and URLs intact where
wrapping would change their meaning. Use reference links for long URLs.

Write precisely and briefly. Put the current contract or conclusion first.
Include examples when they clarify a rule. Keep evidence needed to reproduce an
investigation, but omit conversational history and repeated status reports. Do
not record routine cleanup of ignored files.

Start function documentation with a short, concrete description: what it does,
what input or format it handles, and what it returns or produces. Then describe
relevant requirements, restrictions, errors, or guarantees about how it
operates. Do not replace that description with an abstract rationale inferred
from the implementation. Explain non-obvious choices when useful, without
narrating each line of code. Use intra-doc links for related APIs. Crate Rustdoc
must work with ordinary Cargo builds and when published, without repository
files or a custom documentation build. Do not link to repository-relative
Markdown or separately generated HTML from Rustdoc. External research links
belong in investigations.

## Rendering and API documentation

Keep relative links between repository Markdown pages so they work in
GitHub/GitLab and local previews. See [preview setup and limits][preview] for
browser review and local link checking.

Standard Cargo/Rustdoc builds produce API pages under
`target/doc/<crate_name>/index.html`; see the [root build instructions][build].
Public types and functions document the contracts needed to use each crate.
Specifications and architecture provide broader project context in the
repository; API docs must remain useful independently of those pages. Broken
intra-doc links are build errors through the workspace Rustdoc lint.

Published crates can use docs.rs for API documentation. Once the repository has
a public URL, package metadata can identify it without coupling Rustdoc to its
directory layout. Publishing configuration remains separate future work.

See [research conventions](research.md) for attribution and evidence rules.

[preview]: ../scripts/README.md#preview-repository-documentation
[build]: ../README.md#build-and-check
