import { ResourceInfoView } from '@/pages/resource-info';
import { FakeRealtimeConnection } from '@/test/fakes/realtime';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { screen } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { Route, Routes } from 'react-router';
import { NodeInfoComponents } from '.';

vi.mock('@/lib/monaco', () => ({
  MonacoEditor: ({ value }: { value: string }) => <pre>{value}</pre>,
}));

const platformId = '00000000-0000-0000-0000-000000000200';

describe('NodeInfoComponents', () => {
  it('shows minimal node metadata and tasks using the standard info page', async () => {
    const fake = new FakeRealtimeConnection();
    server.use(
      http.get(`http://localhost/api/v1/platforms/${platformId}/swarm/nodes/node-1`, () => HttpResponse.json(node)),
      http.get(`http://localhost/api/v1/platforms/${platformId}/swarm/tasks`, () =>
        HttpResponse.json({ items: [task(), task({ id: 'task-2', name: 'other.1', nodeId: 'node-2' })] }),
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
    );

    renderCitadel(
      <Routes>
        <Route
          path="/platforms/:platformId/nodes/:resourceId"
          element={<ResourceInfoView Components={NodeInfoComponents} type="Node" />}
        />
      </Routes>,
      {
        route: `/platforms/${platformId}/nodes/node-1`,
        groups: {
          connectionFactory: () => fake.asRealtimeConnection(),
          startConnection: (connection) => connection.start(),
        },
      },
    );

    expect(await screen.findByRole('link', { name: 'web.1' })).toBeVisible();
    expect(screen.queryByRole('link', { name: 'other.1' })).not.toBeInTheDocument();
    expect(screen.getByText('Manager')).toBeVisible();
    expect(screen.getAllByText('Active').length).toBeGreaterThan(0);
    expect(screen.getByText('29.0')).toBeVisible();
    expect(screen.getByText('10.0.0.1')).toBeVisible();
    expect(screen.getByRole('columnheader', { name: 'Service' })).toBeVisible();
    expect(screen.queryByRole('columnheader', { name: 'Node' })).not.toBeInTheDocument();
    expect(screen.getByText('Labels')).toBeVisible();
    expect(screen.getByText('zone')).toBeVisible();
    expect(screen.getByText('primary')).toBeVisible();
    expect(screen.getByRole('tab', { name: 'Inspect' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Active' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Pause' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Drain' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Edit' })).toBeDisabled();
    expect(
      screen.getByRole('tab', { name: 'Inspect' }).compareDocumentPosition(screen.getByText('Labels')),
    ).toBe(Node.DOCUMENT_POSITION_FOLLOWING);
  });
});

const node = {
  id: 'node-1',
  versionIndex: 1,
  hostname: 'manager-1',
  role: 'Manager',
  isLeader: true,
  reachability: 'Reachable',
  status: 'Ready',
  statusMessage: null,
  availability: 'Active',
  engineVersion: '29.0',
  operatingSystem: 'linux',
  architecture: 'x86_64',
  address: '10.0.0.1',
  labels: { zone: 'primary' },
  runningTaskCount: 1,
  desiredTaskCount: 1,
  createdAt: null,
  updatedAt: null,
  observedAt: '2026-08-05T08:00:00Z',
  isStale: false,
  capabilities: {
    canRead: true,
    canWrite: false,
    canExecute: false,
    canViewLogs: false,
    canInspect: false,
    canOpenTerminal: false,
    canPull: false,
  },
};

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
  ports: [],
  statusTimestamp: '2026-08-05T08:00:00Z',
  createdAt: null,
  updatedAt: null,
  observedAt: '2026-08-05T08:00:00Z',
  isStale: false,
  ...overrides,
});
