import { act, screen, waitFor, within } from '@testing-library/react';
import { focusManager } from '@tanstack/react-query';
import { http, HttpResponse } from 'msw';
import { Route, Routes } from 'react-router';
import { AppProvider } from '@/lib/context/app-provider';
import * as transport from '@/lib/createRealtimeConnection';
import { FakeRealtimeConnection } from '@/test/fakes/realtime';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import Board from './board';
import { DashboardGrid } from './grid';
import { makeWidget, metricSettings, supportedConfiguration, type WidgetKind } from './catalogue';
import { DashboardFleetKind, DashboardSourceKind, DashboardMetric } from '@/api/generated/api.types';
import { fromTemplate } from './templates';

it.each([
  { kind: 'ResourceSummary' },
  { kind: 'Platforms', limit: 5 },
  { kind: 'Workloads', limit: 5, resourceKind: 'Deployment' },
])('rejects the retired $kind widget', (settings) => {
  const board = fromTemplate('Overview');
  expect(supportedConfiguration(board)).toBe(true);
  expect(supportedConfiguration({ ...board, widgets: [{ ...board.widgets[0], settings }] })).toBe(false);
});

it('accepts container state counts and rejects individual container pins', () => {
  const board = fromTemplate('Blank');
  const source = { kind: 'Container', id: '91a05d4c-f022-403a-bf11-c2e93fa9c617' };
  const supports = (settings: unknown) =>
    supportedConfiguration({ ...board, widgets: [{ ...makeWidget('ResourceState'), settings }] });
  expect(supports({ kind: 'ResourceState', resourceKind: 'Container' })).toBe(true);
  expect(supports({ ...metricSettings(), source })).toBe(false);
  expect(supports({ kind: 'Watchlist', pins: [source] })).toBe(false);
});

function activityBoard() {
  const kinds: WidgetKind[] = ['OpenAlerts', 'Backups', 'RecentActivity'];
  return {
    ...fromTemplate('Blank'),
    widgets: kinds.map((kind, index) => ({ ...makeWidget(kind), layout: { x: 0, y: index * 4, w: 12, h: 4 } })),
  };
}

