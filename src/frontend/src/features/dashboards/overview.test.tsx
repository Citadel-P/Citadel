import { act, screen, waitFor, within } from '@testing-library/react';
import { focusManager } from '@tanstack/react-query';
import { http, HttpResponse } from 'msw';
import { Route, Routes } from 'react-router';
import { collectionEventKey, collectionGroup } from '@/lib/collection-scope';
import { AppProvider } from '@/lib/context/app-provider';
import * as transport from '@/lib/createRealtimeConnection';
import { FakeRealtimeConnection } from '@/test/fakes/realtime';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import Board from './board';
import { DashboardGrid } from './grid';
import { fromTemplate } from './templates';

it('renders the private Overview with one read per source and one listener for duplicate live tiles', async () => {
  const config = fromTemplate('Overview');
  const alertsTile = config.widgets.find((w) => w.settings.kind === 'OpenAlerts')!;
  config.widgets.push({ ...alertsTile, title: 'Second alert list', id: 'second', layout: { x: 6, y: 8, w: 6, h: 4 } });
  const live = new FakeRealtimeConnection();
  vi.spyOn(transport, 'createRealtimeConnection').mockReturnValue(live);
  const platforms = vi.fn(() =>
    HttpResponse.json({
      platforms: [
        {
          id: 'p',
          name: 'Lab',
          status: 'Online',
          platformDescriptor: { $type: 'Docker', containerCount: 3 },
          stats: [{ cpuUsage: 2, memoryUsage: 25 }],
        },
      ],
    }),
  );
  const alerts = vi.fn(() => HttpResponse.json({ pagedResult: { items: [], totalCount: 0, pageSize: 5 } }));
  const recent = vi.fn(({ request }: { request: Request }) => {
    const query = new URL(request.url).searchParams;
    return HttpResponse.json([
      {
        id: 'event',
        eventType: query.get('eventType') ?? 'StackApplied',
        resourceName: query.has('eventType') ? 'Nightly volumes' : 'Web',
        createdAt: '2026-11-01T10:00:00Z',
        status: 'Success',
      },
    ]);
  });
  server.use(
    http.get('*/api/v1/application/info', () => HttpResponse.json({ realtimeTransport: 'WebSocketV1' })),
    http.get('*/api/v1/profile/preferences', () => HttpResponse.json({})),
    http.get('*/api/v1/profile', () => HttpResponse.json({ authorization: { isAdministrator: false } })),
    http.get('*/api/v1/dashboards/mine', () =>
      HttpResponse.json({
        summary: { id: 'mine', name: 'My overview', visibility: 'Private', canEdit: true },
        configuration: config,
      }),
    ),
    http.get('*/api/v1/platforms', platforms),
    http.get('*/api/v1/alertEvents', alerts),
    http.get('*/api/v1/activities/recent', recent),
  );
  const { unmount } = renderCitadel(
    <AppProvider>
      <Routes>
        <Route path="/dashboards/:id" element={<Board />} />
      </Routes>
    </AppProvider>,
    { route: '/dashboards/mine' },
  );
  await screen.findByRole('link', { name: 'Nightly volumes' });
  expect(await screen.findByRole('link', { name: 'Stack Applied · Web' })).toHaveAttribute('href', '/activities/event');
  expect(screen.getByRole('link', { name: 'Lab' })).toHaveAttribute('href', '/platforms/edit/p');
  expect(screen.getByText('Memory 25.0%')).toBeVisible();
  expect(within(screen.getByRole('region', { name: 'Resource summary' })).getByText('3')).toBeVisible();
  await waitFor(() => expect(live.listenerCount('AlertEventReceived')).toBe(1));
  act(() =>
    live.emit('AlertEventReceived', {
      id: 'alert',
      message: 'Disk nearly full',
      status: 'Active',
      severity: 'Warning',
    }),
  );
  expect(await screen.findAllByRole('link', { name: 'Disk nearly full' })).toHaveLength(2);
  expect(alerts).toHaveBeenCalledTimes(1);
  expect(platforms).toHaveBeenCalledTimes(1);
  expect(recent).toHaveBeenCalledTimes(2);
  expect(recent.mock.calls.map(([{ request }]) => new URL(request.url).searchParams.get('limit'))).toEqual(['5', '5']);
  try {
    act(() => focusManager.setFocused(false));
    expect(screen.queryByRole('region', { name: 'Recent activity' })).toBeNull();
    expect(live.listenerCount('AlertEventReceived')).toBe(1);
    act(() => focusManager.setFocused(true));
    await screen.findByRole('region', { name: 'Recent activity' });
  } finally {
    act(() => focusManager.setFocused(undefined));
  }
  unmount();
  expect(live.listenerCount('AlertEventReceived')).toBe(0);
});

