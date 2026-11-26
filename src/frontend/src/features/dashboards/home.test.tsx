import { act, screen, waitFor, within } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { resourceQueryKey } from '@/api/dispatch';
import { createQueryClient } from '@/query-client-wrapper';
import { appRoutes } from '@/router';
import { renderCitadelRoutes } from '@/test/render-citadel';
import { server } from '@/test/server';

vi.mock('@/layout/layout', async () => {
  const { AppProvider } = await import('@/lib/context/app-provider');
  const { Outlet } = await import('react-router');
  const { LayoutProvider } = await import('@/lib/context/layout.provider');
  return {
    default: () => (
      <AppProvider>
        <LayoutProvider>
          <Outlet />
        </LayoutProvider>
      </AppProvider>
    ),
  };
});

const api = '*/api/v1';
function setup(admin = false, homeId: string | null = null) {
  const home = vi.fn(() => HttpResponse.json({ dashboardId: homeId }));
  const board = vi.fn(({ params }: { params: { id?: string | readonly string[] } }) =>
    HttpResponse.json({
      summary: {
        id: params.id,
        name: 'Board ' + params.id,
        visibility: params.id === 'mine' ? 'Private' : 'Restricted',
        canEdit: params.id === 'mine',
        canDelete: params.id === 'mine',
        revision: 1,
      },
      configuration: { schemaVersion: 2, tags: [], rangeMinutes: 60, widgets: [] },
    }),
  );
  const writes: { path: string; body: unknown }[] = [];
  server.use(
    http.get(api + '/profile/dashboard-default', home),
    http.put(api + '/:scope/dashboard-default', async ({ request }) => {
      writes.push({ path: new URL(request.url).pathname, body: await request.json() });
      return new HttpResponse(null, { status: 204 });
    }),
    http.get(api + '/application/info', () => HttpResponse.json({})),
    http.get(api + '/profile/preferences', () => HttpResponse.json({})),
    http.get(api + '/profile', () => HttpResponse.json({ authorization: { isAdministrator: admin } })),
    http.get(api + '/license/entitlements', () => HttpResponse.json({ capabilities: [] })),
    http.get(api + '/platforms', () => HttpResponse.json({ platforms: [] })),
    http.get(api + '/activities/recent', () => HttpResponse.json([])),
    http.get(api + '/alertEvents', () => HttpResponse.json({ pagedResult: { items: [], pageSize: 5, totalCount: 0 } })),
    http.get(api + '/dashboards', () => HttpResponse.json({ dashboards: [] })),
    http.get(api + '/dashboards/fleet', () =>
      HttpResponse.json({ states: [], rows: [], total: 0, alerts: 0, problems: 0, updates: 0 }),
    ),
    http.get(api + '/dashboards/:id', board),
  );
  return { home, board, writes };
}

it('opens the freshly resolved home without loading the previously cached board', async () => {
  const { board } = setup(false, 'mine');
  const queryClient = createQueryClient();
  queryClient.setQueryData(resourceQueryKey('getDashboardDefault'), { data: { dashboardId: 'old' } });
  renderCitadelRoutes(appRoutes, { queryClient });
  await screen.findByRole('heading', { name: 'Board mine' }, { timeout: 5000 });
  expect(board.mock.calls.map(([{ params }]) => params.id)).toEqual(['mine']);
});

it('shows an empty installation its built-in Overview and lets the user start a copy without writing on visit', async () => {
  const { board, writes } = setup();
  const create = vi.fn(() => HttpResponse.json({}));
  server.use(http.post(api + '/dashboards', create));
  const { user } = renderCitadelRoutes(appRoutes);
  await screen.findByRole('heading', { name: 'Overview' });
  expect(screen.getAllByRole('region')).toHaveLength(4);
  expect(await screen.findByText('No known issues or updates.')).toBeVisible();
  await user.click(screen.getByRole('button', { name: 'Dashboard options' }));
  expect(screen.getByRole('menuitem', { name: 'Dashboards' })).toHaveAttribute('href', '/dashboards');
  await user.click(screen.getByRole('menuitem', { name: 'Save a copy' }));
  expect(within(screen.getByRole('dialog')).getByLabelText('Template')).toHaveTextContent('Overview');
  await user.click(screen.getByRole('button', { name: 'Cancel' }));
  expect(create).not.toHaveBeenCalled();
  expect(board).not.toHaveBeenCalled();
  expect(writes).toEqual([]);
});

it.each(['/', '/dashboards/mine'])(
  'links to the dashboard list for an unsupported configuration at %s',
  async (route) => {
    setup(false, 'mine');
    server.use(
      http.get(api + '/dashboards/mine', () =>
        HttpResponse.json({
          summary: { id: 'mine', name: 'My board', visibility: 'Private', canEdit: true },
          configuration: { schemaVersion: -1 },
        }),
      ),
    );
    const { user, router } = renderCitadelRoutes(appRoutes, { route });
    const panel = within(await screen.findByRole('alert'));
    expect(panel.getByRole('heading', { name: 'Can’t display dashboard' })).toBeVisible();
    expect(panel.getByText('This dashboard uses an unsupported configuration.')).toBeVisible();
    await user.click(panel.getByRole('link', { name: 'View dashboards' }));
    expect(router.state.location.pathname).toBe('/dashboards');
  },
);

