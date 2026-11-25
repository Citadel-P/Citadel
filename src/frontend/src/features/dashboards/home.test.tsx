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
  return {
    default: () => (
      <AppProvider>
        <Outlet />
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
        revision: 1,
      },
      configuration: { schemaVersion: 1, tags: [], rangeMinutes: 60, widgets: [] },
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
  expect(screen.getAllByRole('region')).toHaveLength(5);
  await user.click(screen.getByRole('button', { name: 'Dashboard options' }));
  expect(screen.getByRole('menuitem', { name: 'Dashboards' })).toHaveAttribute('href', '/dashboards');
  await user.click(screen.getByRole('menuitem', { name: 'Save a copy' }));
  expect(within(screen.getByRole('dialog')).getByLabelText('Template')).toHaveTextContent('Overview');
  await user.click(screen.getByRole('button', { name: 'Cancel' }));
  expect(create).not.toHaveBeenCalled();
  expect(board).not.toHaveBeenCalled();
  expect(writes).toEqual([]);
});

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
