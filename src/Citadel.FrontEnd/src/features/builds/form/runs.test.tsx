import { screen, within } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import {
  BuildProjectView,
  BuildRunView,
  BuildProjectBuilderKind,
  ResourceControlState,
} from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { BuildRunsTab } from './runs';
import { BuildsTable } from '../table';
import { FakeRealtimeConnection } from '@/test/fakes/realtime';

const projectId = '00000000-0000-0000-0000-000000000201';
const runId = '00000000-0000-0000-0000-000000000202';

describe('BuildRunsTab', () => {
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
    renderCitadel(<BuildRunsTab resource={{ id: projectId } as BuildProjectView} />, {
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
  } satisfies BuildProjectView;
  renderCitadel(<BuildsTable items={[project]} actions={{}} isLoading={false} />);
  expect(await screen.findByText('platform-fallback')).toBeVisible();
});
