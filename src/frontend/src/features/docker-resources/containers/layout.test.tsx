import { act, screen, within, waitFor } from '@testing-library/react';
import { Provider } from 'jotai';
import { useLocation, useNavigate } from 'react-router';
import {
  BackupCoverageStatus,
  ContainerHealthStatus,
  ContainerStateStatus,
  PlatformType,
  ResourceControlState,
  type ContainerView,
} from '@/api/generated/api.types';
import { RegularResourceView } from '@/pages/regular-resource';
import { renderCitadel } from '@/test/render-citadel';
import { ContainerComponents } from './index';

vi.hoisted(() => {
  document.queryCommandSupported ??= () => false;
});
const data = vi.hoisted(() => ({ items: [] as ContainerView[] }));
vi.mock('./hooks/useContainersGroup', () => ({
  useContainersGroup: () => ({ containersInfo: { containers: data.items }, isLoading: false }),
}));
vi.mock('@/hooks/useRealtimeGroup', () => ({ useRealtimeGroup: () => ({ isConnected: false, isLoading: false }) }));
vi.mock('@/lib/context/app-context', () => ({
  useAppContext: () => ({ currentPlatform: { id: 'platform', type: PlatformType.Docker } }),
}));

const container = (
  name: string,
  health = ContainerHealthStatus.Healthy,
  backup = BackupCoverageStatus.Protected,
): ContainerView => ({
  id: name,
  containerId: `docker-${name}`,
  platformId: 'platform',
  name,
  state: ContainerStateStatus.Running,
  controlState: ResourceControlState.Idle,
  ports: {},
  created: 1,
  updated: 1,
  isSystem: false,
  stackId: null,
  deploymentId: null,
  capabilities: null,
  dockerImageId: 'image',
  dockerNodeId: null,
  nodeHostname: null,
  hasCitadelOwnershipLabels: false,
  isSwarmTask: false,
  lastStats: null,
  projectionObservedAt: null,
  projectionStaleReason: null,
  projectionStaleSince: null,
  stack: null,
  systemRole: null,
  health: { status: health, observedAt: Date.now(), reason: '' },
  imageView: {
    id: 'image',
    name: 'wordpress:latest',
    dockerImageId: 'image',
    platformId: 'platform',
    capabilities: null,
    contentIdentity: null,
    controlState: ResourceControlState.Idle,
    createdAt: new Date().toISOString(),
    dockerNodeId: null,
    nodeHostname: null,
    isInUse: true,
    isStale: false,
    registryId: null,
    repoDigests: null,
    size: 0,
    staleReason: null,
    tags: ['wordpress:latest'],
    updatedAt: null,
  },
  backupCoverage: {
    platformId: 'platform',
    status: backup,
    unavailable: false,
    canConfigure: false,
    hasResourcePolicy: false,
    volumes: [{ name: 'data', status: backup, policies: [], includedInResourcePolicy: false }],
    policies: [],
    reasons: [],
    warnings: [],
  },
});

beforeEach(() => {
  localStorage.clear();
  data.items = [
    container('wordpress-web'),
    container('wordpress-db', ContainerHealthStatus.Unhealthy, BackupCoverageStatus.Failed),
  ];
});
function Location() {
  const location = useLocation();
  const navigate = useNavigate();
  return (
    <>
      <output aria-label="Current query">{location.search}</output>
      <button onClick={() => navigate(-1)}>Back</button>
    </>
  );
}
function View() {
  return (
    <Provider>
      <RegularResourceView
        Components={{ ...ContainerComponents, DropdownActions: {}, GroupActions: undefined }}
        type="Container"
        showTaskSheet={false}
      />
      <Location />
    </Provider>
  );
}

it('keeps separate filters and columns without overview counters', async () => {
  renderCitadel(<View />, { route: '/containers?overview=stopped' });
  const toolbar = within(screen.getByRole('region', { name: 'Containers filters' }));
  expect(toolbar.queryByRole('button', { name: 'Filters' })).not.toBeInTheDocument();
  expect(toolbar.getByRole('switch', { name: 'Show unmanaged containers only' })).toBeVisible();
  expect(toolbar.getByRole('button', { name: 'Columns' })).toBeVisible();
  expect(toolbar.getByRole('button', { name: 'Health Filter' })).toBeVisible();
  expect(screen.getByRole('columnheader', { name: 'Health' })).toBeVisible();
  expect(toolbar.getByRole('button', { name: 'Backup Filter' })).toBeVisible();
  expect(screen.queryByRole('columnheader', { name: 'ID' })).not.toBeInTheDocument();
  expect(screen.getByRole('columnheader', { name: 'Image' })).toBeVisible();
  expect(screen.getByRole('columnheader', { name: 'Backup' })).toBeVisible();
  const row = screen.getByRole('link', { name: 'wordpress-web' }).closest('tr')!;
  const imageLink = within(row).getByRole('link', { name: 'wordpress:latest' });
  expect(imageLink).toBeVisible();
  expect(imageLink.closest('td')).not.toBe(screen.getByRole('link', { name: 'wordpress-web' }).closest('td'));
  expect(screen.queryByRole('region', { name: 'Container overview' })).not.toBeInTheDocument();
  expect(screen.getAllByRole('link', { name: /^wordpress-(web|db)$/ })).toHaveLength(2);
});

