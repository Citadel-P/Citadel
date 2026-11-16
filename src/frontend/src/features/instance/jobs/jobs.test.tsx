import { useEffect } from 'react';
import { act, screen, waitFor, within } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import type { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import type { RealtimeConnection } from '@/lib/realtime-connection';
import {
  JobOwnershipView,
  JobCategory,
  JobStateView,
  JobOutcomeView,
  ResourceType,
  type JobStatusView,
  type JobsSnapshotView,
  type JobHistoryBucketView,
} from '@/api/generated/api.types';
import { InstanceJobs } from './index';
import { JobTrends } from './trends';

const realtime = vi.hoisted(() => ({
  listeners: new Map<string, (value: JobsSnapshotView) => void>(),
  groups: new Set<string>(),
  connected: true,
  invoke: vi.fn<(...args: unknown[]) => Promise<unknown>>(),
  fail: undefined as undefined | ((error: unknown) => void),
}));
vi.mock('@/hooks/useRealtimeGroup', () => ({
  useRealtimeGroup: (options: Parameters<typeof useRealtimeGroup>[0]) => {
    const { groupName, setupEventListeners, removeEventListeners, onJoinedGroup } = options;
    realtime.fail = options.onSubscriptionError;
    const connected = realtime.connected;
    useEffect(() => {
      if (!connected) return;
      const connection = {
        on: (name: string, handler: (value: JobsSnapshotView) => void) => realtime.listeners.set(name, handler),
        off: (name: string) => realtime.listeners.delete(name),
        invoke: realtime.invoke,
      } as unknown as RealtimeConnection;
      realtime.groups.add(groupName as string);
      setupEventListeners(connection);
      onJoinedGroup?.(connection);
      return () => {
        realtime.groups.delete(groupName as string);
        removeEventListeners?.(connection);
      };
    }, [groupName, setupEventListeners, removeEventListeners, onJoinedGroup, connected]);
    return { isConnected: realtime.connected, isLoading: false };
  },
}));
const endpoint = 'http://localhost/api/v1/instance/jobs';
const job = (patch: Partial<JobStatusView> = {}): JobStatusView => ({
  id: 'statistics-compaction',
  name: 'Optimize older monitoring history',
  state: JobStateView.Waiting,
  resourceType: JobCategory.Maintenance,
  ownership: JobOwnershipView.Coordinator,
  activeIterations: 0,
  iterations: 3,
  productive: 1,
  noWork: 1,
  failures: 0,
  budgetExhaustions: 1,
  productiveDurationMs: 0,
  noWorkDurationMs: 0,
  totalDurationMs: 40,
  maxDurationMs: 20,
  backlogIsLowerBound: false,
  trigger: 'After completion: 300s when caught up / 60s with work remaining',
  lastOutcome: JobOutcomeView.NoWork,
  ...patch,
});
const snapshot = (patch: Partial<JobsSnapshotView> = {}): JobsSnapshotView => ({
  bootId: 'old-core',
  startedAt: Date.now() - 60_000,
  observedAt: Date.now(),
  revision: 1,
  historyRevision: 0,
  jobs: [job()],
  ...patch,
});

it('dims empty work-trend charts with a centered No data label but plots measured zero durations', () => {
  const session = { coreId: 'core', heartbeatAt: 0, lostEvents: 0, version: 'dev', snapshot: snapshot() };
  const bucket: JobHistoryBucketView = {
    bootId: session.snapshot.bootId,
    jobId: 'statistics-compaction',
    at: 0,
    through: 300_000,
    resolutionMs: 300_000,
    productive: 0,
    productiveMs: 0,
    noWork: 0,
    noWorkMs: 0,
    elapsedMs: 0,
    iterations: 0,
    failures: 0,
    gap: false,
    backlogIsLowerBound: false,
  };
  const view = (buckets: JobHistoryBucketView[]) => (
    <JobTrends buckets={buckets} sessions={[session]} formatDateTime={String} />
  );
  const { container, rerender } = renderCitadel(view([]));
  for (const buckets of [[], [bucket], [{ ...bucket, productive: 1, gap: true }]]) {
    rerender(view(buckets));
    expect(screen.getByRole('status')).toHaveTextContent('No data');
    expect(screen.getByRole('status')).toHaveClass('absolute', 'inset-0', 'items-center', 'justify-center');
    const chart = container.querySelector('[data-slot="chart"]');
    expect(chart).toHaveClass('opacity-30', 'pointer-events-none');
    expect(chart).toHaveAttribute('inert');
  }
  for (const measured of [
    { ...bucket, productive: 1 },
    { ...bucket, noWork: 1 },
  ]) {
    rerender(view([measured]));
    expect(screen.queryByText('No data')).not.toBeInTheDocument();
    expect(container.querySelector('[data-slot="chart"]')).not.toBeNull();
    expect(container.querySelector('[data-slot="chart"]')).not.toHaveAttribute('inert');
  }
});
const update = (value: JobsSnapshotView) =>
  act(async () => {
    realtime.listeners.get('InstanceJobsUpdated')?.(value);
  });
beforeEach(() => {
  server.use(http.get('http://localhost/api/v1/profile/preferences', () => HttpResponse.json({})));
  server.use(
    http.get(`${endpoint}/history`, () =>
      HttpResponse.json({ observedAt: Date.now(), sessions: [], events: [], buckets: [], truncated: false }),
    ),
  );
  realtime.connected = true;
  realtime.listeners.clear();
  realtime.groups.clear();
  realtime.invoke.mockReset().mockResolvedValue(undefined);
});

it('links a dated reported problem to the exact build run without claiming it recovered after a scheduler check', async () => {
  const buildId = '01900000-0000-7000-8000-000000000001';
  const runId = '01900000-0000-7000-8000-000000000002';
  const lastProblem = {
    at: Date.now() - 60_000,
    summary: 'The build controller reported an error.',
    target: { resourceType: ResourceType.Build, resourceId: buildId, operationId: runId },
  };
  server.use(http.get(endpoint, () => HttpResponse.json(snapshot({ jobs: [job({ lastProblem })] }))));
  const view = renderCitadel(<InstanceJobs />);
  await view.user.click(await screen.findByRole('button', { name: 'Maintenance jobs' }));
  await view.user.click(screen.getByRole('button', { name: 'View last reported problem' }));
  expect(screen.getByRole('link', { name: /Open build run/ })).toHaveAttribute(
    'href',
    `/builds/edit/${buildId}?runId=${runId}#runs`,
  );
  await update(
    snapshot({
      revision: 2,
      jobs: [job({ lastProblem, lastOutcome: JobOutcomeView.Productive, lastCompletedUnits: 1, lastUnit: 'claims' })],
    }),
  );
  expect(screen.getByLabelText('Last reported problem')).toHaveTextContent('does not confirm recovery');
  expect(screen.getByRole('link', { name: /Open build run/ })).toBeVisible();
  await update(
    snapshot({
      revision: 3,
      jobs: [
        job({
          lastOutcome: JobOutcomeView.Failed,
          lastProblem: { at: Date.now(), summary: 'The worker reported a failed check.', target: null },
        }),
      ],
    }),
  );
  expect(screen.queryByRole('link', { name: /Open build run/ })).not.toBeInTheDocument();
  expect(screen.getByLabelText('Last reported problem')).toHaveTextContent('The worker reported a failed check.');
});

it.each([
  [ResourceType.Deployment, null, 'deployment', '/deployments/edit/affected-resource'],
  [ResourceType.Stack, null, 'stack', '/stacks/edit/affected-resource'],
  [
    ResourceType.Platform,
    'affected-container',
    'container',
    '/platforms/affected-resource/containers/affected-container',
  ],
  [ResourceType.SwarmService, null, 'swarm service', '/swarm-services/edit/affected-resource'],
  [ResourceType.Script, null, 'script', '/scripts/edit/affected-resource'],
] as const)(
  'links known %s problems in live details and retained history',
  async (resourceType, containerId, label, path) => {
    const lastProblem = {
      at: Date.now(),
      summary: 'A resource operation could not be confirmed.',
      target: { resourceType, resourceId: 'affected-resource', containerId, operationId: 'original-operation' },
    };
    server.use(
      http.get(endpoint, () => HttpResponse.json(snapshot({ jobs: [job({ lastProblem })] }))),
      http.get(`${endpoint}/history`, () =>
        HttpResponse.json({
          observedAt: Date.now(),
          sessions: [],
          buckets: [],
          truncated: false,
          events: [
            {
              bootId: 'old-core',
              event: {
                sequence: 1,
                jobId: 'statistics-compaction',
                at: Date.now(),
                kind: 'problem',
                problem: lastProblem,
              },
            },
          ],
        }),
      ),
    );
    const { user } = renderCitadel(<InstanceJobs />);
    await user.click(await screen.findByRole('button', { name: 'Maintenance jobs' }));
    await user.click(screen.getByRole('button', { name: 'View last reported problem' }));
    expect(screen.getByRole('link', { name: `Open affected ${label}` })).toHaveAttribute('href', path);
    await user.click(screen.getByRole('button', { name: 'History and work trends' }));
    const dialog = await screen.findByRole('dialog');
    expect(await within(dialog).findByRole('link', { name: `Open affected ${label}` })).toHaveAttribute('href', path);
    await update(snapshot({ revision: 2, jobs: [job({ lastProblem: null })] }));
    expect(within(dialog).getByRole('link', { name: `Open affected ${label}` })).toHaveAttribute('href', path);
  },
);

it('keeps a recorded problem linked to its original platform in retained history', async () => {
  const platformId = '01900000-0000-7000-8000-000000000001';
  server.use(http.get(endpoint, () => HttpResponse.json(snapshot())));
  server.use(
    http.get(`${endpoint}/history`, () =>
      HttpResponse.json({
        observedAt: Date.now(),
        sessions: [],
        buckets: [],
        truncated: false,
        events: [
          {
            bootId: 'old-core',
            event: {
              sequence: 1,
              jobId: 'statistics-compaction',
              at: Date.now() - 60_000,
              kind: 'failed',
              problem: {
                at: Date.now() - 60_000,
                summary: 'Platform inventory could not be refreshed.',
                target: { resourceType: ResourceType.Platform, resourceId: platformId, operationId: null },
              },
            },
          },
        ],
      }),
    ),
  );
  const { user } = renderCitadel(<InstanceJobs />);
  await user.click(await screen.findByRole('button', { name: 'Maintenance jobs' }));
  await user.click(screen.getByRole('button', { name: 'Optimize older monitoring history' }));
  await user.click(screen.getByRole('button', { name: 'History and work trends' }));
  const link = await screen.findByRole('link', { name: 'Open affected platform' });
  expect(link).toHaveAttribute('href', `/platforms/edit/${platformId}`);
  expect(within(screen.getByRole('dialog')).getByText('Platform inventory could not be refreshed.')).toBeVisible();
});

it('starts the job stream on join and reconnect so cards and expanded rows receive new observations', async () => {
  const initial = snapshot();
  const requests = vi.fn(() => HttpResponse.json(initial));
  server.use(http.get(endpoint, requests));
  realtime.invoke.mockImplementation(async (method) => {
    expect(method).toBe('StartInstanceJobs');
  });
  const view = renderCitadel(<InstanceJobs />);
  await view.user.click(await screen.findByRole('button', { name: 'Maintenance jobs' }));
  await view.user.click(screen.getByRole('button', { name: 'Optimize older monitoring history' }));
  expect(realtime.invoke).toHaveBeenCalledTimes(1);
  await update(initial);
  await update(snapshot({ revision: 2, jobs: [job({ state: JobStateView.Running })] }));
  expect(
    within(screen.getByLabelText('Job summary')).getByText('Running').parentElement?.parentElement,
  ).toHaveTextContent('1');
  expect(screen.getAllByText('After completion')).toHaveLength(2);

  realtime.connected = false;
  view.rerender(<InstanceJobs />);
  expect(screen.getByText('Showing last known state. Live observations are unavailable.')).toBeVisible();
  realtime.invoke.mockImplementation(async () => {
    realtime.listeners.get('InstanceJobsUpdated')?.(
      snapshot({ revision: 3, jobs: [job({ state: JobStateView.Disabled })] }),
    );
  });
  realtime.connected = true;
  view.rerender(<InstanceJobs />);
  await waitFor(() => expect(realtime.invoke).toHaveBeenCalledTimes(2));
  expect(screen.getAllByText('Not scheduled')).toHaveLength(2);
  expect(screen.queryByText('Showing last known state. Live observations are unavailable.')).not.toBeInTheDocument();
  expect(requests).toHaveBeenCalledTimes(1);
});

it('uses real snapshots, local filters and accessible details without refresh or polling', async () => {
  const requests = vi.fn(() =>
    HttpResponse.json(
      snapshot({
        jobs: [
          job(),
          job({
            id: 'build-runs',
            resourceType: JobCategory.Build,
            name: 'Dispatch build work',
            activeOperations: 2,
          }),
        ],
      }),
    ),
  );
  server.use(http.get(endpoint, requests));
  const { user, unmount } = renderCitadel(<InstanceJobs />);
  await screen.findByRole('button', { name: 'Maintenance jobs' });
  expect(screen.queryByRole('button', { name: /Refresh|Run now|Pause|Cancel/ })).not.toBeInTheDocument();
  await user.click(screen.getByRole('button', { name: 'Maintenance jobs' }));
  await user.click(screen.getByRole('button', { name: 'Optimize older monitoring history' }));
  expect(screen.getByRole('button', { name: 'Optimize older monitoring history' })).toHaveAttribute(
    'aria-expanded',
    'true',
  );
  await update(
    snapshot({
      revision: 2,
      jobs: [
        job({ completedUnits: 7, unit: 'productive compaction passes' }),
        job({
          id: 'build-runs',
          resourceType: JobCategory.Build,
          name: 'Dispatch build work',
          activeOperations: 2,
        }),
      ],
    }),
  );
  await user.click(screen.getByText('Technical details'));
  expect(screen.getByText('7 productive compaction passes')).toBeVisible();
  await user.type(screen.getByRole('searchbox'), 'build');
  await screen.findByRole('button', { name: 'Dispatch build work' });
  expect(screen.queryByRole('button', { name: 'Optimize older monitoring history' })).not.toBeInTheDocument();
  const summary = screen.getByLabelText('Job summary');
  expect(within(summary).getByText('Needs attention').parentElement).toHaveTextContent('0');
  expect(within(summary).getByText('Status unavailable').parentElement).toHaveTextContent('0');
  expect(requests).toHaveBeenCalledTimes(1);
  unmount();
  expect(realtime.groups.size).toBe(0);
  expect(realtime.listeners.size).toBe(0);
});

it('discards a late HTTP response, older revisions and old Core updates', async () => {
  let finish!: () => void;
  const pending = new Promise<void>((resolve) => {
    finish = resolve;
  });
  const requested = vi.fn();
  server.use(
    http.get(endpoint, async () => {
      requested();
      await pending;
      return HttpResponse.json(snapshot());
    }),
  );
  const { queryClient, user } = renderCitadel(<InstanceJobs />);
  await waitFor(() => expect(requested).toHaveBeenCalled());
  await update(snapshot({ revision: 5, jobs: [job({ id: 'future-worker', name: 'Latest worker' })] }));
  await act(async () => {
    finish();
  });
  await waitFor(() => expect(queryClient.isFetching()).toBe(0));
  await user.click(screen.getByRole('button', { name: 'Maintenance jobs' }));
  expect(screen.getByRole('button', { name: 'Latest worker' })).toBeVisible();
  await update(snapshot({ revision: 2 }));
  expect(screen.getByRole('button', { name: 'Latest worker' })).toBeVisible();
  await update(
    snapshot({
      bootId: 'new-core',
      revision: 1,
      jobs: [job({ id: 'future-worker', name: 'Restarted worker', iterations: 0 })],
    }),
  );
  await update(snapshot({ revision: 100 }));
  expect(screen.getByRole('button', { name: 'Restarted worker' })).toBeVisible();
});

it('keeps the last snapshot on disconnect and replaces it after reconnect', async () => {
  server.use(http.get(endpoint, () => HttpResponse.json(snapshot())));
  const view = renderCitadel(<InstanceJobs />);
  await screen.findByRole('button', { name: 'Maintenance jobs' });
  await update(snapshot());
  realtime.connected = false;
  view.rerender(<InstanceJobs />);
  expect(within(screen.getByRole('button', { name: 'Maintenance jobs' })).getByText('Waiting')).toBeVisible();
  realtime.connected = true;
  view.rerender(<InstanceJobs />);
  expect(within(screen.getByRole('button', { name: 'Maintenance jobs' })).getByText('Waiting')).toBeVisible();
  await update(snapshot({ revision: 7, jobs: [job({ state: JobStateView.Disabled })] }));
  expect(screen.getByText('Disabled')).toBeVisible();
});

it('distinguishes expired deadlines, running work and a disconnected schedule', async () => {
  const overdue = job({ nextCheck: Date.now() - 12 * 60_000 });
  server.use(http.get(endpoint, () => HttpResponse.json(snapshot({ jobs: [overdue] }))));
  const view = renderCitadel(<InstanceJobs />);
  await view.user.click(await screen.findByRole('button', { name: 'Maintenance jobs' }));
  await view.user.click(screen.getByRole('button', { name: 'Optimize older monitoring history' }));
  expect(screen.getAllByText('Live schedule unavailable')).toHaveLength(2);

  await update(snapshot({ jobs: [overdue] }));
  expect(screen.getAllByText('Due · awaiting update')).toHaveLength(2);
  expect(screen.queryByText('12 minutes ago')).not.toBeInTheDocument();
  expect(screen.getAllByText('Due · awaiting update')[0]).toHaveAttribute(
    'title',
    expect.stringContaining('Last reported deadline:'),
  );

  await update(snapshot({ revision: 2, jobs: [{ ...overdue, state: JobStateView.Running }] }));
  expect(screen.getAllByText('After completion')).toHaveLength(2);
  expect(screen.queryByText('Due · awaiting update')).not.toBeInTheDocument();

  const scheduled = job({ nextCheck: Date.now() + 60 * 60_000 });
  await update(snapshot({ revision: 3, jobs: [scheduled] }));
  expect(screen.getAllByText('in an hour')).toHaveLength(2);
  realtime.connected = false;
  view.rerender(<InstanceJobs />);
  expect(screen.getAllByText('Live schedule unavailable')).toHaveLength(2);
  expect(screen.queryByText('in an hour')).not.toBeInTheDocument();
});

it('hides protected data when a rejected subscription confirms revoked access', async () => {
  server.use(http.get(endpoint, () => HttpResponse.json(snapshot())));
  renderCitadel(<InstanceJobs />);
  await screen.findByRole('button', { name: 'Maintenance jobs' });
  await update(snapshot());
  server.use(http.get(endpoint, () => HttpResponse.json({ status: 403 }, { status: 403 })));
  act(() => realtime.fail?.(new Error('Access denied')));
  expect(await screen.findByRole('alert')).toHaveTextContent('Administrator access is required');
  expect(screen.queryByRole('table')).not.toBeInTheDocument();
  await update(snapshot({ revision: 99 }));
  expect(screen.queryByRole('table')).not.toBeInTheDocument();
  expect(screen.getByRole('alert')).toHaveTextContent('Administrator access is required');
});

it('offers retry only after snapshot failure and recovers an empty snapshot', async () => {
  server.use(http.get(endpoint, () => new HttpResponse(null, { status: 503 })));
  const { user } = renderCitadel(<InstanceJobs />);
  await screen.findByRole('alert');
  server.use(http.get(endpoint, () => HttpResponse.json(snapshot({ jobs: [] }))));
  await user.click(screen.getByRole('button', { name: 'Retry' }));
  expect(await screen.findByText('No jobs registered.')).toBeVisible();
});

it('summarizes current attention separately from unavailable telemetry and historical failures', async () => {
  server.use(
    http.get(endpoint, () =>
      HttpResponse.json(
        snapshot({
          jobs: [
            job({ id: 'build-runs', resourceType: JobCategory.Build, state: JobStateView.Running }),
            job({
              id: 'build-cleanup',
              resourceType: JobCategory.Build,
              state: JobStateView.BackingOff,
              lastOutcome: JobOutcomeView.Failed,
            }),
            job({
              id: 'docker-event-source',
              resourceType: JobCategory.Container,
              state: JobStateView.Unavailable,
              lastOutcome: null,
            }),
            job({ id: 'history-retention', failures: 3, lastFailureAt: 100, lastOutcome: JobOutcomeView.NoWork }),
            job({ id: 'scripts-scheduler', resourceType: JobCategory.Script, state: JobStateView.Disabled }),
          ],
        }),
      ),
    ),
  );
  renderCitadel(<InstanceJobs />);
  const summary = await screen.findByLabelText('Job summary');
  for (const [label, count] of [
    ['Catching up', '0'],
    ['Running', '1'],
    ['Retry scheduled', '0'],
    ['Needs attention', '1'],
    ['Status unavailable', '1'],
  ]) {
    const card = within(summary).getByText(label).parentElement!;
    expect(within(card).getByText(count)).toBeVisible();
  }
});

it('keeps recovered failures historical and removes schedules for failed or stopped workers', async () => {
  const recovered = job({ lastOutcome: JobOutcomeView.Completed, lastError: 'Iteration failed; inspect Core logs.' });
  server.use(http.get(endpoint, () => HttpResponse.json(snapshot({ jobs: [recovered] }))));
  const { user } = renderCitadel(<InstanceJobs />);
  await user.click(await screen.findByRole('button', { name: 'Maintenance jobs' }));
  await user.click(screen.getByRole('button', { name: 'Optimize older monitoring history' }));
  expect(screen.getByText('Last failure: Iteration failed; inspect Core logs.')).toHaveClass('text-muted-foreground');
  await update(
    snapshot({
      revision: 2,
      jobs: [
        {
          ...recovered,
          state: JobStateView.Unavailable,
          lastOutcome: JobOutcomeView.Interrupted,
          waitingFor: null,
          nextCheck: null,
        },
      ],
    }),
  );
  expect(screen.getByText('Last failure: Iteration failed; inspect Core logs.')).toHaveClass('text-destructive');
  expect(screen.getAllByText('Not scheduled')).toHaveLength(2);
  await update(
    snapshot({
      revision: 3,
      jobs: [
        {
          ...recovered,
          state: JobStateView.Unavailable,
          lastOutcome: JobOutcomeView.Interrupted,
          waitingFor: 'Awaiting session retry',
        },
      ],
    }),
  );
  expect(screen.getAllByText('Awaiting session retry')).toHaveLength(2);
  await update(
    snapshot({
      revision: 4,
      jobs: [{ ...recovered, state: JobStateView.Stopped, waitingFor: 'Old retry', nextCheck: 123 }],
    }),
  );
  expect(screen.getByText('Last failure: Iteration failed; inspect Core logs.')).toHaveClass('text-muted-foreground');
  expect(screen.getAllByText('Not scheduled')).toHaveLength(2);
  expect(screen.queryByText('Old retry')).not.toBeInTheDocument();
});

it('groups jobs, keeps the full group health while filtering and reveals matching children', async () => {
  server.use(
    http.get(endpoint, () =>
      HttpResponse.json(
        snapshot({
          jobs: [
            job({
              id: 'build-runs',
              resourceType: JobCategory.Build,
              name: 'Dispatch build work',
              state: JobStateView.Running,
            }),
            job({
              id: 'build-cleanup',
              resourceType: JobCategory.Build,
              name: 'Clean up build resources',
              state: JobStateView.BackingOff,
            }),
            job({ id: 'scripts-runs', resourceType: JobCategory.Script, name: 'Dispatch script work' }),
          ],
        }),
      ),
    ),
  );
  const { user } = renderCitadel(<InstanceJobs />);
  const build = await screen.findByRole('button', { name: 'Build jobs' });
  expect(build).toHaveAttribute('aria-expanded', 'false');
  expect(screen.queryByRole('button', { name: 'Dispatch build work' })).not.toBeInTheDocument();
  expect(within(build.closest('tr')!).getByText('Backing off')).toHaveClass('text-orange-700');
  await user.click(build);
  const child = screen.getByRole('button', { name: 'Dispatch build work' });
  expect(within(child.closest('tr')!).getByText('Running')).toHaveClass('text-blue-700');
  await user.click(child);
  await user.click(screen.getByText('Technical details'));
  expect(screen.getByText('Schedule')).toBeVisible();
  await user.click(screen.getByRole('button', { name: 'State Filter' }));
  await user.click(screen.getByRole('option', { name: 'Running' }));
  await user.keyboard('{Escape}');
  expect(screen.getByRole('button', { name: 'Dispatch build work' })).toBeVisible();
  expect(screen.queryByRole('button', { name: 'Clean up build resources' })).not.toBeInTheDocument();
  expect(screen.queryByRole('button', { name: 'Script jobs' })).not.toBeInTheDocument();
  expect(
    within(screen.getByRole('button', { name: 'Build jobs' }).closest('tr')!).getByText('Backing off'),
  ).toBeVisible();
  await user.click(screen.getByRole('button', { name: 'Resource Filter' }));
  await user.click(screen.getByRole('option', { name: 'Script' }));
  await user.keyboard('{Escape}');
  expect(screen.getByText('No jobs match your filters.')).toBeVisible();
  await user.click(screen.getByRole('button', { name: 'Remove Running state filter' }));
  expect(screen.getByRole('button', { name: 'Dispatch script work' })).toBeVisible();
  await user.click(screen.getByRole('button', { name: 'Remove Script resource filter' }));
  expect(screen.getByRole('button', { name: 'Build jobs' })).toBeVisible();
  await user.click(screen.getByRole('button', { name: 'State Filter' }));
  await user.click(screen.getByRole('option', { name: 'Running' }));
  await user.type(screen.getByPlaceholderText('Search states…'), 'backing');
  await user.click(screen.getByRole('option', { name: 'Backing off' }));
  await user.keyboard('{Escape}');
  expect(screen.getByRole('button', { name: 'Dispatch build work' })).toBeVisible();
  expect(screen.getByRole('button', { name: 'Clean up build resources' })).toBeVisible();
  expect(screen.queryByRole('button', { name: 'Script jobs' })).not.toBeInTheDocument();
  await user.click(screen.getByRole('button', { name: 'State Filter' }));
  await user.click(screen.getByRole('button', { name: 'Clear state filters' }));
  await user.keyboard('{Escape}');
  expect(screen.getByRole('button', { name: 'Script jobs' })).toBeVisible();
});

it('retains future jobs and keeps expanded groups stable across live updates', async () => {
  server.use(
    http.get(endpoint, () =>
      HttpResponse.json(
        snapshot({
          jobs: [job({ id: 'future-worker', resourceType: JobCategory.Other, name: 'Future worker' })],
        }),
      ),
    ),
  );
  const { user } = renderCitadel(<InstanceJobs />);
  await user.click(await screen.findByRole('button', { name: 'Other jobs' }));
  await user.click(screen.getByRole('button', { name: 'Future worker' }));
  await update(
    snapshot({
      revision: 2,
      jobs: [
        job({
          id: 'future-worker',
          resourceType: JobCategory.Other,
          name: 'Future worker',
          state: JobStateView.Running,
        }),
      ],
    }),
  );
  expect(screen.getByRole('button', { name: 'Other jobs' })).toHaveAttribute('aria-expanded', 'true');
  expect(screen.getByRole('button', { name: 'Future worker' })).toHaveAttribute('aria-expanded', 'true');
  await user.click(screen.getByText('Technical details'));
  expect(screen.getByText('Schedule')).toBeVisible();
});

it('loads selected job history on demand, preserves view state, and updates only after a history commit', async () => {
  const requests: string[] = [];
  server.use(
    http.get(endpoint, () => HttpResponse.json(snapshot())),
    http.get(`${endpoint}/history`, ({ request }) => {
      requests.push(request.url);
      return HttpResponse.json({ observedAt: Date.now(), sessions: [], events: [], buckets: [], truncated: false });
    }),
  );
  const { user } = renderCitadel(<InstanceJobs />);
  await user.type(await screen.findByRole('searchbox'), 'compaction');
  await user.click(await screen.findByRole('button', { name: 'Optimize older monitoring history' }));
  expect(requests).toHaveLength(0);
  expect(screen.queryByRole('region', { name: 'Job history' })).not.toBeInTheDocument();
  expect(screen.queryByRole('link', { name: /^View / })).not.toBeInTheDocument();
  const trigger = screen.getByRole('button', { name: 'History and work trends' });
  await user.click(trigger);
  const dialog = await screen.findByRole('dialog', { name: 'Optimize older monitoring history' });
  await waitFor(() => expect(requests).toHaveLength(1));
  expect(new URL(requests[0]).searchParams.get('job')).toBe('statistics-compaction');
  await within(dialog).findByText('No recorded events in this range. Missing observations do not confirm success.');
  await user.click(within(dialog).getByRole('combobox', { name: 'History range' }));
  await user.click(screen.getByRole('option', { name: 'Last 7 days' }));
  await waitFor(() => expect(requests).toHaveLength(2));
  await user.click(within(dialog).getByRole('tab', { name: 'Work trends' }));
  await update(snapshot({ revision: 2, historyRevision: 0 }));
  expect(requests).toHaveLength(2);
  await update(snapshot({ revision: 3, historyRevision: 1 }));
  await waitFor(() => expect(requests).toHaveLength(3));
  expect(new URL(requests[2]).searchParams.get('hours')).toBe('168');
  expect(within(dialog).getByRole('tab', { name: 'Work trends' })).toHaveAttribute('aria-selected', 'true');
  expect(within(dialog).getByRole('combobox', { name: 'History range' })).toHaveTextContent('Last 7 days');
  await user.keyboard('{Escape}');
  await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
  expect(trigger).toHaveFocus();
  expect(screen.getByRole('searchbox')).toHaveValue('compaction');
  expect(screen.getByRole('button', { name: 'Optimize older monitoring history' })).toHaveAttribute(
    'aria-expanded',
    'true',
  );
  await update(snapshot({ revision: 4, historyRevision: 2 }));
  expect(requests).toHaveLength(3);
});

it('removes cached history when its read access is revoked', async () => {
  server.use(
    http.get(endpoint, () => HttpResponse.json(snapshot())),
    http.get(`${endpoint}/history`, () =>
      HttpResponse.json({
        observedAt: Date.now(),
        sessions: [],
        buckets: [],
        truncated: false,
        events: [
          { bootId: 'old-core', event: { sequence: 1, jobId: 'private-worker', at: Date.now(), kind: 'failed' } },
        ],
      }),
    ),
  );
  const { user } = renderCitadel(<InstanceJobs />);
  await user.click(await screen.findByRole('button', { name: 'Maintenance jobs' }));
  await user.click(screen.getByRole('button', { name: 'Optimize older monitoring history' }));
  await user.click(screen.getByRole('button', { name: 'History and work trends' }));
  expect(await screen.findByText('private-worker · Attempt failed')).toBeVisible();
  server.use(http.get(`${endpoint}/history`, () => HttpResponse.json({ status: 403 }, { status: 403 })));
  await update(snapshot({ revision: 2, historyRevision: 1 }));
  await waitFor(() => expect(screen.queryByText('private-worker · Attempt failed')).not.toBeInTheDocument());
  expect(screen.getByRole('alert')).toHaveTextContent('History is unavailable');
});
