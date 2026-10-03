import { ResourceInfoView } from '@/pages/resource-info';
import { FakeRealtimeConnection } from '@/test/fakes/realtime';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { act, screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { Route, Routes } from 'react-router';
import { TaskInfoComponents } from '.';

vi.mock('@/lib/monaco', () => ({
  MonacoEditor: ({ value }: { value: string }) => <pre>{value}</pre>,
}));

vi.mock('@/features/docker-resources/containers/container-info/container-stats', () => ({
  ContainerStatsCharts: ({
    resource,
    liveStats,
  }: {
    resource?: { containerStat?: { cpuUsage?: number } };
    liveStats: unknown[];
  }) => (
    <div>
      <span>Memory Usage</span>
      <span>CPU Usage</span>
      <span>Network Usage</span>
      <span>{resource?.containerStat?.cpuUsage?.toFixed(2)}%</span>
      <span data-testid="task-live-stat-count">{liveStats.length}</span>
    </div>
  ),
}));

vi.mock('@/features/docker-resources/containers/container-info/container-exec', () => ({
  SwarmTaskExec: ({ containerId }: { containerId: string }) => <div>Terminal connected to {containerId}</div>,
}));

const platformId = '00000000-0000-0000-0000-000000000200';
const containerId = '00000000-0000-0000-0000-000000000300';
const dockerContainerId = 'abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890';

describe('TaskInfoComponents', () => {
  beforeEach(() => localStorage.clear());

  it('uses resource capabilities and streams stats through the standard info page', async () => {
    const fake = new FakeRealtimeConnection();
    let statsRequests = 0;
    server.use(
      http.get(`http://localhost/api/v1/platforms/${platformId}/swarm/tasks/task-1`, () => HttpResponse.json(task())),
      http.get(`http://localhost/api/v1/platforms/${platformId}`, () =>
        HttpResponse.json({
          status: 'Online',
          capabilities: {
            canRead: false,
            canWrite: false,
            canExecute: false,
            canViewLogs: true,
            canInspect: true,
            canOpenTerminal: false,
            canPull: false,
          },
        }),
      ),
      http.get(`http://localhost/api/v1/platforms/${platformId}/swarm/tasks/task-1/stats`, () => {
        statsRequests++;
        return HttpResponse.json({
          dockerContainerId,
          containerProjectionId: containerId,
          stats: [
            {
              containerId,
              memoryActive: 256,
              memoryCache: 32,
              memoryLimit: 1024,
              cpuUsage: 12.5,
              rxBytes: 2048,
              txBytes: 1024,
              created: 1,
            },
          ],
        });
      }),
    );

    renderCitadel(
      <Routes>
        <Route
          path="/platforms/:platformId/tasks/:resourceId"
          element={<ResourceInfoView Components={TaskInfoComponents} type="Task" />}
        />
      </Routes>,
      {
        route: `/platforms/${platformId}/tasks/task-1`,
        groups: {
          connectionFactory: () => fake.asRealtimeConnection(),
          startConnection: (connection) => connection.start(),
        },
      },
    );

    expect(await screen.findByText('web.1')).toBeVisible();
    expect(screen.getByRole('columnheader', { name: 'Service' })).toBeVisible();
    expect(screen.getByRole('columnheader', { name: 'Node' })).toBeVisible();
    expect(screen.getByRole('columnheader', { name: 'Image' })).toBeVisible();
    expect(screen.queryByRole('columnheader', { name: 'State' })).not.toBeInTheDocument();
    expect(screen.getByRole('columnheader', { name: 'Desired' })).toBeVisible();
    expect(screen.getByRole('tab', { name: 'Logs' })).toBeDisabled();
    expect(screen.getByRole('tab', { name: 'Inspect' })).toBeDisabled();
    expect(screen.getByRole('tab', { name: 'Terminal' })).toBeDisabled();
    expect(screen.getByRole('tab', { name: 'Stats' })).toBeEnabled();
    expect(await screen.findByText('Memory Usage')).toBeVisible();
    expect(await screen.findByText('12.50%')).toBeVisible();
    expect(screen.getByRole('link', { name: 'web' })).toHaveAttribute(
      'href',
      `/platforms/${platformId}/services/service-1`,
    );
    expect(screen.getByRole('link', { name: 'manager-1' })).toHaveAttribute(
      'href',
      `/platforms/${platformId}/nodes/node-1`,
    );
    await waitFor(() => expect(statsRequests).toBe(1));
    await waitFor(() => expect(fake.invoke).toHaveBeenCalledWith('JoinGroup', `containers:${platformId}`));

    act(() => {
      fake.emit('ContainersStatsUpdated', [
        {
          containerId,
          memoryActive: 512,
          memoryCache: 64,
          memoryLimit: 2048,
          cpuUsage: 18.75,
          rxBytes: 4096,
          txBytes: 2048,
        },
      ]);
    });

    expect(await screen.findByText('18.75%')).toBeVisible();
    expect(screen.getByTestId('task-live-stat-count')).toHaveTextContent('1');
    expect(statsRequests).toBe(1);
  });

  it('opens a terminal for a running task on the connected manager with terminal permission', async () => {
    const fake = new FakeRealtimeConnection();
    const terminalTaskId = 'task-terminal';
    server.use(
      http.get(`http://localhost/api/v1/platforms/${platformId}/swarm/tasks/${terminalTaskId}`, () =>
        HttpResponse.json({
          ...task(),
          id: terminalTaskId,
          capabilities: {
            ...task().capabilities,
            canOpenTerminal: true,
          },
        }),
      ),
      http.get(`http://localhost/api/v1/platforms/${platformId}`, () =>
        HttpResponse.json({
          id: platformId,
          platformDescriptor: {
            $type: 'DockerSwarm',
            nodeID: 'node-1',
          },
          capabilities: {
            canRead: true,
            canWrite: false,
            canExecute: false,
            canViewLogs: false,
            canInspect: false,
            canOpenTerminal: true,
            canPull: false,
          },
        }),
      ),
      http.get(`http://localhost/api/v1/platforms/${platformId}/swarm/tasks/${terminalTaskId}/terminal`, () =>
        HttpResponse.json({ dockerContainerId }),
      ),
    );

    renderCitadel(
      <Routes>
        <Route
          path="/platforms/:platformId/tasks/:resourceId"
          element={<ResourceInfoView Components={TaskInfoComponents} type="Task" />}
        />
      </Routes>,
      {
        route: `/platforms/${platformId}/tasks/${terminalTaskId}`,
        groups: {
          connectionFactory: () => fake.asRealtimeConnection(),
          startConnection: (connection) => connection.start(),
        },
      },
    );

    expect(await screen.findByRole('tab', { name: 'Terminal' })).toBeEnabled();
    expect(await screen.findByText(`Terminal connected to ${dockerContainerId}`)).toBeVisible();
    expect(screen.getByRole('tab', { name: 'Inspect' })).toBeDisabled();
  });
});

const task = () => ({
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
  capabilities: {
    canRead: true,
    canWrite: false,
    canExecute: false,
    canViewLogs: false,
    canInspect: false,
    canOpenTerminal: false,
    canPull: false,
  },
});
