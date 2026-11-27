import { screen, within } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { Suspense } from 'react';
import { Route, Routes } from 'react-router';
import type { CreateDashboard } from '@/api/generated/api.types';
import Resources from '@/pages/resource';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import Board from './board';
import { AppProvider } from '@/lib/context/app-provider';

const api = 'http://localhost/api/v1';
async function openCreator(admin = false, team = false) {
  server.use(
    http.get(api + '/application/info', () => HttpResponse.json({})),
    http.get(api + '/profile/preferences', () => HttpResponse.json({})),
    http.get(api + '/platforms', () => HttpResponse.json({ platforms: [] })),
    http.get(api + '/alertEvents', () => HttpResponse.json({ pagedResult: { items: [], pageSize: 5, totalCount: 0 } })),
    http.get(api + '/activities/recent', () => HttpResponse.json([])),
    http.get(api + '/profile', () => HttpResponse.json({ authorization: { isAdministrator: admin } })),
    http.get(api + '/license/entitlements', () =>
      HttpResponse.json({ capabilities: [{ capability: 'CustomAccessControl', enabled: team }] }),
    ),
    http.get(api + '/dashboards', () => HttpResponse.json({ dashboards: [] })),
  );
  const { user } = renderCitadel(
    <Suspense fallback={null}>
      <Routes>
        <Route path="/:type" element={<Resources />} />
        <Route
          path="/dashboards/:id"
          element={
            <AppProvider>
              <Board />
            </AppProvider>
          }
        />
      </Routes>
    </Suspense>,
    { route: '/dashboards' },
  );
  await user.click(await screen.findByRole('button', { name: 'Create dashboard' }));
  return { user, dialog: within(screen.getByRole('dialog')) };
}

it('creates a personal board from Overview and safely retries the same definition', async () => {
  const writes: CreateDashboard[] = [];
  server.use(
    http.post(api + '/dashboards', async ({ request }) => {
      const data = (await request.json()) as CreateDashboard;
      writes.push(data);
      if (writes.length === 1) return HttpResponse.error();
      return HttpResponse.json({ summary: { id: data.id } });
    }),
    http.get(api + '/dashboards/:id', ({ params }) => {
      const saved = writes.find(({ id }) => id === params.id)!;
      return HttpResponse.json({
        summary: {
          id: saved.id,
          name: saved.name,
          revision: 1,
          visibility: saved.visibility,
          canEdit: true,
          canDelete: true,
        },
        configuration: saved.configuration,
      });
    }),
  );
  const { user, dialog } = await openCreator();
  expect(dialog.getByRole('button', { name: 'Create' })).toBeDisabled();
  await user.type(dialog.getByLabelText('Name'), ' On call ');
  await user.type(dialog.getByLabelText('Description'), '**Night shift**');
  await user.click(dialog.getByLabelText('Template'));
  await user.click(screen.getByRole('option', { name: 'Overview' }));
  expect(writes).toHaveLength(0);
  expect(dialog.queryByLabelText('Who can read')).toBeNull();
  await user.click(dialog.getByRole('button', { name: 'Create' }));
  await dialog.findByRole('alert');
  await user.click(dialog.getByRole('button', { name: 'Create' }));
  await screen.findByRole('heading', { name: 'On call' });
  expect(writes[0]).toMatchObject({ name: 'On call', visibility: 'Private' });
  expect(writes[1]).toEqual(writes[0]);
  expect(writes[1].configuration.description).toBe('**Night shift**');
  expect(screen.getByText('Night shift').tagName).toBe('STRONG');
  expect(new Set(writes[0].configuration.widgets.map(({ id }) => id)).size).toBe(4);
  expect(screen.getByText('Recent activity')).toBeInTheDocument();
});

it('creates adjacent Application charts from the selected metric resource', async () => {
  const source = { kind: 'Platform', id: '10000000-0000-4000-8000-000000000001' };
  const writes: CreateDashboard[] = [];
  server.use(
    http.get(api + '/dashboards/sources', () => HttpResponse.json([{ id: source.id, name: 'Main platform' }])),
    http.post(api + '/dashboards', async ({ request }) => {
      const data = (await request.json()) as CreateDashboard;
      writes.push(data);
      return HttpResponse.json({ summary: { id: data.id } });
    }),
  );
  const { user, dialog } = await openCreator();
  await user.type(dialog.getByLabelText('Name'), 'Application board');
  await user.click(dialog.getByLabelText('Template'));
  await user.click(screen.getByRole('option', { name: 'Application' }));
  await user.click(dialog.getByRole('combobox', { name: 'Metric resource' }));
  await user.click(await screen.findByRole('option', { name: 'Main platform' }));
  await user.click(dialog.getByRole('button', { name: 'Create' }));
  await vi.waitFor(() => expect(writes).toHaveLength(1));
  const widgets = writes[0].configuration.widgets;
  expect(widgets.find(({ title }) => title === 'Deployment state')?.layout.h).toBe(5);
  expect(widgets.filter(({ settings }) => settings.kind === 'Metrics')).toMatchObject([
    { layout: { x: 0, y: 10, w: 6, h: 5 }, settings: { metric: 'Cpu', source } },
    { layout: { x: 6, y: 10, w: 6, h: 5 }, settings: { metric: 'Memory', source } },
  ]);
});

it.each([false, true])('offers restricted creation to administrators only with Team (Team=%s)', async (team) => {
  const post = vi.fn(() => HttpResponse.json({}));
  server.use(http.post(api + '/dashboards', post));
  const { user, dialog } = await openCreator(true, team);
  await user.click(await dialog.findByLabelText('Who can read'));
  expect(screen.getByRole('option', { name: 'Private' })).toHaveAttribute('aria-selected', 'true');
  expect(!!screen.queryByRole('option', { name: 'Restricted' })).toBe(team);
  await user.click(screen.getByRole('option', { name: 'Everyone in this instance' }));
  await user.click(dialog.getByRole('button', { name: 'Cancel' }));
  expect(post).not.toHaveBeenCalled();
  expect(screen.queryByRole('dialog')).toBeNull();
});
