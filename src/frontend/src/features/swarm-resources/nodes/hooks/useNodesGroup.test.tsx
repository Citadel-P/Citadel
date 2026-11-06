import { SwarmNodeView } from '@/api/generated/api.types';
import { FakeRealtimeConnection } from '@/test/fakes/realtime';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { act, screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { useHTTPErrorHandler } from '@/lib/hooks';
import { toast } from 'sonner';
import { useNodeInfoGroup, useNodesGroup } from './useNodesGroup';

const platformId = '00000000-0000-0000-0000-000000000200';

const NodesProbe = () => {
  useHTTPErrorHandler();
  const { items } = useNodesGroup(platformId);
  return <span>{items[0] ? `${items[0].hostname}:${items[0].isStale}` : 'loading'}</span>;
};

const NodeInfoProbe = () => {
  useHTTPErrorHandler();
  const { resource } = useNodeInfoGroup(platformId, 'node-1');
  return <span>{resource ? `${resource.hostname}:${resource.isStale}` : 'loading'}</span>;
};

const CapabilitiesProbe = () => {
  const { items } = useNodesGroup(platformId);
  return <span>{items[0] ? `${items[0].hostname}:${items[0].capabilities?.canInspect}` : 'loading'}</span>;
};

describe('useNodesGroup', () => {
  it('applies a realtime snapshot without refetching', async () => {
    const fake = new FakeRealtimeConnection();
    let requestCount = 0;
    server.use(
      http.get(`http://localhost/api/v1/platforms/${platformId}/swarm/nodes`, () => {
        requestCount++;
        return HttpResponse.json({ items: [createNode()] });
      }),
      http.get(`http://localhost/api/v1/platforms/${platformId}/swarm/tasks`, () => HttpResponse.json({ items: [] })),
    );

    renderCitadel(<NodesProbe />, {
      groups: {
        connectionFactory: () => fake.asRealtimeConnection(),
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

  it.each(['list', 'detail'])(
    'keeps a newer realtime snapshot when the initial %s request completes late',
    async (view) => {
      const errorToast = vi.spyOn(toast, 'error');
      const fake = new FakeRealtimeConnection();
      let releaseResponse!: () => void;
      const responseGate = new Promise<void>((resolve) => {
        releaseResponse = resolve;
      });
      let requestCount = 0;
      let requestSignal: AbortSignal | null | undefined;
      const node = createNode({ hostname: 'older-http-state' });
      const payload = view === 'detail' ? node : { items: [node] };
      const response = HttpResponse.json(payload);
      const requestUrl = `http://localhost/api/v1/platforms/${platformId}/swarm/nodes${view === 'detail' ? '/node-1' : ''}`;
      const fetch = globalThis.fetch;
      // Control this transport directly: a pending MSW resolver can escape to
      // localhost. Deliberately finish after abort to exercise stale-result fencing.
      vi.spyOn(globalThis, 'fetch').mockImplementation((input, init) => {
        if (input !== requestUrl) return fetch(input, init);
        requestCount++;
        requestSignal = init?.signal;
        return responseGate.then(() => response);
      });
      server.use(
        http.get(`http://localhost/api/v1/platforms/${platformId}/swarm/tasks`, () => HttpResponse.json({ items: [] })),
      );

      const { queryClient } = renderCitadel(view === 'detail' ? <NodeInfoProbe /> : <NodesProbe />, {
        groups: {
          connectionFactory: () => fake.asRealtimeConnection(),
          startConnection: (connection) => connection.start(),
        },
      });
      await waitFor(() => expect(requestCount).toBe(1));
      await waitFor(() => expect(fake.listenerCount('SwarmInventoryUpdated')).toBe(2));

      act(() => {
        fake.emit('SwarmInventoryUpdated', inventory([createNode({ hostname: 'newer-realtime-state' })]));
      });
      expect(await screen.findByText('newer-realtime-state:false')).toBeVisible();
      expect(requestSignal?.aborted).toBe(true);

      await act(async () => {
        releaseResponse();
        await responseGate;
      });

      // Wait for the generated client to parse the late response, not just for
      // the gate to open, before checking that it cannot overwrite the snapshot.
      await waitFor(() => expect(response).toHaveProperty('data', payload));
      await waitFor(() => expect(screen.getByText('newer-realtime-state:false')).toBeVisible());
      expect(screen.queryByText('older-http-state:false')).not.toBeInTheDocument();
      expect(requestCount).toBe(1);
      expect(errorToast).not.toHaveBeenCalled();
      const queryKey =
        view === 'detail' ? ['getSwarmNode', { platformId, nodeId: 'node-1' }] : ['listSwarmNodes', { platformId }];
      expect(queryClient.getQueryState(queryKey)).toMatchObject({
        status: 'success',
        fetchStatus: 'idle',
        error: null,
      });
    },
  );

  it('preserves caller capabilities when realtime replaces collection items', async () => {
    const fake = new FakeRealtimeConnection();
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
      groups: {
        connectionFactory: () => fake.asRealtimeConnection(),
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
  canManageNodeAgents: false,
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
  capabilities: null,
  ...overrides,
});
