# The storage budget for continuous integration

GitHub Actions meters artifact and cache storage for every repository,
whether the repository is public or private, while it does not meter
execution minutes for a public repository. A repository that builds a
runtime image per platform therefore spends nothing on compute and
accumulates gigabytes of intermediate output that no later run reads.
The accumulation is silent until the account crosses its storage
allowance, at which point GitHub stops scheduling work for that account,
and the symptom resembles a platform outage rather than a billing
condition.

This document describes the storage budget that prevents that
condition, the job that enforces it, and the recovery procedure for an
account that is already over the allowance.

## The failure this document addresses

On 24 October 2026 the account `tamatebako` carried an unbilled artifact
and cache debt of 428.22 United States dollars, accrued during the year
and concentrated in two repositories. The repository `tamatebako/tebako`
alone held 6,353 artifacts totalling 273.7 GiB, and the organization held
467 active caches totalling 51.2 GiB. The repository
`tebako-packages/metanorma` held 1,511 artifacts totalling 284.1 GiB.

The consequence was not a red workflow. Starting at 15:47 UTC no job in
the organization began execution, thirteen workflow runs remained in the
queued state with zero runs in progress, and the platform status page
reported normal operation throughout. A workflow dispatched in a
different organization at 18:32 UTC completed within twenty seconds,
which located the condition in the account rather than in the platform.
The same debt produced a second and more specific symptom in a
repository whose organization was only partly affected: the job
`installer msi (lean)` of `tebako-packages/hello-runtimes` failed at the
step `actions/upload-artifact@v4`, and the skipped `publish` job that
followed left the suite unpublished.

## The job

The workflow `.github/workflows/storage-hygiene.yml` performs four steps
in one job, and it is declared reusable so that every repository calls
one implementation rather than restating the policy.

1. The retention policy is stamped on the repository through the
   artifact and log retention endpoint, so that artifacts created later
   expire without further action.
2. Artifacts created before the artifact cutoff are deleted. An artifact
   created after the cutoff is never touched, which preserves the
   intermediates of a release that is still running.
3. Caches whose last access precedes the cache cutoff are deleted. A
   cache that no run has restored carries no access time, and its
   creation time is used in its place.
4. The artifact and cache totals are measured, reported to the job
   summary, and compared with the budget. A repository over budget fails
   the job with a named error, because an over-budget repository is a
   decision for a human rather than a condition to retry.

The inputs and their default values are listed in the following table.

| Input | Default | Meaning |
| --- | --- | --- |
| `artifact_days` | `14` | Delete artifacts older than this many days. |
| `cache_days` | `14` | Delete caches untouched for this many days. |
| `retention_days` | `14` | Artifact and log retention stamped on the repository. |
| `budget_gb` | `10` | Artifacts plus caches above this fail the job. |

The job requires the `actions: write` permission and no permission over
repository contents, so it cannot alter a branch, a tag or a release
asset. Published release assets are outside its reach by construction,
because the GitHub Releases API and the artifacts API are separate
surfaces.

## Adopting the job in a repository

A repository that consumes the job adds one workflow file, pinned to a
published tag of the product repository in the same manner as a
dependency.

```yaml
name: storage-hygiene
on:
  schedule:
    - cron: "41 4 * * *"
  workflow_dispatch: {}
jobs:
  hygiene:
    uses: tamatebako/tebako/.github/workflows/storage-hygiene.yml@v2.8.62
    permissions:
      actions: write
      contents: read
```

The product repository runs the job from its own `schedule` trigger, and
a caller may pass narrower or wider cutoffs and a different budget when
its build profile justifies them.

## Recovering an account that is already over the allowance

The recovery proceeds in the order below, and each step is bounded by
the rate limit of the interface that performs it.

1. The retention policy is stamped on every active repository, so that
   the debt stops growing while it is being removed.
2. Artifacts older than one week are deleted per repository, largest
   first, until the account reaches the intended share of its allowance.
   The artifact listing reports a size per artifact, which allows the
   deletion order to follow the bytes rather than the age.
3. Caches are deleted per repository. A cache is derived data by
   definition, so its deletion cannot lose a result that the next run is
   unable to recompute.
4. Artifacts that belong to a release in flight are retained, which the
   cutoff enforces without a separate allowlist.
5. The queued runs are inspected once the debt is removed. A run that
   was created while the account was over the allowance may require a
   new trigger, because the scheduler does not necessarily resume a run
   that it declined to start.

The unbilled amount for the current billing period is a separate matter
from the storage in use: removing storage stops further accrual, and it
does not clear the amount already accrued. An account whose spending
limit is reached requires either a raised limit or a settled balance
before the scheduler resumes, and both actions belong to the billing
owner of the organization rather than to an automated job.

## Boundaries

The job does not raise a spending limit, and it does not settle an
accrued balance. It does not delete a release asset, a tag, a branch or
a workflow run, and it therefore preserves the run identifiers that a
release record cites as evidence. It does not prune an artifact created
inside the retention window, so a release that must be re-published
without a rebuild keeps its intermediates for the length of that window.
