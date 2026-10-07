# Working instructions

- Read [README.md](README.md) and [docs/plan.md](docs/plan.md) before substantial work.
- Follow [docs/research.md](docs/research.md) for investigations and evidence, and [docs/architecture.md](docs/architecture.md) for implementation boundaries.
- Keep findings, designs, experiments, and outstanding questions in this repository's Markdown documentation. Update relevant docs when new evidence changes a conclusion.
- Do not commit changes. The user manages Git and commits.
- Download external reference projects into `references/`, which is ignored by Git. Keep temporary specimens and build output in ignored directories; record reproducible experiment instructions and observations in docs.
- Implement independently in Rust. Treat this repository's documented format and behavior as the implementation reference; avoid embedding dependencies on external research repositories or copying their implementation code.
- [EU Directive 2009/24/EC] permits interoperability research under its conditions
  (Articles 5(3) and 6). But do not disassemble or decompile Garmin's compiler
  or SDK.
- Preserve source attribution and distinguish observations, interpretations, and unverified hypotheses. Do not turn assumptions into specifications.
- Start with PRG parsing, lossless writing, and a small instruction modification validated in Garmin's official simulator. Do not start a Monkey C frontend, LLVM backend, or full VM prematurely.
- Keep this file lightweight. Put detailed plans and technical specifications in `docs/` and link them here as needed.

[EU Directive 2009/24/EC]: https://eur-lex.europa.eu/legal-content/EN/TXT/?uri=CELEX:32009L0024
