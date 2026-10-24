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

The [validation policy](validation.md#work-cycle) owns the development sequence,
parallel checks, and retry rules. Measure that workflow; do not add checks or
per-task collectors solely for profiling. Keep run-specific results under
`scratch/workflow/` rather than maintaining another workflow here.
