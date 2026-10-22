# Development and validation

Choose checks by the affected behavior. Keep feedback quick during development,
then validate the final change once. A failed check needs diagnosis; a passing
check needs repetition only after a relevant edit or an unresolved concern.
GitHub CI continues to run the full Rust and documentation checks.

Use the [workflow measurement protocol](workflow-measurement.md) when profiling
development time. Separate tool/job timings from estimated model time and account
for overlapping background work before changing these checks.

## Work cycle

1. Read the next planned increment and relevant contracts. Search for definitions
   before reading narrow source excerpts; reuse context already available.
2. For uncertain SDK behavior, establish one minimal accepted/rejected specimen
   before expanding it. Keep adjacent discoveries in the plan rather than
   automatically widening the change.
3. Implement the affected stages and run focused tests. After parser/AST edits,
   run existing nesting/depth tests early; after frame allocation changes, run
   slot-boundary tests early. Preserve the supported limits.
4. Once implementation is stable, start the applicable final checks below with
   the [workspace runner](../scripts/README.md#workspace-validation). Keep Cargo
   checks serial; run the freshly built example audit alongside them. Prepare
   documentation and the final summary while checks run, then inspect the verdict.
   Update each fact in its authoritative
   document: architecture for component boundaries, specs for contracts and
   rationale, investigations for evidence, plan for remaining work. Link instead
   of repeating explanations.

## Check selection

- **Rust implementation:** relevant tests while iterating, then affected crate
  tests. Finish with workspace tests, Clippy, and formatting once.
- **Language/API, lowering, VM or PRG formats:** focused native/text and malformed
  input tests, then affected specimen acceptance and the full example audit.
- **Public APIs or Rustdoc:** build Rustdoc with the broken-link lint.
- **Rust syntax, dependencies, features, or toolchain metadata:** check compilation
  with the declared minimum Rust version.
- **Markdown:** read changed prose and check local links.
- **Python/shell helpers:** focused helper tests, then exercise changed success
  and failure paths and the relevant real integration.

Several categories can apply together. Retain CI's full checks as a backstop; conditional local
checks do not weaken CI. The full example audit remains a final gate for compiler
and binary-toolchain changes while the implementation is developing. Reuse
unchanged SDK specimens; rebuild when their source, configuration, SDK, or target
changes. Full workspace tests need not run for documentation-only/helper-only work.

See [build commands](../README.md#build-and-check),
[script commands](../scripts/README.md), and
[toolchain completion](plan.md#toolchain-completion) for exact checks. Required
CODE/DATA raw fallback is still a coverage failure. Simulator acceptance requires
the intended console or visible behavior, separately from encoding and signing.

## Mechanical checks and diagnostics

Use existing helpers for repeated workflows. The
[console specimen checker](../scripts/README.md#console-specimen-check) combines
saved-stage equivalence, SDK/generated structured reconstruction, signing, and
expected-output comparison across the Rust and SDK environments. It does not
replace visual checks for interactive apps.

Keep complete logs and artifacts in ignored `scratch/` directories while
validating. Routine output should give a verdict and report/log location;
inspect focused diagnostics
on failure or when investigating unknown behavior. Preserve subprocess failure
statuses and use fresh capture directories so stale output cannot pass a check.
Batch independent checks when practical, while keeping simulator sessions serial.
Use the runner's completion-aware `wait` in its container instead of fixed sleep
and status-poll loops. Reuse its per-job timing report; do not generate another
final-check script for each feature. Changes after launching checks require
repeating only the checks affected by those changes.
Do not add a helper for a trivial one-off command or repeat it manually after its
equivalent automated check passes.

## Scratch retention

After validation for a proposed commit, inspect failures and fallback diagnostics,
record conclusions needed in the relevant documentation, and report the outcome.
Then remove that change's temporary builds, copied executables, section exports,
simulator captures, one-off scripts, and logs. Do not wait for the user to commit.
Keep unresolved evidence until it has been investigated or made reproducible;
do not delete files used by active commands or a preview session.

Preserve `scratch/commit_msg` and inputs that are useful across changes:

- `scratch/garmin-data/`: downloaded device profiles, fonts, and simulator data.
- `scratch/sdk-cache/`: reusable SDK-built example PRGs and `sdk-index.json` for
  `--sdk-index`. Rebuild affected entries when their inputs/settings change.
- `scratch/official-docs/` and `scratch/sdk-manager/`: downloaded references and
  retained local investigation notes.
- `scratch/tools/`, when present: installed helper runtimes needed by scripts.
- `scratch/markdown-preview/`: the reusable Python environment for docs preview.

Treat these as retained local data, not disposable validation output. SDK
installations, keys, reference repositories, and Cargo's shared `target/` cache
also stay outside per-change cleanup. A Git ignore rule alone does not mean a
directory is safe to delete. Keep reusable SDK specimens in the cache rather
than retaining whole audit directories for their inputs.
