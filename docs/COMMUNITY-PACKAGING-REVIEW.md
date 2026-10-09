# Community packaging review

Status: proposal for validation, not an approved entitlement change.

This is separate from the [Community commitment](content/docs/overview/community-commitment.md).
ELv2 and the current Community/Team implementation remain unchanged.
Do not advertise proposed free features before they are implemented and released.

## Questions to validate

| Candidate | Current behavior | Proposed question |
| --- | --- | --- |
| Basic backup schedule | Scheduled backups require Team | Does a simple unattended backup complete the normal individual workflow well enough to belong in Community? |
| Basic Git-triggered deployment | Webhook reception and source synchronization are Community; mutating deployment/apply requires Team | Would one straightforward Git webhook deployment remove a major adoption obstacle without eliminating the value teams buy? |

Interview individual self-hosters and prospective paying teams separately.
Record installation/workflow completion, upgrade objections, and reasons to pay
or renew. Do not use GitHub stars or a single discussion thread as revenue or
adoption forecasts. Do not collect installation telemetry without an explicit
privacy decision and appropriate user consent.

## Before changing the paid boundary

Record the chosen scope, affected capabilities, evidence and trade-offs in a
separate reviewed decision. Identify both backend and frontend enforcement,
scheduler/webhook dispatch, upgrade/downgrade behavior and regression coverage.
Preserve existing authorization and non-destructive recovery. Do not introduce
licensing-driven architectural abstractions merely to rearrange the free tier.

Only then implement the change and align the version-specific edition matrix,
UI, website and release notes. Newly released Community functionality joins
the protected minimum; it must not later be reclaimed as paid-only.

There is no promised delivery date, revised price or free-feature announcement
in this document. Current signed product keys continue to control Team access.
