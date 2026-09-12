# Agent task prompts

Use the relevant template and fill in concrete values. Link to source/evidence
instead of copying conversation history or assigning every worker the full plan.
These templates define outcomes; workers choose routine implementation details.

## Numerical orchestrator

> Work in {checkout, branch, base SHA} toward {specific numerical question}.
> Current evidence: {paths and unresolved result}. Own {files/jobs}; other agents
> own {boundaries}. Use the NSBU numerical-experiment skill when designing or
> evaluating runs. Deliver {comparisons or implemented capability}, source-bound
> evidence, relevant checks and an honest remaining-gap report. Continue through
> implementation, execution, diagnosis and necessary repairs within scope.
> Budget: {UTC deadline, job/resource ceilings, archive margin}. Stop a run when
> {question-specific futility condition}; preserve failures and partial results.
> Escalate changes to mathematical contracts or acceptance interpretation to the
> {reviewer/decision owner}; continue unaffected work while awaiting that decision.
> Routine fixes and experiment scheduling do not need a review pause.

## Architecture or numerical review

> Review {diff/source SHA and evidence paths} for {specific risk or decision}.
> Check {contract/invariants affected}, independently assess whether measurements
> support the stated claim, and identify the smallest correction if they do not.
> Return findings with locations, severity and actionable next steps. Do not
> expand into unrelated code or repeat completed tests without a new concern.

## Complex tests

> Validate {contract/change} in {checkout} using {independent oracle or controls}.
> Own {test files}; coordinate production changes with {owner}. Exercise the
> meaningful failure modes and numerical limits affected by this change. Return
> exact commands/results, source identity and any untested limitation. Budget
> {time/resources}; do not launch a full coverage campaign unless it is the task.

## Narrow edit or evidence inventory

> In {checkout}, modify/check only {explicit paths} to produce {exact result}.
> Preserve {specified content}. Verify with {small relevant check}. Return the
> diff or inventory and observed result; report conflicts without overwriting them.

## Model allocation and handoffs

For the currently authorized campaign, the user selected Sol for orchestration
and bulk coding, Astra for architecture/review, Terra for complex tests and Luna
for narrow edits. Preserve that allocation unless the user changes it. Assign
independent useful work; avoid idle reviewers and duplicate investigations.
Model allocation is not a measured percentage of usage or cost.

At handoff, provide the source SHA, working changes, evidence paths, active job
ownership, deadline and next decision. Report completed checks separately from
queued checks, pilot observations and qualification. Review checkpoints should
resolve a concrete uncertainty; do not stop the whole campaign merely because
one bounded subtask is finished.

This guidance applies the context and completion recommendations in
[OpenAI's prompting article](https://developers.openai.com/blog/rethinking-skills-and-prompts-for-gpt-6-astra).
Scientific constraints come from this project's reviewed design and active plan.
