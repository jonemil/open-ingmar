# Documentation

Start with [architecture](architecture.md) for component boundaries and
[the plan](plan.md) for current work.

## Where information belongs

- **Design** (`design/`): responsibilities, invariants, decisions, tradeoffs,
  and intended evolution. Explain why a component or API exists.
- **Specifications** (`specs/`): source languages, text syntax, binary layouts,
  archive profiles, and external protocols. Define accepted inputs, output
  rules, limits, and errors precisely enough to implement independently.
- **Investigations** (`investigations/`): questions, procedures, observations,
  provenance, and uncertainty. These provide evidence for design and formats.
- **Rustdoc**: what an API does, what it produces, and its requirements and
  restrictions. Include relevant failure conditions and guarantees.

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
relevant requirements, restrictions, errors, or guarantees about how it operates.
Do not replace that description with an abstract rationale inferred from the
implementation. Explain non-obvious choices when useful, without narrating each
line of code. Use intra-doc links for related APIs. Crate Rustdoc must work with
ordinary Cargo builds and when published, without repository files or a custom
documentation build. Do not link to repository-relative Markdown or separately
generated HTML from Rustdoc. External research links belong in investigations.

## Reading and building documentation

Read the root README and linked Markdown pages directly in the repository, in
a terminal or through GitHub/GitLab's Markdown rendering. Keep relative links
between repository pages; no separate documentation generator is required.

For local browser review, run `sh scripts/preview-docs.sh` and open
`http://localhost:8000/README.md`. The optional preview renders Markdown as
you browse and picks up edits on refresh. `--check` verifies local files and
heading links. See [preview setup and limits][preview].
This does not change the Markdown sources or the crate documentation build.

Build crate API documentation with standard Cargo:

```sh
cargo doc --workspace --no-deps
```

The API pages are under `target/doc/<crate_name>/index.html`. Public types and
functions document the contracts needed to use each crate. Design and format
pages provide broader project context in the repository; API docs must remain
useful independently of those pages. Broken intra-doc links are build errors
through the workspace Rustdoc lint.

Published crates can use docs.rs for API documentation. Once the repository has
a public URL, package metadata can identify it without coupling Rustdoc to its
directory layout. Publishing configuration remains separate future work.

See [research conventions](research.md) for attribution and evidence rules.

[preview]: ../scripts/README.md#preview-repository-documentation
