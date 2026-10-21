# Working instructions

- Read [README.md](README.md) and [docs/plan.md](docs/plan.md) before
  substantial work. Reuse context already read. Locate relevant definitions
  with targeted searches, then read narrow excerpts rather than whole files.
- Follow [docs/research.md](docs/research.md) for investigations and evidence,
  and [docs/architecture.md](docs/architecture.md) for implementation
  boundaries and coding conventions.
- Keep findings, designs, experiments, and outstanding questions in this
  repository's Markdown documentation. Update relevant docs when new evidence
  changes a conclusion. The plan covers upcoming work and discovered needs;
  architecture and specs own current structure, intent, and contracts. Keep
  investigations to essential evidence; do not maintain completed-work lists or
  document routine maintenance.
- Keep the core backlog in `docs/plan.md` as one prioritized checklist of
  specific remaining tasks. Delete completed items rather than marking them
  `[x]`; do not retain or replace them with vague ongoing obligations or
  duplicate backlogs in specs.
- Group related functionality into a coherent proposed commit when it shares
  implementation and validation. Prefer larger coherent batches that get more
  coverage from each shared test/audit run and reduce validation overhead.
  Follow backlog priorities, pulling required
  shared prerequisites forward; avoid unrelated scope or one-method increments
  when the rest of the group can be implemented and validated together.
- Follow [docs/README.md](docs/README.md) for authoritative documentation homes,
  readable prose, and concrete Rustdoc descriptions of behavior and requirements.
  Extend existing references by default; split files only when doing so improves
  readability or maintenance under that guide.
  Keep our provisional formats current-only; do not add compatibility paths for
  superseded local grammars.
- Follow [validation policy](docs/validation.md): focused checks while iterating,
  one final check for the affected scope, and concise helper verdicts. Save full
  logs under `scratch/`; inspect failures and unknown behavior. Repeat passed
  checks only after relevant changes or unresolved concerns. Do not replace
  necessary visual/semantic review with a successful process exit.
- Language/API feature work includes required support across the compiler, IR,
  VM encoding/decoding, assembler/disassembler, and affected PRG formats. Follow
  [toolchain completion](docs/plan.md#toolchain-completion); raw export fallback
  does not establish complete feature support.
- Do not commit changes. The user manages Git and commits.
- Refresh `scratch/commit_msg` after repository changes so the user can load it
  as a commit message for the current uncommitted work. Use a precise one-line
  summary, a blank line, and a concise body
  describing the proposed changes and why they were made. Include fixed ticket
  numbers when relevant; do not invent them or include unrelated changes.
- Download external reference projects into `references/`, which is ignored by
  Git. Keep temporary specimens and build output in ignored directories; record
  reproducible experiment instructions and observations in docs.
- Implement independently in Rust. Treat this repository's documented format and
  behavior as the implementation reference; avoid embedding dependencies on
  external research repositories or copying their implementation code.
- [EU Directive 2009/24/EC] permits interoperability research under its conditions
  (Articles 5(3) and 6). But do not disassemble or decompile Garmin's compiler
  or SDK.
- Preserve source attribution and distinguish observations, interpretations, and
  unverified hypotheses. Do not turn assumptions into specifications.
- Keep this file lightweight. Put detailed plans and technical specifications in
  `docs/` and link them here as needed.
- Enable workflow profiling only when the user explicitly requests it; follow
  [workflow measurement](docs/workflow-measurement.md). Normal validation stays
  enabled.
- Reuse the [workspace validation runner](scripts/README.md#workspace-validation)
  for final checks; overlap independent checks and documentation work, and use
  completion-aware waiting instead of fixed sleeps.

[EU Directive 2009/24/EC]: https://eur-lex.europa.eu/legal-content/EN/TXT/?uri=CELEX:32009L0024
