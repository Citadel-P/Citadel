import {
  ManagedSwarmServiceView,
  ResourceControlState,
  SwarmServiceHealth,
  SwarmServiceOwnership,
  SwarmServiceSynchronizationState,
  SwarmServiceView,
  SwarmTaskView,
  UpdateBehavior,
} from '@/api/generated/api.types';
import { RegularResourceView } from '@/pages/regular-resource';
import { FakeHubConnection } from '@/test/fakes/signalr';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { LayoutContext } from '@/lib/context/layout-context';
import { act, screen, waitFor, within } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { Route, Routes } from 'react-router';
import { ServiceComponents } from '.';

vi.mock('@/components/custom/task-sheet', () => ({ default: () => null }));

const platformId = '00000000-0000-0000-0000-000000000200';
const layoutContext = {
  theme: { mode: 'light' as const },
  sidebarMinimized: false,
  mobileMenuVisible: false,
  toggleSidebar: vi.fn(),
  setSidebarOpen: vi.fn(),
  toggleMobileMenu: vi.fn(),
  toggleThemeColor: vi.fn(),
  setThemeMode: vi.fn(),
};

describe('ServiceComponents', () => {
  it('expands service tasks and keeps them synchronized through SignalR', async () => {
    const fake = new FakeHubConnection();
    server.use(
      http.get(`http://localhost/api/v1/platforms/${platformId}/swarm/services`, () =>
        HttpResponse.json({
          items: [
            service({
              runningTaskCount: 2,
              desiredTaskCount: 2,
              updateState: 'Paused',
              updateMessage: 'update paused after a transient task failure',
            }),
          ],
        }),
      ),
      http.get(`http://localhost/api/v1/platforms/${platformId}/swarm/tasks`, () =>
        HttpResponse.json({ items: [task()] }),
      ),
      http.get('http://localhost/api/v1/swarmServices', () =>
        HttpResponse.json({
          swarmServices: [],
          capabilities: { canRead: true, canWrite: true, canExecute: true },
        }),
      ),
    );

    renderCitadel(
      <LayoutContext.Provider value={layoutContext}>
        <Routes>
          <Route
            path="/platforms/:platformId/services"
            element={<RegularResourceView Components={ServiceComponents} type="Service" showTaskSheet={false} />}
          />
          <Route path="/platforms/:platformId/services/add" element={<span>Adopt Service form</span>} />
        </Routes>
      </LayoutContext.Provider>,
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
    expect(serviceLink.parentElement?.querySelector('.bg-green-500')).not.toBeNull();
    expect(screen.getByText('Paused')).toHaveClass('text-orange-500');
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

    expect(screen.queryByRole('button', { name: 'Add Service' })).not.toBeInTheDocument();
    expect(screen.getByLabelText('Unmanaged Service')).toBeVisible();

    await act(async () => screen.getByRole('checkbox', { name: 'Select Service web' }).click());
    expect(screen.getByRole('button', { name: 'View' })).toBeVisible();
    expect(screen.getByRole('button', { name: 'Restart Service' })).toBeVisible();
    expect(screen.getByRole('button', { name: 'Delete' })).toBeVisible();
    await act(async () => screen.getByRole('button', { name: 'Delete' }).click());
    const confirmation = screen.getByRole('dialog');
    expect(within(confirmation).getByRole('listitem')).toHaveTextContent('web');
    await act(async () => within(confirmation).getByRole('button', { name: 'Close' }).click());
    await act(async () => screen.getByRole('button', { name: 'Adopt Service' }).click());
    expect(await screen.findByText('Adopt Service form')).toBeVisible();
  });

  it('shows a managed Service before its first deployment', async () => {
    const fake = new FakeHubConnection();
    server.use(
      http.get(`http://localhost/api/v1/platforms/${platformId}/swarm/services`, () =>
        HttpResponse.json({ items: [] }),
      ),
      http.get(`http://localhost/api/v1/platforms/${platformId}/swarm/tasks`, () => HttpResponse.json({ items: [] })),
      http.get('http://localhost/api/v1/swarmServices', () =>
        HttpResponse.json({
          swarmServices: [managedService()],
          capabilities: { canRead: true, canWrite: true, canExecute: true },
        }),
      ),
    );

    renderCitadel(
      <LayoutContext.Provider value={layoutContext}>
        <Routes>
          <Route
            path="/platforms/:platformId/services"
            element={<RegularResourceView Components={ServiceComponents} type="Service" showTaskSheet={false} />}
          />
        </Routes>
      </LayoutContext.Provider>,
      {
        route: `/platforms/${platformId}/services`,
        signalR: {
          connectionFactory: () => fake.asHubConnection(),
          startConnection: (connection) => connection.start(),
        },
      },
    );

    expect(await screen.findByRole('link', { name: 'draft-web' })).toHaveAttribute(
      'href',
      '/swarm-services/edit/managed-service-1',
    );
    expect(screen.getByText('Citadel Service')).toBeVisible();
    expect(screen.queryByLabelText('Unmanaged Service')).not.toBeInTheDocument();
  });

  it('uses the persisted Docker Service link before ownership labels are applied', async () => {
    const fake = new FakeHubConnection();
    server.use(
      http.get(`http://localhost/api/v1/platforms/${platformId}/swarm/services`, () =>
        HttpResponse.json({ items: [service()] }),
      ),
      http.get(`http://localhost/api/v1/platforms/${platformId}/swarm/tasks`, () => HttpResponse.json({ items: [] })),
      http.get('http://localhost/api/v1/swarmServices', () =>
        HttpResponse.json({
          swarmServices: [managedService({ dockerServiceId: 'service-1' })],
          capabilities: { canRead: true, canWrite: true, canExecute: true },
        }),
      ),
    );

    renderCitadel(
      <LayoutContext.Provider value={layoutContext}>
        <Routes>
          <Route
            path="/platforms/:platformId/services"
            element={<RegularResourceView Components={ServiceComponents} type="Service" showTaskSheet={false} />}
          />
        </Routes>
      </LayoutContext.Provider>,
      {
        route: `/platforms/${platformId}/services`,
        signalR: {
          connectionFactory: () => fake.asHubConnection(),
          startConnection: (connection) => connection.start(),
        },
      },
    );

    expect(await screen.findByRole('link', { name: 'web' })).toHaveAttribute(
      'href',
      `/platforms/${platformId}/services/service-1`,
    );
    expect(screen.getByText('Citadel Service')).toBeVisible();
    expect(screen.queryByLabelText('Unmanaged Service')).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Adopt Service' })).not.toBeInTheDocument();

    await act(async () => screen.getByRole('checkbox', { name: 'Select Service web' }).click());

    expect(screen.getByRole('button', { name: 'View' })).toBeVisible();
    expect(screen.getByRole('button', { name: 'Restart Service' })).toBeVisible();
    expect(screen.getByRole('button', { name: 'Delete' })).toBeVisible();
    expect(screen.getByRole('button', { name: 'Adopt Service' })).toBeDisabled();
  });
});

const managedService = (overrides: Partial<ManagedSwarmServiceView> = {}): ManagedSwarmServiceView => ({
  id: 'managed-service-1',
  platformId,
  name: 'draft-web',
  description: null,
  dockerName: 'draft-web-managed-service-1',
  dockerServiceId: null,
  spec: {
    image: { $type: 'External', registryId: 'registry-1', imageTag: 'nginx:latest' },
    updateBehavior: UpdateBehavior.Disabled,
    schedulingMode: 'Replicated',
    replicas: 2,
    command: [],
    arguments: [],
    environment: [],
    ports: [],
    networkIds: [],
    mounts: [],
    secrets: [],
    configs: [],
    placementConstraints: [],
  },
  health: SwarmServiceHealth.Unknown,
  synchronizationState: SwarmServiceSynchronizationState.NeverApplied,
  controlState: ResourceControlState.Idle,
  autoUpdateState: { lastCheckedAt: '0001-01-01T00:00:00Z', status: 'Unknown' },
  appliedImageDigest: null,
  hasPendingDesiredChanges: true,
  hasRuntimeDrift: false,
  rowVersion: 0,
  createdAt: '2026-08-06T12:00:00Z',
  updatedAt: '2026-08-06T12:00:00Z',
  platformName: 'Swarm',
  platformStatus: 'Online',
  runningTaskCount: null,
  desiredTaskCount: null,
  updateState: null,
  currentOperation: null,
  tags: [],
  ...overrides,
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
