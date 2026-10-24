# Workflow measurement

Profile only when the user explicitly asks. Normal validation is unchanged.
For a timed turn, use one measurement window from task start to completion or
interruption. Prefer the session's existing timing data and the validation
runner's `--timings`; do not add a collector or script for a single run.

## Record

Keep reports in ignored `scratch/workflow/<task>/`. Record:

- Total elapsed time.
- Foreground tool time by purpose: code read/edit, docs or investigation
  read/edit, reference lookup, validation, results inspection, and waiting.
- Background checks separately by category, duration, and result. Show overlap;
  do not add parallel durations to elapsed time.
- Failed attempts and repeated checks, with a short cause when known.
- Time outside measured tool and job intervals as **unattributed**.

Do not reconstruct private reasoning or call unattributed time “reasoning.” It
can include model processing, argument generation, queueing, communication,
compaction, and orchestration. Token snapshots are optional context, not a
subscription-usage calculation. Do not copy prompts, source, or reasoning into
the report.

## Findings from recent runs

The Null Array, nested Array, and source Array turns took 11m40s, 18m35s, and
29m54s. Measured foreground calls took 1m08s, 2m16s, and 2m00s; checks and
background work covered about 4m52s, 9m23s, and 8m22s, with overlap. The
remaining 6m48s, 9m12s, and 21m32s is unattributed, not a measurement of pure
reasoning.

The full example audit took about 4m13s–4m33s and workspace tests about
2m18s–2m35s. Those checks matter, but repeating a full audit or workspace gate
after an avoidable failure is a visible cost. The recent runs also found stale
negative tests for newly supported behavior and late-discovered contract gaps.

## Workflow changes

- Before implementation, inspect the prioritized item, its consumer paths, and
  nearby positive and negative tests. Remove or update obsolete rejection
  expectations in the same change.
- For uncertain SDK behavior, validate one minimal accepted specimen early;
  then build the coherent feature group and its boundary cases together.
- Before the final gate, review the changed contracts and callbacks, ensure the
  focused checks pass, and confirm the example exercises the behavior.
- Run one final workspace runner for the affected scope. Keep the full example
  audit for compiler/format changes, but do not repeat it for unrelated edits.
  After a late edit, rerun only checks that edit can invalidate. A simulator
  rerun is unnecessary when the final signed PRG is proven byte-identical to the
  already tested artifact.
- While checks run, work only on documentation or review that cannot change
  their inputs. Use the runner's completion-aware wait; do not poll or sleep in
  a loop.

These observations justify reducing retries and redundant gates; they do not
show that Rust checks should be removed or that measured tool time explains the
unattributed part of a turn.
