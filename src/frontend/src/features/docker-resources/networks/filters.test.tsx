import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, useLocation, useNavigate } from 'react-router';
import { Provider, createStore } from 'jotai';
import { NetworkRelationship, type NetworkView } from '@/api/generated/api.types';
import { applyNetworkFilters, readNetworkFilters, useNetworkFilters } from './filters';
import { NetworkViewControls, NetworkViews } from './views';

vi.mock('./table', () => ({
  NetworksTable: ({ items }: { items: NetworkView[] }) => <p>List: {items.map((item) => item.name).join(', ')}</p>,
}));
const network = (name: string, patch: Partial<NetworkView> = {}): NetworkView =>
  ({
    id: name,
    name,
    driver: 'bridge',
    scope: 'local',
    containers: {},
    isSystem: false,
    isStale: false,
    inUse: false,
    ...patch,
  }) as NetworkView;
const filter = (items: NetworkView[], query: string) =>
  applyNetworkFilters(items, new URLSearchParams(query)).map((item) => item.name);

it('combines categories, preserves plugin drivers, and ignores invalid filter values', () => {
  const items = [
    network('app'),
    network('overlay', { driver: 'overlay', scope: 'swarm' }),
    network('plugin', { driver: 'vendor-network' }),
    network('system', { isSystem: true }),
  ];
  expect(filter(items, 'driver=bridge&scope=local&system=hide')).toEqual(['app']);
  expect(filter(items, 'driver=bridge&driver=overlay&system=hide')).toEqual(['app', 'overlay']);
  expect(filter(items, 'driver=other')).toEqual(['plugin']);
  expect(filter(items, 'driver=invalid&scope=invalid&usage=invalid&system=invalid')).toEqual(
    items.map((item) => item.name),
  );
  expect(readNetworkFilters(new URLSearchParams('driver=bridge&driver=bridge')).driver).toEqual(['bridge']);
});

it('distinguishes actor-visible evidence, Docker usage, saved references and stale observations', () => {
  const items = [
    network('hidden-endpoints', { inUse: true }),
    network('attached', {
      inUse: true,
      usage: {
        entries: [
          {
            isStale: false,
            workloadProcessing: false,
            relationship: NetworkRelationship.Attachment,
            containerId: 'uuid',
          },
        ],
        isPartial: true,
        truncated: false,
      },
    }),
    network('saved-only', {
      usage: {
        entries: [
          {
            isStale: false,
            workloadProcessing: false,
            relationship: NetworkRelationship.SavedReference,
            workloadId: 'uuid',
          },
        ],
        isPartial: true,
        truncated: false,
      },
    }),
    network('stale', { isStale: true, inUse: true }),
  ];
  expect(filter(items, 'usage=attached')).toEqual(['attached']);
  expect(filter(items, 'usage=in-use')).toEqual(['hidden-endpoints', 'attached']);
  expect(filter(items, 'usage=not-reported')).toEqual(['saved-only']);
  expect(filter(items, 'usage=unknown')).toEqual(['stale']);
  expect(filter(items, 'usage=not-reported&usage=unknown')).toEqual(['saved-only', 'stale']);
});

it('does not let a false usage flag override visible endpoints, or treat stale false as unused', () => {
  expect(
    filter(
      [
        network('endpoint', {
          containers: { id: { name: 'web', endpointId: null, ipV4Address: null, ipv6Address: null, macAddress: null } },
        }),
      ],
      'usage=not-reported',
    ),
  ).toEqual([]);
  expect(filter([network('stale', { isStale: true })], 'usage=not-reported')).toEqual([]);
});

function Harness() {
  const items = useNetworkFilters([
    network('app'),
    network('overlay', { driver: 'overlay' }),
    network('system', { isSystem: true }),
  ]);
  const location = useLocation();
  const navigate = useNavigate();
  return (
    <>
      <NetworkViewControls />
      <NetworkViews items={items} actions={{}} isLoading={false} />
      <output aria-label="Current query">{location.search}</output>
      <button onClick={() => navigate(-1)}>Back</button>
    </>
  );
}

it('preserves sheet focus while filtering and supports browser Back and clear', async () => {
  const user = userEvent.setup();
  render(
    <Provider store={createStore()}>
      <MemoryRouter initialEntries={['/?view=list&network=app&networkPlatform=p&dockerNodeId=node-a']}>
        <Harness />
      </MemoryRouter>
    </Provider>,
  );
  expect(screen.getByText('List: app, overlay, system')).toBeVisible();
  await user.click(screen.getByRole('button', { name: 'Driver' }));
  await user.click(screen.getByRole('option', { name: 'bridge' }));
  await user.keyboard('{Escape}');
  expect(screen.getByText('List: app, system')).toBeVisible();
  await user.click(screen.getByRole('switch', { name: 'Show system networks' }));
  expect(screen.getByText('List: app')).toBeVisible();
  expect(screen.getByRole('button', { name: 'Columns' })).toBeVisible();
  await user.click(screen.getByRole('button', { name: 'Clear filters' }));
  expect(screen.getByText('List: app, overlay, system')).toBeVisible();
  const params = new URLSearchParams(screen.getByLabelText('Current query').textContent!);
  expect(params.get('network')).toBe('app');
  expect(params.get('dockerNodeId')).toBe('node-a');
  expect(params.get('view')).toBe('list');
  await user.click(screen.getByRole('button', { name: 'Back' }));
  expect(screen.getByText('List: app')).toBeVisible();
});
