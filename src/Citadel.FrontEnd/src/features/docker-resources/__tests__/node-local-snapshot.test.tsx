import { act, screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { ImageView, DockerNetworkResultView, DockerVolumeResultView } from '@/api/generated/api.types';
import { SwarmNodeLocalResourcesUpdate } from '@/features/platforms/hooks/useDockerDaemonGroup';
import { renderCitadel } from '@/test/render-citadel';
import { FakeHubConnection } from '@/test/fakes/signalr';
import { server } from '@/test/server';
import { useImagesGroup } from '../images/hooks/useImagesGroup';
import { useNetworksGroup } from '../networks/hooks/useNetworksGroup';
import { useVolumesGroup } from '../volumes/hooks/useVolumesGroup';

const platformId = '00000000-0000-0000-0000-000000000200';

describe('node-local Docker resource snapshots', () => {
  it('keeps a SignalR snapshot received before the initial HTTP responses', async () => {
    const fake = new FakeHubConnection();
    let releaseResponses!: () => void;
    const responseGate = new Promise<void>((resolve) => {
      releaseResponses = resolve;
    });
    server.use(
      http.get(`http://localhost/api/v1/images/${platformId}`, async () => {
        await responseGate;
        return HttpResponse.json({ images: [image('manager')], capabilities: {} });
      }),
      http.get(`http://localhost/api/v1/volumes/${platformId}`, async () => {
        await responseGate;
        return HttpResponse.json({ volumes: [volume('manager')], capabilities: {} });
      }),
      http.get(`http://localhost/api/v1/networks/${platformId}`, async () => {
        await responseGate;
        return HttpResponse.json({ networks: [network('manager')], capabilities: {} });
      }),
    );

    renderCitadel(<ResourceProbe />, {
      signalR: {
        connectionFactory: () => fake.asHubConnection(),
        startConnection: (connection) => connection.start(),
      },
    });

    await waitFor(() => expect(fake.listenerCount('SwarmNodeLocalResourcesUpdated')).toBe(3));
    act(() =>
      fake.emit('SwarmNodeLocalResourcesUpdated', {
        platformId,
        images: [image('worker')],
        volumes: [volume('worker')],
        networks: [network('worker')],
      } satisfies SwarmNodeLocalResourcesUpdate),
    );
    await act(async () => {
      releaseResponses();
      await responseGate;
    });

    expect(await screen.findByTestId('images')).toHaveTextContent('worker');
    expect(await screen.findByTestId('volumes')).toHaveTextContent('worker');
    expect(await screen.findByTestId('networks')).toHaveTextContent('worker');
    expect(screen.queryByText('manager')).not.toBeInTheDocument();
  });
});

function ResourceProbe() {
  const { imagesInfo } = useImagesGroup(platformId);
  const { volumes } = useVolumesGroup(platformId);
  const { networks } = useNetworksGroup(platformId);

  return (
    <>
      <div data-testid="images">{imagesInfo?.images.map((item) => item.dockerNodeId).join(',')}</div>
      <div data-testid="volumes">{volumes?.volumes.map((item) => item.dockerNodeId).join(',')}</div>
      <div data-testid="networks">{networks?.networks.map((item) => item.dockerNodeId).join(',')}</div>
    </>
  );
}

function image(node: string) {
  return { dockerNodeId: node } as ImageView;
}

function volume(node: string) {
  return { dockerNodeId: node } as DockerVolumeResultView;
}

function network(node: string) {
  return { dockerNodeId: node } as DockerNetworkResultView;
}