it('deletes the selected dashboard at its displayed revision only after confirmation', async () => {
  setup();
  const revisions: (string | null)[] = [];
  server.use(
    http.delete(api + '/dashboards/mine', ({ request }) => {
      revisions.push(new URL(request.url).searchParams.get('revision'));
      return new HttpResponse(null, { status: 204 });
    }),
  );
  const { user, router } = renderCitadelRoutes(appRoutes, { route: '/dashboards/mine' });
  const openConfirmation = async () => {
    await user.click(await screen.findByRole('button', { name: 'Dashboard options' }));
    await user.click(screen.getByRole('menuitem', { name: 'Delete dashboard' }));
    return within(await screen.findByRole('dialog', { name: 'Confirm Delete' }));
  };
  const first = await openConfirmation();
  await user.click(first.getByRole('button', { name: 'Close' }));
  expect(screen.queryByRole('dialog')).toBeNull();
  expect(revisions).toEqual([]);
  const second = await openConfirmation();
  const confirm = second.getByRole('button', { name: 'Delete' });
  expect(confirm).toBeDisabled();
  const input = second.getByRole('textbox', { name: 'Enter Board mine to confirm' });
  await user.type(input, 'Board other{Enter}');
  expect(confirm).toBeDisabled();
  expect(revisions).toEqual([]);
  await user.clear(input);
  await user.type(input, 'Board mine');
  await user.click(confirm);
  await waitFor(() => expect(router.state.location.pathname).toBe('/dashboards'));
  expect(revisions).toEqual(['1']);
});

it('scopes dashboard actions to the current page and checks every selected grant before deleting', async () => {
  setup();
  let boards = Array.from({ length: 51 }, (_, i) => ({
    id: String(i),
    name: 'Board ' + i,
    canDelete: i !== 2,
    visibility: 'Private',
    canEdit: i !== 2,
    revision: i + 1,
  }));
  const deletes: string[] = [];
  let conflict = true;
  server.use(
    http.get(api + '/dashboards', ({ request }) => {
      const params = new URL(request.url).searchParams;
      const offset = Number(params.get('offset'));
      const matches = boards.filter((board) => board.name.includes(params.get('search') ?? ''));
      return HttpResponse.json({ dashboards: matches.slice(offset, offset + Number(params.get('limit'))) });
    }),
    http.delete(api + '/dashboards/:id', ({ request, params }) => {
      deletes.push(params.id + ':' + new URL(request.url).searchParams.get('revision'));
      if (params.id === '1' && conflict) {
        conflict = false;
        return HttpResponse.json({ detail: 'Dashboard changed.' }, { status: 409 });
      }
      boards = boards.filter((b) => b.id !== params.id);
      return new HttpResponse(null, { status: 204 });
    }),
  );
  const { user } = renderCitadelRoutes(appRoutes, { route: '/dashboards' });
  const select = (n: number) => screen.getByRole('checkbox', { name: 'Select Board ' + n });
  const del = () => screen.getByRole('button', { name: 'Delete' });
  await screen.findByRole('link', { name: 'Board 0' });
  expect(screen.queryByRole('link', { name: 'Board 50' })).toBeNull();
  await user.click(select(0));
  expect(screen.getByRole('button', { name: 'Set as home' })).toBeEnabled();
  for (const name of ['Open dashboard', 'Set as home', 'Delete']) {
    expect(screen.getByRole('button', { name }).firstElementChild?.tagName).toBe('svg');
  }
  await user.click(screen.getByRole('button', { name: 'Next' }));
  await screen.findByRole('link', { name: 'Board 50' });
  expect(screen.queryByRole('button', { name: 'Delete' })).toBeNull();
  await user.click(select(50));
  const search = screen.getByRole('searchbox');
  await user.type(search, 'Board 0');
  await screen.findByRole('link', { name: 'Board 0' });
  expect(screen.queryByRole('button', { name: 'Delete' })).toBeNull();
  expect(screen.queryByRole('button', { name: 'Previous' })).toBeNull();
  await user.clear(search);
  await screen.findByRole('link', { name: 'Board 2' });
  await user.click(
    within(screen.getByRole('row', { name: /Board 2 Private/ })).getByRole('button', { name: 'Open menu' }),
  );
  expect(screen.getByRole('menuitem', { name: 'Set as home' })).toBeVisible();
  expect(screen.queryByRole('menuitem', { name: 'Delete dashboard' })).toBeNull();
  await user.keyboard('{Escape}');
  await user.click(
    within(screen.getByRole('row', { name: /Board 0 Private/ })).getByRole('button', { name: 'Open menu' }),
  );
  await user.click(screen.getByRole('menuitem', { name: 'Delete dashboard' }));
  const rowDialog = within(await screen.findByRole('dialog', { name: 'Confirm Delete' }));
  expect(rowDialog.getByRole('textbox', { name: 'Enter Board 0 to confirm' })).toBeVisible();
  expect(rowDialog.getByRole('button', { name: 'Delete' })).toBeDisabled();
  await user.keyboard('{Escape}');

  await user.click(select(0));
  await user.click(select(2));
  expect(del()).toBeDisabled();
  expect(screen.getByRole('button', { name: 'Set as home' })).toBeDisabled();
  await user.click(select(2));
  await user.click(select(1));
  await user.click(del());
  const dialog = within(screen.getByRole('dialog', { name: 'Confirm Delete' }));
  expect(dialog.getByRole('button', { name: 'Delete' })).toBeDisabled();
  expect(dialog.getAllByRole('listitem').map((item) => item.textContent)).toEqual(['Board 0', 'Board 1']);
  await user.type(dialog.getByRole('textbox', { name: 'Enter Delete to confirm' }), 'Delete');
  expect(deletes).toEqual([]);
  await user.click(dialog.getByRole('button', { name: 'Delete' }));
  expect(await dialog.findByRole('alert')).toHaveTextContent('Board 1');
  await user.click(dialog.getByRole('button', { name: 'Delete' }));
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  expect(deletes).toEqual(['0:1', '1:2', '1:2']);
  await waitFor(() => expect(screen.queryByRole('link', { name: 'Board 1' })).toBeNull());
}, 20000);

