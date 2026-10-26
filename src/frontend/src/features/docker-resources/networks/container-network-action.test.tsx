vi.mock('@/lib/use-profile-date-time', () => ({ useProfileDateTimeFormatter: () => String }));

import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter } from 'react-router';
import type { ContainerNetworkingView, NetworkView } from '@/api/generated/api.types';
import { ContainerNetworkAction } from './container-network-action';
import type { NetworkGraph } from './map/model';

const state = vi.hoisted(() => ({
  read: vi.fn(),
  inventory: vi.fn(),
  error: undefined as unknown,
  details: undefined as ContainerNetworkingView | undefined,
}));
vi.mock('./hooks/useNetworkWorkloads', () => ({
  useNetworkWorkloads: (...args: unknown[]) => {
    state.read(...args);
    return {
      networking: { data: state.details ? { data: state.details } : undefined, error: state.error, isLoading: false },
    };
  },
}));
vi.mock('./hooks/useNetworksGroup', () => ({
  useNetworksGroup: (...args: unknown[]) => {
    state.inventory(...args);
    return {
      networks: {
        networks: [
          { id: 'network-id', name: 'backend', scope: 'local', dockerNodeId: 'worker', containers: {} } as NetworkView,
        ],
      },
      isLoading: false,
    };
  },
}));
vi.mock('./map/canvas', () => ({
  default: ({ graph }: { graph: NetworkGraph }) => (
    <div data-testid="graph">
      {graph.nodes.map((node) => (
        <span key={node.id}>{node.label}</span>
      ))}
    </div>
  ),
}));
const props = {
  platformId: 'platform',
  networks: { backend: 'network-id' },
  container: {
    containerId: 'persisted-uuid',
    dockerContainerId: 'docker-id',
    dockerNodeId: 'worker',
    containerName: 'app',
    workloadProcessing: false,
    isStale: false,
    containerState: 'Running',
  },
};
// Load test dependencies before measuring lazy mounting, not during DOM wait timeouts.
beforeAll(async () => {
  await import('./map/container-connections');
});

beforeEach(() => {
  state.read.mockClear();
  state.inventory.mockClear();
  state.error = undefined;
  state.details = {
    container: props.container,
    mode: 'networks',
    endpoints: [
      { networkId: 'network-id', name: 'backend', addresses: ['10.0.0.2/24'], aliases: ['app'], attached: true },
    ],
    publishedPorts: [],
    observedAt: 1,
    truncated: false,
  };
});

it('opens a container-scoped map lazily using its UUID and restores focus', async () => {
  render(
    <MemoryRouter>
      <ContainerNetworkAction {...props} />
    </MemoryRouter>,
  );
  expect(state.read).not.toHaveBeenCalled();
  expect(state.inventory).not.toHaveBeenCalled();
  const user = userEvent.setup();
  const trigger = screen.getByRole('button', { name: 'View connections: backend' });
  await user.click(trigger);
  expect(await screen.findByTestId('graph')).toHaveTextContent('backend');
  expect(state.read).toHaveBeenCalledWith('platform', 'persisted-uuid', false);
  expect(screen.getByRole('link', { name: 'Open network details' })).toHaveAttribute(
    'href',
    '/platforms/platform/networks/network-id/?dockerNodeId=worker',
  );
  expect(screen.getByText('Aliases: app')).toBeVisible();
  await user.click(screen.getByRole('button', { name: 'Map options' }));
  expect(screen.queryByRole('menuitem', { name: /Explore workload/ })).not.toBeInTheDocument();
  await user.keyboard('{Escape}');
  await user.click(screen.getByRole('button', { name: 'Maximize network sheet' }));
  await user.keyboard('{Escape}');
  expect(screen.getByRole('dialog')).toBeVisible();
  await user.keyboard('{Escape}');
  await waitFor(() => expect(trigger).toHaveFocus());
});

it.each([
  ['host', 'Uses host network'],
  ['none', 'No network'],
  ['shared', 'Shares another container’s network namespace. Target details are unavailable.'],
])('shows %s mode without a graph or network inventory request', async (mode, label) => {
  state.details = { ...state.details!, mode, endpoints: [] };
  render(
    <MemoryRouter>
      <ContainerNetworkAction {...props} networks={{}} />
    </MemoryRouter>,
  );
  await userEvent.click(screen.getByRole('button', { name: /View connections/ }));
  expect(await screen.findByText(label)).toBeVisible();
  expect(screen.queryByTestId('graph')).not.toBeInTheDocument();
  expect(state.inventory).not.toHaveBeenCalled();
});

it('hides cached networking after permission is denied', async () => {
  state.error = { status: 403 };
  render(
    <MemoryRouter>
      <ContainerNetworkAction {...props} />
    </MemoryRouter>,
  );
  await userEvent.click(screen.getByRole('button', { name: /View connections/ }));
  expect(await screen.findByText('You do not have permission to access this resource.')).toBeVisible();
  expect(screen.queryByTestId('graph')).not.toBeInTheDocument();
  expect(screen.queryByText('Aliases: app')).not.toBeInTheDocument();
  expect(state.inventory).not.toHaveBeenCalled();
});

it('shows a stopped container’s configured network and legend without an active address', async () => {
  state.details = {
    ...state.details!,
    container: { ...state.details!.container, containerState: 'Exited' },
    endpoints: state.details!.endpoints.map((endpoint) => ({ ...endpoint, attached: false })),
  };
  render(
    <MemoryRouter>
      <ContainerNetworkAction {...props} />
    </MemoryRouter>,
  );
  await userEvent.click(screen.getByRole('button', { name: 'View connections: backend' }));
  expect(await screen.findByTestId('graph')).toHaveTextContent('backend');
  expect(screen.getByText('Configured network · Not attached')).toBeVisible();
  expect(screen.getByText('Configured use')).toBeVisible();
  expect(screen.getByText('No active address')).toBeVisible();
  expect(screen.queryByText('10.0.0.2/24')).not.toBeInTheDocument();
  expect(screen.getByRole('link', { name: 'Open network details' })).toHaveAttribute(
    'href',
    '/platforms/platform/networks/network-id/?dockerNodeId=worker',
  );
});
