import { act, screen, waitFor, within } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { FakeRealtimeConnection } from '@/test/fakes/realtime';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { SwarmInventoryUpdate } from '@/features/platforms/hooks/useDockerDaemonGroup';
import { SwarmQuorumState } from '@/api/generated/api.types';
import { useHTTPErrorHandler } from '@/lib/hooks';
import { toast } from 'sonner';
import { createQueryClient } from '@/query-client-wrapper';
import { useSwarmOverview } from './hooks/useSwarmOverview';
import { SwarmPlatformSummary } from './platform-summary';

const platformId = '00000000-0000-0000-0000-000000000200';

describe('SwarmPlatformSummary', () => {
  it('refreshes stale inventory from the manager and clears the warning after success', async () => {
    const fake = new FakeRealtimeConnection();
    let refreshed = false;
    let release!: () => void;
    const gate = new Promise<void>((resolve) => {
      release = resolve;
    });
    const refresh = vi.fn(async () => {
      await gate;
      refreshed = true;
      return new HttpResponse(null, { status: 204 });
    });
    server.use(
      http.get(`http://localhost/api/v1/platforms/${platformId}/swarm`, () =>
        HttpResponse.json(
          refreshed
            ? overview()
            : { ...overview(), health: 'Stale', isStale: true, message: 'Last-known inventory may be out of date.' },
        ),
      ),
      http.post(`http://localhost/api/v1/platforms/${platformId}/swarm/refresh`, refresh),
    );
    const { user } = renderCitadel(
      <SwarmPlatformSummary platformId={platformId} networkCount={4} serviceStatusCounts={managedServiceCounts()} />,
      {
        groups: {
          connectionFactory: () => fake.asRealtimeConnection(),
          startConnection: (connection) => connection.start(),
        },
      },
    );
    await user.click(await screen.findByRole('button', { name: 'Refresh inventory' }));
    expect(await screen.findByRole('button', { name: 'Refreshing…' })).toBeDisabled();
    expect(screen.getByText('Stale')).toBeVisible();
    release();
    await waitFor(() => expect(screen.queryByText('Stale')).not.toBeInTheDocument());
    expect(refresh).toHaveBeenCalledOnce();
    expect(screen.queryByRole('button', { name: 'Refresh inventory' })).not.toBeInTheDocument();
  });

  it('explains a successful manager refresh when node-local inventory is still stale', async () => {
    const fake = new FakeRealtimeConnection();
    server.use(
      http.get(`http://localhost/api/v1/platforms/${platformId}/swarm`, () =>
        HttpResponse.json({
          ...overview(),
          health: 'Stale',
          isStale: true,
          message: 'Some node-local inventory is out of date.',
        }),
      ),
      http.post(
        `http://localhost/api/v1/platforms/${platformId}/swarm/refresh`,
        () => new HttpResponse(null, { status: 204 }),
      ),
    );
    const { user } = renderCitadel(
      <SwarmPlatformSummary platformId={platformId} networkCount={4} serviceStatusCounts={managedServiceCounts()} />,
      {
        groups: {
          connectionFactory: () => fake.asRealtimeConnection(),
          startConnection: (connection) => connection.start(),
        },
      },
    );
    await user.click(await screen.findByRole('button', { name: 'Refresh inventory' }));
    expect(await screen.findByText('Inventory refreshed')).toBeVisible();
    expect(screen.getByText('Stale')).toBeVisible();
    expect(screen.getByText(/Check Node Agent connections/)).toBeVisible();
    expect(screen.queryByText('Inventory refresh failed')).not.toBeInTheDocument();
  });

  it('keeps stale inventory visible, shows the refresh error, and allows another attempt', async () => {
    const fake = new FakeRealtimeConnection();
    const refresh = vi.fn(() =>
      HttpResponse.json(
        { title: 'Bad Gateway', status: 502, detail: 'The Swarm manager could not be reached.' },
        { status: 502 },
      ),
    );
    server.use(
      http.get(`http://localhost/api/v1/platforms/${platformId}/swarm`, () =>
        HttpResponse.json({
          ...overview(),
          health: 'Stale',
          isStale: true,
          message: 'Last-known inventory may be out of date.',
        }),
      ),
      http.post(`http://localhost/api/v1/platforms/${platformId}/swarm/refresh`, refresh),
    );
    const { user } = renderCitadel(
      <SwarmPlatformSummary platformId={platformId} networkCount={4} serviceStatusCounts={managedServiceCounts()} />,
      {
        groups: {
          connectionFactory: () => fake.asRealtimeConnection(),
          startConnection: (connection) => connection.start(),
        },
      },
    );
    await user.click(await screen.findByRole('button', { name: 'Refresh inventory' }));
    expect(await screen.findByText('The Swarm manager could not be reached.')).toBeVisible();
    expect(screen.getByText('Stale')).toBeVisible();
    expect(screen.getByRole('button', { name: 'Refresh inventory' })).toBeEnabled();
    await user.click(screen.getByRole('button', { name: 'Refresh inventory' }));
    await waitFor(() => expect(refresh).toHaveBeenCalledTimes(2));
  });

  it('keeps an early realtime snapshot instead of allowing an older HTTP response to replace it', async () => {
    const errorToast = vi.spyOn(toast, 'error');
    const fake = new FakeRealtimeConnection();
    let releaseResponse!: () => void;
    const responseGate = new Promise<void>((resolve) => {
      releaseResponse = resolve;
    });
    let requestCount = 0;
    server.use(
      http.get(`http://localhost/api/v1/platforms/${platformId}/swarm`, async () => {
        requestCount++;
        await responseGate;
        return HttpResponse.json({
          ...overview(),
          health: 'Stale',
          message: 'older HTTP state',
          isStale: true,
        });
      }),
    );

    function RequestErrors() {
      useHTTPErrorHandler();
      return null;
    }
    const { queryClient } = renderCitadel(
      <>
        <RequestErrors />
        <SwarmPlatformSummary platformId={platformId} networkCount={4} serviceStatusCounts={managedServiceCounts()} />
      </>,
      {
        groups: {
          connectionFactory: () => fake.asRealtimeConnection(),
          startConnection: (connection) => connection.start(),
        },
      },
    );

    await waitFor(() => expect(requestCount).toBe(1));
    await waitFor(() => expect(fake.listenerCount('SwarmInventoryUpdated')).toBe(1));

    act(() =>
      fake.emit('SwarmNodeLocalResourcesUpdated', {
        platformId,
        images: [],
        volumes: [],
        networks: [{ dockerNodeId: 'worker-1' }, { dockerNodeId: 'worker-2' }],
      }),
    );
    act(() => fake.emit('SwarmInventoryUpdated', emptyInventory()));

    const summary = await screen.findByRole('region', { name: 'Swarm cluster summary' });
    await waitFor(() => expect(within(summary).getByRole('link', { name: /Managers/ })).toHaveTextContent('0'));
    const cards = within(summary).getAllByRole('link');
    expect(cards[0]).toHaveTextContent('Managers');
    expect(cards[1]).toHaveTextContent('Nodes');
    expect(cards[2]).toHaveTextContent('Services');
    expect(cards[2]).toHaveTextContent('4');
    expect(cards[2]).toHaveAttribute('href', `/swarm-services?platformId=${platformId}`);
    expect(within(cards[2]).getByLabelText('Healthy: 1')).toBeVisible();
    expect(within(cards[2]).getByLabelText('Failed: 1')).toBeVisible();
    expect(cards[3]).toHaveTextContent('Running tasks');
    expect(cards[4]).toHaveTextContent('Networks');
    expect(cards[4]).toHaveTextContent('2');
    expect(cards[4]).toHaveAttribute('href', `/platforms/${platformId}/networks`);
    expect(cards[5]).toHaveTextContent('Backups');
    expect(cards[5]).toHaveAttribute('href', '/backup-policies');
    expect(screen.queryByText('Swarm cluster')).not.toBeInTheDocument();
    expect(screen.queryByText('Healthy')).not.toBeInTheDocument();
    expect(screen.queryByText(/Last observed/)).not.toBeInTheDocument();
    expect(screen.queryByText('No inventory observed yet.')).not.toBeInTheDocument();
    expect(screen.getByRole('link', { name: /Nodes/ })).toHaveAttribute('href', `/platforms/${platformId}/nodes`);
    expect(screen.getByRole('status', { name: 'Swarm quorum unknown' })).toBeVisible();
    expect(screen.queryByText('older HTTP state')).not.toBeInTheDocument();

    await act(async () => {
      releaseResponse();
      await responseGate;
    });

    await waitFor(() => expect(screen.getByRole('region', { name: 'Swarm cluster summary' })).toBeVisible());
    expect(screen.queryByText('Healthy')).not.toBeInTheDocument();
    expect(screen.queryByText('older HTTP state')).not.toBeInTheDocument();
    expect(requestCount).toBe(1);
    expect(errorToast).not.toHaveBeenCalled();
    expect(queryClient.getQueryState(['getSwarmOverview', { platformId }])).toMatchObject({
      status: 'success',
      fetchStatus: 'idle',
      error: null,
    });
  });

  it.each([false, true])('handles an early node-local snapshot with cached overview: %s', async (hasCachedOverview) => {
    const errorToast = vi.spyOn(toast, 'error');
    const fake = new FakeRealtimeConnection();
    const queryClient = createQueryClient();
    const queryKey = ['getSwarmOverview', { platformId }];
    if (hasCachedOverview) {
      queryClient.setQueryData(queryKey, { data: { ...overview(), nodeCount: 2 } });
    }
    let releaseResponse!: () => void;
    const responseGate = new Promise<void>((resolve) => {
      releaseResponse = resolve;
    });
    const read = vi.fn(async () => {
      await responseGate;
      return HttpResponse.json({ ...overview(), nodeCount: 3 });
    });
    server.use(http.get(`http://localhost/api/v1/platforms/${platformId}/swarm`, read));
    function Probe() {
      useHTTPErrorHandler();
      const { overview: current } = useSwarmOverview(platformId);
      return <span>{current ? `${current.nodeCount} nodes, ${current.volumeCount} volumes` : 'Loading'}</span>;
    }
    renderCitadel(<Probe />, {
      queryClient,
      groups: {
        connectionFactory: () => fake.asRealtimeConnection(),
        startConnection: (connection) => connection.start(),
      },
    });
    try {
      await waitFor(() => expect(read).toHaveBeenCalledOnce());
      await waitFor(() => expect(fake.listenerCount('SwarmNodeLocalResourcesUpdated')).toBe(1));
      await act(async () => fake.emit('SwarmNodeLocalResourcesUpdated', { platformId, volumes: [], volumeCount: 4 }));
      await act(async () => fake.emit('SwarmNodeLocalResourcesUpdated', { platformId, nodeOnly: true, volumes: [] }));
      await act(async () => fake.emit('SwarmNodeLocalResourcesUpdated', { platformId, volumeCount: 5 }));
      expect(queryClient.getQueryState(queryKey)).toMatchObject({
        status: hasCachedOverview ? 'success' : 'pending',
        fetchStatus: hasCachedOverview ? 'idle' : 'fetching',
        error: null,
      });
    } finally {
      await act(async () => {
        releaseResponse();
        await responseGate;
      });
    }
    expect(await screen.findByText(`${hasCachedOverview ? 2 : 3} nodes, 5 volumes`)).toBeVisible();
    expect(queryClient.getQueryState(queryKey)).toMatchObject({ status: 'success', fetchStatus: 'idle', error: null });
    expect(errorToast).not.toHaveBeenCalled();
  });

  it.each(['Stale', 'Offline', 'Degraded'])(
    'replaces a previous %s state with the authoritative recovered overview',
    async (health) => {
      const fake = new FakeRealtimeConnection();
      server.use(
        http.get(`http://localhost/api/v1/platforms/${platformId}/swarm`, () =>
          HttpResponse.json({
            ...overview(),
            health,
            isStale: true,
            message: 'previous state',
          }),
        ),
      );
      const view = renderCitadel(
        <SwarmPlatformSummary platformId={platformId} networkCount={0} serviceStatusCounts={managedServiceCounts()} />,
        {
          groups: {
            connectionFactory: () => fake.asRealtimeConnection(),
            startConnection: (connection) => connection.start(),
          },
        },
      );
      await screen.findByText('previous state');
      await waitFor(() => expect(fake.listenerCount('SwarmInventoryUpdated')).toBe(1));
      const inventory = healthyInventory();
      act(() => fake.emit('SwarmInventoryUpdated', inventory));
      await waitFor(() => expect(screen.queryByText('previous state')).not.toBeInTheDocument());
      expect(view.queryClient.getQueryData(['getSwarmOverview', { platformId }])).toMatchObject({
        data: inventory.overview,
      });
    },
  );

  it.each(['manager-first', 'local-first'])(
    'preserves node-local staleness and counts with %s delivery, including after manager refresh',
    async (order) => {
      const fake = new FakeRealtimeConnection();
      const stale = {
        ...healthyInventory().overview,
        health: 'Stale',
        isStale: true,
        message: 'Some inventory is out of date. Last-known state is shown.',
        volumeCount: 4,
        imageCount: 3,
        localNetworkCount: 2,
        networkCount: 5,
      };
      server.use(
        http.get(`http://localhost/api/v1/platforms/${platformId}/swarm`, () => HttpResponse.json(stale)),
        http.post(
          `http://localhost/api/v1/platforms/${platformId}/swarm/refresh`,
          () => new HttpResponse(null, { status: 204 }),
        ),
      );
      const view = renderCitadel(
        <SwarmPlatformSummary platformId={platformId} networkCount={5} serviceStatusCounts={managedServiceCounts()} />,
        {
          groups: {
            connectionFactory: () => fake.asRealtimeConnection(),
            startConnection: (connection) => connection.start(),
          },
        },
      );
      await screen.findByText(stale.message);
      await waitFor(() => expect(fake.listenerCount('SwarmInventoryUpdated')).toBe(1));
      const managerEvent = () => fake.emit('SwarmInventoryUpdated', { ...healthyInventory(), overview: stale });
      const localEvent = () =>
        fake.emit('SwarmNodeLocalResourcesUpdated', {
          platformId,
          nodeOnly: true,
          volumes: [{ isStale: true }],
          overview: stale,
        });
      act(() => {
        if (order === 'manager-first') {
          managerEvent();
          localEvent();
        } else {
          localEvent();
          managerEvent();
        }
      });
      expect(view.queryClient.getQueryData(['getSwarmOverview', { platformId }])).toMatchObject({ data: stale });
      expect(screen.getByText('Stale')).toBeVisible();
      await view.user.click(screen.getByRole('button', { name: 'Refresh inventory' }));
      expect(await screen.findByText(/Check Node Agent connections/)).toBeVisible();
      expect(screen.getByRole('button', { name: 'Refresh inventory' })).toBeVisible();
      expect(screen.getByText(stale.message)).toBeVisible();
    },
  );

  it('changes only supplied local counts and ignores another Platform', async () => {
    const fake = new FakeRealtimeConnection();
    const initial = {
      ...overview(),
      health: 'Stale',
      isStale: true,
      volumeCount: 7,
      imageCount: 3,
      networkCount: 5,
      localNetworkCount: 2,
    };
    server.use(http.get(`http://localhost/api/v1/platforms/${platformId}/swarm`, () => HttpResponse.json(initial)));
    function Probe() {
      const { overview: value } = useSwarmOverview(platformId);
      return <output>{JSON.stringify(value)}</output>;
    }
    const view = renderCitadel(<Probe />, {
      groups: {
        connectionFactory: () => fake.asRealtimeConnection(),
        startConnection: (connection) => connection.start(),
      },
    });
    await waitFor(() =>
      expect(view.queryClient.getQueryData(['getSwarmOverview', { platformId }])).toMatchObject({ data: initial }),
    );
    await waitFor(() => expect(fake.listenerCount('SwarmNodeLocalResourcesUpdated')).toBe(1));
    act(() =>
      fake.emit('SwarmNodeLocalResourcesUpdated', { platformId, nodeOnly: true, volumes: [{ isStale: true }] }),
    );
    expect(JSON.parse(screen.getByRole('status').textContent!)).toEqual(initial);
    act(() => fake.emit('SwarmNodeLocalResourcesUpdated', { platformId, images: [] }));
    expect(JSON.parse(screen.getByRole('status').textContent!)).toEqual({ ...initial, imageCount: 0 });
    act(() =>
      fake.emit('SwarmNodeLocalResourcesUpdated', { platformId: 'other', overview: healthyInventory().overview }),
    );
    expect(JSON.parse(screen.getByRole('status').textContent!)).toEqual({ ...initial, imageCount: 0 });
  });
});

