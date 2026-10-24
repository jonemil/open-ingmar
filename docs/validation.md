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

1. Implement a coherent feature group and its boundary tests. Read the relevant
   contracts and nearby tests first; probe uncertain SDK behavior early.
2. Run focused tests and review the change. Reuse the
   [shared compiler test helpers](../crates/ciq-compiler/tests/common/mod.rs) for
   pipeline checks and the affected complete application. Finish semantic review before the final gate.
3. Start the applicable final checks with the
   [workspace runner](../scripts/README.md#workspace-validation). Run the slow
   audit speculatively alongside workspace tests and the remaining Cargo checks;
   do not wait for the entire Rust lane to pass first. Finish documentation while
   checks run, then inspect their verdicts and diagnostics.

After a code fix, rerun focused tests and the affected example. Repeat the full
example audit only if the fix affects shared behavior across examples or its
scope is uncertain. Retain other passing results. Documentation-only edits need
documentation checks. A simulator rerun is unnecessary when the final signed PRG
is byte-identical to the already tested artifact.

The shared Rust helper checks saved IR, direct/resumed assembly equality,
assembly text, unsigned PRG generation, CODE decode/re-encode, and a complete
DATA walk. Use its pipeline check for feature cases and its application check
for complete examples. It does not replace semantic assertions, malformed-input
tests, SDK/CLI reconstruction, signing, or runtime acceptance in the audit and
specimen helpers. Extend existing tests rather than duplicating the full audit
in Rust or adding a separate preflight checklist.

## Check selection

- **Rust production code:** focused tests while iterating, then workspace tests,
  Clippy, and formatting once.
- **Rust tests/test helpers only:** affected test targets, Clippy, and formatting;
  no full example audit unless toolchain behavior also changes.
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
Keep simulator sessions serial. Use the runner's completion-aware `wait` in its
container instead of fixed sleeps and status-poll loops. Reuse its reports and
concise verdicts; inspect detailed logs on failure. Do not generate a final-check
script for each feature.
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
