import { render, screen } from '@testing-library/react';
import type { DockerNetworkDetailsView } from '@/api/generated/api.types';
import { NetworkInfoComponents } from '.';

describe('network details', () => {
  it('shows driver options even when the network has no attached containers', () => {
    const Content = NetworkInfoComponents.Tabs[0].Content;
    const resource: DockerNetworkDetailsView = {
      id: 'network-id',
      name: 'empty-network',
      driver: 'bridge',
      scope: 'local',
      attachable: false,
      internal: false,
      ingress: false,
      containers: {},
      options: { 'com.docker.network.driver.mtu': '1450' },
      labels: {},
      ipam: null,
      created: '2026-09-27T00:00:00Z',
      enableIPv4: true,
      enableIPv6: false,
      configOnly: false,
      configFrom: null,
      peers: [],
      isSystem: false,
      dockerNodeId: null,
    };

    render(<Content resource={resource} />);

    expect(screen.getByRole('heading', { name: 'Driver options' })).toBeVisible();
    expect(screen.getByText('com.docker.network.driver.mtu')).toBeVisible();
    expect(screen.getByText('1450')).toBeVisible();
    expect(screen.getByText('No containers are connected to this network.')).toBeVisible();
    expect(screen.queryByRole('heading', { name: 'IP address management' })).not.toBeInTheDocument();
  });
});
