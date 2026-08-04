import { act, screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { SwarmNodeView } from '@/api/generated/api.types';
import { FakeHubConnection } from '@/test/fakes/signalr';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { useSwarmNodes } from './use-swarm-nodes';

const platformId = '00000000-0000-0000-0000-000000000200';

function NodesProbe() {
  const { nodes } = useSwarmNodes(platformId);
  return <span>{nodes?.items[0] ? `${nodes.items[0].hostname}:${nodes.items[0].isStale}` : 'loading'}</span>;
}

describe('useSwarmNodes', () => {
  it('applies the post-commit SignalR snapshot without refetching', async () => {
    const fake = new FakeHubConnection();
    const initial = createNode({ hostname: 'manager-1', isStale: false });
    let requestCount = 0;
    server.use(
      http.get(`http://localhost/api/v1/platforms/${platformId}/swarm/nodes`, () => {
        requestCount++;
        return HttpResponse.json({ items: [initial] });
      }),
    );

    const { queryClient } = renderCitadel(<NodesProbe />, {
      signalR: {
        connectionFactory: () => fake.asHubConnection(),
        startConnection: (connection) => connection.start(),
      },
    });

    expect(await screen.findByText('manager-1:false')).toBeVisible();
    await waitFor(() => expect(fake.listenerCount('SwarmInventoryUpdated')).toBe(1));

    act(() => {
      fake.emit('SwarmInventoryUpdated', {
        platformId,
        nodes: { items: [createNode({ hostname: 'manager-renamed', isStale: true })] },
        services: { items: [] },
        tasks: { items: [] },
        networks: { items: [] },
        secrets: { items: [] },
        configs: { items: [] },
      });
    });

    await waitFor(() => expect(screen.getByText('manager-renamed:true')).toBeVisible());
    expect(requestCount).toBe(1);

    act(() => {
      fake.emit('SwarmInventoryUpdated', {
        platformId: '00000000-0000-0000-0000-000000000201',
        nodes: { items: [createNode({ hostname: 'wrong-platform' })] },
        services: { items: [] },
        tasks: { items: [] },
        networks: { items: [] },
        secrets: { items: [] },
        configs: { items: [] },
      });
    });
    expect(screen.queryByText('wrong-platform:false')).not.toBeInTheDocument();

    act(() => {
      queryClient.setQueryData(['listSwarmNodes', { platformId }], {
        data: { items: [createNode({ hostname: 'newer-http-state', isStale: false })] },
      });
    });
    await waitFor(() => expect(screen.getByText('newer-http-state:false')).toBeVisible());
    expect(requestCount).toBe(1);
  });

  it('keeps a newer SignalR snapshot when the initial request completes late', async () => {
    const fake = new FakeHubConnection();
    let releaseResponse!: () => void;
    const responseGate = new Promise<void>((resolve) => {
      releaseResponse = resolve;
    });
    let requestCount = 0;
    server.use(
      http.get(`http://localhost/api/v1/platforms/${platformId}/swarm/nodes`, async () => {
        requestCount++;
        await responseGate;
        return HttpResponse.json({ items: [createNode({ hostname: 'older-http-state' })] });
      }),
    );

    renderCitadel(<NodesProbe />, {
      signalR: {
        connectionFactory: () => fake.asHubConnection(),
        startConnection: (connection) => connection.start(),
      },
    });
    await waitFor(() => expect(requestCount).toBe(1));
    await waitFor(() => expect(fake.listenerCount('SwarmInventoryUpdated')).toBe(1));

    act(() => {
      fake.emit('SwarmInventoryUpdated', {
        platformId,
        nodes: { items: [createNode({ hostname: 'newer-signalr-state' })] },
        services: { items: [] },
        tasks: { items: [] },
        networks: { items: [] },
        secrets: { items: [] },
        configs: { items: [] },
      });
    });
    expect(await screen.findByText('newer-signalr-state:false')).toBeVisible();

    await act(async () => {
      releaseResponse();
      await responseGate;
    });

    await waitFor(() => expect(screen.getByText('newer-signalr-state:false')).toBeVisible());
    expect(screen.queryByText('older-http-state:false')).not.toBeInTheDocument();
    expect(requestCount).toBe(1);
  });
});

function createNode(overrides: Partial<SwarmNodeView> = {}): SwarmNodeView {
  return {
    id: 'node-1',
    versionIndex: 1,
    hostname: 'manager-1',
    role: 'Manager',
    isLeader: true,
    reachability: 'Reachable',
    status: 'Ready',
    statusMessage: null,
    availability: 'Active',
    engineVersion: '28.0',
    operatingSystem: 'linux',
    architecture: 'x86_64',
    address: '10.0.0.1',
    labels: {},
    runningTaskCount: 1,
    desiredTaskCount: 1,
    createdAt: null,
    updatedAt: null,
    observedAt: '2026-08-03T12:00:00Z',
    isStale: false,
    ...overrides,
  };
}
