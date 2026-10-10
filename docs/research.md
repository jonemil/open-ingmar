# Research and documentation conventions

## Evidence

Distinguish these categories explicitly:

- **Source-derived:** observed in a named external implementation at a pinned
  revision; not yet confirmed against Garmin output.
- **Experimentally observed:** reproduced with a recorded artifact, toolchain,
  target, and procedure.
- **Interpretation:** a conclusion drawn from evidence, including its limits.
- **Hypothesis:** an unverified assumption or proposed explanation.

Cross-check findings between implementations and controlled Garmin-generated
specimens. Treat conflicting results as open questions until resolved. Record
simulator acceptance and changed behavior separately from successful parsing or
writing.

## Investigation records

Create a Markdown record under `docs/investigations/` for each substantial
investigation or experiment. Include:

1. Date, question, and scope.
2. Source URLs, exact revisions, relevant source locations, and applicable
   licenses.
3. SDK/compiler/simulator versions, device identifier, and build options when
   used.
4. Minimal source inputs, commands, artifact hashes, and relevant byte offsets
   or hex excerpts.
5. Findings with evidence categories, uncertainty, and conflicting observations.
6. Implications for local specs or Rust APIs, and the next experiment.

Avoid committing SDK distributions, credentials, signing keys, downloaded
repositories, or incidental build artifacts. Use `scratch/` for local specimens.
If an independently redistributable fixture is needed later, deliberately choose
its location and document its origin and reproducibility.

## Specifications and design

Follow [the documentation guide](README.md) for formatting and scope. Keep
component intent and decisions in `docs/design/`, format/language/protocol
contracts in `docs/specs/`, and API guarantees in Rustdoc. Investigations should
be concise while retaining the evidence needed to reproduce their conclusions.

Explain formats and instructions in our own documentation under `docs/specs/`.
Link specifications to local evidence records; keep detailed external-source
provenance in those records. Do not present imported implementation assumptions
as facts established by our experiments.

Instruction specifications should cover the observed opcode byte, mnemonic,
operand encoding and byte order, instruction length, execution or stack behavior
if known, reference semantics, version constraints, and evidence. Mark unknown
fields and semantics explicitly.

Use independent implementations and black-box differential experiments. Do not
copy Garmin implementation code. Investigate upstream licenses before
incorporating any third-party code; the default is to document behavior and
write our own implementation.

Keep [plan.md](plan.md) current as milestones advance and
[architecture.md](architecture.md) current as evidence changes the design.
