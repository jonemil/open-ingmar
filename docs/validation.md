# Development and validation

Choose checks by the affected behavior. Use focused checks during development,
then run one final check for the affected scope. Diagnose failures before
restarting gates; repeat a passed check only after a relevant edit or an
unresolved concern. GitHub CI continues to run the full Rust and documentation
checks.

Use the [workflow measurement protocol](workflow-measurement.md) only when
profiling is requested. Keep measured tool/job time separate from unattributed
time and account for overlapping background work.

## Work cycle

1. Read the prioritized item, relevant contracts, and nearby positive and
   negative tests. Reuse existing fixtures and update obsolete rejection
   expectations as part of the feature change.
2. If SDK behavior is uncertain, test a minimal specimen early. Then implement
   the coherent feature group across its affected stages, with boundary tests.
3. Before the final gate, review the changed contracts and callback paths, run
   focused tests, and confirm the example exercises the feature.
4. Run one applicable final check with the
   [workspace runner](../scripts/README.md#workspace-validation). Its Cargo
   checks are serial and its audit lane overlaps them. While checks run, update
   documentation or review only work that cannot change check inputs. Inspect
   the final verdict and diagnostics. After a late edit, repeat only affected
   checks. Put each fact in its authoritative document: architecture for
   boundaries, specs for contracts, investigations for evidence, and plan for
   remaining work. Link rather than duplicate explanations.

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

Several categories can apply together. Retain CI's full checks as a backstop;
conditional local checks do not weaken CI. The full example audit is a final
gate for compiler and binary-toolchain changes. Reuse unchanged SDK specimens;
rebuild when their source, configuration, SDK, or target changes. Full workspace
tests need not run for documentation-only/helper-only work.

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
