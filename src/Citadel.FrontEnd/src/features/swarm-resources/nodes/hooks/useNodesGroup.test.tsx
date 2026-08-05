import { SwarmNodeView } from '@/api/generated/api.types';
import { FakeHubConnection } from '@/test/fakes/signalr';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { act, screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { useNodesGroup } from './useNodesGroup';

const platformId = '00000000-0000-0000-0000-000000000200';

const NodesProbe = () => {
  const { items } = useNodesGroup(platformId);
  return <span>{items[0] ? `${items[0].hostname}:${items[0].isStale}` : 'loading'}</span>;
};

const CapabilitiesProbe = () => {
  const { items } = useNodesGroup(platformId);
  return <span>{items[0] ? `${items[0].hostname}:${items[0].capabilities?.canInspect}` : 'loading'}</span>;
};

describe('useNodesGroup', () => {
  it('applies a SignalR snapshot without refetching', async () => {
    const fake = new FakeHubConnection();
    let requestCount = 0;
    server.use(
      http.get(`http://localhost/api/v1/platforms/${platformId}/swarm/nodes`, () => {
        requestCount++;
        return HttpResponse.json({ items: [createNode()] });
      }),
      http.get(`http://localhost/api/v1/platforms/${platformId}/swarm/tasks`, () => HttpResponse.json({ items: [] })),
    );

    renderCitadel(<NodesProbe />, {
      signalR: {
        connectionFactory: () => fake.asHubConnection(),
        startConnection: (connection) => connection.start(),
      },
    });

    expect(await screen.findByText('manager-1:false')).toBeVisible();
    await waitFor(() => expect(fake.listenerCount('SwarmInventoryUpdated')).toBe(2));

    act(() => {
      fake.emit('SwarmInventoryUpdated', inventory([createNode({ hostname: 'manager-renamed', isStale: true })]));
    });

    await waitFor(() => expect(screen.getByText('manager-renamed:true')).toBeVisible());
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
      http.get(`http://localhost/api/v1/platforms/${platformId}/swarm/tasks`, () => HttpResponse.json({ items: [] })),
    );

    renderCitadel(<NodesProbe />, {
      signalR: {
        connectionFactory: () => fake.asHubConnection(),
        startConnection: (connection) => connection.start(),
      },
    });
    await waitFor(() => expect(requestCount).toBe(1));
    await waitFor(() => expect(fake.listenerCount('SwarmInventoryUpdated')).toBe(2));

    act(() => {
      fake.emit('SwarmInventoryUpdated', inventory([createNode({ hostname: 'newer-signalr-state' })]));
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

  it('preserves caller capabilities when SignalR replaces collection items', async () => {
    const fake = new FakeHubConnection();
    server.use(
      http.get(`http://localhost/api/v1/platforms/${platformId}/swarm/nodes`, () =>
        HttpResponse.json({
          items: [createNode({ capabilities: platformCapabilities })],
          capabilities: platformCapabilities,
        }),
      ),
      http.get(`http://localhost/api/v1/platforms/${platformId}/swarm/tasks`, () => HttpResponse.json({ items: [] })),
    );

    renderCitadel(<CapabilitiesProbe />, {
      signalR: {
        connectionFactory: () => fake.asHubConnection(),
        startConnection: (connection) => connection.start(),
      },
    });

    expect(await screen.findByText('manager-1:true')).toBeVisible();
    await waitFor(() => expect(fake.listenerCount('SwarmInventoryUpdated')).toBe(2));

    act(() => {
      fake.emit('SwarmInventoryUpdated', inventory([createNode({ hostname: 'manager-renamed' })]));
    });

    expect(await screen.findByText('manager-renamed:true')).toBeVisible();
  });
});

const platformCapabilities = {
  canRead: true,
  canWrite: false,
  canExecute: false,
  canViewLogs: false,
  canInspect: true,
  canOpenTerminal: false,
  canPull: false,
};

const inventory = (nodes: SwarmNodeView[]) => ({
  platformId,
  nodes: { items: nodes },
  services: { items: [] },
  tasks: { items: [] },
  networks: { items: [] },
  secrets: { items: [] },
  configs: { items: [] },
});

const createNode = (overrides: Partial<SwarmNodeView> = {}): SwarmNodeView => ({
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
});
