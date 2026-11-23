import { collectionGroup, collectionEventKey } from '@/lib/collection-scope';
import { act, screen, waitFor } from '@testing-library/react';
import type { AuthorizedProject } from '@/api/generated/api.types';
import { http, HttpResponse } from 'msw';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { FakeWebSocket } from '@/test/fakes/websocket';
import { useVolumesGroup } from '@/features/docker-resources/volumes/hooks/useVolumesGroup';
import { useNetworksGroup } from '@/features/docker-resources/networks/hooks/useNetworksGroup';
import { useDeploymentGroup } from '@/features/deployments/form/hooks/useDeploymentGroup';
import { usePlatformGroup } from '@/features/platforms/forms/hooks/usePlatformGroup';
import { usePlatformsGroup } from '@/features/platforms/hooks/usePlatformsGroup';

import { FakeRealtimeConnection } from '@/test/fakes/realtime';
import { useGitRepoGroup } from '@/features/git-repos/form/hooks/useGitRepoGroup';
import { useStackGroup } from '@/features/stacks/form/hooks/useStackGroup';
import { useRead } from '@/lib/hooks';
import { BuildFormComponents } from '@/features/builds/form';
import { BuildPoolFormComponents } from '@/features/build-pools/form';
import { useActivitiesGroup } from '@/features/activities/hooks/useActivitiesGroup';
import { Route, Routes } from 'react-router';
import { ComponentProps, useCallback, useState } from 'react';
import { Platforms } from '@/features/platforms/platforms';
import { useQueryReconciliation } from '@/hooks/useQueryReconciliation';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import { GROUP_CAPACITY_ERROR, type RealtimeConnection } from '@/lib/realtime-connection';

afterEach(() => vi.useRealTimers());

function connect(socket: FakeWebSocket) {
  socket.send.mockImplementation((text: string) => {
    const request = JSON.parse(text);
    if (request.kind === 'invoke')
      queueMicrotask(() =>
        socket.message({
          protocolVersion: 1,
          kind: 'completion',
          invocationId: request.invocationId,
        }),
      );
  });
  act(() => socket.message({ protocolVersion: 1, kind: 'subscribed' }));
}
const deliver = (socket: FakeWebSocket, target: string, ...args: unknown[]) =>
  act(() => socket.message({ protocolVersion: 1, kind: 'event', target, arguments: args }));

it('updates an open Build activity tab and project status when a webhook queues a run', async () => {
  const project = { id: 'b1', name: 'Build', enabled: true, controlState: 'Idle', latestRun: null };
  const read = vi.fn(() => HttpResponse.json(project));
  const activities = vi.fn(() =>
    HttpResponse.json({ pagedResult: { items: [], totalCount: 0, page: 1, pageSize: 20 } }),
  );
  server.use(http.get('*/api/v1/buildProjects/b1', read), http.get('*/api/v1/activities', activities));
  function Probe() {
    const { item } = BuildFormComponents.EditForm!.useData('b1');
    const build = item as AuthorizedProject | undefined;
    const { pagedActivities } = useActivitiesGroup('b1', 'Build', 20);
    return (
      <>
        <output>{build?.latestRun?.status ?? 'Idle'}</output>
        {pagedActivities?.items.map((activity) => (
          <output key={activity.id}>{activity.eventType}</output>
        ))}
      </>
    );
  }
  const socket = new FakeWebSocket();
  const { queryClient } = renderCitadel(<Probe />, {
    groups: { realtimeTransport: 'WebSocketV1', webSocketFactory: () => socket.asWebSocket() },
  });
  expect(await screen.findByText('Idle')).toBeVisible();
  await waitFor(() => expect(activities).toHaveBeenCalledTimes(1));
  connect(socket);
  await waitFor(() => {
    const joins = socket.send.mock.calls
      .map(([text]) => JSON.parse(text))
      .filter((message) => message.kind === 'invoke');
    expect(joins).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ arguments: ['build-project:b1'] }),
        expect.objectContaining({ arguments: ['activity:Build:b1'] }),
      ]),
    );
  });
  // Connecting reconciles activity history before streamed updates take over.
  await waitFor(() => {
    expect(activities).toHaveBeenCalledTimes(2);
    expect(queryClient.isFetching()).toBe(0);
  });
  const activity = {
    id: 'a1',
    resourceId: 'b1',
    resourceType: 'Build',
    eventType: 'BuildWebhookReceived',
    status: 'Success',
  };
  deliver(socket, 'ActivityEventReceived', activity);
  deliver(socket, 'ActivityEventReceived', activity);
  deliver(
    socket,
    'BuildProjectInfoUpdated',
    {
      ...project,
      controlState: 'Processing',
      currentRunId: 'run1',
      latestRun: { id: 'run1', buildProjectId: 'b1', status: 'Queued', trigger: 'Webhook' },
    },
    'update',
  );
  expect(await screen.findByText('Queued')).toBeVisible();
  expect(screen.getAllByText('BuildWebhookReceived')).toHaveLength(1);
  expect(read).toHaveBeenCalledTimes(1);
  expect(activities).toHaveBeenCalledTimes(2);
});

