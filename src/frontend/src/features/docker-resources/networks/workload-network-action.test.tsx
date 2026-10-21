import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter } from 'react-router';
import { NetworkRelationship, NetworkWorkloadKind, type NetworkView } from '@/api/generated/api.types';
import { WorkloadNetworkAction } from './workload-network-action';
import type { NetworkGraph } from './map/model';

const state = vi.hoisted(() => ({
  read: vi.fn(),
  networks: [] as NetworkView[],
  error: undefined as unknown,
}));
vi.mock('./hooks/useNetworksGroup', () => ({
  useNetworksGroup: (...args: unknown[]) => {
    state.read(...args);
    return { networks: { networks: state.networks }, error: state.error, isLoading: false, refetch: vi.fn() };
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

// Module transformation is separate from the lazy-mount behavior under test.
beforeAll(async () => {
  await import('./map/workload-connections');
});

beforeEach(() => {
  state.read.mockClear();
  state.error = undefined;
  state.networks = [];
});

it.each(['Deployment', 'Stack', 'SwarmService'] as const)(
  'opens a scoped %s map on demand and restores focus',
  async (kind) => {
    state.networks = [
      {
        id: 'network',
        name: 'backend',
        driver: 'bridge',
        scope: 'local',
        containers: {},
        usage: {
          isPartial: true,
          truncated: false,
          entries: [
            {
              relationship: NetworkRelationship.Attachment,
              containerId: 'c',
              dockerContainerId: 'docker-c',
              containerName: 'app',
              workloadKind: NetworkWorkloadKind[kind],
              workloadId: 'owner',
              workloadName: 'Application',
              isStale: false,
              workloadProcessing: false,
            },
            {
              relationship: NetworkRelationship.Attachment,
              containerId: 'other',
              dockerContainerId: 'docker-other',
              containerName: 'unrelated',
              workloadKind: NetworkWorkloadKind[kind],
              workloadId: 'other-owner',
              isStale: false,
              workloadProcessing: false,
            },
          ],
        },
      } as NetworkView,
    ];
    render(
      <MemoryRouter>
        <WorkloadNetworkAction resource={{ id: 'owner', name: 'Application', platformId: 'p' }} kind={kind} />
      </MemoryRouter>,
    );
    expect(state.read).not.toHaveBeenCalled();
    const user = userEvent.setup();
    const trigger = screen.getByRole('button', { name: 'View connections' });
    await user.click(trigger);
    expect(await screen.findByTestId('graph')).toHaveTextContent('backend');
    expect(state.read).toHaveBeenCalledWith('p', true);
    expect(screen.getByRole('link', { name: 'Open network details' })).toHaveAttribute(
      'href',
      '/platforms/p/networks/network/',
    );
    expect(screen.getByRole('dialog')).not.toHaveTextContent('unrelated');
    await user.click(screen.getByRole('button', { name: 'Map options' }));
    expect(screen.queryByRole('menuitem', { name: /Explore workload/ })).not.toBeInTheDocument();
    await user.keyboard('{Escape}');
    await user.click(screen.getByRole('button', { name: 'Maximize network sheet' }));
    await user.keyboard('{Escape}');
    expect(screen.getByRole('dialog')).toBeVisible();
    await user.keyboard('{Escape}');
    await waitFor(() => expect(trigger).toHaveFocus());
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  },
);

it('shows an empty state and hides cached data after access is denied', async () => {
  const element = (
    <MemoryRouter>
      <WorkloadNetworkAction resource={{ id: 'owner', name: 'Application', platformId: 'p' }} kind="Stack" />
    </MemoryRouter>
  );
  const { rerender } = render(element);
  await userEvent.click(screen.getByRole('button', { name: 'View connections' }));
  expect(await screen.findByText('No network connections reported for this workload.')).toBeVisible();
  state.error = { status: 403 };
  rerender(
    <MemoryRouter>
      <WorkloadNetworkAction resource={{ id: 'owner', name: 'Application', platformId: 'p' }} kind="Stack" />
    </MemoryRouter>,
  );
  expect(
    within(screen.getByRole('dialog')).queryByText('No network connections reported for this workload.'),
  ).not.toBeInTheDocument();
  expect(screen.queryByTestId('graph')).not.toBeInTheDocument();
});

it('does not offer connections without a platform', () => {
  render(<WorkloadNetworkAction resource={{ id: 'owner', name: 'Application', platformId: null }} kind="Stack" />);
  expect(screen.queryByRole('button', { name: 'View connections' })).not.toBeInTheDocument();
  expect(state.read).not.toHaveBeenCalled();
});

it.each(['Deployment', 'Stack', 'SwarmService'] as const)(
  'shows a stopped %s’s configured networks by default',
  async (kind) => {
    state.networks = [
      {
        id: 'configured-network',
        name: 'application_default',
        driver: kind === 'Deployment' ? 'bridge' : 'overlay',
        scope: kind === 'Deployment' ? 'local' : 'swarm',
        inUse: false,
        containers: {},
        usage: {
          isPartial: true,
          truncated: false,
          entries: [
            {
              relationship: NetworkRelationship.SavedReference,
              workloadKind: NetworkWorkloadKind[kind],
              workloadId: 'owner',
              workloadName: 'Application',
              workloadStatus: 'Stopped',
              workloadProcessing: false,
              isStale: false,
              reference: 'application_default',
            },
          ],
        },
      } as NetworkView,
    ];
    render(
      <MemoryRouter>
        <WorkloadNetworkAction resource={{ id: 'owner', name: 'Application', platformId: 'p' }} kind={kind} />
      </MemoryRouter>,
    );
    await userEvent.click(screen.getByRole('button', { name: 'View connections' }));
    expect(await screen.findByTestId('graph')).toHaveTextContent('application_default');
    expect(screen.getByText('Configured use')).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'Map options' }));
    const configured = screen.getByRole('menuitemcheckbox', { name: 'Show configured networks' });
    expect(configured).toHaveAttribute('aria-checked', 'true');
    await userEvent.click(configured);
    expect(await screen.findByText('No network connections reported for this workload.')).toBeVisible();
  },
);
