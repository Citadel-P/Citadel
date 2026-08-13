import {
  ManagedSwarmServiceView,
  ResourceControlState,
  SwarmServiceHealth,
  SwarmServiceOperationState,
  SwarmServiceSchedulingMode,
} from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { FakeHubConnection } from '@/test/fakes/signalr';
import { server } from '@/test/server';
import { act, screen, waitFor, within } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { Route, Routes } from 'react-router';
import { SwarmServiceFormComponents } from '.';

vi.mock('@/lib/monaco', () => ({
  MonacoEditor: () => null,
  MonacoToArrayEditor: () => null,
  MonacoDiff: () => null,
}));

vi.mock('@/features/swarm-resources/tasks/task-info/terminal', () => ({
  TaskTerminal: ({ task, toolbarStart }: { task: { name: string }; toolbarStart?: React.ReactNode }) => (
    <div>
      {toolbarStart}
      <span>Terminal connected to {task.name}</span>
    </div>
  ),
}));

vi.mock('@/features/docker-resources/containers/container-info/container-stats', () => ({
  ContainerStatsCharts: ({ resource }: { resource?: { containerStat?: { cpuUsage?: number } } }) => (
    <div>
      <span>CPU Usage</span>
      <span>{resource?.containerStat?.cpuUsage?.toFixed(2)}%</span>
    </div>
  ),
}));

