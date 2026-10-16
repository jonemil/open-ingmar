# Workflow measurement

Measure where a development turn spends elapsed time before changing the work
cycle. Use existing session metadata and timers inside helpers; avoid extra
agent calls just to record timestamps. Keep raw events and reports in ignored
`scratch/workflow/`. This protocol defines collection; a report generator is
not implemented yet.

## Activity categories

Classify by purpose, separately from the tool used to perform it.

| Category | Includes |
|---|---|
| `read.code`, `edit.code` | Rust, tests, scripts, example sources/configuration |
| `read.docs`, `edit.docs` | README, instructions, architecture, specs, plan |
| `read.investigation`, `edit.investigation` | `docs/investigations/` |
| `reference` | External documentation, reference projects, SDK data |
| `rust.tests` | Focused tests and workspace tests |
| `rust.build` | Cargo build/check, including the minimum Rust version |
| `rust.lint-format` | Clippy and formatting |
| `rust.docs` | Rustdoc generation |
| `sdk.build` | Official SDK compilation of authored examples |
| `simulator` | Launch, loading, observation, capture, cleanup |
| `example.audit` | Compilation/stage parity and PRG reconstruction audit |
| `docs.check` | Markdown links/rendering checks |
| `results.inspect` | Reading validation reports or failure logs |
| `repo.inspect` | Git status/diff and general repository inspection |
| `poll-wait` | Status polling, sleep, waiting for running jobs |
| `communication` | AgentMessage: progress updates and final responses |
| `other`, `mixed`, `unknown` | Remaining work or insufficient evidence |

Investigation paths take precedence over other Markdown. Rustdoc embedded in
Rust files is `edit.code` with a `rustdoc` tag. Transport names such as
`functions.exec`, `exec_command`, `apply_patch`, and `broker_exec` are separate
fields, as are the container/session and job identifiers.

Split batched work only when child timings exist. Otherwise retain one `mixed`
span with its constituent categories; do not divide time equally or assign it
all to the first command. Record classification as explicit, inferred, or
unknown. Failed commands and retries retain their category and outcome.

## What to record

Use one measurement window per user-triggered turn, from task start to final
completion or interruption. Link multiple turns when they belong to one change.
Record:

- Start/end, elapsed duration, completion/interruption, model and effort.
- Per operation: ID/parent ID, category, classification confidence, tool,
  start/end, duration, success/exit status, and foreground/background execution.
- For checks: scope, first run/retry, and job/process identity. Preserve separate
  timings for SDK compilation, simulation, Rust checks, and example audits even
  when one helper runs them all.
- Input/output byte counts where available, plus truncation, to identify reads
  that force large amounts of material back into context.
- Reported input, cached-input, output, and reasoning-token counts, with their
  response/turn scope. Do not add cumulative snapshots together or assume that
  reasoning tokens are additional to an output total that already includes them.

Prefer monotonic durations inside helpers and UTC timestamps for the timeline.
Cross-container timelines need aligned clocks; otherwise flag uncertain overlap.
Record metadata rather than copies of source, prompts, responses, or reasoning.
Token counts are workload evidence, not a calculation of subscription limits.

## Time accounting

Report these separately:

1. **Turn elapsed time:** the user's total wait.
2. **Foreground tool time:** the union of recorded tool intervals. Also show
   summed durations by category; parallel calls can make sums exceed elapsed time.
3. **Background job time:** actual start/end for each job and its subprocesses.
   The short call launching a background job does not measure the job's runtime.
4. **Explicit polling/waiting:** call count and elapsed time, linked to the job
   being awaited. Do not add a sleep and the concurrent job duration as if they
   were sequential work.
5. **Communication:** recorded message-emission intervals and message count.
   These do not include all preparation of those messages.
6. **Unattributed/model-orchestration time:** gaps outside reliable foreground
   tool/message intervals. These can contain model processing, reasoning,
   tool-argument generation, queue/network delay, compaction, and orchestration.
   Background work may also be running during these gaps.

Show overlaps explicitly. A check running while the agent edits documentation
has both durations but contributes only their union to the wall-clock timeline.
Only report exclusive waiting for a check when the trace establishes that the
agent was blocked on it; otherwise report overlap and uncertainty. Parent wrapper
spans and child commands must not both be added to the same elapsed-time total.

## Reasoning and phase attribution

Exact internal thinking time is not established by the available session log.
In the inspected session, Reasoning items have nearly instantaneous recorded
spans; those are event records, not usable timers. Report reasoning-token counts
when present, separately from elapsed time.

For phase comparisons, use coarse labels: understanding/design, implementation,
verification/debugging, documentation, and orchestration. Attach a phase to work
when known, preferably through metadata in an existing call. Gaps associated with
that phase are estimates of model/orchestration time, not measurements of pure
reasoning about that subject. Keep unassignable gaps unknown; do not reconstruct
or store private reasoning text to classify them.

Codex documents tool durations, turn durations, and token metrics through
[OpenTelemetry][otel]. Availability depends on the running client and collection
configuration. The current local rollout also has task, tool, message, and token
events. Treat that event schema as an implementation detail and preserve missing
or unreliable fields as unknown rather than manufacturing measurements.

## Report and first collection

The report should give total elapsed time, a category table with counts,
successes/failures, summed duration and interval coverage, followed by background
overlap, explicit waiting, communication, and unattributed time. Include phase
estimates and token counts only with their confidence and measurement limits.

Start by post-processing the existing rollout metadata. Add subprocess timers
to validation helpers so background checks become visible, then measure several
comparable changes. Include collector overhead in `other`. Compare repeated
reads, failed/retried calls, model gaps, and blocking checks before changing the
validation policy. Lines changed are context, not a measure of task complexity.

[otel]: https://developers.openai.com/codex/config-advanced/#observability-and-telemetry