it('offers retry when resolving home fails', async () => {
  const { home } = setup(false, 'mine');
  home.mockImplementationOnce(() => HttpResponse.json({ dashboardId: null }, { status: 500 }));
  const { user } = renderCitadelRoutes(appRoutes);
  await user.click(await screen.findByRole('button', { name: 'Retry' }));
  await screen.findByRole('heading', { name: 'Board mine' }, { timeout: 5000 });
  expect(home).toHaveBeenCalledTimes(2);
});

it.each([
  { admin: false, path: '/dashboards/shared', scope: 'profile', id: 'shared' },
  { admin: true, path: '/dashboards/mine', scope: 'profile', id: 'mine' },
  { admin: true, path: '/dashboards/shared', scope: 'instance', id: 'shared' },
  { admin: true, path: '/dashboards', scope: 'instance', id: null },
  { admin: false, path: '/dashboards', scope: 'profile', id: null },
])('saves $scope home from $path (administrator=$admin)', async ({ admin, path, scope, id }) => {
  const { home, writes } = setup(admin, 'other');
  const { user } = renderCitadelRoutes(appRoutes, { route: path });
  if (id) await user.click(await screen.findByRole('button', { name: 'Dashboard options' }));
  await user.click(await screen.findByRole(id ? 'menuitem' : 'button', { name: id ? 'Set as home' : 'Reset home' }));
  const dialog = within(screen.getByRole('dialog'));
  if (scope === 'instance') await user.click(await dialog.findByRole('radio', { name: 'Instance default' }));
  else expect(dialog.queryByRole('radio', { name: 'Instance default' })).toBeNull();
  await user.click(
    dialog.getByRole('button', { name: scope === 'instance' ? 'Save instance default' : 'Save my home' }),
  );
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  expect(writes).toEqual([{ path: '/api/v1/' + scope + '/dashboard-default', body: { dashboardId: id } }]);
  expect(home).not.toHaveBeenCalled();
});

it.each(['Discard', 'Save and leave'])('keeps a dirty dashboard open until the user chooses %s', async (choice) => {
  setup();
  const save = vi.fn(async ({ request }: { request: Request }) => {
    const body = await request.json();
    return HttpResponse.json({
      summary: { id: 'mine', name: body.name, visibility: 'Private', canEdit: true, revision: 2 },
      configuration: body.configuration,
    });
  });
  server.use(http.put(api + '/dashboards/mine', save));
  const { router, user } = renderCitadelRoutes(appRoutes, { route: '/dashboards/shared' });
  await screen.findByRole('heading', { name: 'Board shared' });
  await act(() => router.navigate('/dashboards/mine'));
  await user.click(await screen.findByRole('button', { name: 'Edit dashboard' }));
  const name = await screen.findByLabelText('Dashboard name');
  await user.type(name, ' draft');

  const back = () => act(() => router.navigate(-1));
  await back();
  const dialog = () => within(screen.getByRole('dialog', { name: 'Leave dashboard editor?' }));
  expect(router.state.location.pathname).toBe('/dashboards/mine');
  await user.click(dialog().getByRole('button', { name: 'Keep editing' }));
  expect(name).toHaveValue('Board mine draft');
  expect(save).not.toHaveBeenCalled();
  await back();
  await user.click(dialog().getByRole('button', { name: choice }));
  expect(await screen.findByRole('heading', { name: 'Board shared' })).toBeVisible();
  expect(save).toHaveBeenCalledTimes(choice === 'Discard' ? 0 : 1);
});
