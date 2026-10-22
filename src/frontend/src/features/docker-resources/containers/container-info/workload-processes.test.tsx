import { act, screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { renderCitadel as renderBase } from '@/test/render-citadel';
import { FakeRealtimeConnection } from '@/test/fakes/realtime';
import type { ReactElement } from 'react';
import { server } from '@/test/server';
import { DeploymentInspect, StackInspect } from './container-inspect';
import { TaskInspect } from '@/features/swarm-resources/tasks/task-info/inspect';
import { ServiceInspect } from '@/features/swarm-resources/services/service-info/inspect';
import { RuntimeProcesses } from './runtime-processes';

vi.mock('@/lib/monaco', () => ({ MonacoEditor: () => <div>Inspection JSON</div> }));

const platformId = 'platform-one';
const capabilities = {
  canRead: true,
  canInspect: true,
  canWrite: false,
  canExecute: false,
  canManageNodeAgents: false,
  canOpenTerminal: false,
  canPull: false,
  canViewLogs: false,
};
function renderCitadel(ui: ReactElement) {
  const realtime = new FakeRealtimeConnection();
  return {
    ...renderBase(ui, {
      groups: {
        connectionFactory: () => realtime,
        startConnection: (connection) => connection.start(),
      },
    }),
    realtime,
  };
}
const task = (id: string, nodeId: string, serviceId = 'service-one') => ({
  id,
  nodeId,
  serviceId,
  capabilities,
  name: `web.${id}`,
  nodeHostname: nodeId,
  state: 'running',
  isStale: false,
  desiredState: 'running',
  image: 'nginx',
  serviceName: 'web',
  slot: 1,
  versionIndex: 1,
  error: null,
  statusMessage: null,
  observedAt: '2026-10-22T00:00:00Z',
  statusTimestamp: null,
  createdAt: null,
  updatedAt: null,
  ports: [],
});
const container = (id: string, dockerId: string, nodeId: string | null = null, taskId?: string) => ({
  id,
  containerId: dockerId,
  platformId,
  dockerNodeId: nodeId,
  name: taskId ? `web.1.${taskId}` : 'web',
  isSwarmTask: !!taskId,
  projectionStaleSince: null,
  capabilities,
});
const processes = (command: string) => ({
  running: true,
  titles: ['PID', 'COMMAND'],
  processes: [['42', command]],
  truncated: false,
});
const processReads = vi.fn(({ params }: { params: Record<string, unknown> }) =>
  HttpResponse.json(processes(String(params.id))),
);

beforeEach(() => {
  processReads.mockClear();
  server.use(http.get('http://localhost/api/v1/containers/:id/processes', processReads));
});

it('adds deployment processes below inspection and queries by persisted UUID only when expanded', async () => {
  server.use(
    http.get('http://localhost/api/v1/deployments/deployment-one/inspect', () =>
      HttpResponse.json({ id: 'docker-one' }),
    ),
  );
  const { user } = renderCitadel(<DeploymentInspect deploymentId="deployment-one" resourceId="persisted-one" />);
  const trigger = screen.getByRole('button', { name: 'Processes' });
  expect(
    screen.getByText('Inspection JSON').compareDocumentPosition(trigger) & Node.DOCUMENT_POSITION_FOLLOWING,
  ).toBeTruthy();
  expect(processReads).not.toHaveBeenCalled();
  await user.click(trigger);
  expect(await screen.findByRole('cell', { name: 'persisted-one' })).toBeVisible();
  expect(processReads).toHaveBeenCalledTimes(1);
});

it('resolves the selected Stack container on its exact node without depending on statistics', async () => {
  server.use(
    http.get('http://localhost/api/v1/stacks/stack-one/containers/:id/inspect', () =>
      HttpResponse.json({ id: 'docker-one' }),
    ),
    http.get(`http://localhost/api/v1/platforms/${platformId}/containers`, () =>
      HttpResponse.json({
        capabilities,
        containers: [
          container('wrong-node', 'docker-one', 'node-two'),
          container('persisted-one', 'docker-one', 'node-one'),
          container('persisted-two', 'docker-two', 'node-one'),
        ],
      }),
    ),
  );
  const { user, rerender } = renderCitadel(
    <StackInspect stackId="stack-one" platformId={platformId} containerId="docker-one" dockerNodeId="node-one" />,
  );
  await user.click(screen.getByRole('button', { name: 'Processes' }));
  expect(await screen.findByRole('cell', { name: 'persisted-one' })).toBeVisible();
  expect(screen.queryByRole('cell', { name: 'wrong-node' })).not.toBeInTheDocument();
  rerender(
    <StackInspect stackId="stack-one" platformId={platformId} containerId="docker-two" dockerNodeId="node-one" />,
  );
  expect(await screen.findByRole('cell', { name: 'persisted-two' })).toBeVisible();
  expect(screen.queryByRole('cell', { name: 'persisted-one' })).not.toBeInTheDocument();
});

it('adds processes to task inspection and scopes the task to its node', async () => {
  server.use(
    http.get(`http://localhost/api/v1/platforms/${platformId}/swarm/tasks/task-one/inspect`, () =>
      HttpResponse.json({ id: 'docker-one' }),
    ),
    http.get(`http://localhost/api/v1/platforms/${platformId}/containers`, () =>
      HttpResponse.json({
        capabilities,
        containers: [
          container('wrong-node', 'docker-other', 'node-two', 'task-one'),
          container('task-container', 'docker-one', 'node-one', 'task-one'),
        ],
      }),
    ),
  );
  const { user } = renderCitadel(<TaskInspect task={{ ...task('task-one', 'node-one'), platformId }} />);
  await user.click(screen.getByRole('button', { name: 'Processes' }));
  expect(await screen.findByRole('cell', { name: 'task-container' })).toBeVisible();
  expect(processReads).toHaveBeenCalledTimes(1);
});

it('reads only the selected running task in service Inspect', async () => {
  const tasksRead = vi.fn(({ request }: { request: Request }) => {
    expect(new URL(request.url).searchParams.get('serviceId')).toBe('service-one');
    return HttpResponse.json({
      capabilities,
      items: [
        task('task-one', 'node-one'),
        task('task-two', 'node-two'),
        { ...task('stopped', 'node-one'), state: 'shutdown' },
        task('foreign-task', 'node-one', 'other-service'),
      ],
    });
  });
  server.use(
    http.get(`http://localhost/api/v1/platforms/${platformId}/swarm/services/service-one/inspect`, () =>
      HttpResponse.json({ id: 'service-one' }),
    ),
    http.get(`http://localhost/api/v1/platforms/${platformId}/swarm/tasks`, tasksRead),
    http.get(`http://localhost/api/v1/platforms/${platformId}/containers`, () =>
      HttpResponse.json({
        capabilities,
        containers: [
          container('persisted-one', 'docker-one', 'node-one', 'task-one'),
          container('persisted-two', 'docker-two', 'node-two', 'task-two'),
        ],
      }),
    ),
  );
  const { user, realtime } = renderCitadel(<ServiceInspect service={{ id: 'service-one', platformId }} />);
  expect(tasksRead).not.toHaveBeenCalled();
  expect(processReads).not.toHaveBeenCalled();
  await user.click(screen.getByRole('button', { name: 'Processes' }));
  expect(await screen.findByRole('cell', { name: 'persisted-one' })).toBeVisible();
  await user.click(screen.getByRole('combobox', { name: 'Processes task' }));
  expect(screen.getAllByRole('option')).toHaveLength(2);
  await user.click(screen.getByRole('option', { name: /node-two/ }));
  expect(await screen.findByRole('cell', { name: 'persisted-two' })).toBeVisible();
  expect(screen.queryByRole('cell', { name: 'persisted-one' })).not.toBeInTheDocument();
  expect(processReads).toHaveBeenCalledTimes(2);
  await waitFor(() => expect(realtime.invoke).toHaveBeenCalledWith('JoinGroup', 'container-processes:persisted-two'));
  expect(realtime.invoke).toHaveBeenCalledWith('LeaveGroup', 'container-processes:persisted-one');
  await act(async () => {
    realtime.emit('ContainerProcessesUpdated', { resourceId: 'persisted-one', data: processes('wrong-task') });
    realtime.emit('ContainerProcessesUpdated', { resourceId: 'persisted-two', data: processes('live-task') });
  });
  expect(screen.queryByRole('cell', { name: 'wrong-task' })).not.toBeInTheDocument();
  expect(screen.getByRole('cell', { name: 'live-task' })).toBeVisible();
  expect(processReads).toHaveBeenCalledTimes(2);
  await user.click(screen.getByRole('button', { name: 'Processes' }));
  await waitFor(() => expect(realtime.invoke).toHaveBeenCalledWith('LeaveGroup', 'container-processes:persisted-two'));
});

it.each(['denied', 'stale', 'missing'] as const)('does not read processes from %s inventory', async (state) => {
  server.use(
    http.get(`http://localhost/api/v1/platforms/${platformId}/containers`, () =>
      HttpResponse.json({
        capabilities: { ...capabilities, canInspect: state !== 'denied' },
        containers:
          state === 'missing'
            ? []
            : [{ ...container('persisted-one', 'docker-one'), projectionStaleSince: state === 'stale' ? 123 : null }],
      }),
    ),
  );
  const { user } = renderCitadel(<RuntimeProcesses platformId={platformId} containerId="docker-one" />);
  await user.click(screen.getByRole('button', { name: 'Processes' }));
  expect(
    await screen.findByText(
      state === 'denied'
        ? /Platform Read and Inspect permission/
        : state === 'stale'
          ? /inventory is stale/
          : /not available in the current inventory/,
    ),
  ).toBeVisible();
  expect(processReads).not.toHaveBeenCalled();
});