it('uses the unchanged Deployment hook to apply a named event without refetching', async () => {
  const read = vi.fn(() => HttpResponse.json({ id: 'd1', status: 'Applying' }));
  server.use(http.get('*/api/v1/deployments/d1', read));
  function Probe() {
    const { deployment } = useDeploymentGroup('d1');
    return <output>{deployment?.status}</output>;
  }
  const socket = new FakeWebSocket();
  const rendered = renderCitadel(<Probe />, {
    groups: { realtimeTransport: 'WebSocketV1', webSocketFactory: () => socket.asWebSocket() },
  });
  expect(await screen.findByText('Applying')).toBeVisible();
  connect(socket);
  await waitFor(() => expect(socket.send).toHaveBeenCalled());
  deliver(socket, 'DeploymentInfoUpdated', { id: 'other', status: 'Failed' }, 'update');
  expect(screen.getByText('Applying')).toBeVisible();
  deliver(socket, 'DeploymentInfoUpdated', { id: 'd1', status: 'Healthy' }, 'update');
  expect(await screen.findByText('Healthy')).toBeVisible();
  expect(read).toHaveBeenCalledTimes(1);
  rendered.unmount();
});

it('uses resource events to remove Volumes and Networks without refetching', async () => {
  const volumes = vi.fn(() =>
    HttpResponse.json({ volumes: [{ id: 'volume-one', name: 'volume-one' }], capabilities: {} }),
  );
  const networks = vi.fn(() => HttpResponse.json({ networks: [{ id: 'n1', name: 'network-one' }], capabilities: {} }));
  server.use(http.get('*/api/v1/volumes/p1', volumes), http.get('*/api/v1/networks/p1', networks));
  function Probe() {
    const { volumes } = useVolumesGroup('p1');
    const { networks } = useNetworksGroup('p1');
    return (
      <>
        <output>volumes: {volumes?.volumes.length}</output>
        <output>networks: {networks?.networks.length}</output>
      </>
    );
  }
  const socket = new FakeWebSocket();
  const rendered = renderCitadel(<Probe />, {
    groups: { realtimeTransport: 'WebSocketV1', webSocketFactory: () => socket.asWebSocket() },
  });
  expect(await screen.findByText('volumes: 1')).toBeVisible();
  expect(await screen.findByText('networks: 1')).toBeVisible();
  connect(socket);
  await waitFor(() => expect(socket.send).toHaveBeenCalled());
  deliver(socket, 'VolumesUpdated', { platformId: 'p1', scope: 'one', id: 'volume-one', volumes: [] });
  deliver(socket, 'NetworkEventReceived', null, 'destroy', 'n1');
  expect(await screen.findByText('volumes: 0')).toBeVisible();
  expect(await screen.findByText('networks: 0')).toBeVisible();
  expect(volumes).toHaveBeenCalledTimes(1);
  expect(networks).toHaveBeenCalledTimes(1);
  rendered.unmount();
});

