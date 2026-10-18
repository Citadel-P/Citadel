import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Provider, createStore } from 'jotai';
import { MemoryRouter, Route, Routes, useLocation } from 'react-router';
import { NetworkView, PlatformType, NetworkRelationship, NetworkWorkloadKind } from '@/api/generated/api.types';
import { NetworksTable } from './table';
import { NetworkColumnsButton, networkColumnsAtom } from './table-columns';
import { canDeleteNetwork, filterNetworks, networkUsageLabel, networkMapParams } from './network-presentation';
import { ResourceHeader } from '@/pages/resource-header';
import { NetworkComponents } from '.';

const { selected, platform, inspection } = vi.hoisted(() => ({
  selected: vi.fn(),
  inspection: {
    data: undefined as { data: NetworkView } | undefined,
    error: undefined as unknown,
    isLoading: false,
    refetch: vi.fn(),
  },
  platform: { id: 'platform-a', type: 'Docker' },
}));
vi.mock('./map/canvas', () => ({ default: () => <div data-testid="canvas">Connections diagram</div> }));
vi.mock('./hooks/useNetworkInspection', () => ({ useNetworkInspection: () => inspection }));
vi.mock('@/lib/context/app-context', () => ({ useAppContext: () => ({ currentPlatform: platform }) }));
vi.mock('@/lib/atoms', () => ({ useSelectedResources: () => [[], selected] }));
vi.mock('@/components/custom/dropdown-with-dialog', () => ({ RowActionMenu: () => null }));
vi.mock('@/lib/use-profile-date-time', () => ({ useProfileDateTimeFormatter: () => String }));

const network = (overrides: Partial<NetworkView> = {}): NetworkView => ({
  id: 'network-id',
  name: 'application',
  driver: 'bridge',
  scope: 'local',
  created: '',
  enableIPv4: true,
  enableIPv6: true,
  internal: false,
  attachable: true,
  ingress: false,
  configOnly: false,
  inUse: false,
  configFrom: null,
  options: {},
  labels: {},
  containers: {},
  peers: [],
  isSystem: false,
  dockerNodeId: null,
  nodeHostname: null,
  isStale: false,
  staleReason: null,
  capabilities: null,
  ipam: {
    driver: 'default',
    options: {},
    config: [
      { subnet: '172.21.0.0/24', gateway: null, ipRange: null },
      { subnet: 'fd00:21::/64', gateway: null, ipRange: null },
      { subnet: '172.22.0.0/24', gateway: null, ipRange: null },
    ],
  },
  ...overrides,
});

function CurrentQuery() {
  const location = useLocation();
  return <output aria-label="Current query">{location.search}</output>;
}

function renderTable(items = [network()], initialEntry = '/platforms/platform-a/networks') {
  const store = createStore();
  store.set(networkColumnsAtom, []);
  const result = render(
    <Provider store={store}>
      <MemoryRouter initialEntries={[initialEntry]}>
        <Routes>
          <Route
            path="/platforms/:platformId/networks"
            element={
              <>
                <CurrentQuery />
                <NetworkColumnsButton />
                <NetworksTable items={items} actions={{}} isLoading={false} />
              </>
            }
          />
        </Routes>
      </MemoryRouter>
    </Provider>,
  );
  return { ...result, store };
}

beforeEach(() => {
  selected.mockClear();
  localStorage.removeItem('networks-view');
  inspection.data = undefined;
  inspection.error = undefined;
  platform.type = PlatformType.Docker;
});

