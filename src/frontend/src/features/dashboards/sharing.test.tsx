import { screen, waitFor, within } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { Route, Routes } from 'react-router';
import { Suspense } from 'react';
import { resourceQueryKey } from '@/api/dispatch';
import { AccessComponents } from '@/features/access';
import Resources from '@/pages/resource';
import { createQueryClient } from '@/query-client-wrapper';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import Board from './board';

const api = 'http://localhost/api/v1';
const board = { id: 'board', name: 'On call', revision: 7, visibility: 'Restricted', canEdit: true, canDelete: true };
const privateBoard = { ...board, id: 'mine', name: 'Personal', visibility: 'Private' };
function setup(isAdministrator: boolean, team: boolean, isSharedAccount = false) {
  const getBoard = vi.fn(() => board);
  const list = vi.fn(() => HttpResponse.json({ dashboards: [getBoard(), privateBoard] }));
  const profile = vi.fn(() =>
    HttpResponse.json({ authorization: { isAdministrator }, authentication: { isSharedAccount } }),
  );
  server.use(
    http.get(api + '/profile', profile),
    http.get(api + '/profile/dashboard-default', () => HttpResponse.json({ dashboardId: null })),
    http.get(api + '/license/entitlements', () =>
      HttpResponse.json({ capabilities: [{ capability: 'CustomAccessControl', enabled: team }] }),
    ),
    http.get(api + '/dashboards', list),
    http.get(api + '/dashboards/:id', ({ params }) =>
      HttpResponse.json({
        summary: params.id === 'mine' ? privateBoard : getBoard(),
        configuration: { schemaVersion: 2, tags: [], rangeMinutes: 60, widgets: [] },
      }),
    ),
  );
  return { getBoard, list, profile };
}
function openPage(route: string, queryClient = createQueryClient()) {
  return renderCitadel(
    <Suspense fallback={null}>
      <Routes>
        <Route path="/:type" element={<Resources />} />
        <Route path="/dashboards/:id" element={<Board />} />
      </Routes>
    </Suspense>,
    { route, queryClient, groups: { realtimeTransport: 'Disabled' } },
  );
}

it('waits for current access before showing a previously opened board', async () => {
  const client = createQueryClient();
  client.setQueryData(resourceQueryKey('getDashboard', { id: 'board' }), {
    data: { summary: board, configuration: { schemaVersion: 2, tags: [], rangeMinutes: 60, widgets: [] } },
  });
  server.use(http.get(api + '/dashboards/board', () => HttpResponse.json({ status: 404 }, { status: 404 })));
  openPage('/dashboards/board', client);
  expect(screen.queryByRole('heading', { name: board.name })).toBeNull();
  await screen.findByRole('img', { name: 'Error 404' });
  expect(screen.queryByRole('button', { name: 'Sharing' })).toBeNull();
});

it('opens a board from Dashboards and confirms sharing only on the selected board', async () => {
  const { getBoard } = setup(true, false);
  const saves: unknown[] = [];
  server.use(
    http.put(api + '/dashboards/board/visibility', async ({ request }) => {
      saves.push(await request.json());
      const shared = { ...board, revision: 8, visibility: 'InstanceShared' };
      getBoard.mockReturnValue(shared);
      return HttpResponse.json(shared);
    }),
  );
  const { user } = openPage('/dashboards');
  const link = await screen.findByRole('link', { name: 'On call' });
  expect(screen.queryByRole('button', { name: 'Sharing' })).toBeNull();
  await user.click(link);
  await screen.findByRole('heading', { name: 'On call' });
  await user.click(await screen.findByRole('button', { name: 'Sharing' }));
  expect(saves).toHaveLength(0);
  await user.click(within(screen.getByRole('dialog')).getByRole('button', { name: 'Share with everyone' }));
  await screen.findByText('Everyone in this instance');
  expect(saves).toEqual([{ revision: 7, visibility: 'InstanceShared' }]);
  expect(screen.getByRole('button', { name: 'Sharing' })).toBeDisabled();
  expect(AccessComponents.Tabs.some((tab) => tab.slug === 'dashboard-sharing')).toBe(false);
});

