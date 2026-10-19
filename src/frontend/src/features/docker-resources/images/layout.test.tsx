import { act, screen, within, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { ImagesTable } from './table';
import { ImageInfoComponents } from './image-info';
import { ImageComponents } from './index';
import { Provider } from 'jotai';
import { Route, Routes, useLocation, useNavigate } from 'react-router';
import { RegularResourceView } from '@/pages/regular-resource';
import type { ImageView, ImageInspectionView } from '@/api/generated/api.types';
vi.mock('@/lib/context/app-context', () => ({
  useAppContext: () => ({ currentPlatform: { id: 'platform-1', name: 'Host' } }),
}));
vi.mock('@/hooks/useRealtimeGroup', () => ({ useRealtimeGroup: () => {} }));
vi.mock('@/features/platforms/hooks/useDockerDaemonGroup', () => ({ useDockerDaemonGroup: () => {} }));
beforeEach(() => {
  for (const key of ['images-show-id', 'images-show-registry', 'images-show-created']) localStorage.removeItem(key);
  server.use(http.get('http://localhost/api/v1/profile/preferences', () => HttpResponse.json({})));
});
const image = {
  id: 'image-1',
  platformId: 'platform-1',
  dockerImageId: `sha256:${'a'.repeat(64)}`,
  name: 'nginx',
  tags: ['nginx:17'],
  size: 1024,
  isInUse: true,
  createdAt: '2026-10-16T00:00:00Z',
  isStale: false,
  usedBy: { containers: 2, deployments: 1, stacks: 1, swarmServices: 0, isPartial: true },
  updates: [{ reference: 'nginx:17', status: 'UpToDate', isStale: false }],
} as ImageView;
it('renders update status and usage directly from the image list payload', () => {
  renderCitadel(<ImagesTable items={[image]} isLoading={false} actions={{}} />);
  expect(screen.getByText('Update status')).toBeVisible();
  expect(screen.getByText('Used by')).toBeVisible();
  expect(screen.getByText('Up to date')).toBeVisible();
  expect(screen.getByText('2 containers')).toBeVisible();
  expect(screen.getByText('1 deployment')).toBeVisible();
  expect(screen.getByText('1 stack')).toBeVisible();
});
it('shows additional tags on demand and remembers optional columns', async () => {
  localStorage.removeItem('images-show-registry');
  localStorage.removeItem('images-show-created');
  const taggedImage = { ...image, tags: ['localhost:5000/team/nginx:17', 'team/nginx:stable'] };
  const HeaderActions = ImageComponents.header!.Extra!;
  const { user, unmount } = renderCitadel(
    <Provider>
      <div role="toolbar" aria-label="Image actions">
        <HeaderActions />
      </div>
      <ImagesTable items={[taggedImage]} isLoading={false} actions={{}} />
    </Provider>,
  );
  const toolbar = within(screen.getByRole('toolbar', { name: 'Image actions' }));
  expect(toolbar.getByRole('button', { name: 'Columns' })).toBeVisible();
  expect(toolbar.getByRole('button', { name: 'Pull Image' })).toBeVisible();
  expect(screen.getAllByRole('button', { name: 'Columns' })).toHaveLength(1);
  expect(screen.getAllByRole('columnheader')).toHaveLength(6);
  expect(screen.queryByRole('columnheader', { name: 'ID' })).not.toBeInTheDocument();
  expect(screen.queryByTitle(taggedImage.dockerImageId)).not.toBeInTheDocument();
  expect(screen.getByRole('link', { name: 'Show image details: localhost:5000/team/nginx' })).toBeVisible();
  expect(screen.getByText('17')).toBeVisible();
  expect(screen.queryByText('team/nginx:stable')).not.toBeInTheDocument();
  await user.click(screen.getByRole('button', { name: 'Show all 2 image tags' }));
  expect(screen.getByText('team/nginx:stable')).toBeVisible();
  await user.keyboard('{Escape}');
  await user.click(screen.getByRole('button', { name: 'Columns' }));
  await user.click(screen.getByRole('menuitemcheckbox', { name: 'Created' }));
  await user.click(screen.getByRole('menuitemcheckbox', { name: 'ID' }));
  await user.keyboard('{Escape}');
  expect(screen.getByRole('columnheader', { name: 'Created' })).toBeVisible();
  expect(screen.getByRole('columnheader', { name: 'ID' })).toBeVisible();
  expect(screen.getByTitle(taggedImage.dockerImageId)).toBeVisible();
  unmount();
  renderCitadel(
    <Provider>
      <ImagesTable items={[taggedImage]} isLoading={false} actions={{}} />
    </Provider>,
  );
  expect(screen.getByRole('columnheader', { name: 'Created' })).toBeVisible();
  expect(screen.queryByRole('columnheader', { name: 'Registry' })).not.toBeInTheDocument();
  expect(screen.getByRole('columnheader', { name: 'ID' })).toBeVisible();
  expect(screen.getByRole('button', { name: 'Copy to clipboard' })).toBeInTheDocument();
  localStorage.removeItem('images-show-id');
  localStorage.removeItem('images-show-created');
  localStorage.removeItem('images-show-registry');
});
it('does not refetch profile preferences when toggling image columns', async () => {
  localStorage.removeItem('images-show-registry');
  localStorage.removeItem('images-show-created');
  const preferences = vi.fn(() => HttpResponse.json({ timeZone: 'UTC', dateTimeFormat: 'System' }));
  server.use(http.get('http://localhost/api/v1/profile/preferences', preferences));
  const HeaderActions = ImageComponents.header!.Extra!;
  const { user, queryClient } = renderCitadel(
    <Provider>
      <HeaderActions />
      <ImagesTable items={[image]} isLoading={false} actions={{}} />
    </Provider>,
  );
  await waitFor(() => expect(preferences).toHaveBeenCalled());
  await waitFor(() => expect(queryClient.isFetching()).toBe(0));
  const before = preferences.mock.calls.length;
  for (const column of ['Registry', 'Created', 'ID']) {
    for (const visible of [true, false]) {
      await user.click(screen.getByRole('button', { name: 'Columns' }));
      await user.click(screen.getByRole('menuitemcheckbox', { name: column }));
      expect(screen.getByRole('menuitemcheckbox', { name: column })).toHaveAttribute('aria-checked', String(visible));
      await user.keyboard('{Escape}');
      if (visible) expect(screen.getByRole('columnheader', { name: column })).toBeVisible();
      else expect(screen.queryByRole('columnheader', { name: column })).not.toBeInTheDocument();
      await waitFor(() => expect(queryClient.isFetching()).toBe(0));
      expect(preferences).toHaveBeenCalledTimes(before);
    }
  }
  // Profile changes still update date formatting through the existing query cache.
  await act(async () => {
    await queryClient.invalidateQueries({ queryKey: ['getProfilePreferences'] });
  });
  expect(preferences).toHaveBeenCalledTimes(before + 1);
});

it('extends the existing details section with saved workloads instead of adding a Used by tab', async () => {
  const requests = vi.fn();
  expect(ImageInfoComponents.Tabs.map((tab) => tab.label)).toEqual(['Inspect']);
  server.use(
    http.get(`http://localhost/api/v1/images/platform-1/${image.dockerImageId}/usage`, () => {
      requests();
      return HttpResponse.json({
        entries: [
          {
            relationship: 'SavedDigest',
            workloadKind: 'SwarmService',
            workloadId: 'service-1',
            workloadName: 'Saved service',
            workloadStatus: 'Degraded',
            reference: 'nginx@sha256:123',
            isStale: true,
          },
          {
            relationship: 'SavedLocalImage',
            workloadKind: 'Deployment',
            workloadId: 'deployment-1',
            workloadName: 'Saved deployment',
            workloadStatus: 'Healthy',
            isStale: false,
          },
          {
            relationship: 'SavedTag',
            workloadKind: 'Stack',
            workloadId: 'stack-1',
            workloadName: 'Saved stack',
            workloadStatus: 'Stopped',
            reference: 'nginx:17',
            isStale: false,
          },
        ],
        nextOffset: null,
        coverageComplete: false,
        limitations: [],
      });
    }),
  );
  const Content = ImageInfoComponents.Tabs[0].Content;
  const resource = {
    id: image.dockerImageId,
    name: 'nginx',
    tag: '17',
    repoTags: image.tags,
    size: 1024,
    os: 'linux',
    architecture: 'amd64',
    created: image.createdAt,
    containers: [
      { id: '7e4173963de7', name: '/wordpress-db-1', state: 'running', networks: {}, volumes: [], ports: [] },
    ],
    layers: [],
    labels: {},
    exposedPorts: [],
    cmd: [],
    entryPoint: [],
    env: [],
    volumes: [],
    updates: image.updates ?? [],
  } as ImageInspectionView;
  const { user } = renderCitadel(<Content resource={resource} />);
  expect(screen.queryByRole('heading', { name: 'Update status' })).not.toBeInTheDocument();
  const status = screen.getByRole('button', { name: 'Update status: Up to date' });
  expect(status.closest('section')).toContainElement(screen.getByRole('heading', { name: 'Image details' }));
  expect(status.textContent).toBe('');
  expect(screen.getByText('Workloads using this image')).toBeVisible();
  const runningContainer = screen.getByRole('link', { name: 'wordpress-db-1' });
  expect(runningContainer).toHaveAttribute('href', '/platforms/platform-1/containers/7e4173963de7');
  expect(runningContainer.closest('td')!.querySelector('.rounded-full')).toHaveClass('bg-green-500');
  expect(screen.queryByText(/Containers using this image/)).not.toBeInTheDocument();
  expect(await screen.findByRole('link', { name: 'Saved deployment' })).toHaveAttribute(
    'href',
    '/deployments/edit/deployment-1',
  );
  expect(await screen.findByRole('link', { name: 'Saved stack' })).toHaveAttribute('href', '/stacks/edit/stack-1');
  const title = screen.getByRole('heading', { name: 'Workloads using this image' });
  const header = title.parentElement!.parentElement!;
  const refresh = within(header).getByRole('button', { name: 'Refresh references' });
  expect(refresh.textContent).toBe('');
  expect(screen.getAllByRole('button', { name: 'Refresh references' })).toHaveLength(1);
  for (const [group, name, status] of [
    ['Deployments', 'Saved deployment', 'Healthy'],
    ['Stacks', 'Saved stack', 'Stopped'],
    ['Swarm Services', 'Saved service', 'Degraded'],
  ]) {
    const section = within(screen.getByRole('region', { name: group }));
    const table = section.getByRole('table');
    expect(within(table).getAllByRole('row')).toHaveLength(2);
    for (const heading of ['Name', 'Reference', 'Observed'])
      expect(within(table).getByRole('columnheader', { name: heading })).toBeVisible();
    const cell = within(table).getByRole('link', { name }).closest('td')!;
    expect(within(cell).getByLabelText(`${name}: ${status}`)).toBeVisible();
    expect(cell.querySelector('[data-slot="tooltip-trigger"]')).toBeTruthy();
  }
  const services = within(screen.getByRole('region', { name: 'Swarm Services' }));
  expect(services.getByRole('link', { name: 'Saved service' })).toHaveAttribute(
    'href',
    '/swarm-services/edit/service-1',
  );
  expect(services.getByText('Inventory observation is stale.')).toBeVisible();
  await waitFor(() => expect(refresh).toBeEnabled());
  const before = requests.mock.calls.length;
  await user.click(refresh);
  await waitFor(() => expect(requests).toHaveBeenCalledTimes(before + 1));
});

it('filters fresh image updates from the URL, composes with search, and follows cached realtime results', async () => {
  const update = (status: string, isStale = false) => ({ reference: 'nginx:17', status, isStale });
  const images = [
    { name: 'ready-update', updates: [update('UpdateAvailable')] },
    {
      name: 'mixed-update',
      updates: [update('UpToDate'), { ...update('UpdateAvailable'), reference: 'nginx:stable' }],
    },
    { name: 'current', updates: [update('UpToDate')] },
    { name: 'stale-result', updates: [update('UpdateAvailable', true)] },
    { name: 'stale-image', isStale: true, updates: [update('UpdateAvailable')] },
    { name: 'failed', updates: [update('CheckFailed')] },
    { name: 'unchecked', updates: [] },
  ].map((overrides) => ({ ...image, ...overrides, id: overrides.name, tags: [`${overrides.name}:17`] })) as ImageView[];
  const requests = vi.fn(() => HttpResponse.json({ images, capabilities: { canRead: true, canPull: true } }));
  server.use(
    http.get('http://localhost/api/v1/images/platform-1', requests),
    http.get('http://localhost/api/v1/lookup', () => HttpResponse.json([])),
  );
  function Page() {
    const location = useLocation();
    const navigate = useNavigate();
    return (
      <>
        <RegularResourceView Components={ImageComponents} type="Image" showTaskSheet={false} />
        <output aria-label="Location">{location.search}</output>
        <button onClick={() => navigate(-1)}>Back</button>
      </>
    );
  }
  const { user, queryClient } = renderCitadel(
    <Provider>
      <Routes>
        <Route path="/platforms/:platformId/images" element={<Page />} />
      </Routes>
    </Provider>,
    { route: '/platforms/platform-1/images?view=compact&updates=available' },
  );
  const link = (name: string) => screen.queryByRole('link', { name: `Show image details: ${name}` });
  expect(await screen.findByRole('link', { name: 'Show image details: ready-update' })).toBeVisible();
  expect(link('mixed-update')).toBeVisible();
  for (const name of ['current', 'stale-result', 'stale-image', 'failed', 'unchecked'])
    expect(link(name)).not.toBeInTheDocument();
  const toolbar = within(screen.getByRole('region', { name: 'Images filters' }));
  const filter = toolbar.getByRole('button', { name: 'Updates available' });
  expect(filter).toHaveAttribute('aria-pressed', 'true');
  expect(filter.nextElementSibling).toBe(toolbar.getByRole('button', { name: 'Pull Image' }));
  expect(toolbar.getByRole('button', { name: 'Pull Image' }).nextElementSibling).toBe(
    toolbar.getByRole('button', { name: 'Columns' }),
  );
  await user.click(filter);
  expect(link('current')).toBeVisible();
  expect(screen.getByLabelText('Location')).toHaveTextContent('?view=compact');
  await user.click(screen.getByRole('button', { name: 'Back' }));
  expect(filter).toHaveAttribute('aria-pressed', 'true');
  expect(link('current')).not.toBeInTheDocument();
  const search = screen.getByPlaceholderText('Search images…');
  await user.type(search, 'mixed');
  await waitFor(() => expect(link('ready-update')).not.toBeInTheDocument());
  expect(link('mixed-update')).toBeVisible();
  await user.clear(search);
  await waitFor(() => expect(link('ready-update')).toBeVisible());
  await act(async () => {
    queryClient.setQueryData(['listImages', { platformId: 'platform-1' }], {
      data: {
        images: images.map((item) => ({ ...item, updates: [update('UpToDate')] })),
        capabilities: { canRead: true, canPull: true },
      },
    });
  });
  expect(await screen.findByText('No images match the current filters.')).toBeVisible();
  expect(filter).toHaveAttribute('aria-pressed', 'true');
  await user.click(filter);
  expect(link('ready-update')).toBeVisible();
  expect(requests).toHaveBeenCalledTimes(1);
});

it.each(['pointer', 'keyboard'])(
  'uses the single-image confirmation without reopening the menu (%s)',
  async (interaction) => {
    const { ImageDropdownActions } = await import('./actions');
    const row = {
      ...image,
      isInUse: false,
      capabilities: { canRead: true, canWrite: true, canExecute: true, canPull: true, canInspect: true },
    };
    const deleted = vi.fn();
    server.use(
      http.delete('http://localhost/api/v1/images', async ({ request }) => {
        deleted(await request.json());
        return HttpResponse.json({ deletedImages: [] });
      }),
    );
    const { user } = renderCitadel(<ImagesTable items={[row]} isLoading={false} actions={ImageDropdownActions} />);
    const copy = vi.spyOn(navigator.clipboard, 'writeText');
    const trigger = screen.getByRole('button', { name: 'Open menu' });
    await user.click(trigger);
    await user.keyboard('{Escape}');
    await waitFor(() => expect(trigger).toHaveFocus());
    await user.click(trigger);
    const menuStates: (string | null)[] = [];
    const observer = new MutationObserver((records) => {
      menuStates.push(...records.map((record) => record.oldValue));
    });
    observer.observe(trigger, { attributes: true, attributeFilter: ['aria-expanded'], attributeOldValue: true });
    try {
      const deleteItem = screen.getByRole('menuitem', { name: 'Delete' });
      if (interaction === 'keyboard') {
        act(() => deleteItem.focus());
        await user.keyboard('{Enter}');
      } else {
        await user.click(deleteItem);
      }
      await waitFor(() => expect(screen.getByRole('textbox')).toHaveFocus());
      expect(trigger).toHaveAttribute('aria-expanded', 'false');
      expect(screen.queryByRole('menu')).not.toBeInTheDocument();
      expect(menuStates).toEqual(['true']);
    } finally {
      observer.disconnect();
    }
    const dialog = within(await screen.findByRole('dialog'));
    const name = row.name;
    expect(dialog.queryByRole('list')).not.toBeInTheDocument();
    await user.click(dialog.getByRole('button', { name: `Copy ${name}` }));
    expect(copy).toHaveBeenCalledWith(name);
    const input = dialog.getByRole('textbox', { name: `Enter ${name} to confirm` });
    await user.type(input, 'Delete');
    expect(dialog.getByRole('button', { name: 'Delete' })).toBeDisabled();
    await user.clear(input);
    await user.type(input, name);
    await waitFor(() => expect(dialog.getByRole('button', { name: 'Delete' })).toBeEnabled());
    await user.click(dialog.getByRole('button', { name: 'Delete' }));
    await waitFor(() =>
      expect(deleted).toHaveBeenCalledExactlyOnceWith({
        platformId: 'platform-1',
        ids: [row.dockerImageId],
        force: false,
        noPrune: true,
      }),
    );
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
  },
);