it('recovers the shared Platforms list after capacity rejection while another group stays live', async () => {
  const fake = new FakeRealtimeConnection();
  let oversized = false;
  fake.invoke.mockImplementation(async (method, name) => {
    if (method === 'JoinGroup' && name === 'platforms' && oversized) throw new Error(GROUP_CAPACITY_ERROR);
  });
  server.use(http.get('*/api/v1/platforms', () => HttpResponse.json({ platforms: [], capabilities: {} })));
  function PlatformOwner({ page = false }: { page?: boolean }) {
    const { platformsMessage } = usePlatformsGroup({ useTagFilter: false });
    return (
      <>
        <output>
          {page ? 'page' : 'global'}: {platformsMessage?.[0]?.name}
        </output>
        {page && <Platforms items={[]} actions={{}} isLoading={false} isFiltered />}
      </>
    );
  }
  function App() {
    const [page, setPage] = useState(true);
    const [other, setOther] = useState('waiting');
    useRealtimeGroup({
      groupName: 'alert-events',
      setupEventListeners: useCallback((conn: RealtimeConnection) => conn.on('OtherUpdate', setOther), []),
      removeEventListeners: useCallback((conn: RealtimeConnection) => conn.off('OtherUpdate', setOther), []),
    });
    return (
      <>
        <PlatformOwner />
        {page && <PlatformOwner page />}
        <button onClick={() => setPage(!page)}>toggle page</button>
        <output>{other}</output>
      </>
    );
  }
  const { user, rerender } = renderCitadel(<App />, {
    groups: { connectionFactory: () => fake, startConnection: (connection) => connection.start() },
  });
  const wireCount = (method: string, name: string) =>
    fake.invoke.mock.calls.filter(([m, n]) => m === method && n === name).length;
  await waitFor(() => expect(wireCount('JoinGroup', 'platforms')).toBe(1));
  oversized = true;
  act(() => fake.emit('RealtimeGroupCapacity', 'platforms'));
  expect(await screen.findByRole('button', { name: 'Retry live updates' })).toBeVisible();
  await user.click(screen.getByRole('button', { name: 'toggle page' }));
  expect(wireCount('LeaveGroup', 'platforms')).toBe(0);
  await user.click(screen.getByRole('button', { name: 'toggle page' }));
  await waitFor(() => expect(wireCount('JoinGroup', 'platforms')).toBe(2));
  act(() => fake.emit('OtherUpdate', 'still live'));
  expect(screen.getByText('still live')).toBeVisible();
  oversized = false;
  await user.click(screen.getByRole('button', { name: 'Retry live updates' }));
  await waitFor(() => expect(screen.queryByRole('button', { name: 'Retry live updates' })).toBeNull());
  act(() => fake.emit('PlatformsUpdated', [{ id: 'p1', name: 'Recovered' }]));
  expect(screen.getByText('global: Recovered')).toBeVisible();
  expect(screen.getByText('page: Recovered')).toBeVisible();
  expect(wireCount('JoinGroup', 'platforms')).toBe(3);
  expect(wireCount('JoinGroup', 'alert-events')).toBe(1);
  expect(wireCount('LeaveGroup', 'alert-events')).toBe(0);
  await user.click(screen.getByRole('button', { name: 'toggle page' }));
  expect(wireCount('LeaveGroup', 'platforms')).toBe(0);
  rerender(<></>);
  expect(wireCount('LeaveGroup', 'platforms')).toBe(1);
});
it('delivers Platform statistics through the existing PlatformStatsUpdated handler', async () => {
  const read = vi.fn(() =>
    HttpResponse.json({
      platforms: [{ id: 'p1', type: 'Docker', stats: [{ cpuUsage: 10 }], platformDescriptor: {} }],
      capabilities: {},
    }),
  );
  server.use(http.get('*/api/v1/platforms', read));
  function Probe() {
    const { platformsMessage } = usePlatformsGroup({ useTagFilter: false });
    return <output>cpu: {platformsMessage?.[0].stats?.[0].cpuUsage}</output>;
  }
  const socket = new FakeWebSocket();
  const rendered = renderCitadel(<Probe />, {
    groups: { realtimeTransport: 'WebSocketV1', webSocketFactory: () => socket.asWebSocket() },
  });
  expect(await screen.findByText('cpu: 10')).toBeVisible();
  connect(socket);
  await waitFor(() => expect(socket.send).toHaveBeenCalled());
  deliver(socket, 'PlatformStatsUpdated', {
    platformId: 'p1',
    stat: { cpuUsage: 35 },
    memTotal: 1024,
    networkCount: 1,
    volumeCount: 2,
    imageCount: 3,
  });
  expect(await screen.findByText('cpu: 35')).toBeVisible();
  deliver(socket, 'PlatformStatsUpdated', {
    platformId: 'p1',
    ioOnly: true,
    ioNodeId: 'worker',
    stat: { created: 100, cpuUsage: null, containerIo: [{ containerId: 'task', rxBytes: 10, txBytes: 20 }] },
  });
  expect(screen.getByText('cpu: 35')).toBeVisible();
  expect(read).toHaveBeenCalledTimes(1);
  rendered.unmount();
});