function emptyInventory(): SwarmInventoryUpdate {
  return {
    platformId,
    overview: { ...overview(), health: 'Degraded', networkCount: 2, localNetworkCount: 2 },
    nodes: {
      items: [],
      capabilities: {
        canRead: true,
        canWrite: false,
        canExecute: false,
        canInspect: true,
        canViewLogs: true,
        canOpenTerminal: false,
        canPull: false,
        canManageNodeAgents: false,
      },
    },
    services: {
      items: [],
      capabilities: {
        canRead: true,
        canWrite: false,
        canExecute: false,
        canInspect: true,
        canViewLogs: true,
        canOpenTerminal: false,
        canPull: false,
        canManageNodeAgents: false,
      },
    },
    tasks: {
      items: [],
      capabilities: {
        canRead: true,
        canWrite: false,
        canExecute: false,
        canInspect: true,
        canViewLogs: true,
        canOpenTerminal: false,
        canPull: false,
        canManageNodeAgents: false,
      },
    },
    networks: {
      items: [],
      capabilities: {
        canRead: true,
        canWrite: false,
        canExecute: false,
        canInspect: true,
        canViewLogs: true,
        canOpenTerminal: false,
        canPull: false,
        canManageNodeAgents: false,
      },
    },
    secrets: {
      items: [],
      capabilities: {
        canRead: true,
        canWrite: false,
        canExecute: false,
        canInspect: true,
        canViewLogs: true,
        canOpenTerminal: false,
        canPull: false,
        canManageNodeAgents: false,
      },
    },
    configs: {
      items: [],
      capabilities: {
        canRead: true,
        canWrite: false,
        canExecute: false,
        canInspect: true,
        canViewLogs: true,
        canOpenTerminal: false,
        canPull: false,
        canManageNodeAgents: false,
      },
    },
  };
}

