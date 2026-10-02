import { act, screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import {
  ContainerRuntimeView,
  ContainerStateStatus,
  ContainerView,
  ResourceControlState,
} from '@/api/generated/api.types';
import { FakeRealtimeConnection } from '@/test/fakes/realtime';
import { AppContext } from '@/lib/context/app-context';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { useStackInfoGroup } from './useStackInfoGroup';

const stackId = '00000000-0000-0000-0000-000000000001';
const platformId = '00000000-0000-0000-0000-000000000200';
const containerId = '0123456789abcdef';

function StackContainersProbe() {
  const { containersInfo } = useStackInfoGroup(stackId, platformId);
  const container = containersInfo[0];

  return (
    <>
      <span data-testid="state">{container?.state}</span>
      <span data-testid="cpu">{container?.containerStat?.cpuUsage ?? '-'}</span>
      <span data-testid="swarm-task">{String(container?.isSwarmTask ?? false)}</span>
      <span data-testid="docker-node">{container?.dockerNodeId ?? '-'}</span>
    </>
  );
}

function renderProbe(fake: FakeRealtimeConnection) {
  renderCitadel(
    <AppContext.Provider
      value={{
        isLoading: false,
        currentPlatform: undefined,
        platforms: undefined,
        applicationInfo: undefined,
        unresolvedAlertCount: 0,
        liveAlertEvents: {},
        receivedAlertEventIds: [],
      }}>
      <StackContainersProbe />
    </AppContext.Provider>,
    {
      groups: {
        connectionFactory: () => fake.asRealtimeConnection(),
        startConnection: (connection) => connection.start(),
      },
    },
  );
}

describe('useStackInfoGroup', () => {
  it('keeps a Docker lifecycle event when a stale stats snapshot follows it', async () => {
    const fake = new FakeRealtimeConnection();
    const initial = createContainer(ContainerStateStatus.Running);
    server.use(
      http.get(`http://localhost/api/v1/stacks/${stackId}/data`, () => HttpResponse.json({ containers: [initial] })),
    );

    renderProbe(fake);

    await waitFor(() => expect(screen.getByTestId('state')).toHaveTextContent(ContainerStateStatus.Running));
    await waitFor(() => {
      expect(fake.listenerCount('ContainerEventReceived')).toBe(1);
      expect(fake.listenerCount('ReceiveStackContainersInfo')).toBe(1);
    });

    act(() => {
      fake.emit(
        'ContainerEventReceived',
        {
          id: '00000000-0000-0000-0000-000000000300',
          platformId,
          containerId,
          name: '/beszel-copy-beszel-1',
          dockerImageId: 'image-id',
          created: 1,
          state: ContainerStateStatus.Exited,
          controlState: ResourceControlState.Idle,
          updated: 2,
          stack: 'different-compose-project',
          lastStats: null,
          ports: {},
          deploymentId: null,
          stackId,
          isSwarmTask: true,
          dockerNodeId: 'worker-1',
        } as ContainerView,
        'die',
      );
    });
    expect(screen.getByTestId('state')).toHaveTextContent(ContainerStateStatus.Exited);
    expect(screen.getByTestId('swarm-task')).toHaveTextContent('true');
    expect(screen.getByTestId('docker-node')).toHaveTextContent('worker-1');

    act(() => {
      fake.emit('ReceiveStackContainersInfo', [
        {
          ...initial,
          state: ContainerStateStatus.Running,
          containerStat: { cpuUsage: 42 },
        },
      ]);
    });

    expect(screen.getByTestId('state')).toHaveTextContent(ContainerStateStatus.Exited);
    expect(screen.getByTestId('cpu')).toHaveTextContent('42');
    expect(screen.getByTestId('swarm-task')).toHaveTextContent('true');

    // Start/stop now arrives as a compact patch rather than a full Docker event.
    act(() => {
      fake.emit('ContainerStateChanged', [
        {
          id: 'database-id',
          containerId,
          dockerNodeId: 'worker-1',
          state: ContainerStateStatus.Running,
        },
      ]);
    });
    expect(screen.getByTestId('state')).toHaveTextContent(ContainerStateStatus.Running);
    act(() => {
      fake.emit('ContainerStateChanged', [
        { id: 'other', containerId: 'another-container', state: ContainerStateStatus.Exited },
        { id: 'other-node', containerId, dockerNodeId: 'worker-2', state: ContainerStateStatus.Exited },
      ]);
      fake.emit('ReceiveStackContainersInfo', [{ ...initial, state: ContainerStateStatus.Exited }]);
    });
    expect(screen.getByTestId('state')).toHaveTextContent(ContainerStateStatus.Running);
  });

  it('retains complete runtime metadata from a realtime snapshot and does not resurrect destroyed containers', async () => {
    const fake = new FakeRealtimeConnection();
    server.use(http.get(`http://localhost/api/v1/stacks/${stackId}/data`, () => HttpResponse.json({ containers: [] })));
    renderProbe(fake);
    await waitFor(() => expect(fake.listenerCount('ReceiveStackContainersInfo')).toBe(1));

    const running = createContainer(ContainerStateStatus.Running);
    act(() => fake.emit('ReceiveStackContainersInfo', [running]));
    expect(screen.getByTestId('state')).toHaveTextContent(ContainerStateStatus.Running);
    act(() =>
      fake.emit('ContainerStateChanged', [
        {
          id: 'database-id',
          containerId,
          state: ContainerStateStatus.Exited,
        },
      ]),
    );
    expect(screen.getByTestId('state')).toHaveTextContent(ContainerStateStatus.Exited);
    act(() => {
      fake.emit('ContainerEventReceived', { containerId, stackId }, 'destroy');
      fake.emit('ReceiveStackContainersInfo', [running]);
    });
    expect(screen.getByTestId('state')).toBeEmptyDOMElement();
  });
});

const createContainer = (state: ContainerStateStatus): ContainerRuntimeView =>
  ({
    name: '/beszel-copy-beszel-1',
    image: 'example/beszel:latest',
    id: containerId,
    imageId: 'image-id',
    state,
    controlState: ResourceControlState.Idle,
    created: 1,
    stack: 'beszel-copy',
    containerStat: null,
    ports: {},
    deploymentId: null,
    stackId,
  }) as ContainerRuntimeView;