it.each([
  {
    path: 'deployments',
    group: 'deployment',
    event: 'DeploymentInfoUpdated',
    useResource: (id: string) => useDeploymentGroup(id).deployment,
  },
  {
    path: 'gitRepositories',
    group: 'git-repo',
    event: 'GitRepositoryInfoUpdated',
    useResource: (id: string) => useGitRepoGroup(id).gitRepo,
  },
  {
    path: 'buildProjects',
    group: 'build-project',
    event: 'BuildProjectInfoUpdated',
    useResource: (id: string) => BuildFormComponents.EditForm!.useData(id).item,
  },
  {
    path: 'buildAgentPools',
    group: 'build-agent-pool',
    event: 'BuildAgentPoolInfoUpdated',
    useResource: (id: string) => BuildPoolFormComponents.EditForm!.useData(id).item,
  },
  {
    path: 'stacks',
    group: 'stack',
    event: 'StackInfoUpdated',
    useResource: (id: string) => useStackGroup(id).stack,
  },
])(
  'refreshes $group details from the rejoined group without another GET',
  async ({ path, group, event, useResource }) => {
    const read = vi.fn(() => HttpResponse.json({ id: 'r1', name: 'Resource', status: 'Healthy' }));
    const listRead = vi.fn(() => HttpResponse.json({ deployments: [] }));
    server.use(http.get(`*/api/v1/${path}/r1`, read), http.get('*/api/v1/deployments', listRead));
    const fake = new FakeRealtimeConnection();
    let snapshot = { id: 'r1', name: 'Resource', status: 'Healthy' };
    fake.invoke.mockImplementation(async (method, name) => {
      if (method === 'JoinGroup' && name === `${group}:r1`) fake.emit(event, snapshot, 'update');
    });
    function Probe() {
      const resource = useResource('r1');
      useRead('listDeployments');
      return <output>{resource?.name}</output>;
    }
    renderCitadel(<Probe />, {
      groups: {
        connectionFactory: () => fake.asRealtimeConnection(),
        startConnection: (connection) => connection.start(),
      },
    });
    expect(await screen.findByText('Resource')).toBeVisible();
    await waitFor(() => expect(fake.invoke).toHaveBeenCalledWith('JoinGroup', `${group}:r1`));
    snapshot = { ...snapshot, name: 'Updated resource' };
    act(() => {
      fake.reconnecting();
      fake.reconnected();
    });
    expect(await screen.findByText('Updated resource')).toBeVisible();
    await waitFor(() => expect(listRead).toHaveBeenCalledTimes(2));
    expect(read).toHaveBeenCalledOnce();
  },
);