describe('SwarmServiceFormComponents', () => {
  beforeEach(() => {
    server.use(
      http.get('http://localhost/api/v1/platforms/:platformId/containers', () =>
        HttpResponse.json({ containers: [], capabilities: {} }),
      ),
    );
  });

  it('shows the persisted Docker rollout failure above the Service tabs', () => {
    const SubHeader = SwarmServiceFormComponents.EditForm!.SubHeader!;
    const resource = managedService({
      health: SwarmServiceHealth.Failed,
      updateMessage: 'update paused because a Task failed',
    });

    renderCitadel(<SubHeader resource={resource} />);

    expect(screen.getByText('Service operation failed')).toBeVisible();
    expect(screen.getByText('update paused because a Task failed')).toBeVisible();
  });

  it('shows a rejected operation even when the existing Service remains healthy', () => {
    const SubHeader = SwarmServiceFormComponents.EditForm!.SubHeader!;
    const resource = managedService({
      currentOperation: {
        state: SwarmServiceOperationState.Rejected,
        resultMessage: 'Docker rejected the Service update',
      } as ManagedSwarmServiceView['currentOperation'],
    });

    renderCitadel(<SubHeader resource={resource} />);

    expect(screen.getByText('Service operation failed')).toBeVisible();
    expect(screen.getByText('Docker rejected the Service update')).toBeVisible();
  });

  it('does not show a failure alert for a healthy Service', () => {
    const SubHeader = SwarmServiceFormComponents.EditForm!.SubHeader!;

    const { container } = renderCitadel(<SubHeader resource={managedService()} />);

    expect(container).toBeEmptyDOMElement();
  });

  it('explains that an adopted stopped Service must be applied to use the configured replicas', () => {
    const SubHeader = SwarmServiceFormComponents.EditForm!.SubHeader!;
    const resource = managedService({
      health: SwarmServiceHealth.Stopped,
      runningTaskCount: 0,
      desiredTaskCount: 0,
      hasPendingDesiredChanges: true,
      spec: {
        schedulingMode: SwarmServiceSchedulingMode.Replicated,
        replicas: 1,
        image: { $type: 'External', registryId: 'registry-1', imageTag: 'redis:latest' },
      },
    });

    renderCitadel(<SubHeader resource={resource} />);

    expect(screen.getByText('Changes not applied')).toBeVisible();
    expect(screen.getByText(/Docker is currently scaled to 0.*configured for 1.*Select Apply/)).toBeVisible();
  });

  it('renders edit actions as buttons rather than orphaned dropdown items', () => {
    const ActionButtons = SwarmServiceFormComponents.EditForm!.Header.ActionButtons;
    const resource = managedService();

    renderCitadel(<ActionButtons resource={resource} />);

    expect(screen.getByRole('button', { name: 'Scale' })).toBeVisible();
    expect(screen.getByRole('button', { name: 'Apply' })).toBeVisible();
    expect(screen.getByRole('button', { name: 'Restart Tasks' })).toBeVisible();
    expect(screen.getByRole('button', { name: 'Delete' })).toBeVisible();
    expect(screen.getByRole('button', { name: 'Check for Updates' })).toBeVisible();
  });

  it('returns to the Swarm Services page after deleting from the edit page', async () => {
    const ActionButtons = SwarmServiceFormComponents.EditForm!.Header.ActionButtons;
    const resource = managedService();
    let submitted: unknown;
    server.use(
      http.delete('http://localhost/api/v1/swarmServices', async ({ request }) => {
        submitted = await request.json();
        return new HttpResponse(null, { status: 204 });
      }),
    );

    const { user } = renderCitadel(
      <Routes>
        <Route path="/swarm-services/edit/:id" element={<ActionButtons resource={resource} />} />
        <Route path="/swarm-services" element={<span>Swarm Services list</span>} />
      </Routes>,
      { route: `/swarm-services/edit/${resource.id}` },
    );

    await user.click(screen.getByRole('button', { name: 'Delete' }));
    const dialog = await screen.findByRole('dialog');
    await user.type(within(dialog).getByRole('textbox', { name: `Enter ${resource.name} to confirm` }), resource.name);
    await user.click(within(dialog).getByRole('button', { name: 'Delete' }));

    expect(await screen.findByText('Swarm Services list')).toBeVisible();
    expect(submitted).toEqual([resource.id]);
  });

  it('opens a platform-scoped duplicate draft from the resource header', async () => {
    const Tags = SwarmServiceFormComponents.EditForm!.Header.Tags;
    const resource = managedService();
    server.use(
      http.get('http://localhost/api/v1/tags', () =>
        HttpResponse.json({
          tags: [],
          capabilities: { canRead: true, canWrite: true, canExecute: false },
        }),
      ),
    );
    const { user } = renderCitadel(
      <Routes>
        <Route path="/swarm-services/edit/:id" element={<Tags resource={resource} />} />
        <Route path="/platforms/:platformId/services/add" element={<span>Duplicate Service form</span>} />
      </Routes>,
      { route: `/swarm-services/edit/${resource.id}` },
    );

    await user.click(screen.getByRole('button', { name: 'Duplicate Config' }));

    expect(await screen.findByText('Duplicate Service form')).toBeVisible();
  });

  it('opens a terminal for Tasks on the connected manager and worker nodes', async () => {
    const Runtime = SwarmServiceFormComponents.EditForm!.Tabs.find((tab) => tab.label === 'Runtime')!.Content;
    const resource = managedService();
    server.use(
      http.get(`http://localhost/api/v1/platforms/${resource.platformId}`, () =>
        HttpResponse.json({
          id: resource.platformId,
          platformDescriptor: { $type: 'DockerSwarm', nodeID: 'manager-node' },
          capabilities: { canOpenTerminal: true },
        }),
      ),
      http.get(`http://localhost/api/v1/platforms/${resource.platformId}/swarm/tasks`, () =>
        HttpResponse.json({
          items: [
            task({ id: 'local-task', name: 'redis.1', nodeId: 'manager-node', nodeHostname: 'manager' }),
            task({ id: 'remote-task', name: 'redis.2', nodeId: 'worker-node', nodeHostname: 'worker' }),
          ],
          capabilities: { canOpenTerminal: true },
        }),
      ),
      http.get(`http://localhost/api/v1/swarmServices/${resource.id}/logs`, () => HttpResponse.json({ lines: [] })),
    );

    const { user } = renderRuntime(<Runtime resource={resource} />);

    await user.click(await screen.findByRole('tab', { name: 'Terminal' }));
    expect(await screen.findByText('Terminal connected to redis.1')).toBeVisible();

    await user.click(screen.getByRole('combobox', { name: 'Task' }));
    const remoteTask = await screen.findByRole('option', { name: /redis\.2.*worker/i });
    expect(remoteTask).not.toHaveAttribute('data-disabled');
    await user.click(remoteTask);
    expect(await screen.findByText('Terminal connected to redis.2')).toBeVisible();
  });

  it('shows the standard log viewer controls for Service logs', async () => {
    const Runtime = SwarmServiceFormComponents.EditForm!.Tabs.find((tab) => tab.label === 'Runtime')!.Content;
    const resource = managedService();
    server.use(
      http.get(`http://localhost/api/v1/platforms/${resource.platformId}/swarm/tasks`, () =>
        HttpResponse.json({ items: [task({ name: '' })], capabilities: {} }),
      ),
      http.get(`http://localhost/api/v1/swarmServices/${resource.id}/logs`, () =>
        HttpResponse.json({
          lines: [
            '2026-08-06T10:00:00Z com.docker.swarm.node.id=manager-node,com.docker.swarm.service.id=docker-service-1,com.docker.swarm.task.id=task-1 service output',
            '2026-08-06T10:00:01Z managed-web.1.task-1@manager | source output',
          ],
          truncated: false,
        }),
      ),
    );

    const { user } = renderRuntime(<Runtime resource={resource} />);

    expect(await screen.findByText('service output', undefined, { timeout: 5_000 })).toBeVisible();
    expect(screen.getByText('source output')).toBeVisible();
    expect(screen.getAllByText('[managed-web.1]')).toHaveLength(2);
    expect(screen.queryByText('[]')).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Timestamps' })).toBeVisible();
    expect(screen.getByRole('button', { name: 'Container filter' })).toBeVisible();
    expect(screen.getByRole('button', { name: 'Wrap Lines' })).toBeVisible();
    const clear = screen.getByRole('button', { name: 'Clear Console' });
    expect(clear).toBeVisible();
    await user.click(clear);
    expect(screen.getByText('No logs available...')).toBeVisible();
  });

  it('synchronizes the Runtime summary, Tasks, and Terminal after scaling', async () => {
    const Runtime = SwarmServiceFormComponents.EditForm!.Tabs.find((tab) => tab.label === 'Runtime')!.Content;
    const resource = managedService();
    server.use(
      http.get(`http://localhost/api/v1/platforms/${resource.platformId}`, () =>
        HttpResponse.json({
          id: resource.platformId,
          platformDescriptor: { $type: 'DockerSwarm', nodeID: 'manager-node' },
          capabilities: { canOpenTerminal: true },
        }),
      ),
      http.get(`http://localhost/api/v1/platforms/${resource.platformId}/swarm/tasks`, () =>
        HttpResponse.json({ items: [task()], capabilities: { canOpenTerminal: true } }),
      ),
      http.get(`http://localhost/api/v1/swarmServices/${resource.id}/logs`, () => HttpResponse.json({ lines: [] })),
    );

    const { fake, user } = renderRuntime(<Runtime resource={resource} />);

    expect(await screen.findByText('redis.1')).toBeVisible();
    await waitFor(() => expect(fake.listenerCount('SwarmInventoryUpdated')).toBe(2));

    act(() => {
      fake.emit(
        'SwarmInventoryUpdated',
        inventory(resource.platformId, 2, 2, [task(), task({ id: 'task-2', name: 'redis.2' })]),
      );
    });

    expect(await screen.findByText('2/2')).toBeVisible();
    expect(await screen.findByText('redis.2')).toBeVisible();

    await user.click(screen.getByRole('tab', { name: 'Terminal' }));
    expect(await screen.findByText('Terminal connected to redis.1')).toBeVisible();

    act(() => {
      fake.emit(
        'SwarmInventoryUpdated',
        inventory(resource.platformId, 3, 1, [task({ id: 'task-2', name: 'redis.2' })]),
      );
    });

    expect(await screen.findByText('1/1')).toBeVisible();
    expect(await screen.findByText('Terminal connected to redis.2')).toBeVisible();
    expect(screen.queryByText('Terminal connected to redis.1')).not.toBeInTheDocument();
  });

  it('streams node-agent statistics into the Task table and Service charts', async () => {
    const Runtime = SwarmServiceFormComponents.EditForm!.Tabs.find((tab) => tab.label === 'Runtime')!.Content;
    const resource = managedService();
    const projectionId = '019f0000-0000-7000-8000-000000000301';
    server.use(
      http.get(`http://localhost/api/v1/platforms/${resource.platformId}/swarm/tasks`, () =>
        HttpResponse.json({ items: [task()], capabilities: {} }),
      ),
      http.get(`http://localhost/api/v1/platforms/${resource.platformId}/containers`, () =>
        HttpResponse.json({
          containers: [
            {
              id: projectionId,
              platformId: resource.platformId,
              containerId: 'abcdef012345',
              name: '/managed-web.1.task-1',
              dockerImageId: 'sha256:redis',
              created: 1,
              state: 'Running',
              controlState: 'Idle',
              updated: 1,
              stack: null,
              isSystem: false,
              systemRole: null,
              hasCitadelOwnershipLabels: true,
              isSwarmTask: true,
              dockerNodeId: 'manager-node',
              nodeHostname: 'manager',
              projectionObservedAt: 1,
              projectionStaleSince: null,
              projectionStaleReason: null,
              lastStats: {
                containerId: projectionId,
                cpuUsage: 10,
                memoryActive: 128,
                memoryLimit: 512,
                created: 1,
              },
              ports: {},
              deploymentId: null,
              stackId: null,
            },
          ],
          capabilities: {},
        }),
      ),
      http.get(
        `http://localhost/api/v1/platforms/${resource.platformId}/swarm/services/${resource.dockerServiceId}/stats`,
        () =>
          HttpResponse.json({
            dockerServiceId: resource.dockerServiceId,
            observedTasks: 0,
            expectedTasks: 1,
            complete: false,
            observedContainerProjectionIds: [projectionId],
            missingDockerNodeIds: ['manager-node'],
            oldestSampleAt: '2026-08-06T10:00:00Z',
            newestSampleAt: '2026-08-06T10:00:00Z',
            stats: [
              {
                containerId: projectionId,
                cpuUsage: 10,
                memoryActive: 128,
                memoryLimit: 512,
                created: 1,
              },
            ],
          }),
      ),
      http.get(`http://localhost/api/v1/swarmServices/${resource.id}/logs`, () => HttpResponse.json({ lines: [] })),
    );

    const { fake, user } = renderRuntime(<Runtime resource={resource} />);

    const taskLink = await screen.findByRole('link', { name: 'redis.1' });
    const taskRow = taskLink.closest('tr');
    expect(taskRow).not.toBeNull();
    expect(await within(taskRow!).findByText(/10.*00/)).toBeVisible();
    expect(await within(taskRow!).findByText(/128.*512/)).toBeVisible();

    await user.click(screen.getByRole('tab', { name: 'Stats' }));
    expect(await screen.findByText('CPU Usage')).toBeVisible();
    expect(await screen.findByText('Partial Service statistics')).toBeVisible();
    await waitFor(() => expect(fake.listenerCount('ContainersStatsUpdated')).toBe(2));

    act(() => {
      fake.emit('ContainersStatsUpdated', [
        {
          containerId: projectionId,
          cpuUsage: 25,
          memoryActive: 256,
          memoryLimit: 512,
          created: 2,
        },
      ]);
    });

    await waitFor(() => expect(within(taskRow!).getByText(/25.*00/)).toBeVisible());
    expect(screen.getAllByText(/25.*00/).length).toBeGreaterThanOrEqual(2);
    await waitFor(() => expect(screen.queryByText('Partial Service statistics')).not.toBeInTheDocument());
  });
});

