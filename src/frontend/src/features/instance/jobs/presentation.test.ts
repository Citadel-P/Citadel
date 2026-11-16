import {
  JobCategory,
  JobOutcomeView,
  JobStateView,
  JobOwnershipView,
  ResourceType,
  type JobStatusView,
} from '@/api/generated/api.types';
import { groupJobs } from './presentation';

const job = (patch: Partial<JobStatusView> = {}): JobStatusView => ({
  id: 'build-runs',
  name: 'Build runs',
  resourceType: JobCategory.Build,
  ownership: JobOwnershipView.Coordinator,
  state: JobStateView.Waiting,
  activeIterations: 0,
  iterations: 0,
  productive: 0,
  noWork: 0,
  failures: 0,
  budgetExhaustions: 0,
  productiveDurationMs: 0,
  noWorkDurationMs: 0,
  totalDurationMs: 0,
  maxDurationMs: 0,
  backlogIsLowerBound: false,
  ...patch,
});

it('prioritizes current failures, then work, without carrying historical failures forward', () => {
  const running = job({ state: JobStateView.Running });
  const failure = job({
    id: 'build-cleanup',
    state: JobStateView.Unavailable,
    lastOutcome: JobOutcomeView.Interrupted,
  });
  expect(groupJobs([running, failure])[0].state).toBe('failed');
  expect(groupJobs([running, job({ state: JobStateView.BackingOff })])[0].state).toBe('backingOff');
  const recovered = job({
    id: 'build-cleanup',
    lastFailureAt: 100,
    lastError: 'Previous failure',
    failures: 1,
    lastOutcome: JobOutcomeView.NoWork,
  });
  expect(groupJobs([running, recovered])[0].state).toBe('running');
  expect(groupJobs([recovered])[0].state).toBe('waiting');
});

it('preserves unknown coverage and inactive states without claiming health or activity', () => {
  const listener = job({ state: JobStateView.Unavailable });
  expect(groupJobs([listener, job()])[0].state).toBe('unavailable');
  expect(groupJobs([listener])[0].summary).toBe('1 unmeasured');
  expect(groupJobs([job({ state: JobStateView.Disabled })])[0].state).toBe('disabled');
  expect(groupJobs([job({ state: JobStateView.Stopped }), job({ state: JobStateView.Disabled })])[0].state).toBe(
    'stopped',
  );
  expect(groupJobs([job({ activeOperations: 2 })])[0].state).toBe('waiting');
});

it('uses backend classification even when the ID suggests a different resource', () => {
  const jobs = [
    job({ id: 'swarm-task-pruning', resourceType: JobCategory.SwarmService }),
    job({ id: 'platform-threshold-alerts', resourceType: JobCategory.Alert }),
    job({ id: 'platform-deployment-catch-up', resourceType: JobCategory.Deployment }),
    job({ id: 'renamed-worker', resourceType: JobCategory.Build }),
    job({ id: 'build-runs', resourceType: JobCategory.Maintenance }),
  ];
  expect(groupJobs(jobs).map((group) => [group.name, group.jobs.map((j) => j.id)])).toEqual([
    ['Alert', ['platform-threshold-alerts']],
    ['Build', ['renamed-worker']],
    ['Deployment', ['platform-deployment-catch-up']],
    ['Maintenance', ['build-runs']],
    ['Swarm service', ['swarm-task-pruning']],
  ]);
});

import { attention, latestResult } from './descriptions';
import { diagnostics } from './diagnostics';
import { trendPoints } from './trends';
import type { JobHistoryBucketView, JobsSnapshotView } from '@/api/generated/api.types';

it('does not infer retries, catch-up, or last-run work from cumulative counters', () => {
  expect(attention(job({ state: JobStateView.BackingOff }))).toBe('attention');
  expect(attention(job({ state: JobStateView.BackingOff, nextCheck: 123 }))).toBe('retrying');
  expect(attention(job({ backlog: 100, state: JobStateView.Waiting }))).toBeUndefined();
  expect(
    attention(job({ backlog: 100, state: JobStateView.Running, currentStartedAt: 100, lastProgressAt: 120 })),
  ).toBe('catchingUp');
  expect(latestResult(job({ lastOutcome: JobOutcomeView.Productive, completedUnits: 1200 }))).not.toContain('1200');
});

it('copies only diagnostic fields and replaces raw errors with a safe explanation', () => {
  const snapshot: JobsSnapshotView = {
    bootId: 'boot',
    startedAt: 1,
    observedAt: 2,
    revision: 1,
    historyRevision: 0,
    jobs: [
      job({
        lastError: 'password=hunter2',
        trigger: '--token=secret',
        waitingFor: 'sensitive payload',
        lastProblem: {
          at: 2,
          summary: 'sensitive payload',
          target: {
            resourceType: ResourceType.Platform,
            resourceId: '01900000-0000-7000-8000-000000000001',
            operationId: null,
          },
        },
      }),
    ],
  };
  const text = diagnostics(snapshot, undefined, false);
  expect(text).not.toMatch(/hunter2|secret|sensitive payload/);
  expect(JSON.parse(text).live).toBe(false);
  expect(JSON.parse(text).version).toBeNull();
  expect(JSON.parse(text).jobs[0].lastProblem.target.resourceId).toBe('01900000-0000-7000-8000-000000000001');
});

it('does not connect measurement gaps or divide by empty productive counts', () => {
  const bucket = {
    at: 0,
    resolutionMs: 300_000,
    productive: 2,
    productiveMs: 100,
    noWork: 10,
    noWorkMs: 20,
    gap: false,
  } as JobHistoryBucketView;
  const points = trendPoints([bucket, { ...bucket, at: 900_000, gap: true }]);
  expect(points[0]).toEqual({ at: 0, productive: 50, empty: 2 });
  expect(points[1].productive).toBeNull();
  expect(points[2].productive).toBeNull();
});

it('connects adjacent hourly and five-minute summaries without inventing gaps', () => {
  const hour = {
    at: 0,
    resolutionMs: 3_600_000,
    productive: 2,
    productiveMs: 100,
    noWork: 1,
    noWorkMs: 10,
    gap: false,
  } as JobHistoryBucketView;
  const fine = { ...hour, at: 3_600_000, resolutionMs: 300_000 };
  expect(trendPoints([fine, hour])).toHaveLength(2);
  const points = trendPoints([hour, { ...fine, at: 3_900_000 }]);
  expect(points[1]).toEqual({ at: 3_600_000, productive: null, empty: null });
});
it('shows last collection counts without substituting cumulative work or declaring operation success', () => {
  const collector = job({
    id: 'local-container-stats',
    lastOutcome: JobOutcomeView.Productive,
    lastCompletedUnits: 12,
    lastUnit: 'samples',
    completedUnits: 1200,
  });
  expect(latestResult(collector)).toBe('Collected measurements from 12 containers');
  expect(
    latestResult({ ...collector, lastOutcome: JobOutcomeView.Failed, lastCompletedUnits: 10, lastFailedUnits: 2 }),
  ).toBe('Collected 10 samples; 2 containers could not be measured');
  expect(latestResult({ ...collector, lastCompletedUnits: null })).toBe('Last check processed work');
  expect(latestResult(job({ lastOutcome: JobOutcomeView.Productive, lastCompletedUnits: 1, lastUnit: 'claims' }))).toBe(
    'Claimed 1 queued operation',
  );
});