it('shares reads and realtime listeners across the activity and alert widgets of a private board', async () => {
  const config = activityBoard();
  const alertsTile = config.widgets.find((w) => w.settings.kind === 'OpenAlerts')!;
  config.widgets.push({
    ...alertsTile,
    title: 'Second alert list',
    id: 'second',
    layout: { x: 0, y: 20, w: 12, h: 4 },
  });
  const live = new FakeRealtimeConnection();
  vi.spyOn(transport, 'createRealtimeConnection').mockReturnValue(live);
  const platforms = vi.fn(() => HttpResponse.json({ platforms: [] }));
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

it('keeps alerts and activity scoped when tag membership or the selected scope changes', async () => {
  const config = { ...activityBoard(), tags: ['Production'] };
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
    http.get('*/api/v1/platforms', () => HttpResponse.json({ platforms: [] })),
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
  expect(await screen.findByRole('link', { name: 'Scoped alert' })).toBeVisible();
  expect(await screen.findByRole('link', { name: 'Tagged policy' })).toBeVisible();
  expect(await screen.findByRole('link', { name: 'Stack Applied · Tagged stack' })).toBeVisible();
  prodAlerts = [];
  const outside = { id: 'outside', message: 'Outside alert', severity: 'Warning', status: 'Active' };
  act(() => {
    live.emit('AlertEventReceived', outside);
    live.emit('AlertEventsUpdated', [outside]);
  });
  await waitFor(() => expect(screen.queryByText('Scoped alert')).toBeNull());
  expect(screen.queryByText('Outside alert')).toBeNull();
  ownerTagged = false;
  act(() => live.emit('TagCatalogChanged'));
  expect(await screen.findByText('No completed backups.')).toBeVisible();
  expect(await screen.findByText('No recent activity.')).toBeVisible();
  rerender(renderBoard(['Unassigned']));
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
  config.widgets.push({
    ...makeWidget('Metrics'),
    title: 'Network traffic',
    settings: metricSettings(DashboardMetric.Network),
  });
  config.widgets = config.widgets
    .filter((w) => w.settings.kind === 'Metrics')
    .map((w) =>
      w.settings.kind === 'Metrics'
        ? { ...w, settings: { ...w.settings, source: { kind: DashboardSourceKind.Platform, id } } }
        : w,
    );
  const history = vi.fn(() => HttpResponse.json({ stats: [] }));
  const metadata = vi.fn(() =>
    HttpResponse.json([{ id, name: 'Host', status: 'Online', matchesTags: true, platformId: id }]),
  );
  server.use(
    http.get('*/api/v1/dashboards/sources', metadata),
    http.get('*/api/v1/platforms/:id/stats', history),
    http.get('*/api/v1/profile/preferences', () => HttpResponse.json({})),
    http.get('*/api/v1/application/info', () => HttpResponse.json({})),
    http.get('*/api/v1/platforms', () => HttpResponse.json({ platforms: [] })),
    http.get('*/api/v1/alertEvents', () => HttpResponse.json({ pagedResult: { items: [], totalCount: 0 } })),
  );
  const settled = vi.fn();
  const board = (paused: boolean, refresh: number) => (
    <AppProvider>
      <DashboardGrid configuration={config} paused={paused} refresh={refresh} onSettled={settled} />
    </AppProvider>
  );
  const { rerender, queryClient } = renderCitadel(board(false, 0));
  await waitFor(() => expect(screen.getAllByText('No measurements in this range.')).toHaveLength(3), { timeout: 5000 });
  expect(history).toHaveBeenCalledTimes(1);
  expect(metadata).toHaveBeenCalledTimes(1);
  config.widgets = config.widgets.map((w) => ({
    ...w,
    title: w.title + ' edited',
    layout: { ...w.layout, h: w.layout.h + 1 },
  }));
  rerender(board(false, 0));
  expect(history).toHaveBeenCalledTimes(1);
  expect(metadata).toHaveBeenCalledTimes(1);
  rerender(board(true, 0));
  const unrelated = ['getPlatformStats', { id: 'other', query: { minutes: 60 } }];
  queryClient.setQueryData(unrelated, { data: { stats: [] } });
  await act(async () => {
    await queryClient.invalidateQueries({ queryKey: unrelated });
  });
  expect(screen.getAllByText('No measurements in this range.')).toHaveLength(3);
  let release!: () => void;
  const pending = queryClient.fetchQuery({
    queryKey: unrelated,
    queryFn: () =>
      new Promise((resolve) => {
        release = () => resolve({ data: { stats: [] } });
      }),
    staleTime: 0,
  });
  settled.mockClear();
  rerender(board(true, 1));
  await waitFor(() => expect(history).toHaveBeenCalledTimes(2));
  await waitFor(() => expect(screen.getAllByText('No measurements in this range.')).toHaveLength(3));
  await waitFor(() => expect(settled).toHaveBeenCalled());
  expect(queryClient.isFetching({ queryKey: unrelated })).toBe(1);
  release();
  await pending;
  expect(metadata).toHaveBeenCalledTimes(2);
  await act(async () => {
    await queryClient.invalidateQueries({ queryKey: ['getPlatformStats', { id }] });
  });
  expect(screen.queryByText('No measurements in this range.')).toBeNull();
});

it('links to full statistics without fetching an oversized stack history', async () => {
  const id = '10000000-0000-4000-8000-000000000002';
  const config = fromTemplate('Infrastructure and capacity');
  config.widgets = config.widgets
    .filter((w) => w.settings.kind === 'Metrics')
    .slice(0, 1)
    .map((w) =>
      w.settings.kind === 'Metrics'
        ? { ...w, settings: { ...w.settings, source: { kind: DashboardSourceKind.Stack, id } } }
        : w,
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

it('investigates the full Updates selection with its original scope and server-side paging', async () => {
  const board = fromTemplate('Overview');
  board.widgets = board.widgets.filter((w) => w.settings.kind === 'NeedsAttention');
  for (const w of board.widgets) {
    if (w.settings.kind === 'NeedsAttention') w.settings.kinds = [DashboardFleetKind.Stack];
  }
  const reads: URLSearchParams[] = [];
  server.use(
    http.get('*/api/v1/dashboards/fleet', ({ request }) => {
      const q = new URL(request.url).searchParams;
      reads.push(q);
      const page = Number(q.get('offset') ?? 0);
      const name = q.get('tab') === 'Updates' ? (page === 25 ? 'Last stack' : 'Update only stack') : 'Offline platform';
      return HttpResponse.json({
        rows: [
          {
            id: 'row' + page,
            kind: 'Stack',
            name,
            state: 'Healthy',
            reasons: [{ code: 'New Git commit', occurrences: 1 }],
          },
        ],
        total: 31,
        updates: 30,
        problems: 2,
        alerts: 0,
        states: [],
      });
    }),
    http.get('*/api/v1/application/info', () => HttpResponse.json({})),
    http.get('*/api/v1/profile/preferences', () => HttpResponse.json({})),
    http.get('*/api/v1/platforms', () => HttpResponse.json({ platforms: [] })),
    http.get('*/api/v1/alertEvents', () => HttpResponse.json({ pagedResult: { items: [], totalCount: 0 } })),
  );
  const { user } = renderCitadel(
    <AppProvider>
      <DashboardGrid configuration={board} />
    </AppProvider>,
  );
  await screen.findByText('Offline platform');
  await user.click(screen.getByRole('radio', { name: 'Updates 30' }));
  await screen.findByText('Update only stack');
  await user.click(screen.getByRole('button', { name: 'View all 30 resources' }));
  const sheet = within(await screen.findByRole('dialog', { name: 'Needs attention' }));
  await sheet.findByText('Update only stack');
  const next = sheet.getByRole('link', { name: /next/i });
  await user.click(next);
  expect(await sheet.findByText('Last stack')).toBeVisible();
  expect(reads.every((q) => q.get('kinds') === 'Stack')).toBe(true);
  expect(reads.filter((q) => q.get('limit') === '25').map((q) => [q.get('tab'), q.get('offset')])).toEqual([
    ['Updates', '0'],
    ['Updates', '25'],
  ]);
});

it('resolves shared watchlist pins once, preserves order, and hides unavailable sources', async () => {
  const board = fromTemplate('Blank');
  const ids = [1, 2, 3, 4].map((i) => '10000000-0000-4000-8000-00000000000' + i);
  const pins = ids.map((id) => ({ kind: DashboardSourceKind.Deployment, id }));
  board.widgets = [
    { ...makeWidget('Watchlist'), title: 'My watchlist', settings: { kind: 'Watchlist', pins } },
    {
      ...makeWidget('Watchlist'),
      title: 'Other watchlist',
      layout: { x: 6, y: 0, w: 6, h: 5 },
      settings: { kind: 'Watchlist', pins: [pins[1]] },
    },
  ];
  const metadata = vi.fn(() =>
    HttpResponse.json([
      { id: ids[1], name: 'Beta app', status: 'Healthy', matchesTags: true },
      { id: ids[0], name: 'Zulu app', status: 'Degraded', matchesTags: true, platformId: 'platform-a' },
      { id: ids[3], name: 'Hidden by filter', status: 'Failed', matchesTags: false },
    ]),
  );
  const details = vi.fn(() => HttpResponse.json({ id: ids[0], name: 'Zulu app', status: 'Degraded' }));
  server.use(
    http.get('*/api/v1/deployments/:id', details),
    http.get('*/api/v1/dashboards/sources', metadata),
    http.get('*/api/v1/application/info', () => HttpResponse.json({})),
    http.get('*/api/v1/profile/preferences', () => HttpResponse.json({})),
    http.get('*/api/v1/platforms', () => HttpResponse.json({ platforms: [] })),
    http.get('*/api/v1/alertEvents', () => HttpResponse.json({ pagedResult: { items: [], totalCount: 0 } })),
  );
  const { user } = renderCitadel(
    <AppProvider>
      <DashboardGrid configuration={board} />
    </AppProvider>,
  );
  const widget = within(await screen.findByRole('region', { name: 'My watchlist' }));
  await widget.findByText('Zulu app');
  expect(widget.getAllByRole('listitem').map((row) => row.textContent)).toEqual([
    expect.stringContaining('Zulu app'),
    expect.stringContaining('Beta app'),
    'Resource unavailable.',
    'Outside current filter.',
  ]);
  expect(screen.queryByText('Hidden by filter')).toBeNull();
  expect(metadata).toHaveBeenCalledTimes(1);
  expect(details).not.toHaveBeenCalled();
  await user.click(widget.getByRole('button', { name: 'View health for Zulu app' }));
  await waitFor(() => expect(details).toHaveBeenCalledTimes(1));
});
