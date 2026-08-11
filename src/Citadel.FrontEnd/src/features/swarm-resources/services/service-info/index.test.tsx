import { SwarmServiceOwnership } from '@/api/generated/api.types';
import { ResourceInfoView } from '@/pages/resource-info';
import { FakeHubConnection } from '@/test/fakes/signalr';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { screen } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { Route, Routes } from 'react-router';
import { ServiceInfoComponents } from '.';

vi.mock('@/lib/monaco', () => ({
  MonacoEditor: ({ value }: { value: string }) => <pre>{value}</pre>,
}));

const platformId = '00000000-0000-0000-0000-000000000200';

describe('ServiceInfoComponents', () => {
  it('shows current tasks as the summary and exposes only supported tabs', async () => {
    const fake = new FakeHubConnection();
    server.use(
      http.get(`http://localhost/api/v1/platforms/${platformId}/swarm/services/service-1`, () =>
        HttpResponse.json(
          service({
            updateState: 'Paused',
            updateMessage: 'update paused after a transient task failure',
          }),
        ),
      ),
      http.get(`http://localhost/api/v1/platforms/${platformId}/swarm/tasks`, () =>
        HttpResponse.json({
          items: [task(), task({ id: 'other-task', name: 'other.1', serviceId: 'service-2', serviceName: 'other' })],
        }),
      ),
      http.get(`http://localhost/api/v1/platforms/${platformId}/swarm/services/service-1/stats`, () =>
        HttpResponse.json({
          dockerServiceId: 'service-1',
          observedTasks: 0,
          expectedTasks: 1,
          complete: false,
          observedContainerProjectionIds: [],
          missingDockerNodeIds: ['node-1'],
          oldestSampleAt: null,
          newestSampleAt: null,
          stats: [],
        }),
      ),
      http.get(`http://localhost/api/v1/platforms/${platformId}`, () =>
        HttpResponse.json({
          status: 'Online',
          capabilities: {
            canRead: true,
            canWrite: false,
            canExecute: false,
            canViewLogs: false,
            canInspect: false,
            canOpenTerminal: false,
            canPull: false,
          },
        }),
      ),
      http.get('http://localhost/api/v1/swarmServices', () =>
        HttpResponse.json({
          swarmServices: [],
          capabilities: { canRead: true, canWrite: true, canExecute: false },
        }),
      ),
    );

    renderCitadel(
      <Routes>
        <Route
          path="/platforms/:platformId/services/:resourceId"
          element={<ResourceInfoView Components={ServiceInfoComponents} type="Service" />}
        />
      </Routes>,
      {
        route: `/platforms/${platformId}/services/service-1`,
        signalR: {
          connectionFactory: () => fake.asHubConnection(),
          startConnection: (connection) => connection.start(),
        },
      },
    );

    expect(await screen.findByRole('link', { name: 'web.1' })).toBeVisible();
    expect(screen.getByText('Replicated')).toBeVisible();
    expect(screen.getByText('1/1')).toBeVisible();
    expect(screen.getByText('80/tcp')).toBeVisible();
    expect(screen.getByRole('heading', { name: 'Tasks' })).toBeVisible();
    expect(screen.getByText('Service update paused')).toBeVisible();
    expect(screen.getByText('update paused after a transient task failure')).toBeVisible();
    expect(screen.queryByRole('link', { name: 'other.1' })).not.toBeInTheDocument();
    expect(screen.queryByRole('columnheader', { name: 'Service' })).not.toBeInTheDocument();
    expect(screen.getByRole('columnheader', { name: 'Node' })).toBeVisible();
    expect(screen.getByRole('columnheader', { name: 'Desired' })).toBeVisible();
    expect(screen.getByRole('tab', { name: 'Logs' })).toBeDisabled();
    expect(screen.getByRole('tab', { name: 'Inspect' })).toBeDisabled();
    expect(screen.getByRole('tab', { name: 'Stats' })).toBeEnabled();
    expect(screen.queryByRole('button', { name: 'View' })).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Adopt Service' })).toBeVisible();
    const actionGroup = screen.getByRole('group');
    expect(actionGroup).toContainElement(screen.getByRole('button', { name: 'Restart Service' }));
    expect(actionGroup).toContainElement(screen.getByRole('button', { name: 'Delete' }));
  });
});

const service = (overrides: Record<string, unknown> = {}) => ({
  id: 'service-1',
  versionIndex: 1,
  name: 'web',
  mode: 'Replicated',
  image: 'nginx:latest',
  runningTaskCount: 1,
  desiredTaskCount: 1,
  updateState: 'Completed',
  updateMessage: null,
  ports: ['80/tcp'],
  networkIds: [],
  secretIds: [],
  configIds: [],
  labels: {},
  ownership: SwarmServiceOwnership.Unmanaged,
  dockerStackNamespace: null,
  ownershipDiagnostic: null,
  createdAt: '2026-08-04T11:00:00Z',
  updatedAt: '2026-08-04T12:00:00Z',
  observedAt: '2026-08-04T12:00:00Z',
  isStale: false,
  ...overrides,
});

const task = (overrides: Record<string, unknown> = {}) => ({
  id: 'task-1',
  versionIndex: 1,
  name: 'web.1',
  serviceId: 'service-1',
  serviceName: 'web',
  slot: 1,
  nodeId: 'node-1',
  nodeHostname: 'manager-1',
  desiredState: 'Running',
  state: 'Running',
  statusMessage: null,
  error: null,
  image: 'nginx:latest',
  ports: ['80/tcp'],
  statusTimestamp: '2026-08-04T12:00:00Z',
  createdAt: '2026-08-04T11:59:00Z',
  updatedAt: '2026-08-04T12:00:00Z',
  observedAt: '2026-08-04T12:00:00Z',
  isStale: false,
  ...overrides,
});