it('combines health and backup filters, keeps active chips visible, and preserves unrelated query parameters', async () => {
  const { user } = renderCitadel(<View />, { route: '/containers?tags=prod&overview=running' });
  await user.click(screen.getByRole('button', { name: 'Health Filter' }));
  await user.click(screen.getByRole('option', { name: 'Unhealthy' }));
  await user.keyboard('{Escape}');
  await user.click(screen.getByRole('button', { name: 'Backup Filter' }));
  await user.click(screen.getByRole('option', { name: 'Backup failed' }));
  await user.keyboard('{Escape}');
  expect(screen.queryByRole('link', { name: 'wordpress-web' })).not.toBeInTheDocument();
  expect(screen.getByRole('link', { name: 'wordpress-db' })).toBeVisible();
  expect(screen.getByRole('button', { name: 'Remove Backup failed backup filter' })).toBeVisible();
  expect(screen.getByRole('button', { name: 'Remove Unhealthy health filter' })).toBeVisible();
  await user.click(screen.getByRole('button', { name: 'Remove Unhealthy health filter' }));
  await user.click(screen.getByRole('button', { name: 'Remove Backup failed backup filter' }));
  expect(screen.getByRole('link', { name: 'wordpress-web' })).toBeVisible();
  expect(screen.getByLabelText('Current query')).toHaveTextContent('?tags=prod&overview=running');
  await user.click(screen.getByRole('button', { name: 'Back' }));
  expect(screen.getByRole('button', { name: 'Remove Backup failed backup filter' })).toBeVisible();
});

it('toggles multiple options without closing the filter menu and supports search and ownership', async () => {
  data.items[1].stackId = 'managed-stack';
  const { user } = renderCitadel(<View />);
  await user.click(screen.getByRole('button', { name: 'Health Filter' }));
  await user.click(screen.getByRole('option', { name: 'Healthy' }));
  await user.click(screen.getByRole('option', { name: 'Unhealthy' }));
  expect(screen.getAllByRole('link', { name: /^wordpress-(web|db)$/ })).toHaveLength(2);
  await user.type(screen.getByPlaceholderText('Search health states...'), 'Starting');
  expect(screen.getByRole('option', { name: 'Starting' })).toBeVisible();
  expect(screen.queryByRole('option', { name: 'Unavailable' })).not.toBeInTheDocument();
  await user.keyboard('{Escape}');
  await user.click(screen.getByRole('switch', { name: 'Show unmanaged containers only' }));
  expect(screen.queryByRole('link', { name: 'wordpress-db' })).not.toBeInTheDocument();
  expect(screen.getByRole('switch', { name: 'Show unmanaged containers only' })).toBeChecked();
});

it('remembers optional health and image visibility and keeps Backup available', async () => {
  const { user, unmount } = renderCitadel(<View />);
  await user.click(screen.getByRole('button', { name: 'Columns' }));
  await user.click(screen.getByRole('menuitemcheckbox', { name: 'ID' }));
  await user.click(screen.getByRole('menuitemcheckbox', { name: 'CPU' }));
  await user.click(screen.getByRole('menuitemcheckbox', { name: 'Image' }));
  await user.click(screen.getByRole('menuitemcheckbox', { name: 'Health' }));
  expect(screen.queryByRole('menuitemcheckbox', { name: 'Backup' })).not.toBeInTheDocument();
  await user.keyboard('{Escape}');
  expect(screen.getByRole('columnheader', { name: 'ID' })).toBeVisible();
  expect(screen.queryByRole('columnheader', { name: 'CPU' })).not.toBeInTheDocument();
  unmount();
  renderCitadel(<View />);
  await waitFor(() => expect(screen.getByRole('columnheader', { name: 'ID' })).toBeVisible());
  expect(screen.queryByRole('columnheader', { name: 'CPU' })).not.toBeInTheDocument();
  expect(screen.queryByRole('columnheader', { name: 'Health' })).not.toBeInTheDocument();
  expect(screen.queryByRole('button', { name: 'Health Filter' })).not.toBeInTheDocument();
  expect(screen.queryByRole('columnheader', { name: 'Image' })).not.toBeInTheDocument();
  expect(screen.queryByRole('link', { name: 'wordpress:latest' })).not.toBeInTheDocument();
  await user.click(screen.getByRole('button', { name: 'Columns' }));
  await user.click(screen.getByRole('menuitemcheckbox', { name: 'Health' }));
  await user.keyboard('{Escape}');
  expect(screen.getByRole('columnheader', { name: 'Health' })).toBeVisible();
  expect(screen.getByRole('button', { name: 'Health Filter' })).toBeVisible();
  expect(screen.getByRole('columnheader', { name: 'Backup' })).toBeVisible();
  await user.click(screen.getByRole('button', { name: 'Columns' }));
  await user.click(screen.getByRole('menuitemcheckbox', { name: 'Image' }));
  await user.keyboard('{Escape}');
  expect(screen.getByRole('columnheader', { name: 'Image' })).toBeVisible();
  expect(screen.getAllByRole('link', { name: 'wordpress:latest' })).toHaveLength(2);
});

