# Working instructions

- Read [README.md](README.md) and [docs/plan.md](docs/plan.md) before
  substantial work. Reuse context already read; follow relevant links rather
  than rereading the documentation tree.
- Follow [docs/research.md](docs/research.md) for investigations and evidence,
  and [docs/architecture.md](docs/architecture.md) for implementation
  boundaries.
- Keep findings, designs, experiments, and outstanding questions in this
  repository's Markdown documentation. Update relevant docs when new evidence
  changes a conclusion. The plan covers upcoming work and discovered needs;
  design describes current structure and intent. Do not maintain completed-work
  lists or document routine maintenance.
- Follow [docs/README.md](docs/README.md) for readable prose, design/spec
  boundaries, and concrete Rustdoc descriptions of behavior and requirements.
  Keep our provisional formats current-only; do not add compatibility paths for
  superseded local grammars.
- For repeated mechanical checks, reuse existing commands or add a small helper
  with meaningful pass/fail exit codes and concise diagnostics. Save large logs
  under `scratch/`; inspect relevant excerpts on failure or when investigating
  unknown behavior. Do not build helpers for trivial one-off checks or replace
  necessary visual/semantic review with a successful process exit.
- Run checks appropriate to the change and required final checks. Repeat passed
  checks only after relevant changes or when unresolved concerns justify it.
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

[EU Directive 2009/24/EC]: https://eur-lex.europa.eu/legal-content/EN/TXT/?uri=CELEX:32009L0024