it('loads the current board revision before another decision after a conflict', async () => {
  const { getBoard, list } = setup(true, true);
  const save = vi.fn(() => {
    getBoard.mockReturnValue({ ...board, revision: 8, visibility: 'InstanceShared' });
    return HttpResponse.json({ status: 409 }, { status: 409 });
  });
  server.use(http.put(api + '/dashboards/board/visibility', save));
  const { user } = openPage('/dashboards/board');
  await user.click(await screen.findByRole('button', { name: 'Sharing' }));
  await user.click(within(screen.getByRole('dialog')).getByRole('button', { name: 'Share with everyone' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('dashboard has changed');
  expect(screen.getByRole('button', { name: 'Share with everyone' })).toBeDisabled();
  await user.click(screen.getByRole('button', { name: 'Cancel' }));
  await screen.findByText('Everyone in this instance');
  await user.click(screen.getByRole('button', { name: 'Sharing' }));
  expect(within(screen.getByRole('dialog')).getByRole('button', { name: 'Restrict access' })).toBeEnabled();
  expect(save).toHaveBeenCalledTimes(1);
  expect(list).not.toHaveBeenCalled();
});

it.each([
  [false, 'board', 'On call'],
  [true, 'mine', 'Personal'],
] as const)('hides Sharing for admin=%s on %s', async (admin, id, name) => {
  setup(admin, true);
  const { queryClient } = openPage('/dashboards/' + id);
  await screen.findByRole('heading', { name });
  await waitFor(() => expect(queryClient.isFetching()).toBe(0));
  expect(screen.queryByRole('button', { name: 'Sharing' })).toBeNull();
});

it('groups board actions and marks the current board in the switcher', async () => {
  const { list } = setup(true, true);
  const { user } = openPage('/dashboards/board');
  const actions = await screen.findByRole('group', { name: 'Dashboard actions' });
  for (const name of ['Edit dashboard', 'Set as home', 'Delete']) {
    expect(await within(actions).findByRole('button', { name })).toBeVisible();
  }
  expect(within(actions).queryByRole('button', { name: 'Duplicate' })).toBeNull();
  expect(screen.getByRole('button', { name: 'Duplicate' })).toBeVisible();
  await user.click(screen.getByRole('button', { name: board.name }));
  const current = await screen.findByRole('link', { current: 'page' });
  expect(current).toHaveTextContent(board.name);
  await user.click(current);
  await user.click(screen.getByRole('button', { name: board.name }));
  expect(await screen.findByRole('link', { current: 'page' })).toBeVisible();
  expect(list).toHaveBeenCalledTimes(1);
});

it('keeps dashboard viewing available to a shared account and hides dashboard changes', async () => {
  const { getBoard } = setup(true, true, true);
  getBoard.mockReturnValue({ ...board, canEdit: false, canDelete: false });
  const { user, queryClient } = openPage('/dashboards');
  const link = await screen.findByRole('link', { name: 'On call' });
  await waitFor(() => expect(queryClient.isFetching()).toBe(0));
  expect(screen.queryByRole('button', { name: 'Create dashboard' })).not.toBeInTheDocument();
  expect(screen.queryByRole('button', { name: 'Reset home' })).not.toBeInTheDocument();

  await user.click(link);
  expect(await screen.findByRole('heading', { name: 'On call' })).toBeVisible();
  await waitFor(() => expect(queryClient.isFetching()).toBe(0));
  for (const name of ['Edit dashboard', 'Duplicate', 'Sharing', 'Set as home', 'Delete']) {
    expect(screen.queryByRole('button', { name })).not.toBeInTheDocument();
  }
});
