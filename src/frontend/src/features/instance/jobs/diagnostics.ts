import type { JobsSnapshotView, JobHistoryView } from '@/api/generated/api.types';
// Explicit allowlist. Raw errors, trigger strings and resource payloads are never copied.
export function diagnostics(snapshot: JobsSnapshotView, history: JobHistoryView | undefined, live: boolean) {
  return JSON.stringify(
    {
      observedAt: snapshot.observedAt,
      bootId: snapshot.bootId,
      startedAt: snapshot.startedAt,
      live,
      core: history?.sessions.find((s) => s.snapshot.bootId === snapshot.bootId)?.coreId ?? null,
      version: history?.sessions.find((s) => s.snapshot.bootId === snapshot.bootId)?.version ?? null,
      jobs: snapshot.jobs.map((j) => ({
        id: j.id,
        state: j.state,
        lastOutcome: j.lastOutcome,
        lastStartedAt: j.lastStartedAt,
        lastCompletedAt: j.lastCompletedAt,
        lastFailureAt: j.lastFailureAt,
        iterations: j.iterations,
        failures: j.failures,
        completedUnits: j.completedUnits,
        elapsedMs: j.totalDurationMs,
        lastElapsedMs: j.lastDurationMs,
        lastCompletedUnits: j.lastCompletedUnits,
        lastFailedUnits: j.lastFailedUnits,
        lastProblem: j.lastProblem
          ? {
              at: j.lastProblem.at,
              target: j.lastProblem.target
                ? {
                    resourceType: j.lastProblem.target.resourceType,
                    resourceId: j.lastProblem.target.resourceId,
                    operationId: j.lastProblem.target.operationId,
                    containerId: j.lastProblem.target.containerId,
                  }
                : null,
            }
          : null,
        backlog: j.backlog,
        backlogIsLowerBound: j.backlogIsLowerBound,
        error: j.lastError ? 'A worker failure was reported; inspect authorized Core logs.' : null,
      })),
      historyObservedAt: history?.observedAt ?? null,
      sessions: history?.sessions.map((s) => ({
        coreId: s.coreId,
        bootId: s.snapshot.bootId,
        version: s.version,
        startedAt: s.snapshot.startedAt,
        heartbeatAt: s.heartbeatAt,
        stoppedAt: s.stoppedAt,
        lostEvents: s.lostEvents,
      })),
      events: history?.events.map(({ bootId, event }) => ({
        bootId,
        sequence: event.sequence,
        jobId: event.jobId,
        at: event.at,
        kind: event.kind,
        target: event.problem?.target
          ? {
              resourceType: event.problem.target.resourceType,
              resourceId: event.problem.target.resourceId,
              operationId: event.problem.target.operationId,
              containerId: event.problem.target.containerId,
            }
          : null,
      })),
      buckets: history?.buckets.map((b) => ({
        bootId: b.bootId,
        jobId: b.jobId,
        at: b.at,
        through: b.through,
        resolutionMs: b.resolutionMs,
        iterations: b.iterations,
        productive: b.productive,
        noWork: b.noWork,
        failures: b.failures,
        elapsedMs: b.elapsedMs,
        productiveMs: b.productiveMs,
        noWorkMs: b.noWorkMs,
        units: b.units,
        backlog: b.backlog,
        backlogIsLowerBound: b.backlogIsLowerBound,
        gap: b.gap,
      })),
    },
    null,
    2,
  );
}