it('shows Health and Image for existing column preferences without those entries', async () => {
  localStorage.setItem('containers-columns', JSON.stringify({ containerId: true, CPU: false }));
  renderCitadel(<View />);
  await waitFor(() => expect(screen.getByRole('columnheader', { name: 'ID' })).toBeVisible());
  expect(screen.getByRole('columnheader', { name: 'Health' })).toBeVisible();
  expect(screen.getByRole('columnheader', { name: 'Image' })).toBeVisible();
});

it('pauses health filtering while its column is hidden and restores the selection when shown', async () => {
  localStorage.setItem('containers-columns', JSON.stringify({ health: false }));
  const { user } = renderCitadel(<View />, { route: '/containers?health=Unhealthy&tags=prod' });
  await waitFor(() => expect(screen.queryByRole('columnheader', { name: 'Health' })).not.toBeInTheDocument());
  expect(screen.queryByRole('button', { name: 'Health Filter' })).not.toBeInTheDocument();
  expect(screen.getAllByRole('link', { name: /^wordpress-(web|db)$/ })).toHaveLength(2);
  await user.click(screen.getByRole('button', { name: 'Columns' }));
  await user.click(screen.getByRole('menuitemcheckbox', { name: 'Health' }));
  await user.keyboard('{Escape}');
  expect(screen.getByRole('button', { name: 'Remove Unhealthy health filter' })).toBeVisible();
  expect(screen.queryByRole('link', { name: 'wordpress-web' })).not.toBeInTheDocument();
  expect(screen.getByRole('link', { name: 'wordpress-db' })).toBeVisible();
  await user.click(screen.getByRole('button', { name: 'Columns' }));
  await user.click(screen.getByRole('menuitemcheckbox', { name: 'Health' }));
  await user.keyboard('{Escape}');
  expect(screen.queryByRole('button', { name: 'Remove Unhealthy health filter' })).not.toBeInTheDocument();
  expect(screen.getAllByRole('link', { name: /^wordpress-(web|db)$/ })).toHaveLength(2);
  expect(screen.getByLabelText('Current query')).toHaveTextContent('?health=Unhealthy&tags=prod');
});

it('shows full stack health when health and search filters hide a failing member', async () => {
  data.items = data.items.map((item) => ({ ...item, stack: 'wordpress' }));
  const { user, rerender } = renderCitadel(<View />);
  await user.click(screen.getByRole('button', { name: 'Health Filter' }));
  await user.click(screen.getByRole('option', { name: 'Healthy' }));
  await user.keyboard('{Escape}');
  await user.type(screen.getByPlaceholderText('Search containers…'), 'wordpress-web');
  await waitFor(() => expect(screen.getByText('1 container')).toBeVisible());
  const trigger = screen.getByRole('button', { name: 'Stack health: Unhealthy' });
  expect(trigger.textContent).toBe('');
  await user.hover(trigger);
  const card = within(screen.getByRole('dialog', { name: 'Stack health' }));
  expect(card.getByText(/Based on 2 accessible containers/)).toBeVisible();
  expect(card.getByText('Healthy')).toBeVisible();
  expect(card.getAllByText('Unhealthy')).toHaveLength(2);
  expect(card.queryByRole('button', { name: 'View details' })).not.toBeInTheDocument();
  await user.keyboard('{Escape}');
  data.items = data.items.map((item) => ({
    ...item,
    health: { ...item.health!, status: ContainerHealthStatus.Healthy },
  }));
  rerender(<View />);
  expect(screen.getByRole('button', { name: 'Stack health: Healthy' })).toBeVisible();
  data.items = data.items.map((item) => ({ ...item, health: { ...item.health!, observedAt: Date.now() - 240_000 } }));
  // Remove health filtering to inspect the group's now-stale evidence.
  await user.click(screen.getByRole('button', { name: 'Remove Healthy health filter' }));
  expect(screen.getByRole('button', { name: 'Stack health: Unknown' })).toBeVisible();
});

it('expires a quiet stack health summary without new inventory or requests', async () => {
  vi.useFakeTimers();
  try {
    data.items = data.items.map((item) => ({
      ...item,
      stack: 'wordpress',
      health: { ...item.health!, status: ContainerHealthStatus.Healthy },
    }));
    renderCitadel(<View />);
    expect(screen.getByRole('button', { name: 'Stack health: Healthy' })).toBeVisible();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(210_000);
    });
    expect(screen.getByRole('button', { name: 'Stack health: Unknown' })).toBeVisible();
  } finally {
    vi.useRealTimers();
  }
});