it('shares the tagged platform feed across tiles and excludes global data when membership or scope changes', async () => {
  const config = { ...fromTemplate('Overview'), tags: ['Production'] };
  const tile = config.widgets.find(({ settings }) => settings.kind === 'Platforms')!;
  config.widgets.push({ ...tile, id: 'second', title: 'Second platform list', layout: { x: 6, y: 8, w: 6, h: 4 } });
  const platform = (id: string, name: string, containerCount: number) => ({
    id,
    name,
    status: 'Online',
    platformDescriptor: { $type: 'Docker', containerCount },
  });
  const global = platform('all', 'Outside this filter', 80);
  const prod = platform('prod', 'Production host', 7);
  const reads = vi.fn(({ request }: { request: Request }) => {
    const tags = new URL(request.url).searchParams.getAll('tags');
    return HttpResponse.json({ platforms: tags.length ? (tags[0] === 'production' ? [prod] : []) : [global, prod] });
  });
  let ownerTagged = true;
  let prodAlerts = [{ id: 'scoped', message: 'Scoped alert', severity: 'Warning', status: 'Active' }];
  const recent = vi.fn(({ request }: { request: Request }) => {
    const params = new URL(request.url).searchParams;
    const backup = params.has('eventType');
    return HttpResponse.json(
      ownerTagged && params.get('tags') === 'production'
        ? [
            {
              id: backup ? 'backup' : 'stack',
              resourceName: backup ? 'Tagged policy' : 'Tagged stack',
              eventType: backup ? 'BackupRunCompleted' : 'StackApplied',
              status: 'Success',
              createdAt: '2026-11-01T10:00:00Z',
            },
          ]
        : [],
    );
  });
  const alerts = vi.fn(({ request }: { request: Request }) => {
    const tag = new URL(request.url).searchParams.get('tags');
    const items = tag === 'production' ? prodAlerts : [];
    return HttpResponse.json({ pagedResult: { items, pageSize: 5, totalCount: tag ? items.length : 99 } });
  });
  const live = new FakeRealtimeConnection();
  vi.spyOn(transport, 'createRealtimeConnection').mockReturnValue(live);
  server.use(
    http.get('*/api/v1/application/info', () => HttpResponse.json({ realtimeTransport: 'WebSocketV1' })),
    http.get('*/api/v1/profile/preferences', () => HttpResponse.json({})),
    http.get('*/api/v1/platforms', reads),
    http.get('*/api/v1/tags/_filter', ({ request }) => {
      const tags = new URL(request.url).searchParams.getAll('tags');
      return HttpResponse.json(Object.fromEntries(tags.map((name) => [name, name.toLowerCase()])));
    }),
    http.get('*/api/v1/alertEvents', alerts),
    http.get('*/api/v1/activities/recent', recent),
  );
  const renderBoard = (tags: string[]) => (
    <AppProvider>
      <DashboardGrid configuration={{ ...config, tags }} />
    </AppProvider>
  );
  const { rerender } = renderCitadel(renderBoard(config.tags));
  const summary = () => within(screen.getByRole('region', { name: 'Resource summary' }));
  expect(await screen.findAllByRole('link', { name: prod.name })).toHaveLength(2);
  expect(screen.queryByText(global.name)).toBeNull();
  expect(summary().getByText('7')).toBeVisible();
  expect(await screen.findByRole('link', { name: 'Scoped alert' })).toBeVisible();
  expect(await screen.findByRole('link', { name: 'Tagged policy' })).toBeVisible();
  expect(await screen.findByRole('link', { name: 'Stack Applied · Tagged stack' })).toBeVisible();
  expect(summary().getByText('Open alerts').nextElementSibling).toHaveTextContent('1');
  expect(summary().queryByText('99')).toBeNull();
  prodAlerts = [];
  const outside = { id: 'outside', message: 'Outside alert', severity: 'Warning', status: 'Active' };
  act(() => {
    live.emit('AlertEventReceived', outside);
    live.emit('AlertEventsUpdated', [outside]);
  });
  await waitFor(() => expect(screen.queryByText('Scoped alert')).toBeNull());
  expect(screen.queryByText('Outside alert')).toBeNull();
  expect(summary().getByText('Open alerts').nextElementSibling).toHaveTextContent('0');
  const scope = collectionGroup('platforms', ['production']);
  await waitFor(() => expect(live.listenerCount(collectionEventKey(scope, 'PlatformUpdated'))).toBe(1));
  act(() => {
    live.emit('PlatformUpdated', global);
    live.emit(collectionEventKey(scope, 'PlatformsDeleted'), prod.id);
  });
  expect(screen.getAllByText('No visible platforms match these tags.')).toHaveLength(2);
  expect(summary().getAllByText('0')).toHaveLength(3);
  const updated = platform('match', 'New matching host', 6);
  act(() => live.emit(collectionEventKey(scope, 'PlatformUpdated'), updated));
  expect(screen.getAllByText(updated.name)).toHaveLength(2);
  expect(summary().getByText('6')).toBeVisible();
  ownerTagged = false;
  act(() => live.emit('TagCatalogChanged'));
  expect(await screen.findByText('No completed backups.')).toBeVisible();
  expect(await screen.findByText('No recent activity.')).toBeVisible();
  rerender(renderBoard(['Unassigned']));
  expect(screen.queryByText(updated.name)).toBeNull();
  await waitFor(() => expect(live.listenerCount(collectionEventKey(scope, 'PlatformUpdated'))).toBe(0));
  expect(await screen.findAllByText('No visible platforms match these tags.')).toHaveLength(2);
  expect(reads.mock.calls.map(([{ request }]) => new URL(request.url).searchParams.getAll('tags'))).toEqual([
    [],
    ['production'],
    ['unassigned'],
  ]);
  expect(await screen.findByText('No completed backups.')).toBeVisible();
  expect(screen.queryByText('Tagged policy')).toBeNull();
  expect(screen.getByText('No recent activity.')).toBeVisible();
  expect(recent.mock.calls.map(([{ request }]) => new URL(request.url).searchParams.get('tags')).sort()).toEqual([
    'production',
    'production',
    'production',
    'production',
    'unassigned',
    'unassigned',
  ]);
  expect(alerts.mock.calls.map(([{ request }]) => new URL(request.url).searchParams.get('tags'))).toEqual([
    null,
    'production',
    'production',
    'production',
    'unassigned',
  ]);
});

