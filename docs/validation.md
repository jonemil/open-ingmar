# Development and validation

Choose checks by the affected behavior. Keep feedback quick during development,
then validate the final change once. A failed check needs diagnosis; a passing
check needs repetition only after a relevant edit or an unresolved concern.
GitHub CI continues to run the full Rust and documentation checks.

## Work cycle

1. Read the next planned increment and relevant contracts. Search for definitions
   before reading narrow source excerpts; reuse context already available.
2. For uncertain SDK behavior, establish one minimal accepted/rejected specimen
   before expanding it. Keep adjacent discoveries in the plan rather than
   automatically widening the change.
3. Implement the affected stages and run focused tests. After parser/AST edits,
   run existing nesting/depth tests early; after frame allocation changes, run
   slot-boundary tests early. Preserve the supported limits.
4. Run the applicable final checks below. Update each fact in its authoritative
   document: design for structure, spec for contracts, investigation for evidence,
   plan for remaining work. Link instead of repeating explanations.

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

Keep complete logs and artifacts in ignored `scratch/` directories. Routine
output should give a verdict and report/log location; inspect focused diagnostics
on failure or when investigating unknown behavior. Preserve subprocess failure
statuses and use fresh capture directories so stale output cannot pass a check.
Batch independent checks when practical, while keeping simulator sessions serial.
Do not add a helper for a trivial one-off command or repeat it manually after its
equivalent automated check passes.