it('refreshes Edge details on connection changes and reconnect without polling on health snapshots', async () => {
  vi.useFakeTimers({ toFake: ['setInterval', 'clearInterval'] });
  let connectionStatus = 'Connected';
  const read = vi.fn(() => HttpResponse.json({ connectionStatus }));
  const pool = {
    id: 'r1',
    name: 'Builder',
    connectionStatus,
    providerSpec: { $type: 'SelfManagedVm', connectionMode: 'EdgeAgent', architecture: 'Amd64' },
  };
  server.use(
    http.get('*/api/v1/buildAgentPools/r1', () => HttpResponse.json(pool)),
    http.get('*/api/v1/buildAgentPools/r1/edge/status', read),
  );
  const fake = new FakeRealtimeConnection();
  function Probe() {
    const { item } = BuildPoolFormComponents.EditForm!.useData('r1');
    const Content = BuildPoolFormComponents.EditForm!.Tabs[0].Content;
    return item && Content ? <Content resource={item as ComponentProps<typeof Content>['resource']} /> : null;
  }
  const rendered = renderCitadel(
    <Routes>
      <Route path="/build-pools/edit/:id" element={<Probe />} />
    </Routes>,
    {
      route: '/build-pools/edit/r1',
      groups: { connectionFactory: () => fake, startConnection: (connection) => connection.start() },
    },
  );
  expect(await screen.findByText('Connected')).toBeVisible();
  await waitFor(() => expect(fake.invoke).toHaveBeenCalledWith('JoinGroup', 'build-agent-pool:r1'));
  await act(() => vi.advanceTimersByTimeAsync(90_000));
  expect(read).toHaveBeenCalledOnce();
  act(() => fake.emit('BuildAgentPoolInfoUpdated', { ...pool, lastValidationStatus: 'Ready' }, 'update'));
  await act(() => vi.advanceTimersByTimeAsync(10_000));
  expect(read).toHaveBeenCalledOnce();
  connectionStatus = 'Offline';
  act(() => fake.emit('BuildAgentPoolInfoUpdated', { ...pool, connectionStatus }, 'update'));
  expect(await screen.findByText('Offline')).toBeVisible();
  expect(read).toHaveBeenCalledTimes(2);
  connectionStatus = 'Connected';
  fake.invoke.mockImplementation(async (method, name) => {
    if (method === 'JoinGroup' && name === 'build-agent-pool:r1') {
      fake.emit('BuildAgentPoolInfoUpdated', { ...pool, connectionStatus }, 'update');
    }
  });
  act(() => {
    fake.reconnecting();
    fake.reconnected();
  });
  expect(await screen.findByText('Connected')).toBeVisible();
  expect(read).toHaveBeenCalledTimes(3);
  rendered.unmount();
});

it.each(['list', 'detail'] as const)(
  'rejects stale Platform metadata with its counters in the %s hook',
  async (view) => {
    const platform = {
      id: 'p1',
      type: 'Docker',
      memTotal: 1024,
      stats: [{ created: 100, cpuUsage: 10, containerIo: [{ containerId: 'host', rxBytes: 100, txBytes: 100 }] }],
      platformDescriptor: {},
    };
    server.use(
      http.get('*/api/v1/platforms', () => HttpResponse.json({ platforms: [platform], capabilities: {} })),
      http.get('*/api/v1/platforms/p1', () => HttpResponse.json(platform)),
    );
    const Output = ({ value }: { value: typeof platform | undefined }) => (
      <output>
        cpu: {value?.stats?.[0]?.cpuUsage}, memory: {value?.memTotal}
      </output>
    );
    function ListProbe() {
      const { platformsMessage } = usePlatformsGroup({ useTagFilter: false });
      return <Output value={platformsMessage?.[0] as typeof platform | undefined} />;
    }
    function DetailProbe() {
      const { platform: current } = usePlatformGroup('p1');
      return <Output value={current as typeof platform | undefined} />;
    }
    const socket = new FakeWebSocket();
    const rendered = renderCitadel(view === 'list' ? <ListProbe /> : <DetailProbe />, {
      groups: { realtimeTransport: 'WebSocketV1', webSocketFactory: () => socket.asWebSocket() },
    });
    expect(await screen.findByText('cpu: 10, memory: 1024')).toBeVisible();
    connect(socket);
    await waitFor(() => expect(socket.send).toHaveBeenCalled());
    deliver(socket, 'PlatformStatsUpdated', {
      platformId: 'p1',
      stat: { created: 90, cpuUsage: 99, containerIo: [{ containerId: 'host', rxBytes: 90, txBytes: 90 }] },
      memTotal: 999,
      networkCount: 1,
      volumeCount: 2,
      imageCount: 3,
    });
    expect(screen.getByText('cpu: 10, memory: 1024')).toBeVisible();
    rendered.unmount();
  },
);