describe('network list', () => {
  it('keeps search and creation available alongside Columns', () => {
    render(
      <Provider>
        <MemoryRouter>
          <ResourceHeader {...NetworkComponents.header} type="Network" onAdd={vi.fn()} onSearch={vi.fn()} />
        </MemoryRouter>
      </Provider>,
    );
    expect(screen.getByPlaceholderText('Search networks…')).toBeVisible();
    expect(screen.getByRole('button', { name: 'Add Network' })).toBeVisible();
    expect(screen.getByRole('button', { name: 'Columns' })).toBeVisible();
  });

  it('shows every subnet without a gateway and hides optional columns initially', () => {
    renderTable();
    expect(screen.getByText('172.21.0.0/24')).toBeVisible();
    expect(screen.getByText('fd00:21::/64')).toBeVisible();
    expect(screen.getByText('172.22.0.0/24')).toBeVisible();
    expect(screen.getByRole('columnheader', { name: 'Driver / scope' })).toBeVisible();
    expect(screen.getByRole('columnheader', { name: 'Used by' })).toBeVisible();
    expect(screen.queryByRole('columnheader', { name: 'Gateway' })).not.toBeInTheDocument();
    expect(screen.queryByRole('columnheader', { name: 'Network ID' })).not.toBeInTheDocument();
    expect(screen.queryByRole('columnheader', { name: 'Node' })).not.toBeInTheDocument();
  });

  it('toggles optional columns through the shared menu and persists the selection locally', async () => {
    const user = userEvent.setup();
    renderTable();
    await user.click(screen.getByRole('button', { name: 'Columns' }));
    await user.click(screen.getByRole('menuitemcheckbox', { name: 'Gateway' }));
    expect(screen.getByRole('menuitemcheckbox', { name: 'Gateway' })).toBeChecked();
    await user.keyboard('{Escape}');
    expect(screen.getByRole('columnheader', { name: 'Gateway' })).toBeVisible();
    expect(JSON.parse(localStorage.getItem('networks-optional-columns')!)).toEqual(['gateway']);
    await user.click(screen.getByRole('button', { name: 'Columns' }));
    await user.click(screen.getByRole('menuitemcheckbox', { name: 'Gateway' }));
    await user.keyboard('{Escape}');
    expect(screen.queryByRole('columnheader', { name: 'Gateway' })).not.toBeInTheDocument();
  });

  it('keeps same-ID networks on different Swarm nodes distinct during selection', async () => {
    platform.type = PlatformType.DockerSwarm;
    const first = network({ name: 'first', dockerNodeId: 'node-a', nodeHostname: 'node-a' });
    const second = network({ name: 'second', dockerNodeId: 'node-b', nodeHostname: 'node-b' });
    renderTable([first, second]);
    expect(screen.getByRole('columnheader', { name: 'Node' })).toBeVisible();
    fireEvent.click(screen.getAllByRole('checkbox', { name: 'Select network' })[0]);
    await waitFor(() => expect(selected).toHaveBeenLastCalledWith([first]));
  });

  it('opens a keyboard-accessible panel and preserves node scope in the inspection link', async () => {
    const user = userEvent.setup();
    renderTable([network({ dockerNodeId: 'node-a', isStale: true, staleReason: 'Node is unavailable.' })]);
    const trigger = screen.getByRole('button', { name: 'Show network details for application' });
    trigger.focus();
    await user.keyboard('{Enter}');
    const panel = screen.getByRole('dialog');
    expect(await within(panel).findByText('Node is unavailable.')).toBeVisible();
    expect(within(panel).getByRole('link', { name: 'Open network details' })).toHaveAttribute(
      'href',
      '/platforms/platform-a/networks/network-id/?dockerNodeId=node-a',
    );
    expect(within(panel).getByText(/not the whole Swarm/)).toBeVisible();
    await user.keyboard('{Escape}');
    await waitFor(() => expect(trigger).toHaveFocus());
  });

  it('shows the map in the sheet, preserves filters, and restores size before closing on Escape', async () => {
    const user = userEvent.setup();
    renderTable([network({ dockerNodeId: 'node-b' })], '/platforms/platform-a/networks?driver=bridge&usage=in-use');
    const trigger = screen.getByRole('button', { name: 'Show network details for application' });
    await user.click(trigger);
    expect(await screen.findByTestId('canvas')).toBeVisible();
    expect(screen.queryByRole('button', { name: 'View on map' })).not.toBeInTheDocument();
    const query = new URLSearchParams(screen.getByLabelText('Current query').textContent!);
    expect(query.get('network')).toBe('network-id');
    expect(query.get('networkPlatform')).toBe('platform-a');
    expect(query.get('dockerNodeId')).toBe('node-b');
    expect(query.get('driver')).toBe('bridge');
    expect(query.get('usage')).toBe('in-use');
    await user.click(screen.getByRole('button', { name: 'Maximize network sheet' }));
    expect(screen.getByRole('button', { name: 'Restore network sheet' })).toHaveAttribute('aria-pressed', 'true');
    await user.keyboard('{Escape}');
    expect(screen.getByRole('dialog')).toBeVisible();
    expect(screen.getByRole('button', { name: 'Maximize network sheet' })).toBeVisible();
    await user.keyboard('{Escape}');
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
    expect(trigger).toHaveFocus();
    expect(screen.getByLabelText('Current query')).toHaveTextContent('?driver=bridge&usage=in-use');
  });

  it('opens the correct Swarm node directly from the URL and refuses another platform selection', async () => {
    renderTable(
      [network({ name: 'first', dockerNodeId: 'node-a' }), network({ name: 'second', dockerNodeId: 'node-b' })],
      '/platforms/platform-a/networks?networkPlatform=platform-a&network=network-id&dockerNodeId=node-b',
    );
    expect(screen.getByRole('dialog', { name: 'second' })).toBeVisible();
    expect(await screen.findByTestId('canvas')).toBeVisible();
  });

  it('clears old node scope when focusing a manager network without mutating existing params', () => {
    const before = new URLSearchParams('dockerNodeId=old-node&networkPlatform=old-platform&scope=swarm');
    const after = networkMapParams(before, 'new-platform', network());
    expect(after.has('dockerNodeId')).toBe(false);
    expect(after.get('networkPlatform')).toBe('new-platform');
    expect(after.get('scope')).toBe('swarm');
    expect(before.get('dockerNodeId')).toBe('old-node');
  });

  it('does not present a link to inspection when that capability is denied', async () => {
    renderTable([network({ capabilities: { canRead: true, canWrite: false, canExecute: false, canInspect: false } })]);
    await userEvent.click(screen.getByRole('button', { name: 'Show network details for application' }));
    expect(screen.queryByRole('link', { name: 'Open network details' })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'View on map' })).not.toBeInTheDocument();
  });

  it('separates live attachments from saved references and uses authorized resource links', async () => {
    inspection.data = {
      data: network({
        usage: {
          isPartial: true,
          truncated: false,
          entries: [
            {
              isStale: false,
              workloadProcessing: false,
              relationship: NetworkRelationship.Attachment,
              containerId: 'container-uuid',
              dockerContainerId: 'docker-id',
              dockerNodeId: 'node-a',
              containerName: 'web-1',
              workloadId: 'deployment-uuid',
              workloadName: 'Application',
              workloadKind: NetworkWorkloadKind.Deployment,
              reference: null,
            },
            {
              isStale: false,
              workloadProcessing: false,
              relationship: NetworkRelationship.SavedReference,
              containerId: null,
              dockerContainerId: null,
              dockerNodeId: null,
              containerName: null,
              workloadId: 'stack-uuid',
              workloadName: 'Saved Stack',
              workloadKind: NetworkWorkloadKind.Stack,
              reference: 'application',
            },
          ],
        },
        containers: {},
      }),
    };
    renderTable();
    await userEvent.click(screen.getByRole('button', { name: 'Show network details for application' }));
    const panel = screen.getByRole('dialog');
    expect(await within(panel).findByRole('link', { name: 'web-1' })).toHaveAttribute(
      'href',
      '/platforms/platform-a/containers/container-uuid',
    );
    expect(within(panel).getByRole('link', { name: 'Application' })).toHaveAttribute(
      'href',
      '/deployments/edit/deployment-uuid/#config',
    );
    expect(within(panel).getByRole('link', { name: 'Saved Stack' })).toHaveAttribute(
      'href',
      '/stacks/edit/stack-uuid/#config',
    );
    expect(within(panel).getByRole('heading', { name: 'Connected to network' })).toBeVisible();
    expect(within(panel).getByRole('heading', { name: 'Configured workloads' })).toBeVisible();
  });

  it('hides cached inspection data after access is revoked', async () => {
    inspection.error = { status: 403 };
    renderTable();
    await userEvent.click(screen.getByRole('button', { name: 'Show network details for application' }));
    const panel = screen.getByRole('dialog');
    expect(within(panel).getByRole('alert')).toHaveTextContent('You do not have permission');
    expect(within(panel).queryByRole('heading', { name: 'Connected to network' })).not.toBeInTheDocument();
    expect(within(panel).queryByRole('button', { name: 'View on map' })).not.toBeInTheDocument();
  });

  it('searches subnets without gateways, driver and scope', () => {
    const value = network();
    for (const term of [' FD00:21 ', '172.22', 'bridge', 'local', 'network-id']) {
      expect(filterNetworks([value], term)).toEqual([value]);
    }
    expect(filterNetworks([value], 'other-network')).toEqual([]);
  });

  it('keeps deletion blocked when attachment details are hidden, stale or node-scoped', () => {
    expect(canDeleteNetwork(network())).toBe(true);
    for (const overrides of [
      { inUse: true },
      { isStale: true },
      { dockerNodeId: 'node' },
      { isSystem: true },
      { labels: { 'com.citadel.stack-id': 'stack' } },
    ])
      expect(canDeleteNetwork(network(overrides))).toBe(false);
  });

  it('does not equate an empty or stale attachment map with an unused network', () => {
    expect(networkUsageLabel(network({ inUse: true }))).toBe('In use');
    expect(networkUsageLabel(network())).toBe('No connected containers reported');
    expect(networkUsageLabel(network({ isStale: true }))).toBe('Unknown');
  });
});
