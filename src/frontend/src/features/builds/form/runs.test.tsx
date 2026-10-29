import { act, screen, waitFor, within } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import {
  AuthorizedProject,
  BuildRunView,
  BuildProjectBuilderKind,
  ResourceControlState,
} from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { BuildRunsTab } from './runs';
import { BuildsTable } from '../table';
import { FakeRealtimeConnection } from '@/test/fakes/realtime';

vi.mock('@/components/custom/log-viewer', () => ({
  LogViewer: ({ logs }: { logs: unknown }) => <pre>{JSON.stringify(logs)}</pre>,
}));

const projectId = '00000000-0000-0000-0000-000000000201';
const runId = '00000000-0000-0000-0000-000000000202';

describe('BuildRunsTab', () => {
  it('shows and advances a webhook run without reloading the run list', async () => {
    const read = vi.fn(() => HttpResponse.json({ runs: [] }));
    server.use(
      http.get('http://localhost/api/v1/buildRuns', read),
      http.get('http://localhost/api/v1/profile/preferences', () => HttpResponse.json({})),
    );
    const fake = new FakeRealtimeConnection();
    renderCitadel(<BuildRunsTab resource={{ id: projectId } as AuthorizedProject} />, {
      groups: {
        connectionFactory: () => fake.asRealtimeConnection(),
        startConnection: (connection) => connection.start(),
      },
    });
    await waitFor(() => expect(read).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(fake.invoke).toHaveBeenCalledWith('JoinGroup', `build-runs:${projectId}`));
    const run = {
      id: runId,
      buildProjectId: projectId,
      status: 'Queued',
      trigger: 'Webhook',
      imageReferences: [],
      queuedAt: '2026-10-04T12:00:00Z',
    };
    act(() => fake.emit('BuildRunInfoUpdated', run, 'update'));
    expect(await screen.findByText('Webhook')).toBeVisible();
    expect(screen.getByText('Queued', { selector: '[data-slot="badge"]' })).toBeVisible();
    act(() => fake.emit('BuildRunInfoUpdated', { ...run, status: 'Running' }, 'update'));
    expect(await screen.findByText('Running')).toBeVisible();
    expect(read).toHaveBeenCalledTimes(1);
  });

  it.each([
    ['missing snapshot', undefined, 'Not available'],
    ['pool without a platform', { id: null, name: null, address: null }, 'Not available'],
    [
      'platform snapshot',
      { id: 'platform-id', name: 'Local Docker', address: 'http://localhost.docker' },
      'Local Docker',
    ],
  ])('opens newly queued run logs with %s', async (_case, platformSnapshot, label) => {
    const run = {
      id: runId,
      buildProjectId: projectId,
      projectNameSnapshot: 'Application image',
      gitRepositoryId: 'repo-id',
      gitRepositoryNameSnapshot: 'Application source',
      platformSnapshot,
      branch: 'main',
      status: 'Queued',
      trigger: 'Manual',
      queuedAt: '2026-09-22T10:00:00Z',
      startedAt: null,
      completedAt: null,
      imageReferences: [],
      imageRepository: 'test/application',
      dockerfilePath: 'Dockerfile',
      contextPath: '.',
    };
    server.use(
      http.get('http://localhost/api/v1/buildRuns', () => HttpResponse.json({ runs: [run] })),
      http.get(`http://localhost/api/v1/buildRuns/${runId}`, () => HttpResponse.json(run)),
      http.get(`http://localhost/api/v1/buildRuns/${runId}/logs`, () => HttpResponse.json({ logs: [] })),
    );
    const fake = new FakeRealtimeConnection();
    renderCitadel(<BuildRunsTab resource={{ id: projectId } as AuthorizedProject} />, {
      route: `/builds/edit/${projectId}?runId=${runId}#runs`,
      groups: {
        connectionFactory: () => fake.asRealtimeConnection(),
        startConnection: (connection) => connection.start(),
      },
    });
    const sheet = await screen.findByRole('dialog');
    expect(await within(sheet).findByText('Application image run')).toBeVisible();
    expect(within(sheet).getByText(String(label))).toBeVisible();
    expect(within(sheet).getByText('Application source')).toBeVisible();
  });
});

