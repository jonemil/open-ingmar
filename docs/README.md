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
- **Design** (`design/`): responsibilities, invariants, decisions, tradeoffs,
  current structure, and intent. Explain why a component or API exists.
- **Plan** (`plan.md`): where we are heading, next work, newly discovered needs,
  and deferred directions. Do not keep a list of completed steps.
- **Specifications** (`specs/`): source languages, text syntax, binary layouts,
  archive profiles, and external protocols. Define accepted inputs, output
  rules, limits, and errors precisely enough to implement independently.
- **Investigations** (`investigations/`): questions, procedures, observations,
  provenance, and uncertainty. These provide evidence for design and formats.
- **Research conventions** (`research.md`): evidence categories, attribution,
  and requirements for reproducible investigations.
- **Rustdoc**: what an API does, what it produces, and its requirements and
  restrictions. Include relevant failure conditions and guarantees.

Design and specs describe current intent, supported behavior, and open design
questions. Investigations can retain relevant history when it explains the
evidence; they do not need a record of every completed step. The plan owns work
sequencing; link to it instead of maintaining extension orders in design pages.

Use Git history for superseded designs and implementation evolution. Our
provisional formats have one current grammar and unnumbered headers; update
their readers, writers, tests, and examples together without retaining older
profiles. Garmin format and SDK/device versions identify compatibility rules
and the scope of investigation evidence.

Specs describe our supported profiles, not complete Garmin compatibility. Keep
unknowns explicit and link to evidence instead of repeating experiments. An API
model belongs in design even when it also has a text serialization; the
serialization grammar belongs in specs.

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
Public types and
functions document the contracts needed to use each crate. Design and format
pages provide broader project context in the repository; API docs must remain
useful independently of those pages. Broken intra-doc links are build errors
through the workspace Rustdoc lint.

Published crates can use docs.rs for API documentation. Once the repository has
a public URL, package metadata can identify it without coupling Rustdoc to its
directory layout. Publishing configuration remains separate future work.

See [research conventions](research.md) for attribution and evidence rules.

[preview]: ../scripts/README.md#preview-repository-documentation
[build]: ../README.md#build-and-check