const renderRuntime = (ui: React.ReactElement) => {
  const fake = new FakeHubConnection();
  return {
    fake,
    ...renderCitadel(ui, {
      signalR: {
        connectionFactory: () => fake.asHubConnection(),
        startConnection: (connection) => connection.start(),
      },
    }),
  };
};

const inventory = (platformId: string, running: number, desired: number, tasks: ReturnType<typeof task>[]) => ({
  platformId,
  services: {
    items: [
      {
        id: 'docker-service-1',
        runningTaskCount: running,
        desiredTaskCount: desired,
        updateState: running === desired ? 'completed' : 'updating',
      },
    ],
  },
  tasks: { items: tasks },
});

const task = (overrides: Partial<Record<'id' | 'name' | 'nodeId' | 'nodeHostname', string>> = {}) => ({
  id: 'task-1',
  versionIndex: 1,
  name: 'redis.1',
  serviceId: 'docker-service-1',
  serviceName: 'managed-web',
  slot: 1,
  nodeId: 'manager-node',
  nodeHostname: 'manager',
  desiredState: 'Running',
  state: 'Running',
  statusMessage: null,
  error: null,
  image: 'redis:latest',
  ports: [],
  statusTimestamp: '2026-08-06T10:00:00Z',
  createdAt: '2026-08-06T09:00:00Z',
  updatedAt: '2026-08-06T10:00:00Z',
  observedAt: '2026-08-06T10:00:00Z',
  isStale: false,
  capabilities: { canOpenTerminal: true },
  ...overrides,
});

const managedService = (overrides: Partial<ManagedSwarmServiceView> = {}) =>
  ({
    id: '019fd6f2-4cdd-7cba-8d5e-cd6429b538b7',
    platformId: '019fc87c-af18-71a4-923d-c89fe6fc42c9',
    name: 'managed-web',
    dockerServiceId: 'docker-service-1',
    controlState: ResourceControlState.Idle,
    health: SwarmServiceHealth.Healthy,
    appliedImageDigest: 'sha256:applied',
    runningTaskCount: 1,
    desiredTaskCount: 1,
    spec: {
      schedulingMode: SwarmServiceSchedulingMode.Replicated,
      replicas: 1,
      image: { $type: 'External', registryId: 'registry-1', imageTag: 'nginx:latest' },
    },
    capabilities: {
      canRead: true,
      canWrite: true,
      canExecute: true,
      canApply: true,
      canViewLogs: true,
    },
    ...overrides,
  }) as unknown as ManagedSwarmServiceView;