it.each([false, true])(
  'repairs a capacity-limited platform filter (first recovery fails: %s)',
  async (failRecovery) => {
    const fake = new FakeRealtimeConnection();
    const scope = collectionGroup('platforms', ['prod']);
    let name = 'Initial';
    let failed = false;
    const read = vi.fn(() => {
      if (failRecovery && !failed && read.mock.calls.length === 3) {
        failed = true;
        return new HttpResponse(null, { status: 500 });
      }
      return HttpResponse.json({ platforms: [{ id: 'p', name }], capabilities: {} });
    });
    server.use(
      http.get('*/api/v1/platforms', read),
      http.get('*/api/v1/tags/_filter', () => HttpResponse.json({ prod: 'prod' })),
    );
    function Page() {
      const { platformsMessage, groupName, refetch, error } = usePlatformsGroup();
      return (
        <>
          {error && <span>Read failed</span>}
          <output>{platformsMessage?.[0]?.name}</output>
          <button onClick={() => void refetch()}>Read list</button>
          <Platforms items={[]} actions={{}} isLoading={false} isFiltered groupName={groupName} />
        </>
      );
    }
    const { user } = renderCitadel(<Page />, {
      route: '/?tags=prod',
      groups: { connectionFactory: () => fake, startConnection: (c) => c.start() },
    });
    expect(await screen.findByText('Initial')).toBeVisible();
    await waitFor(() => expect(fake.invoke).toHaveBeenCalledWith('JoinGroup', scope));
    act(() => {
      fake.emit(collectionEventKey(scope, 'CollectionSnapshotStarted'));
      fake.emit(collectionEventKey(scope, 'PlatformUpdated'), { id: 'p', name: 'Live' });
    });
    name = 'Fallback';
    await user.click(screen.getByText('Read list'));
    await waitFor(() => expect(read).toHaveBeenCalledTimes(2));
    expect(screen.getByText('Live')).toBeVisible();
    act(() => fake.emit('RealtimeGroupCapacity', scope));
    if (failRecovery) {
      expect(await screen.findByText('Read failed')).toBeVisible();
      expect(screen.getByText('Live')).toBeVisible();
      await user.click(screen.getByText('Read list'));
    }
    expect(await screen.findByText('Fallback')).toBeVisible();
    expect(read).toHaveBeenCalledTimes(failRecovery ? 4 : 3);
    await user.click(screen.getByRole('button', { name: 'Retry live updates' }));
    expect(fake.invoke.mock.calls.filter(([method, key]) => method === 'JoinGroup' && key === scope)).toHaveLength(2);
  },
);

it('reconciles each cached mutation result according to its own live collection scope', async () => {
  const fake = new FakeRealtimeConnection();
  const scope = collectionGroup('scripts', ['prod']);
  const listeners = () => {};
  function Page() {
    const { isConnected } = useRealtimeGroup({ groupName: scope, setupEventListeners: listeners });
    const reconcile = useQueryReconciliation();
    return (
      <button disabled={!isConnected} onClick={() => void reconcile('scripts', 'listScripts')}>
        Reconcile
      </button>
    );
  }
  const { user, queryClient } = renderCitadel(<Page />, {
    groups: { connectionFactory: () => fake, startConnection: (c) => c.start() },
  });
  const prod = ['listScripts', { query: { tags: ['prod'] } }];
  const stage = ['listScripts', { query: { tags: ['stage'] } }];
  const all = ['listScripts', {}];
  for (const key of [prod, stage, all]) queryClient.setQueryData(key, []);
  const button = screen.getByRole('button', { name: 'Reconcile' });
  await waitFor(() => expect(button).toBeEnabled());
  await user.click(button);
  await waitFor(() => expect(queryClient.getQueryState(all)?.isInvalidated).toBe(true));
  expect(queryClient.getQueryState(stage)?.isInvalidated).toBe(true);
  expect(queryClient.getQueryState(prod)?.isInvalidated).toBe(false);
});

it('refreshes an open tag usage count from the viewer’s authorized list on a committed change', async () => {
  const fake = new FakeRealtimeConnection();
  let count = 2;
  server.use(http.get('*/api/v1/tags', () => HttpResponse.json({ tags: [{ id: 'tag', usageCount: count }] })));
  function Catalogue() {
    const { data } = useRead('listTags');
    return <output>{data?.data.tags[0]?.usageCount} resources</output>;
  }
  renderCitadel(<Catalogue />, { groups: { connectionFactory: () => fake, startConnection: (c) => c.start() } });
  expect(await screen.findByText('2 resources')).toBeVisible();
  await waitFor(() => expect(fake.listenerCount('TagCatalogChanged')).toBe(1));
  count = 1;
  act(() => fake.emit('TagCatalogChanged'));
  expect(await screen.findByText('1 resources')).toBeVisible();
});
