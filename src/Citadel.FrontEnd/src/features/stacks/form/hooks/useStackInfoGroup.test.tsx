import { act, screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import {
  ContainerDataView,
  ContainerStateStatus,
  ContainerView,
  ResourceControlState,
} from '@/api/generated/api.types';
import { FakeHubConnection } from '@/test/fakes/signalr';
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
    </>
  );
}

describe('useStackInfoGroup', () => {
  it('keeps a Docker lifecycle event when a stale stats snapshot follows it', async () => {
    const fake = new FakeHubConnection();
    const initial = createContainer(ContainerStateStatus.Running);
    server.use(
      http.get(`http://localhost/api/v1/stacks/${stackId}/data`, () => HttpResponse.json({ containers: [initial] })),
    );

    renderCitadel(
      <AppContext.Provider
        value={{
          isLoading: false,
          currentPlatform: undefined,
          platforms: undefined,
          unresolvedAlertCount: 0,
          liveAlertEvents: {},
          receivedAlertEventIds: [],
        }}>
        <StackContainersProbe />
      </AppContext.Provider>,
      {
        signalR: {
          connectionFactory: () => fake.asHubConnection(),
          startConnection: (connection) => connection.start(),
        },
      },
    );

    expect(await screen.findByTestId('state')).toHaveTextContent(ContainerStateStatus.Running);
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
        } as ContainerView,
        'die',
      );
    });
    expect(screen.getByTestId('state')).toHaveTextContent(ContainerStateStatus.Exited);

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
  });
});

const createContainer = (state: ContainerStateStatus): ContainerDataView =>
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
  }) as ContainerDataView;
