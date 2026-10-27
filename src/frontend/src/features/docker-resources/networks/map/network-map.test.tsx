vi.mock('@/lib/use-profile-date-time', () => ({ useProfileDateTimeFormatter: () => String }));

import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter } from 'react-router';
import type { NetworkView, NetworkContainer, ContainerNetworkingView } from '@/api/generated/api.types';
import { NetworkRelationship } from '@/api/generated/api.types';
import NetworkMap, { FocusedMap } from './network-map';
import type { NetworkGraph } from './model';

const state = vi.hoisted(() => ({
  calls: vi.fn(),
  catalog: { data: { data: { containers: [] as NetworkContainer[], truncated: false } }, error: undefined as unknown },
  networking: { data: undefined as { data: ContainerNetworkingView } | undefined, error: undefined as unknown },
}));
vi.mock('../hooks/useNetworkWorkloads', () => ({
  useNetworkWorkloads: (...args: unknown[]) => {
    state.calls(...args);
    return { catalog: state.catalog, networking: state.networking };
  },
}));
vi.mock('./canvas', () => ({
  default: ({ graph, onSelect }: { graph: NetworkGraph; onSelect: (id: string) => void }) => (
    <div data-testid="canvas">
      {graph.nodes.map((node) => (
        <button key={node.id} onClick={() => onSelect(node.id)}>
          {node.label}
        </button>
      ))}
    </div>
  ),
}));
const network = {
  id: 'proxy',
  name: 'proxy',
  dockerNodeId: null,
  scope: 'local',
  driver: 'bridge',
  containers: {},
  ipam: null,
  capabilities: { canInspect: true },
  usage: {
    isPartial: false,
    truncated: false,
    entries: [
      {
        relationship: 'SavedReference',
        workloadKind: 'Stack',
        workloadId: 's',
        workloadName: 'Saved stack',
        reference: 'proxy',
      },
    ],
  },
} as NetworkView;
const props = { network, networks: [network], platformId: 'p', loading: false, maximized: false };

it('keeps the info-page map scoped even when the URL includes a workload exploration', async () => {
  const attachedNetwork: NetworkView = {
    ...network,
    usage: {
      isPartial: false,
      truncated: false,
      entries: [
        {
          relationship: NetworkRelationship.Attachment,
          containerId: 'attached',
          dockerContainerId: 'docker-attached',
          containerName: 'Attached application',
          isStale: false,
          workloadProcessing: false,
        },
      ],
    },
  };
  render(
    <MemoryRouter initialEntries={['/?networkPlatform=p&networkContainer=unrelated']}>
      <NetworkMap {...props} network={attachedNetwork} context="details" />
    </MemoryRouter>,
  );
  expect(screen.getByTestId('canvas')).toHaveTextContent('proxy');
  expect(screen.queryByRole('button', { name: /Back to/ })).not.toBeInTheDocument();
  expect(screen.queryByRole('button', { name: 'Explore connections' })).not.toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Connection details' })).toBeVisible();
  await userEvent.click(screen.getByRole('button', { name: 'Map options' }));
  expect(screen.queryByRole('menuitem', { name: /Explore workload/ })).not.toBeInTheDocument();
  expect(state.calls).not.toHaveBeenCalled();
});

beforeEach(() => {
  state.calls.mockClear();
  state.catalog.data.data.containers = [];
  state.catalog.error = undefined;
  state.networking.data = undefined;
  state.networking.error = undefined;
});

it('reuses the sheet snapshot without issuing a catalog or networking request', async () => {
  render(
    <MemoryRouter>
      <NetworkMap {...props} />
    </MemoryRouter>,
  );
  expect(screen.getByTestId('canvas')).toHaveTextContent('proxy');
  await userEvent.click(screen.getByRole('button', { name: 'Map options' }));
  await userEvent.click(screen.getByRole('menuitemcheckbox', { name: 'Show configured workloads' }));
  expect(screen.getByTestId('canvas')).toHaveTextContent('Saved stack');
  expect(state.calls).not.toHaveBeenCalled();
});

it('keeps selected details and reference settings when maximizing', async () => {
  const view = render(
    <MemoryRouter>
      <NetworkMap {...props} />
    </MemoryRouter>,
  );
  await userEvent.click(screen.getByRole('button', { name: 'Map options' }));
  await userEvent.click(screen.getByRole('menuitemcheckbox', { name: 'Show configured workloads' }));
  await userEvent.click(within(screen.getByTestId('canvas')).getByRole('button', { name: 'Saved stack' }));
  view.rerender(
    <MemoryRouter>
      <NetworkMap {...props} maximized />
    </MemoryRouter>,
  );
  expect(
    within(screen.getByLabelText('Map selection details')).getByRole('heading', { name: 'Saved stack' }),
  ).toBeVisible();
  expect(screen.getByTestId('canvas')).toHaveTextContent('Saved stack');
});