it('shares pinned chart history and freezes a paused board after one explicit refresh', async () => {
  const id = '10000000-0000-4000-8000-000000000001';
  const config = fromTemplate('Infrastructure and capacity');
  config.widgets = config.widgets
    .filter((w) => w.settings.kind === 'Metrics')
    .map((w) =>
      w.settings.kind === 'Metrics' ? { ...w, settings: { ...w.settings, source: { kind: 'Platform', id } } } : w,
    );
  const history = vi.fn(() => HttpResponse.json({ stats: [] }));
  const metadata = vi.fn(() =>
    HttpResponse.json([{ id, name: 'Host', status: 'Online', matchesTags: true, platformId: id }]),
  );
  server.use(
    http.get('*/api/v1/dashboards/sources', metadata),
    http.get('*/api/v1/platforms/:id/stats', history),
    http.get('*/api/v1/application/info', () => HttpResponse.json({})),
    http.get('*/api/v1/platforms', () => HttpResponse.json({ platforms: [] })),
    http.get('*/api/v1/alertEvents', () => HttpResponse.json({ pagedResult: { items: [], totalCount: 0 } })),
  );
  const board = (paused: boolean, refresh: number) => (
    <AppProvider>
      <DashboardGrid configuration={config} paused={paused} refresh={refresh} />
    </AppProvider>
  );
  const { rerender } = renderCitadel(board(false, 0));
  await waitFor(() => expect(screen.getAllByText('No measurements in this range.')).toHaveLength(3));
  expect(history).toHaveBeenCalledTimes(1);
  expect(metadata).toHaveBeenCalledTimes(1);
  rerender(board(true, 0));
  expect(screen.getAllByText('No measurements in this range.')).toHaveLength(3);
  rerender(board(true, 1));
  await waitFor(() => expect(history).toHaveBeenCalledTimes(2));
  await waitFor(() => expect(screen.getAllByText('No measurements in this range.')).toHaveLength(3));
  expect(metadata).toHaveBeenCalledTimes(2);
});

it('links to full statistics without fetching an oversized stack history', async () => {
  const id = '10000000-0000-4000-8000-000000000002';
  const config = fromTemplate('Infrastructure and capacity');
  config.widgets = config.widgets
    .filter((w) => w.settings.kind === 'Metrics')
    .slice(0, 1)
    .map((w) =>
      w.settings.kind === 'Metrics' ? { ...w, settings: { ...w.settings, source: { kind: 'Stack', id } } } : w,
    );
  const history = vi.fn(() => HttpResponse.json({ services: [] }));
  server.use(
    http.get('*/api/v1/dashboards/sources', () =>
      HttpResponse.json([
        { id, name: 'Large stack', status: 'Running', matchesTags: true, exceedsChartSeriesLimit: true },
      ]),
    ),
    http.get('*/api/v1/stacks/:id/stats', history),
    http.get('*/api/v1/application/info', () => HttpResponse.json({})),
    http.get('*/api/v1/platforms', () => HttpResponse.json({ platforms: [] })),
    http.get('*/api/v1/alertEvents', () => HttpResponse.json({ pagedResult: { items: [], totalCount: 0 } })),
  );
  renderCitadel(
    <AppProvider>
      <DashboardGrid configuration={config} />
    </AppProvider>,
  );
  expect(await screen.findByRole('link', { name: 'Full statistics' })).toHaveAttribute(
    'href',
    expect.stringContaining(id),
  );
  expect(history).not.toHaveBeenCalled();
});