it('renders the build list when a queued run has no platform snapshot', async () => {
  const project = {
    capabilities: { canRead: true, canWrite: true, canExecute: true },
    id: projectId,
    name: 'New build',
    normalizedName: 'new-build',
    description: null,
    enabled: true,
    gitRepositoryId: 'repo-id',
    contextPath: '.',
    dockerfilePath: 'Dockerfile',
    target: null,
    buildArgs: [],
    buildSecrets: [],
    buildAgentPoolId: null,
    pushToRegistry: true,
    registryId: 'registry-id',
    webhook: null,
    timeoutSeconds: 300,
    retentionRunCount: 10,
    currentRunId: null,
    controlState: ResourceControlState.Idle,
    controlStartedAt: null,
    createdByActorId: 'actor-id',
    createdAt: '2026-09-22T10:00:00Z',
    updatedAt: '2026-09-22T10:00:00Z',
    archivedAt: null,
    rowVersion: 1,
    branch: 'main',
    platformId: 'platform-fallback',
    builderKind: BuildProjectBuilderKind.Platform,
    imageRepository: 'test/app',
    tagTemplates: ['latest'],
    tags: [],
    latestRun: { id: runId, status: 'Queued', queuedAt: '2026-09-22T10:00:00Z' } as BuildRunView,
  } satisfies AuthorizedProject;
  renderCitadel(<BuildsTable items={[project]} actions={{}} isLoading={false} />);
  expect(await screen.findByText('platform-fallback')).toBeVisible();
});

it('streams build logs without fetching them on status changes and catches up after reconnect', async () => {
  const run = {
    id: runId,
    buildProjectId: projectId,
    projectNameSnapshot: 'Application',
    status: 'Queued',
    trigger: 'Manual',
    queuedAt: '2026-10-04T12:00:00Z',
    imageReferences: [],
  };
  let savedLogs: unknown[] = [];
  const readLogs = vi.fn(() => HttpResponse.json({ logs: savedLogs }));
  server.use(
    http.get('http://localhost/api/v1/profile/preferences', () => HttpResponse.json({})),
    http.get('http://localhost/api/v1/buildRuns', () => HttpResponse.json({ runs: [run] })),
    http.get(`http://localhost/api/v1/buildRuns/${runId}`, () => HttpResponse.json(run)),
    http.get(`http://localhost/api/v1/buildRuns/${runId}/logs`, readLogs),
  );
  const fake = new FakeRealtimeConnection();
  const { queryClient } = renderCitadel(<BuildRunsTab resource={{ id: projectId } as AuthorizedProject} />, {
    route: `/builds/edit/${projectId}?runId=${runId}#runs`,
    groups: {
      connectionFactory: () => fake.asRealtimeConnection(),
      startConnection: (connection) => connection.start(),
    },
  });
  await screen.findByRole('dialog');
  await waitFor(() => expect(fake.invoke).toHaveBeenCalledWith('JoinGroup', `build-run:${runId}`));
  await waitFor(() => expect(readLogs).toHaveBeenCalled());
  await waitFor(() => expect(queryClient.isFetching({ queryKey: ['getBuildRunLogs'] })).toBe(0));
  const readsBefore = readLogs.mock.calls.length;
  await act(async () => {
    fake.emit('BuildRunInfoUpdated', { ...run, status: 'Running' }, 'update');
    fake.emit('BuildRunLogsAppended', runId, [
      {
        id: 'log-1',
        buildRunId: runId,
        createdAt: '2026-10-04T12:00:01Z',
        stream: 'stdout',
        message: 'Live build output',
      },
    ]);
  });
  expect(await screen.findByText(/Live build output/)).toBeVisible();
  expect(readLogs).toHaveBeenCalledTimes(readsBefore);
  savedLogs = [
    {
      id: 'log-2',
      buildRunId: runId,
      createdAt: '2026-10-04T12:00:02Z',
      stream: 'stdout',
      message: 'Missed while offline',
    },
  ];
  act(() => {
    fake.reconnecting();
    fake.reconnected();
  });
  expect(await screen.findByText(/Missed while offline/)).toBeVisible();
  expect(readLogs).toHaveBeenCalledTimes(readsBefore + 1);
  await act(async () => fake.reconnecting());
  await act(async () => fake.reconnected());
  await waitFor(() => expect(readLogs).toHaveBeenCalledTimes(readsBefore + 2));
  await waitFor(() => expect(queryClient.isFetching({ queryKey: ['getBuildRunLogs'] })).toBe(0));
  expect(readLogs).toHaveBeenCalledTimes(readsBefore + 2);
});
