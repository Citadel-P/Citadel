import { act, screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { FakeWebSocket } from '@/test/fakes/websocket';
import { useVolumesGroup } from '@/features/docker-resources/volumes/hooks/useVolumesGroup';
import { useNetworksGroup } from '@/features/docker-resources/networks/hooks/useNetworksGroup';
import { useDeploymentGroup } from '@/features/deployments/form/hooks/useDeploymentGroup';
import { usePlatformsGroup } from '@/features/platforms/hooks/usePlatformsGroup';

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

it('uses the unchanged Deployment hook to apply a named event without refetching', async () => {
  const read = vi.fn(() => HttpResponse.json({ id: 'd1', status: 'Applying' }));
  server.use(http.get('*/api/v1/deployments/d1', read));
  function Probe() {
    const { deployment } = useDeploymentGroup('d1');
    return <output>{deployment?.status}</output>;
  }
  const socket = new FakeWebSocket();
  const rendered = renderCitadel(<Probe />, {
    signalR: { realtimeTransport: 'WebSocketV1', webSocketFactory: () => socket.asWebSocket() },
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

it('uses existing daemon handlers to remove Volumes and Networks without refetching', async () => {
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
    signalR: { realtimeTransport: 'WebSocketV1', webSocketFactory: () => socket.asWebSocket() },
  });
  expect(await screen.findByText('volumes: 1')).toBeVisible();
  expect(await screen.findByText('networks: 1')).toBeVisible();
  connect(socket);
  await waitFor(() => expect(socket.send).toHaveBeenCalled());
  deliver(socket, 'VolumeEventReceived', null, 'destroy', 'volume-one');
  deliver(socket, 'NetworkEventReceived', null, 'destroy', 'n1');
  expect(await screen.findByText('volumes: 0')).toBeVisible();
  expect(await screen.findByText('networks: 0')).toBeVisible();
  expect(volumes).toHaveBeenCalledTimes(1);
  expect(networks).toHaveBeenCalledTimes(1);
  rendered.unmount();
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
    signalR: { realtimeTransport: 'WebSocketV1', webSocketFactory: () => socket.asWebSocket() },
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
  expect(read).toHaveBeenCalledTimes(1);
  rendered.unmount();
});