it('opens an authorized container outside the truncated catalog and removes details on permission loss', () => {
  state.catalog.data.data.containers = [
    {
      isStale: false,
      containerState: 'Running',
      containerId: 'uuid-app',
      dockerContainerId: 'docker-app',
      containerName: 'app',
      dockerNodeId: null,
      workloadProcessing: false,
    },
  ];
  state.networking.data = {
    data: {
      container: state.catalog.data.data.containers[0],
      mode: 'networks',
      endpoints: [
        { attached: true, networkId: 'proxy', name: 'proxy', aliases: ['application'], addresses: ['172.20.0.2'] },
      ],
      publishedPorts: [],
      observedAt: 1,
      truncated: false,
    },
  };
  state.catalog.data.data.containers = [];
  state.catalog.data.data.truncated = true;
  const view = render(
    <MemoryRouter initialEntries={['/?networkPlatform=p&networkContainer=uuid-app']}>
      <NetworkMap {...props} />
    </MemoryRouter>,
  );
  expect(screen.getByTestId('canvas')).toHaveTextContent('app');
  expect(screen.getByText('Aliases: application')).toBeVisible();
  expect(state.calls).toHaveBeenCalledWith('p', 'uuid-app', false);
  state.networking.error = { status: 403 };
  view.rerender(
    <MemoryRouter>
      <NetworkMap {...props} />
    </MemoryRouter>,
  );
  expect(screen.queryByTestId('canvas')).not.toBeInTheDocument();
  expect(screen.queryByText('Aliases: application')).not.toBeInTheDocument();
  expect(screen.getByRole('alert')).toHaveTextContent('You do not have permission');
});

it('ignores workload focus from another platform', () => {
  render(
    <MemoryRouter initialEntries={['/?networkPlatform=other&networkContainer=uuid-app']}>
      <NetworkMap {...props} />
    </MemoryRouter>,
  );
  expect(state.calls).not.toHaveBeenCalled();
  expect(screen.getByTestId('canvas')).toHaveTextContent('proxy');
});

it('opens workload search on demand, including containers without an ordinary network attachment', async () => {
  state.catalog.data.data.containers = [
    {
      isStale: false,
      containerId: 'host-app',
      workloadProcessing: false,
      dockerContainerId: 'host-docker',
      containerName: 'host application',
      dockerNodeId: null,
    },
  ];
  state.networking.data = {
    data: {
      container: state.catalog.data.data.containers[0],
      mode: 'host',
      endpoints: [],
      publishedPorts: [],
      observedAt: 1,
      truncated: false,
    },
  };
  render(
    <MemoryRouter initialEntries={['/?networkPlatform=p&network=proxy']}>
      <NetworkMap {...props} />
    </MemoryRouter>,
  );
  expect(state.calls).not.toHaveBeenCalled();
  await userEvent.click(screen.getByRole('button', { name: 'Map options' }));
  await userEvent.click(screen.getByRole('menuitem', { name: 'Explore workload…' }));
  expect(screen.getByRole('dialog', { name: 'Explore workload' })).toBeVisible();
  await userEvent.click(screen.getByRole('option', { name: 'host application' }));
  expect(screen.queryByRole('dialog', { name: 'Explore workload' })).not.toBeInTheDocument();
  expect(screen.getByTestId('canvas')).toHaveTextContent('host application');
  expect(state.calls).toHaveBeenCalledWith('p', 'host-app', false);
  await userEvent.click(screen.getByRole('button', { name: 'Back to proxy' }));
  expect(screen.getByTestId('canvas')).toHaveTextContent('proxy');
});

it('links the workload sheet footer to the selected network and preserves node scope', async () => {
  const container: NetworkContainer = {
    containerId: 'app',
    dockerContainerId: 'docker-app',
    containerName: 'app',
    dockerNodeId: 'worker',
    isStale: false,
    workloadProcessing: false,
  };
  const networking: ContainerNetworkingView = {
    container,
    mode: 'networks',
    publishedPorts: [],
    observedAt: 1,
    truncated: false,
    endpoints: ['proxy', 'backend'].map((id) => ({
      name: id,
      networkId: id,
      attached: true,
      addresses: [],
      aliases: [],
    })),
  };
  render(
    <MemoryRouter>
      <FocusedMap
        platformId="p"
        loading={false}
        maximized={false}
        showNetworkDetails
        workload={{ container, networking }}
        networks={['proxy', 'backend'].map((id) => ({ ...network, id, name: id, dockerNodeId: 'worker' }))}
      />
    </MemoryRouter>,
  );
  expect(screen.getByRole('link', { name: 'Open network details' })).toHaveAttribute(
    'href',
    '/platforms/p/networks/proxy/?dockerNodeId=worker',
  );
  await userEvent.click(within(screen.getByTestId('canvas')).getByRole('button', { name: 'backend' }));
  expect(screen.getByRole('link', { name: 'Open network details' })).toHaveAttribute(
    'href',
    '/platforms/p/networks/backend/?dockerNodeId=worker',
  );
});