function healthyInventory(): SwarmInventoryUpdate {
  const inventory = emptyInventory();
  inventory.nodes.items = [manager('manager-1', true, 'Reachable')];
  inventory.overview = {
    ...overview(),
    nodeCount: 1,
    managerCount: 1,
    quorum: { state: SwarmQuorumState.Healthy, reachableManagers: 1, requiredManagers: 1, hasLeader: true },
  };
  return inventory;
}

function manager(id: string, isLeader: boolean, reachability: string, isStale = false) {
  return {
    id,
    role: 'Manager',
    isLeader,
    reachability,
    isStale,
  } as SwarmInventoryUpdate['nodes']['items'][number];
}

function overview() {
  return {
    platformId,
    health: 'Healthy',
    capabilities: {
      canRead: true,
      canWrite: false,
      canExecute: false,
      canInspect: true,
      canViewLogs: true,
      canOpenTerminal: false,
      canPull: false,
      canManageNodeAgents: false,
    },
    message: null,
    isStale: false,
    nodeCount: 0,
    managerCount: 0,
    quorum: {
      state: SwarmQuorumState.Unknown,
      reachableManagers: 0,
      requiredManagers: 0,
      hasLeader: false,
    },
    serviceCount: 0,
    serviceStatusCounts: {
      total: 0,
      healthy: 0,
      degraded: 0,
      failed: 0,
      stopped: 0,
      paused: 0,
      inProgress: 0,
      unknown: 0,
    },
    runningTaskCount: 0,
    desiredTaskCount: 0,
    networkCount: 0,
    localNetworkCount: 0,
    volumeCount: 0,
    imageCount: 0,
  };
}

function managedServiceCounts() {
  return {
    total: 4,
    healthy: 1,
    degraded: 1,
    failed: 1,
    stopped: 0,
    paused: 0,
    inProgress: 1,
    unknown: 0,
  };
}
