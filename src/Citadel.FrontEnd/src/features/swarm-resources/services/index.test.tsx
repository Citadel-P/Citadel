import { SwarmServiceOwnership, SwarmServiceView, SwarmTaskView } from '@/api/generated/api.types';
import { RegularResourceView } from '@/pages/regular-resource';
import { FakeHubConnection } from '@/test/fakes/signalr';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { act, screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { Route, Routes } from 'react-router';
import { ServiceComponents } from '.';

vi.mock('@/components/custom/task-sheet', () => ({ default: () => null }));

const platformId = '00000000-0000-0000-0000-000000000200';

describe('ServiceComponents', () => {
  it('expands service tasks and keeps them synchronized through SignalR', async () => {
    const fake = new FakeHubConnection();
    server.use(
      http.get(`http://localhost/api/v1/platforms/${platformId}/swarm/services`, () =>
        HttpResponse.json({ items: [service()] }),
      ),
      http.get(`http://localhost/api/v1/platforms/${platformId}/swarm/tasks`, () =>
        HttpResponse.json({ items: [task()] }),
      ),
    );

    renderCitadel(
      <Routes>
        <Route
          path="/platforms/:platformId/services"
          element={<RegularResourceView Components={ServiceComponents} type="Service" showTaskSheet={false} />}
        />
      </Routes>,
      {
        route: `/platforms/${platformId}/services`,
        signalR: {
          connectionFactory: () => fake.asHubConnection(),
          startConnection: (connection) => connection.start(),
        },
      },
    );

    const serviceLink = await screen.findByRole('link', { name: 'web' });
    expect(serviceLink).toHaveAttribute('href', `/platforms/${platformId}/services/service-1`);
    expect(serviceLink.parentElement?.querySelector('.bg-orange-500')).not.toBeNull();
    expect(screen.queryByRole('link', { name: 'web.1' })).not.toBeInTheDocument();

    await act(async () => screen.getByRole('button', { name: 'Expand service web' }).click());

    expect(screen.getByRole('link', { name: 'web.1' })).toHaveAttribute(
      'href',
      `/platforms/${platformId}/tasks/task-1`,
    );
    expect(screen.getByRole('link', { name: 'manager-1' })).toHaveAttribute(
      'href',
      `/platforms/${platformId}/nodes/node-1`,
    );
    expect(screen.getByText('Running')).toBeVisible();
    await waitFor(() => expect(fake.listenerCount('SwarmInventoryUpdated')).toBe(2));

    act(() => {
      fake.emit(
        'SwarmInventoryUpdated',
        inventory([service({ runningTaskCount: 2 })], [task({ id: 'task-2', name: 'web.2' })]),
      );
    });

    expect(await screen.findByRole('link', { name: 'web.2' })).toBeVisible();
    expect(screen.queryByRole('link', { name: 'web.1' })).not.toBeInTheDocument();
  });
});

const service = (overrides: Partial<SwarmServiceView> = {}): SwarmServiceView => ({
  id: 'service-1',
  versionIndex: 1,
  name: 'web',
  mode: 'Replicated',
  image: 'nginx:latest',
  runningTaskCount: 1,
  desiredTaskCount: 2,
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

const task = (overrides: Partial<SwarmTaskView> = {}): SwarmTaskView => ({
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

const inventory = (services: SwarmServiceView[], tasks: SwarmTaskView[]) => ({
  platformId,
  nodes: { items: [] },
  services: { items: services },
  tasks: { items: tasks },
  networks: { items: [] },
  secrets: { items: [] },
  configs: { items: [] },
});
